use std::cmp;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

use crate::transparent_storage::StorageSystem;
use crate::transparent_storage::StorageSystemError;
use crate::transparent_storage::Substream;

pub struct FileStorageSystem {
    file:File,  //has to be properly instantiated allowing read and write.
    file_path:String,
    copy_buf:Vec<u8>
}

impl FileStorageSystem {
    pub fn create_leave_source_intact(path:&str) -> FileStorageSystem {
        return FileStorageSystem {
            file:OpenOptions::new().create(true).read(true).write(true).open(path).expect("path could no be opened."),
            file_path:path.to_owned(),
            copy_buf:Vec::with_capacity(8192)
        }
    }
    pub fn create_leave_source_intact_with_custom_buf_size(path:&str, internal_copy_buf_size:usize) -> FileStorageSystem {
        return FileStorageSystem {
            file:OpenOptions::new().create(true).read(true).write(true).open(path).expect("path could no be opened."),
            file_path:path.to_owned(),
            copy_buf:Vec::with_capacity(internal_copy_buf_size)
        }
    }
}

impl StorageSystem for FileStorageSystem {
    fn set_content(&mut self, bytes: &[u8]) -> Result<(), StorageSystemError> {
        //the following is ugly as f*ck.
        if let Ok(_) = self.file.seek(SeekFrom::Start(0)) {
            if let Ok(bytes_written) = self.file.write(bytes) {
                if bytes_written == bytes.len() {
                    if let Ok(_) = self.file.set_len(bytes.len() as u64) {
                        return Ok(())
                    }
                }
            }
        }
        Err(StorageSystemError::n())
    }

    // returns as much of the content as possible(has to be first copied into ram). But since RAM is limited this is not entirely possible.
    //   USING THIS METHOD IS NOT RECOMMENDED
    //not really needed, and no idea how to implement(efficiently).
    fn get_content(&mut self) -> Result<Vec<u8>, StorageSystemError> {
        match self.file.metadata() {
            Ok(mdata) => {
                let buf_len = mdata.len() as usize;
                let mut buf = Vec::with_capacity(buf_len);
                unsafe {buf.set_len(buf_len)}
                self.file.seek(SeekFrom::Start(0)).expect("could not seek");
                self.file.read(&mut buf).expect("could not read");
                return Ok(buf)
            },
            Err(e) => return Err(StorageSystemError::from(e))
        }
    }

    // returns the content size as an i64. Which i realize is a bit dumb, but also you won't be storing 2^63-1 bytes, so it's fine.
    fn content_size(&self) -> Result<i64, StorageSystemError> {
        match self.file.metadata() {
            Ok(mdata) => {
                Ok(mdata.len() as i64)
            },
            Err(_) => {
                Err(StorageSystemError::n())
            },
        }
    }

    fn delete(&mut self, start: i64, end: i64) -> Result<(), StorageSystemError> {
        if start < 0 {
            return Err(StorageSystemError::new("start smaller than 0"))
        } else if end < start {
            return Err(StorageSystemError::new("end < start"))
        }
        let content_size_i64 = self.content_size()?;
        let content_size = content_size_i64 as u64;
        let start = start as u64;
        let end = cmp::min(end as u64, content_size);
        let tail_len = content_size - end;

        //todo precache that buffer? possibly don't allocate it each time.
        let buf_size = cmp::min(tail_len as usize, self.copy_buf.capacity());
        unsafe { self.copy_buf.set_len(buf_size) }
        let mut bytes_transferred_counter: u64 = 0;
        while bytes_transferred_counter < tail_len {
            let read_pos = end + bytes_transferred_counter;
            let write_pos = start + bytes_transferred_counter;
            self.file.seek(SeekFrom::Start(read_pos))?;
            let bytes_read = self.file.read(&mut self.copy_buf)?;
            if bytes_read <= 0 {
                break
            } else {
                self.file.seek(SeekFrom::Start(write_pos))?;
                let bytes_written = self.file.write(&self.copy_buf[..bytes_read])?;
                if bytes_written != bytes_read {
                    return Err(StorageSystemError::new("could not entirely copy buffer"))
                }
            }
            bytes_transferred_counter += self.copy_buf.len() as u64;
        }
        self.file.set_len(start + tail_len)?;
        Ok(())
    }

    fn append(&mut self, bytes: &[u8]) -> Result<(), StorageSystemError> {
        let content_size = self.content_size()?;
        let cont_length = content_size as u64;
        let new_write_pos = self.file.seek(SeekFrom::Start(cont_length))?;
        if new_write_pos!=cont_length {
            return Err(StorageSystemError::new("seeked to wrong position - seek failed"))
        }
        self.file.write_all(bytes)?;
        Ok(())

//        self.file.write_at(bytes, self.content_size());  apparently platform dependent for some bloody reason
    }

    fn append_stream(&mut self, stream: &mut dyn Read, stream_length: i64) -> Result<(), StorageSystemError> {
        println!("file storage system append_stream");
        let content_size = self.content_size()?;
        let cont_length = content_size as u64;
        let new_write_pos = self.file.seek(SeekFrom::Start(cont_length))?;
        if new_write_pos!=cont_length {
            return Err(StorageSystemError::new("seeked to wrong position - seek failed"))
        }

        let mut bytes_transferred_counter = 0;

        let buf_size = cmp::min(stream_length as usize, self.copy_buf.capacity());
        unsafe {self.copy_buf.set_len(buf_size)}

        while bytes_transferred_counter < stream_length {
            let bytes_read = stream.read(&mut self.copy_buf)?;
            let bytes_written = self.file.write(&self.copy_buf[..bytes_read])?;

            if bytes_written != bytes_read {
                return Err(StorageSystemError::new("Could not write correct amount of bytes"))
            }

            bytes_transferred_counter += self.copy_buf.len() as i64;
        }

        Ok(())
    }

    fn subarray(&mut self, start: i64, end: i64) ->  Result<Vec<u8>, StorageSystemError> {
        if start > end {
            Err(StorageSystemError::new("start index greater than end index. That doesn't make much sense to this code"))
        } else {
            let subvec_len = (end-start) as usize;
            let mut subvec = Vec::with_capacity(subvec_len);
            unsafe {subvec.set_len(subvec_len)}

            self.file.seek(SeekFrom::Start(start as u64))?;
            self.file.read(&mut subvec)?;

            return Ok(subvec);
//            let start:usize = start as usize;
//            let end:usize = cmp::min(end, self.content_size()) as usize;
//            Some(Vec::from_iter(self.data[start..end].iter().cloned()))
        }
    }
    fn substream(&self, start: i64, end: i64) -> Result<Substream<File>, StorageSystemError> {
        if start > end {
            Err(StorageSystemError::new("start index greater than end index. That doesn't make much sense to this code"))
        } else {
            let path = &self.file_path;
            let orig = OpenOptions::new().read(true).open(path)?;
            Ok(Substream::new(orig, start as u64, end as u64))
        }
    }
}