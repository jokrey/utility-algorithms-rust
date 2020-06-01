extern crate byteorder;

use std::cmp;
use std::fs::File;
use std::io::Cursor;
use std::io::Read;

use transparent_storage::{StorageSystem, StorageSystemError};
use transparent_storage::bytes::vec_storage_system::VecStorageSystem;
use transparent_storage::Substream;

use self::byteorder::{BigEndian, ReadBytesExt};

pub trait LIbaeTraits {
    fn set_content(&mut self, bytes : &[u8]) -> Result<(), StorageSystemError>;
    fn get_content(&mut self) -> Result<Vec<u8>, StorageSystemError>;

    fn li_encode_single(&mut self, bytes : &[u8]) -> Result<(), StorageSystemError>;
    fn li_encode_single_stream(&mut self, stream : &mut dyn Read, stream_length:i64) -> Result<(), StorageSystemError>;

    fn li_decode_single(&mut self) -> Option<Vec<u8>>;
    fn li_decode_single_stream(&mut self) -> Option<(Substream<File>, i64)>;

    fn reset_read_pointer(&mut self);

    fn li_delete_single(&mut self) -> Option<Vec<u8>>;
    fn li_skip_single(&mut self) -> i64;

    fn li_decode_all(&mut self) -> Vec<Vec<u8>>;
}


pub struct LIbae<T:StorageSystem> {
    read_pointer: i64,
    pub storage_system:T // public so that the user can still directly access the storage system
                         // for example to get it's raw(encoded) size or content. Or if it is a File of sort to close it.
                         // All not strictly functionality of libae
}
impl<T:StorageSystem> LIbae<T> {
    pub fn new(storagesystem:T) -> LIbae<T> {
        return LIbae {
            read_pointer:0,
            storage_system: storagesystem,
        }
    }

    pub fn manually_get_read_pointer(&self) -> i64 {
        return self.read_pointer
    }
}
impl LIbae<VecStorageSystem> {
    pub fn ram() -> LIbae<VecStorageSystem> {
        return LIbae {
            read_pointer:0,
            storage_system: VecStorageSystem::new_empty(),
        }
    }
}

impl<T:StorageSystem> Iterator for LIbae<T> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        self.li_decode_single()
    }
}

impl<T:StorageSystem> LIbaeTraits for LIbae<T> {
    fn set_content(&mut self, bytes: &[u8]) -> Result<(), StorageSystemError> {
        return self.storage_system.set_content(bytes)
    }
    fn get_content(&mut self) -> Result<Vec<u8>, StorageSystemError> {
        self.storage_system.get_content()
    }

    fn li_encode_single(&mut self, bytes: &[u8]) -> Result<(), StorageSystemError> {
        match self.storage_system.append(&get_length_indicator_for(bytes.len() as i64)[..]) {
            Err(e) => {return Err(e)},
            Ok(_) => {
                return self.storage_system.append(bytes)
            },
        }
    }

    fn li_encode_single_stream(&mut self, stream: &mut dyn Read, stream_length: i64) -> Result<(), StorageSystemError> {
        println!("li_encode_single_stream");
        self.storage_system.append(&get_length_indicator_for(stream_length)[..])?;
        println!("aft append li");
        self.storage_system.append_stream(stream, stream_length)
    }

    fn reset_read_pointer(&mut self) {
        (self.read_pointer) = 0;
    }

    fn li_decode_single(&mut self) -> Option<Vec<u8>> {
        match get_start_and_end_index_of_next_li_chunk(self.read_pointer, &mut self.storage_system) {
            None => return None,
            Some(start_end) => {
                match self.storage_system.subarray(start_end.0, start_end.1) {
                    Err(_) => return None,
                    Ok(decoded) => {
                        self.read_pointer = start_end.1;
                        return Some(decoded)
                    }
                }
            }
        }
    }

    fn li_decode_single_stream(&mut self) -> Option<(Substream<File>, i64)> {
        match get_start_and_end_index_of_next_li_chunk(self.read_pointer, &mut self.storage_system) {
            None => return None,
            Some(start_end) => {
                match self.storage_system.substream(start_end.0, start_end.1) {
                    Err(_) => return None,
                    Ok(stream) => {
                        self.read_pointer = start_end.1;
                        let stream_length = start_end.1 - start_end.0;
                        return Some((stream, stream_length));
                    }
                }
            }
        }
    }

    fn li_delete_single(&mut self) -> Option<Vec<u8>> {
        match get_start_and_end_index_of_next_li_chunk(self.read_pointer, &mut self.storage_system) {
            None => None,
            Some(start_end) => {
                if let Ok(decoded) = self.storage_system.subarray(start_end.0, start_end.1) {
                    if let Ok(_) = self.storage_system.delete(self.read_pointer, start_end.1) {
                        return Some(decoded);
                    }
                }
                return None
            }
        }
    }

    fn li_skip_single(&mut self) -> i64 {
        match get_start_and_end_index_of_next_li_chunk(self.read_pointer, &mut self.storage_system) {
            None => return -1,
            Some(start_end) => {
                self.read_pointer = start_end.1;
                return start_end.1-start_end.0;
            }
        }
    }

    fn li_decode_all(&mut self) -> Vec<Vec<u8>> {
        let mut all = Vec::new();

        while let Some(single) = self.li_decode_single() {
            all.push(single);
        }

        return all;
    }
}




//actual LIBAE FUNCTIONALITY

fn get_length_indicator_for(length:i64) -> Vec<u8> {
    let mut li_bytes = get_minimal_bytes(length); //cannot be more than 8 in size.
    let leading_li = li_bytes.len() as u8; //cast possible because it cannot be more than 8 anyways.
    let mut li_bytes_with_leading_li = vec![leading_li];
    li_bytes_with_leading_li.append(&mut li_bytes);
    return li_bytes_with_leading_li;
}

fn get_start_and_end_index_of_next_li_chunk(start_index:i64, storage_system:&mut dyn StorageSystem) -> Option<(i64, i64)> {
    if let Ok(content_size) = storage_system.content_size() { //threating content_size error as "last element reached"
        let mut i = start_index;
        if i + 1 > content_size {
            return None;
        }
        let content_size = content_size;
        return match storage_system.subarray(i, i + 9) { //cache maximum number of required bytes. (to minimize possibly slow subarray calls)
            Err(_) => None,
            Ok(cache) => {
                let leading_li = cache[0] as i64;

                let length_indicator = &cache[1..(1 + leading_li) as usize];
                let length_indicator_as_int = get_int(length_indicator);
                if length_indicator_as_int == -1 || i + length_indicator_as_int > content_size {
                    return None;
                }
                i += leading_li + 1; //to skip all the li information.
                return Some((i, i + length_indicator_as_int));
            }
        }
    }
    return None;
}

fn get_int(bytearr:&[u8]) -> i64 {// big-endian
    if bytearr.len() == 8 {
        let mut rdr = Cursor::new(bytearr);
        return rdr.read_i64::<BigEndian>().unwrap();
    } else if bytearr.len() < 8 {
        let mut morebytes: [u8; 8] = [0,0,0,0,0,0,0,0];
        let start_index = morebytes.len()-bytearr.len();
        for i in 0..bytearr.len() {
            morebytes[start_index+i] = bytearr[i];
        }
        return get_int(&morebytes);
    } else { // cannot really happen in context
        return -1;
    }
}

fn get_minimal_bytes(x:i64) -> Vec<u8> {// big-endian
    let xf64 = x as f64;
    let byte_count = cmp::max(0, ((xf64.log2() + 1_f64).floor() / 8_f64).ceil() as i64);

    let mut bytes = vec![0_u8;byte_count as usize];
    for i in 0..bytes.len() {
        let pos = (bytes.len()-1)-i;
        bytes[i] = ((x >> (pos<<3)) & 0x000000FF) as u8;
    }
    return bytes;
}