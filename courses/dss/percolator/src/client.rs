use futures::{executor::block_on, Future};
use labrpc::*;

use crate::msg::*;
use crate::{service::{TSOClient, TransactionClient}, RUNTIME};

// BACKOFF_TIME_MS is the wait time before retrying to send the request.
// It should be exponential growth. e.g.
//|  retry time  |  backoff time  |
//|--------------|----------------|
//|      1       |       100      |
//|      2       |       200      |
//|      3       |       400      |
const BACKOFF_TIME_MS: u64 = 100;
// RETRY_TIMES is the maximum number of times a client attempts to send a request.
const RETRY_TIMES: usize = 3;

/// Client mainly has two purposes:
/// One is getting a monotonically increasing timestamp from TSO (Timestamp Oracle).
/// The other is do the transaction logic.
#[derive(Clone)]
pub struct Client {
    // Your definitions here.
    tso_client: TSOClient,
    txn_client: TransactionClient,
    txn: Option<Transaction>,
}

#[derive(Clone)]
struct Transaction {
    start_ts: u64,
    writes: Vec<Write>,
}

impl Transaction {
    fn new(start_ts: u64) -> Self {
        Self {
            start_ts,
            writes: Vec::new(),
        }
    }

    fn add_write(&mut self, write: Write) {
        self.writes.push(write);
    }
}

impl Client {
    /// Creates a new Client.
    pub fn new(tso_client: TSOClient, txn_client: TransactionClient) -> Client {
        // Your code here.
        Client {
            tso_client,
            txn_client,
            txn: None,
        }
    }

    async fn get_timestamp_async(&self) {
        let res = self.tso_client.get_timestamp(&TimestampRequest {}).await;
    }

    /// Gets a timestamp from a TSO.
    pub fn get_timestamp(&self) -> Result<u64> {
        let res = back_off(|| self.tso_client.get_timestamp(&TimestampRequest {}));
        match res {
            Ok(resp) => Ok(resp.timestamp),
            Err(e) => Err(Error::Timeout),
        }
    }

    /// Begins a new transaction.
    pub fn begin(&mut self) {
        if self.txn.is_some() {
            panic!("already exist another txn!")
        }
        let ts = self.get_timestamp().unwrap();
        self.txn = Some(Transaction::new(ts));
    }

    /// Gets the value for a given key.
    pub fn get(&self, key: Vec<u8>) -> Result<Vec<u8>> {
        // Your code here.
        let start_ts = match self.txn {
            Some(ref txn) => txn.start_ts,
            None => self.get_timestamp().unwrap(),
        };
        let mut req = GetRequest { key, start_ts };
        let res = back_off(|| self.txn_client.get(&req));
        match res {
            Ok(resp) => Ok(resp.value),
            Err(e) => Err(Error::Timeout),
        }
    }

    /// Sets keys in a buffer until commit time.
    pub fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        // Your code here.
        let txn = self.txn.as_mut().unwrap();
        txn.add_write(Write {
            key,
            value,
        });
    }

    /// Commits a transaction.
    pub fn commit(&mut self) -> Result<bool> {
        // Your code here.
        RUNTIME.block_on(self.commit_async())
    }

    async fn commit_async(&mut self) -> Result<bool> {
        let Transaction {
            start_ts,
            writes,
        } = self
            .txn
            .to_owned()
            .expect("no transaction to commit");
        if writes.is_empty() {
            return Ok(true);
        }
        let keys = writes.iter().map(|w| w.key.to_owned()).collect::<Vec<_>>();
        let primary_key = writes.get(0).unwrap().to_owned().key;
        // prewrite
        for write in writes.into_iter() {
            let mut req = PrewriteRequest {
                primary_key: primary_key.clone(),
                start_ts,
                mutations: Some(write),
            };
            let res = back_off(|| self.txn_client.prewrite(&req));
            match res {
                Ok(resp) => {
                    if !resp.success {
                        return Ok(false);
                    }
                }
                Err(e) => return Err(Error::Timeout),
            }
        }

        // commit 
        let commit_ts = self.get_timestamp().unwrap();
        for (i, key) in keys.into_iter().enumerate() {
            let mut req = CommitRequest {
                is_primary: i == 0,
                start_ts,
                commit_ts,
                key,
            };
            let res = back_off(|| self.txn_client.commit(&req));
            match res {
                Ok(resp) => {
                    if !resp.success {
                        return Ok(false);
                    }
                }
                Err(e) => return Err(Error::Timeout),
            }
        }

        self.txn.take();
        Ok(true)
    }
}

fn back_off<T, F>(action: impl Fn() -> F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        for i in 0..RETRY_TIMES {
            match RUNTIME.block_on(async { 
                action().await
            }) {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    std::thread::sleep(std::time::Duration::from_millis(
                        BACKOFF_TIME_MS * (1 << i) as u64,
                    ));
                }
            }
        }
        Err(Error::Timeout)
    }
