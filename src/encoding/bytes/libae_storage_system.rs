extern crate core;

use std::io::Read;
use std::fs::File;
use super::Substream;

//todo do a substream of a read(not a way to specific file) of sorts.. I know it's hard.
pub trait StorageSystem {
    /// sets the content of this entire system.
    /// The following condition should hold true
    /// let bytes = ...;
    /// __.set_content(bytes)
    /// get_content() == bytes
    fn set_content(&mut self, bytes : &[u8]);
    /// Returns the content as an immutable slice into the system.
    /// Should, but may not make sense with every implementation.
    ///   Maybe impossible due to RAM size or prohibited. Will always returns empty slice then.
    fn get_content(&self) -> &[u8];
    /// Retrieves the contents size in bytes.
    fn content_size(&self) -> i64;

    /// deletes all bytes from start(incl) to end(excl)
    /// Often done byte copying the bytes from end to len to start and truncating the storage.
    /// So it may be much quicker deleting a huge chunk than a small chunk with an end << len.
    fn delete(&mut self, start:i64, end:i64);
    ///appends provided bytes to the end of storage.
    fn append(&mut self, bytes : &[u8]);
    ///copies all bytes from stream to the end of storage until stream_length is reached.
    /// If stream ends before stream_length is reached behaviour is undefined, though storage MAY pad the remaining bytes with something.
    fn append_stream(&mut self, stream : &mut Read, stream_length:i64);

    ///returns a copy of the bytes between start(incl) and end(excl)
    /// start has to be >= 0  and end < len
    /// very large subarrays should not be read. (because memory and stuff)
    fn subarray(&mut self, start:i64, end:i64) -> Option<Vec<u8>>;

    /// Alternative to subarray for very far apart start and end.
    ///   May currently not work with every implementation.
    //todo remove the File there.
    fn substream(&self, start:i64, end:i64) -> Option<Substream<File>>;
}