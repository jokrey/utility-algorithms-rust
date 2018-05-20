extern crate core;

use std::io::Read;
use std::fs::File;
use super::Substream;
use std::error::Error;
use std;
use std::fmt;
use std::convert::From;

//todo do a substream of a read(not a way to specific "File" type) of sorts.. I know it's weirdly hard.
pub trait StorageSystem {
    /// sets the content of this entire system.
    /// The following condition should hold true
    /// let bytes = ...;
    /// __.set_content(bytes)
    /// get_content() == bytes
    fn set_content(&mut self, bytes : &[u8]) -> Result<(), StorageSystemError>;
    /// Returns the content as an immutable slice into the system.
    /// Should, but may not make sense with every implementation.
    ///   Maybe impossible due to RAM size or prohibited. Will always returns empty slice then.
    fn get_content(&mut self) -> Result<Vec<u8>, StorageSystemError>;
    /// Retrieves the contents size in bytes.
    fn content_size(&self) -> Result<i64, StorageSystemError>;

    /// deletes all bytes from start(incl) to end(excl)
    /// Often done byte copying the bytes from end to len to start and truncating the storage.
    /// So it may be much quicker deleting a huge chunk than a small chunk with an end << len.
    fn delete(&mut self, start:i64, end:i64) -> Result<(), StorageSystemError>;
    ///appends provided bytes to the end of storage.
    fn append(&mut self, bytes : &[u8]) -> Result<(), StorageSystemError>;
    ///copies all bytes from stream to the end of storage until stream_length is reached.
    /// If stream ends before stream_length is reached behaviour is undefined, though storage MAY pad the remaining bytes with something.
    fn append_stream(&mut self, stream : &mut Read, stream_length:i64) -> Result<(), StorageSystemError>;

    ///returns a copy of the bytes between start(incl) and end(excl)
    /// start has to be >= 0  and end < len
    /// very large subarrays should not be read. (because memory and stuff)
    fn subarray(&mut self, start:i64, end:i64) -> Result<Vec<u8>, StorageSystemError>;

    /// Alternative to subarray for very far apart start and end.
    ///   May currently not work with every implementation.
    //todo remove the File here.
    fn substream(&self, start:i64, end:i64) -> Result<Substream<File>, StorageSystemError>;
}



#[derive(Debug)]
pub struct StorageSystemError {
    descr:String
}

impl StorageSystemError {
    pub fn new(descr:&str) -> StorageSystemError {
        StorageSystemError {
            descr:descr.to_string()
        }
    }
    pub fn n() -> StorageSystemError {
        StorageSystemError::new("")
    }
}

impl Error for StorageSystemError {
    fn description(&self) -> &str {
        return &self.descr
    }
}

impl fmt::Display for StorageSystemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Storage System Failed({})", self.descr)
    }
}

impl From<std::io::Error> for StorageSystemError {
    fn from(io_err: std::io::Error) -> Self {
        let mut our_str = String::from("internal io error: (");
        our_str.push_str(&io_err.to_string());
        our_str.push_str(").");
        StorageSystemError::new(&our_str)
    }
}