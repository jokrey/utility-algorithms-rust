use std::io::Read;
use std::cmp;
use super::libae_storage_system::StorageSystem;
use super::libae_storage_system::StorageSystemError;
use std::fs::File;
use super::Substream;
use core::ptr;
use core::iter::FromIterator;

pub struct VecStorageSystem {
    data:Vec<u8>
}

impl VecStorageSystem {
    // obviously has to exist, but makes very little sense for most contexts.
    //   (because it would mean that you'd not want to add anything, but also have nothing to decode)
    pub fn new_empty() -> VecStorageSystem {
        return VecStorageSystem {
            data: vec![0; 0],
        }
    }
    // should be used if element will be added shortly. Allocates memory the size of cap.
    //  makes encoding less costly for the first cap elements. Will not shrink on it's own.
    pub fn new_with_prealloc_cap(cap:usize) -> VecStorageSystem {
        return VecStorageSystem {
            data: Vec::with_capacity(cap),
        }
    }

    fn set_content(&mut self, bytes: &[u8], new_cap: usize) -> Result<(), StorageSystemError> {
        self.data.clear();
        if bytes.len() > new_cap {
            self.set_content(bytes, bytes.len())
        } else {
            self.data.reserve_exact(new_cap);   //allocates the perfect amount, because self.data.len()==0
            self.append(bytes)?;                           //fits because bytes.len() < new_cap
            if self.data.capacity() > new_cap {           //if capacity has been way larger before, then this might make sense.
                self.data.shrink_to_fit();                //because self.data.clear() + self.data.reserve_exact() don't actually shrink internal capacity
            }
            Ok(())
        }
    }
}

impl StorageSystem for VecStorageSystem {
    //as it sets the entire content
    //  (which is typically used at decoding time, but not when more content will be encoded)
    //   we also set the capacity according to the length. Meaning it is memory efficient, but adding more elements may be costly.
    //      to reset the capacity yourself(or not set it) use the method: set_content(&mut self, bytes: &[u8], new_cap: usize)
    fn set_content(&mut self, bytes: &[u8]) -> Result<(), StorageSystemError> {
        self.set_content(bytes, bytes.len())
    }

    // returns the content at that point in time as a slice of the internal vector.
    //   only actually useful if the content is then stored or send afterwards.
    //   though it can be kept for a long time if it is turned into a Vec using: Vec::from(get_content
    //     but that is slow, inefficient and not recommended.
    fn get_content(&mut self) -> Result<Vec<u8>, StorageSystemError> {
        Ok(self.data.to_owned())
    }

    // returns the content size as an i64. Which i realize is a bit dumb, but also you won't be storing 2^63-1 bytes, so it's fine.
    fn content_size(&self) -> Result<i64, StorageSystemError> {
        Ok(self.data.len() as i64)
    }

    fn delete(&mut self, start: i64, end: i64) -> Result<(), StorageSystemError>  {
        if start < 0 {
            panic!("start smaller than 0");
        } else if end < start {
            panic!("end < start");
        }
        let start = start as usize;
        let end = end as usize;
        //unsafe block required, because it is MUCH(*1000) faster than:
            //self.data.drain(start..end);
            //not entirely sure, why. May have something to do with iter, and overhead.
            //but essentially it is the same copy code.
        unsafe {
            let tail_len= self.data.len() - end;

            let src = self.data.as_ptr().offset(end as isize);
            let dst = self.data.as_mut_ptr().offset(start as isize);
            ptr::copy(src, dst, tail_len);
            self.data.set_len(start + tail_len);
        }
        Ok(())
    }

    fn append(&mut self, bytes: &[u8]) -> Result<(), StorageSystemError>  {
        self.data.extend_from_slice(bytes);
        Ok(())
    }

    fn append_stream(&mut self, stream: &mut Read, stream_length: i64) -> Result<(), StorageSystemError> {
        let mut buf = vec![0u8; stream_length as usize];
        match stream.read_exact(&mut buf) {
            _ => {} //if it goes right, cool. If it doesn't the rest of the buffer is filled with 0's which is also fine.
        }
//        self.data.reserve_exact(stream_length); //done internally in append
        self.data.append(&mut buf);
        Ok(())
    }

    fn subarray(&mut self, start: i64, end: i64) ->  Result<Vec<u8>, StorageSystemError> {
        if start > end {
            Err(StorageSystemError::new("start index greater than end index. That doesn't make much sense to this code"))
        } else {
            let content_size = self.content_size()?;
            let start: usize = start as usize;
            let end: usize = cmp::min(end, content_size) as usize;
            Ok(Vec::from_iter(self.data[start..end].iter().cloned()))
        }
    }


    //not really needed, and no idea how to implement.
    fn substream(&self, _start: i64, _end: i64) -> Result<Substream<File>, StorageSystemError> {
        //todo, also requires different trait type
        unimplemented!()
    }
}