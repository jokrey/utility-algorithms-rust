use std::cmp;
use std::io::Read;
use super::libae_storage_system::StorageSystem;
use std::io::Write;
use std::io::Seek;
use std::io::SeekFrom;
use std::fs::{File, OpenOptions};
use super::Substream;

pub struct FileStorageSystem {
    file:File,  //has to be properly instantiated allowing read and write.
    file_path:String,
    copy_buf_size:usize
}

impl FileStorageSystem {
    pub fn create_leave_source_intact(path:&str) -> FileStorageSystem {
        return FileStorageSystem {
            file:OpenOptions::new().create(true).read(true).write(true).open(path).expect("path could no be opened."),
            file_path:path.to_owned(),
            copy_buf_size:8192
        }
    }
    pub fn create_leave_source_intact_with_custom_buf_size(path:&str, internal_copy_buf_size:usize) -> FileStorageSystem {
        return FileStorageSystem {
            file:OpenOptions::new().create(true).read(true).write(true).open(path).expect("path could no be opened."),
            file_path:path.to_owned(),
            copy_buf_size:internal_copy_buf_size
        }
    }
}

//todo switch from panic error handling to recoverable error handling.
impl StorageSystem for FileStorageSystem {
    fn set_content(&mut self, bytes: &[u8]) {
        //the following is ugly as f*ck.
        if let Ok(_) = self.file.seek(SeekFrom::Start(0)) {
            if let Ok(bytes_written) = self.file.write(bytes) {
                if bytes_written == bytes.len() {
                    if let Ok(_) = self.file.set_len(bytes.len() as u64) {
                        return;
                    }
                }
            }
        }
        panic!("set content failed in one operation.");
    }

    // returns as much of the content as possible(has to be first copied into ram). But since RAM is limited this is not entirely possible.
    //   USING THIS METHOD IS NOT RECOMMENDED
    //not really needed, and no idea how to implement(efficiently).
    fn get_content(&self) -> &[u8] {
        &[]
    }

    // returns the content size as an i64. Which i realize is a bit dumb, but also you won't be storing 2^63-1 bytes, so it's fine.
    fn content_size(&self) -> i64 {
        match self.file.metadata() {
            Ok(mdata) => {mdata.len() as i64},
            Err(_) => {-1},
        }
    }

    fn delete(&mut self, start: i64, end: i64) {
        if start < 0 {
            panic!("start smaller than 0");
        } else if end < start {
            panic!("end < start");
        }
        let content_size = self.content_size() as u64;
        let start = start as u64;
        let end = cmp::min(end as u64, content_size);
        let tail_len= content_size - end;

        //todo precache that buffer? don't allocate it each time.
        let buf_size = cmp::min(tail_len as usize, self.copy_buf_size);
        let mut buf = Vec::with_capacity(buf_size);
        unsafe {buf.set_len(buf_size)}
        let mut bytes_transferred_counter:u64 = 0;
        while bytes_transferred_counter < tail_len {
            let read_pos = end+bytes_transferred_counter;
            let write_pos = start+bytes_transferred_counter;
            if let Ok(_) = self.file.seek(SeekFrom::Start(read_pos)) {
                match self.file.read(&mut buf) {
                    Ok(bytes_read) => {
                        if bytes_read <= 0 {
                            break
                        } else {
//                        if bytes_read < buf.len() {
//                            buf.truncate(bytes_read);  //not needed, done by slicing the vec
//                        }
                            if let Ok(_) = self.file.seek(SeekFrom::Start(write_pos)) {
                                match self.file.write(&buf[..bytes_read]) {
                                    Ok(bytes_written) => {
                                        if bytes_written != bytes_read {
                                            panic!("could not entirely copy buffer.");
                                        }
                                    },
                                    Err(_) => {
                                        panic!("Error writing read bytes");
                                    },
                                }
                            } else {
                                panic!("seek to write pos failed.");
                            }
                        }
                    },
                    Err(_) => {
                        break;
                    },
                }
            } else {
                panic!("seek to read pos failed.");
            }
            bytes_transferred_counter+=buf.len() as u64;
        }
        if let Err(_) = self.file.set_len(start + tail_len) {
            panic!("set len failed.");
        }
    }

    fn append(&mut self, bytes: &[u8]) {
        let cont_length = self.content_size() as u64;
        if let Ok(new_write_pos) = self.file.seek(SeekFrom::Start(cont_length)) {
            if new_write_pos!=cont_length {
                panic!("Seek was wrong.");
            }
        }
        if let Err(_) = self.file.write_all(bytes) {
            panic!("Could not properly write all bytes");
        }
//        self.file.write_at(bytes, self.content_size());  apparently platform dependent
    }

    fn append_stream(&mut self, stream: &mut Read, stream_length: i64) {
        let cont_length = self.content_size() as u64;
        if let Ok(new_write_pos) = self.file.seek(SeekFrom::Start(cont_length)) {
            if new_write_pos!=cont_length {
                panic!("Seek was wrong.");
            }
        }

        let mut bytes_transferred_counter = 0;

        let buf_size = cmp::min(stream_length as usize, self.copy_buf_size);
        let mut buf = Vec::with_capacity(buf_size);
        unsafe {buf.set_len(buf_size)}

        while bytes_transferred_counter < stream_length {
            match stream.read(&mut buf) {
                Ok(bytes_read) => {
//                    if bytes_read < buf.len() {
//                        buf.truncate(bytes_read);  //not needed. done by slicing the vec
//                    }
                    match self.file.write(&buf[..bytes_read]) {
                        Ok(bytes_written) => {
                            if bytes_written != bytes_read {
                                panic!("Could not write correct amount of bytes.");
                            }
                        },
                        Err(_) => {
                            panic!("Error writing read bytes");
                        },
                    }
                },
                Err(_) => {
                    panic!("error whilst reading your Read stream");
                },
            }
            bytes_transferred_counter+=buf.len() as i64;
        }
    }

    fn subarray(&mut self, start: i64, end: i64) ->  Option<Vec<u8>> {
        if start > end {
            None
        } else {//todo error handling
            let subvec_len = (end-start) as usize;
            let mut subvec = Vec::with_capacity(subvec_len);
            unsafe {subvec.set_len(subvec_len)}

            self.file.seek(SeekFrom::Start(start as u64)).expect("could not seek");
            self.file.read(&mut subvec).expect("could not read");

            return Some(subvec);
//            let start:usize = start as usize;
//            let end:usize = cmp::min(end, self.content_size()) as usize;
//            Some(Vec::from_iter(self.data[start..end].iter().cloned()))
        }
    }
    fn substream(&self, start: i64, end: i64) -> Option<Substream<File>> {
        if start > end {
            None
        } else {
            let path = &self.file_path;
            let orig = OpenOptions::new().read(true).open(path).expect("copy of original stream could no be attained");
            Some(Substream::new(orig, start as u64, end as u64))
        }
    }
}