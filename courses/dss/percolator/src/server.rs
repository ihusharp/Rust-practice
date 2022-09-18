use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::ops::{Bound, RangeBounds};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use crate::msg::*;
use crate::service::*;
use crate::*;

// TTL is used for a lock key.
// If the key's lifetime exceeds this value, it should be cleaned up.
// Otherwise, the operation should back off.
const TTL: u64 = Duration::from_millis(100).as_nanos() as u64;

#[derive(Clone, Default)]
pub struct TimestampOracle {
    // You definitions here if needed.
    timestamp_generator: Arc<AtomicU64>,
}

#[async_trait::async_trait]
impl timestamp::Service for TimestampOracle {
    // example get_timestamp RPC handler.
    async fn get_timestamp(&self, _: TimestampRequest) -> labrpc::Result<TimestampResponse> {
        // Your code here.
        Ok(TimestampResponse {
            timestamp: self.timestamp_generator.fetch_add(1, Ordering::SeqCst),
        })
    }
}

// Key is a tuple (raw key, timestamp).
pub type Key = (Vec<u8>, u64);

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Timestamp(u64),
    Vector(Vec<u8>),
}

impl Value {
    fn as_ts(&self) -> u64 {
        match self {
            Value::Timestamp(ts) => *ts,
            _ => panic!("not a timestamp"),
        }
    }

    fn as_bytes(&self) -> &[u8] {
        match self {
            Value::Timestamp(_) => panic!(),
            Value::Vector(bytes) => bytes,
        }
    }
}

#[derive(Debug)]
pub enum Column {
    Write,
    Data,
    Lock,
}

// KvTable is used to simulate Google's Bigtable.
// It provides three columns: Write, Data, and Lock.
#[derive(Clone, Default)]
pub struct KvTable {
    write: BTreeMap<Key, Value>,
    data: BTreeMap<Key, Value>,
    lock: BTreeMap<Key, Value>,
}

impl KvTable {
    fn column_ref(&self, column: Column) -> &BTreeMap<Key, Value> {
        match column {
            Column::Write => &self.write,
            Column::Data => &self.data,
            Column::Lock => &self.lock,
        }
    }

    fn column_mut(&mut self, column: Column) -> &mut BTreeMap<Key, Value> {
        match column {
            Column::Write => &mut self.write,
            Column::Data => &mut self.data,
            Column::Lock => &mut self.lock,
        }
    }
}

impl KvTable {
    // Reads the latest key-value record from a specified column
    // in MemoryStorage with a given key and a timestamp range.
    #[inline]
    fn read(
        &self,
        key: &[u8],
        column: Column,
        ts_range: impl RangeBounds<u64>,
    ) -> Option<(&Key, &Value)> {
        // Your code here.
        let column = self.column_ref(column);
        let key_start = match ts_range.start_bound() {
            Bound::Included(ts) => Bound::Included((key.to_vec(), *ts)),
            Bound::Excluded(ts) => Bound::Excluded((key.to_vec(), *ts)),
            Bound::Unbounded => Bound::Included((key.to_vec(), 0)),
        };

        let key_end = match ts_range.end_bound() {
            Bound::Included(ts) => Bound::Included((key.to_vec(), *ts)),
            Bound::Excluded(ts) => Bound::Excluded((key.to_vec(), *ts)),
            Bound::Unbounded => Bound::Included((key.to_vec(), u64::MAX)),
        };

        column
            .range((key_start, key_end))
            .last()
            .map(|(k, v)| (k, v))
    }

    fn read_owned(
        &self,
        key: &[u8],
        column: Column,
        ts_range: impl RangeBounds<u64>,
    ) -> Option<(Key, Value)> {
        self.read(key, column, ts_range)
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
    }

    // Writes a record to a specified column in MemoryStorage.
    #[inline]
    fn write(&mut self, key: Vec<u8>, column: Column, ts: u64, value: Value) {
        // Your code here.
        let column = self.column_mut(column);
        column.insert((key, ts), value);
    }

    #[inline]
    // Erases a record from a specified column in MemoryStorage.
    fn erase(&mut self, key: Vec<u8>, column: Column, start_ts: u64) {
        // Your code here.
        let column = self.column_mut(column);
        column.remove(&(key, start_ts));
    }

    #[inline]
    fn contains_in_write_column(&self, primary: &[u8], start_ts: u64) -> Option<u64> {
        for (key, value) in self.write.range((
            Bound::Included((primary.to_vec(), 0)),
            Bound::Included((primary.to_vec(), u64::MAX)),
        )) {
            match value {
                Value::Timestamp(time) => {
                    if *time == start_ts {
                        return Some(key.1);
                    }
                }
                _ => continue,
            }
        }
        None
    }
}

// MemoryStorage is used to wrap a KvTable.
// You may need to get a snapshot from it.
#[derive(Clone, Default)]
pub struct MemoryStorage {
    data: Arc<Mutex<KvTable>>,
}

#[async_trait::async_trait]
impl transaction::Service for MemoryStorage {
    // example get RPC handler.
    async fn get(&self, req: GetRequest) -> labrpc::Result<GetResponse> {
        // Your code here.
        let GetRequest { key, start_ts } = req;

        // inspect if exist txn conflict by lock
        let data = loop {
            let mut data = self.data.lock().unwrap();
            let lock = data.read_owned(&key, Column::Lock, 0..=start_ts);
            if lock.is_none() {
                break data;
            } else {
                // Back off.
                if let Some(data) =
                    self.back_off_maybe_clean_up_lock(start_ts, key.clone(), lock, data)
                {
                    break data;
                }
            }
        };

        // read will read write fast to get start_ts and then read data with start_ts
        let value = if let Some((_, lastest_write)) = data.read(&key, Column::Write, 0..=start_ts) {
            let data_ts = lastest_write.as_ts();
            let value = data.read(&key, Column::Data, data_ts..=data_ts).unwrap().1;
            value.as_bytes().to_vec()
        } else {
            vec![]
        };

        Ok(GetResponse { value })
    }

    // example prewrite RPC handler.
    async fn prewrite(&self, req: PrewriteRequest) -> labrpc::Result<PrewriteResponse> {
        // Your code here.
        let PrewriteRequest {
            primary_key,
            start_ts,
            mutation,
        } = req;
        let Write { key, value } = mutation.unwrap();

        let success = {
            let mut data = self.data.lock().unwrap();
            // abort on writes after our start stimestamp ...
            let write = data.read(&key, Column::Write, start_ts..);
            // or locks at any timestamp.
            let lock = data.read(&key, Column::Lock, 0..);

            if lock.is_some() || write.is_some() {
                false
            } else {
                let lock = if key == primary_key {
                    Value::Timestamp(0) // any value is ok, while check it in `back_off_maybe_clean_up_lock`
                } else {
                    Value::Vector(primary_key) // The primary's location.
                };
                data.write(key.clone(), Column::Data, start_ts, Value::Vector(value));
                data.write(key, Column::Lock, start_ts, lock);
                true
            }
        };
        Ok(PrewriteResponse { success })
    }

    // example commit RPC handler.
    async fn commit(&self, req: CommitRequest) -> labrpc::Result<CommitResponse> {
        // Your code here.
        let CommitRequest {
            is_primary,
            key,
            start_ts,
            commit_ts,
        } = req;

        let success = {
            let mut data = self.data.lock().unwrap();
            // inspect lock
            let lock = data.read(&key, Column::Lock, start_ts..=start_ts);
            if is_primary {
                if lock.is_some() {
                    data.write(
                        key.clone(),
                        Column::Write,
                        commit_ts,
                        Value::Timestamp(start_ts),
                    );
                    data.erase(key.clone(), Column::Lock, start_ts);
                    true
                } else {
                    false
                }
            } else {
                assert!(lock.is_some(), "non-primary key must have a lock");
                data.write(
                    key.clone(),
                    Column::Write,
                    commit_ts,
                    Value::Timestamp(start_ts),
                );
                data.erase(key.clone(), Column::Lock, start_ts);
                true
            }
        };

        Ok(CommitResponse { success: true })
    }
}

/*
对于 percolator 这种事务模型，primary key 的提交与否便是整个事务提交与否的标志。
任何事务在读某一 key 时，如果遇到遗留的 Lock 列锁，在 sleep 超过 TTL 时间后，
可以接着获取该冲突 key1 在 lock 列 key 中的 start_ts 和 value 中存的 primary 值。
然后再去 Write 列中寻找 (primarykey，0) 和 (primarykey， u64::MAX) 范围内是否有指向 start_ts 的记录。
如果存在，则说明该事务已经提交且能够获取到 commit_ts，此时对该 key1 做 commit 处理即可，
即清理 Lock 列并在 Write 列添加对应的记录。
如果不存在，则说明该事务尚未提交，
且其他任何 rpc 再执行的时候都能够确定性的判断出该事务并未提交（即便是乱序到达的 primary commit rpc，
    其也会检测 lock 记录是否存在，只有存在时才能 commit），
此时只需要将当前 key1 的遗留 lock 清理即可。尽管也可以顺便检测清理其他的遗留 key，
但让其他的遗留 key 在需要清理时再进行清理也不影响 safety，
因而只用清理 key1 即可。在 key1 清理完之后，当前事务便可以正常读取 key 的值了。
*/
impl MemoryStorage {
    fn back_off_maybe_clean_up_lock<'a>(
        &self,
        start_ts: u64,
        key: Vec<u8>,
        lock: Option<(Key, Value)>,
        mut data: MutexGuard<'a, KvTable>,
    ) -> Option<MutexGuard<'a, KvTable>> {
        // Your code here.
        thread::sleep(Duration::from_nanos(TTL));
        // get primary
        if let Some(entry) = data.read(&key, Column::Lock, 0..=start_ts) {
            let ts = entry.0 .1;
            if let Value::Vector(primary) = entry.1 {
                // check primary's commit
                match data.contains_in_write_column(primary, ts) {
                    None => {
                        info!(
                            "Recovery rollback tx: erase key=({:?}, {}) in Column {:?}",
                            key,
                            ts,
                            Column::Lock
                        );
                        // primary not commit, clean up lock
                        data.erase(key, Column::Lock, ts);
                    }
                    // mean this txn has committed
                    Some(commit_ts) => {
                        info!("erase key=({:?}, {}) in Column {:?}", key, ts, Column::Lock);
                        data.erase(key.clone(), Column::Lock, ts);
                        info!(
                            "Recovery commit tx: write key=({:?}, {}), value={:?} to Column {:?}",
                            key,
                            commit_ts,
                            Value::Timestamp(ts),
                            Column::Write
                        );
                        data.write(key, Column::Write, commit_ts, Value::Timestamp(ts));
                    }
                }
            }
        }
        Some(data)
    }
}
