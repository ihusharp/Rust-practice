use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Seek, Write},
    ops::Range,
    path::{Path, PathBuf}, sync::{Arc, atomic::{AtomicU64, Ordering}, Mutex}, cell::RefCell,
};

use crossbeam_skiplist::SkipMap;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use log::error;

use crate::{KvsEngine, KvsError, Result};

const COMPACTION_THRESHOLD: u64 = 1024 * 1024;

/// The `KvStore` stores string key/value pairs.
///
/// Key/value pairs are stored in a `HashMap` in memory and persisted to disk by add to log.
///
/// ```rust
/// use kvs::{KvStore, Result};
/// fn try_main() -> Result<()> {
///     use std::env::current_dir;
///     let mut store = KvStore::open(current_dir()?)?;
///     store.set("key".to_owned(), "value".to_owned())?;
///     let val = store.get("key".to_owned())?;
///     assert_eq!(val, Some("value".to_owned()));
///     Ok(())
/// }
/// ```
///
#[derive(Clone)]
pub struct KvStore {
    // writer of the current log.
    writer: Arc<Mutex<KvStoreWriter>>,
    reader: KvStoreReader,
    // map generation number to the file reader
    index: Arc<SkipMap<String, CommandPos>>,
}

struct KvStoreWriter {
    path: Arc<PathBuf>,
    reader: KvStoreReader,
    writer: BufWriterWithPos<File>,
    index: Arc<SkipMap<String, CommandPos>>,
    current_gen: u64,
    // the number of bytes representing "stale" commands that could be
    // deleted during a compaction.
    uncompacted: u64,
}

impl KvStoreWriter {
    /// Sets the value of a string key to a string.
    ///
    /// If the key already exists, the previous value will be overwritten.
    fn set(&mut self, key: String, value: String) -> Result<()> {
        let cmd = Command::set(key, value);
        let pos = self.writer.pos;
        serde_json::to_writer(&mut self.writer, &cmd)?;
        self.writer.flush()?;
        if let Command::Set { key, .. } = cmd {
            if let Some(old_cmd) = self.index.get(key.as_str()) {
                self.uncompacted += old_cmd.value().len;
            }
            self.index.insert(
                key, ( self.current_gen, pos..self.writer.pos ).into());
        }
        if self.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
        }
        Ok(())
    }

    /// Remove a given key.
    fn remove(&mut self, key: String) -> Result<()> {
        if self.index.contains_key(&key) {
            let cmd = Command::remove(key);
            serde_json::to_writer(&mut self.writer, &cmd)?;
            self.writer.flush()?;
            if let Command::Remove { key } = cmd {
                self.index.remove(&key);
            }
            Ok(())
        } else {
            Err(KvsError::KeyNotFound)?
        }
    }

    /// Clears stale entries in the log.
    pub fn compact(&mut self) -> Result<()> {
        // increase current gen by 2. current_gen + 1 is for the compaction file.
        let compaction_gen = self.current_gen + 1;
        self.current_gen += 2;
        self.writer = new_log_file(&self.path, self.current_gen)?;

        let mut compaction_writer = new_log_file(&self.path, compaction_gen)?;
        let mut new_pos = 0; // pos in the new log file.
        for entry in &mut self.index.iter() {
            let len = self.reader.read_(*entry.value(), |mut entry_reader| {
                Ok(io::copy(&mut entry_reader, &mut compaction_writer)?)
            })?;
            self.index.insert(
                entry.key().clone(), 
                (compaction_gen, new_pos..new_pos+len).into());
            new_pos += len;
        }
        compaction_writer.flush()?;

        self.reader.safe_point.store(compaction_gen, Ordering::SeqCst);
        self.reader.clear_stale_handles();
        // remove stale log files.
        // Note that actually these files are not deleted immediately because `KvStoreReader`s
        // still keep open file handles. When `KvStoreReader` is used next time, it will clear
        // its stale file handles. On Unix, the files will be deleted after all the handles
        // are closed. On Windows, the deletions below will fail and stale files are expected
        // to be deleted in the next compaction.
        let stale_gens = sorted_gen_list(&self.path)?
            .into_iter()
            .filter(|gen| *gen < compaction_gen);
        for stale_gen in stale_gens {
            let file_path = log_path(&self.path, stale_gen);
            if let Err(e) = fs::remove_file(log_path(&self.path, stale_gen)) {
                error!("{:?} cannot be deleted: {}", file_path, e);
            }
        }

        self.uncompacted = 0;

        Ok(())
    }
}


/// A single thread reader.
///
/// Each `KvStore` instance has its own `KvStoreReader` and
/// `KvStoreReader`s open the same files separately. So the user
/// can read concurrently through multiple `KvStore`s in different
/// threads.
struct KvStoreReader {
    path: Arc<PathBuf>,
    // generation of the latest compaction file.
    safe_point: Arc<AtomicU64>,
    readers: RefCell<BTreeMap<u64, BufReaderWithPos<File>>>,
}

impl KvStoreReader {
    /// Close file handles with generation number less than safe_point.
    ///
    /// `safe_point` is updated to the latest compaction gen after a compaction finishes.
    /// The compaction generation contains the sum of all operations before it and the
    /// in-memory index contains no entries with generation number less than safe_point.
    /// So we can safely close those file handles and the stale files can be deleted.
    fn clear_stale_handles(&self) {
        let mut readers = self.readers.borrow_mut();
        while !readers.is_empty() {
            let first_gen = *readers.keys().next().unwrap();
            if first_gen >= self.safe_point.load(Ordering::SeqCst) {
                break;
            }
            readers.remove(&first_gen);
        }
    }
    
    /// Gets the string value of a given string key.
    ///
    /// Returns `None` if the given key does not exist.
    fn read_<F, R>(&self, cmd_pos: CommandPos, f: F) -> Result<R>
    where 
        F: FnOnce(io::Take<&mut BufReaderWithPos<File>>) -> Result<R>,
    {
        self.clear_stale_handles();

        let mut readers = self.readers.borrow_mut();
        if !readers.contains_key(&cmd_pos.gen) {
            let reader = BufReaderWithPos::new(
                File::open(log_path(&self.path, cmd_pos.gen))?)?;
            readers.insert(cmd_pos.gen, reader);
        }
        let reader = readers.get_mut(&cmd_pos.gen).unwrap();
        reader.seek(io::SeekFrom::Start(cmd_pos.pos))?;
        let cmd_reader = reader.take(cmd_pos.len);
        f(cmd_reader)
    }

    fn get(&self, cmd_pos: CommandPos) -> Result<Command> {
        self.read_(cmd_pos, |mut cmd_reader| {
            Ok(serde_json::from_reader(&mut cmd_reader)?)
        })
    }

    
}

impl Clone for KvStoreReader {
    fn clone(&self) -> Self {
        KvStoreReader {
            path: Arc::clone(&self.path),
            safe_point: Arc::clone(&self.safe_point),
            readers: RefCell::new(BTreeMap::new()),
        }
    }
}

impl KvStore {
    /// Open the KvStore at a given path. Return the KvStore.
    /// 
    /// This will create a new directory if the given one does not exist.
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let path = Arc::new(path.into());

        let mut index = Arc::new(SkipMap::new());
        let mut readers = BTreeMap::new();
        let mut uncompacted = 0;

        let gen_list = sorted_gen_list(&path)?;
        for &gen in &gen_list {
            let mut reader = BufReaderWithPos::new(File::open(log_path(&path, gen))?)?;
            uncompacted += load(gen, &mut reader, &mut index)?;
            readers.insert(gen, reader);
        }
        let current_gen = gen_list.last().unwrap_or(&0) + 1;
        
        let reader = KvStoreReader {
            path: Arc::clone(&path),
            safe_point: Arc::new(AtomicU64::new(0)),
            readers: RefCell::new(readers),
        };
        let writer = KvStoreWriter {
            path: Arc::clone(&path),
            reader: reader.clone(),
            writer: BufWriterWithPos::new(File::create(log_path(&path, current_gen))?)?,
            index: Arc::clone(&index),
            current_gen,
            uncompacted,
        };

        Ok(KvStore {
            reader,
            writer: Arc::new(Mutex::new(writer)),
            index,
        })
    }
}


impl KvsEngine for KvStore {
    /// Sets the value of a string key to a string.
    ///
    /// If the key already exists, the previous value will be overwritten.
    fn set(&self, key: String, value: String) -> Result<()> {
        self.writer.lock().unwrap().set(key, value)
    }

    /// Gets the string value of a given string key.
    ///
    /// Returns `None` if the given key does not exist.
    fn get(&self, key: String) -> Result<Option<String>> {
        if let Some(cmd_pos) = self.index.get(&key) {
            if let Command::Set { key: _, value } = self.reader.get(*cmd_pos.value())? {
                Ok(Some(value))
            } else {
                Err(KvsError::NotValidType)?
            }
        } else {
            Ok(None)
        }
    }

    /// Remove a given key.
    fn remove(&self, key: String) -> Result<()> {
        self.writer.lock().unwrap().remove(key)
    }
}

/// Create a new log file with given generation number and add the reader to the readers map.
///
/// Returns the writer to the log.
fn new_log_file(
    path: &Path,
    gen: u64,
) -> Result<BufWriterWithPos<File>> {
    let path = log_path(&path, gen);
    let writer = BufWriterWithPos::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&path)?,
    )?;
    Ok(writer)
}

/// Returns sorted generation numbers in the given directory.
fn sorted_gen_list(path: &Path) -> Result<Vec<u64>> {
    let mut gen_list: Vec<u64> = fs::read_dir(&path)?
        .flat_map(|res| -> Result<_> { Ok(res?.path()) })
        .filter(|path| path.is_file() && path.extension() == Some("log".as_ref()))
        .flat_map(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(|s| s.trim_end_matches(".log"))
                .map(str::parse::<u64>)
        })
        .flatten()
        .collect();
    gen_list.sort();
    Ok(gen_list)
}

/// Load the whole log file and store value locations in the index map.
///
/// Returns how many bytes can be saved after a compaction.
fn load(
    gen: u64,
    reader: &mut BufReaderWithPos<File>,
    index: &SkipMap<String, CommandPos>,
) -> Result<u64> {
    let mut pos = 0;

    let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();
    while let Some(cmd) = stream.next() {
        let new_pos = stream.byte_offset() as u64;
        let cmd = cmd?;
        match cmd {
            Command::Set { key, .. } => {
                index.insert(key, (gen, pos..new_pos).into());
                pos = new_pos;
            }
            Command::Remove { key } => {
                index.remove(&key);
            }
        }
    }
    Ok(pos)
}

fn log_path(dir: &Path, gen: u64) -> PathBuf {
    dir.join(format!("{}.log", gen))
}

/// Struct representing a command.
#[derive(Serialize, Deserialize, Debug)]
enum Command {
    Set { key: String, value: String },
    Remove { key: String },
}

impl Command {
    pub fn set(key: String, value: String) -> Command {
        Command::Set { key, value }
    }
    pub fn remove(key: String) -> Command {
        Command::Remove { key }
    }
}

struct BufReaderWithPos<R: Read + Seek> {
    reader: BufReader<R>,
    pos: u64,
}

impl<R: Read + Seek> BufReaderWithPos<R> {
    fn new(mut inner: R) -> Result<Self> {
        let pos = inner.seek(io::SeekFrom::Current(0))?;
        Ok(BufReaderWithPos {
            reader: BufReader::new(inner),
            pos: pos,
        })
    }
}

impl<R: Read + Seek> Read for BufReaderWithPos<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.reader.read(buf)?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl<R: Read + Seek> Seek for BufReaderWithPos<R> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let n = self.reader.seek(pos)?;
        self.pos += n as u64;
        Ok(n)
    }
}

struct BufWriterWithPos<W: Write + Seek> {
    writer: BufWriter<W>,
    pos: u64,
}

impl BufWriterWithPos<File> {
    fn new(mut file: File) -> Result<Self> {
        let pos = file.seek(io::SeekFrom::Current(0))?;
        Ok(BufWriterWithPos {
            writer: BufWriter::new(file),
            pos,
        })
    }
}

impl<W: Write + Seek> Write for BufWriterWithPos<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.pos += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// Represents the position and length of a json-serialized command in the log.
#[derive(Clone, Copy)]
struct CommandPos {
    gen: u64,
    pos: u64,
    len: u64,
}

impl From<(u64, Range<u64>)> for CommandPos {
    fn from((gen, range): (u64, Range<u64>)) -> Self {
        CommandPos {
            gen,
            pos: range.start,
            len: range.end - range.start,
        }
    }
}
