///:author jokrey

use std::io::Read;
use std::cmp;
use std::io::Seek;
use std::io::SeekFrom;

pub mod libae;
pub mod ubae;
pub mod libae_storage_system;
pub mod vec_storage_system;
pub mod file_storage_system;
pub mod ubae_directory_encoder;
pub mod remote;

#[cfg(test)]
mod tests;


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