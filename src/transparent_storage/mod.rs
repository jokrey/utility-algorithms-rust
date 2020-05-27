extern crate core;

pub mod bytes;

use std::io::Read;
use std::fs::File;
use std::error::Error;
use std;
use std::fmt;
use std::convert::From;
use std::io::SeekFrom;
use std::io::Seek;
use std::cmp;

//todo do a substream of a read(not a way to specific "File" type) of sorts.. Looks ugly with generics
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






#[derive(Debug)]
pub struct Substream<R:Read> {
    orig_file:R,
    cur_pos:u64,
    end_pos:u64
}
impl<R:Read + Seek> Substream<R> {
    pub fn new(mut orig:R, start:u64, end:u64) -> Substream<R> {
        orig.seek(SeekFrom::Start(start)).expect("Seeking to start position failed"); //just has to be done once, after that it automatically seeks with reading.
        Substream {
            orig_file:orig,
            cur_pos:start,
            end_pos:end
        }
    }
}
impl <R:Read> Substream<R> {
    pub fn new_from_start(orig:R, end:u64) -> Substream<R> {
        Substream {
            orig_file:orig,
            cur_pos:0,
            end_pos:end
        }
    }
}
impl<R:Read> Read for Substream<R> {
    fn read(&mut self, buf: &mut [u8]) -> ::std::io::Result<usize> {
        if self.cur_pos >= self.end_pos {
            return Ok(0)
        }
        let remaining_len = self.end_pos-self.cur_pos;
        match self.orig_file.read(buf) {
            Ok(bytes_read) => {
                self.cur_pos += bytes_read as u64;
                return Ok(cmp::min(remaining_len as usize, bytes_read));
            },
            Err(e) => {
                return Err(e);
            },
        }
    }
}