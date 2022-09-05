use serde::{Deserialize, Serialize};

/// Request type for kvs.
#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    /// Get the value of a given key from the server.
    Get { key: String },
    /// Set the value of a given key from the server.
    Set { key: String, value: String },
    /// Remove a given key from the server.
    Remove { key: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum GetResponse {
    Ok(Option<String>),
    Err(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SetResponse {
    Ok(()),
    Err(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RemoveResponse {
    Ok(()),
    Err(String),
}
