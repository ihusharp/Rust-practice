use sled::{Db, Tree};

use crate::{KvsEngine, Result, KvsError};

/// Wrapper of `sled::Db`
#[derive(Debug)]
pub struct SledKvStore(Db);

impl SledKvStore {
    /// Creates a `SledKvsEngine` from `sled::Db`.
    pub fn new(db: Db) -> Self {
        SledKvStore(db)
    }
}

impl KvsEngine for SledKvStore {
    fn get(&mut self, key: String) -> Result<Option<String>> {
        let tree: &Tree = &self.0;

        Ok(tree
            .get(key)?
            .map(|i_vec| AsRef::<[u8]>::as_ref(&i_vec).to_vec())
            .map(String::from_utf8)
            .transpose()?)
    }
    fn set(&mut self, key: String, value: String) -> Result<()> {
        let tree: &Tree = &self.0;

        tree.insert(key, value.into_bytes()).map(|_| ())?;
        tree.flush()?;
        Ok(())
    }
    fn remove(&mut self, key: String) -> Result<()> {
        let tree: &Tree = &self.0;

        tree.remove(key)?.ok_or(KvsError::KeyNotFound)?;
        tree.flush()?;
        Ok(())
    }
}
