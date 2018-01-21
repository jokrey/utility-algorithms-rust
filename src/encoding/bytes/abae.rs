use super::libae::LIbae;
use super::libae::LIbaeTraits;
use super::libae_storage_system::StorageSystem;
use std::io::Read;
use super::Substream;
use std::fs::File;

/// Minimum traits required to meet protocol standards
///   Missing convenience wrappers for data types as of now.
pub trait AbaeTraits {
    fn set_content(&mut self, bytes : &[u8]);
    fn get_content(&self) -> &[u8];

    fn get_tags(&mut self) -> &[String];
    fn tag_exists(&mut self, tag:&str) -> bool;
    fn tag_length(&mut self, tag:&str) -> i64;
    fn get_entry(&mut self, tag:&str) -> Option<Vec<u8>>;
    fn get_entry_as_stream(&mut self, tag:&str) -> Option<Substream<File>>;
    fn delete_entry(&mut self, tag:&str) -> Option<Vec<u8>>;
    fn delete_entry_noreturn(&mut self, tag:&str);

    fn add_entry(&mut self, tag:&str, content:&[u8]);
    fn add_entry_nocheck(&mut self, tag:&str, content:&[u8]);
    fn add_entry_from_stream(&mut self, tag:&str, stream : &mut Read, stream_length:i64);
    fn add_entry_from_stream_nocheck(&mut self, tag:&str, stream : &mut Read, stream_length:i64);
}

pub struct Abae<T:StorageSystem> {
    libae:LIbae<T>
}
impl<T:StorageSystem> Abae<T> {
    /// Creates a new abae system with the provided storage system.
    pub fn new(storagesystem:T) -> Abae<T> {
        Abae {
            libae:LIbae::new(storagesystem)
        }
    }

    /// Creates a new abae system iterator with the provided storage system.
    pub fn new_tag_stream_iterator(storagesystem:T) -> AbaeStreamIter<T> {
        return AbaeStreamIter {
            libae:LIbae::new(storagesystem)
        }
    }
}

///Simple iterator for an abae system.
///   Iterates over the tags and streams to their entry content.
///Panics easily
pub struct AbaeStreamIter<T:StorageSystem> {
    libae:LIbae<T>
}
impl <T:StorageSystem> Iterator for AbaeStreamIter<T> {
    type Item = (String, Substream<File>);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(decoded_tag) = self.libae.li_decode_single() {
            Some((String::from_utf8(decoded_tag).unwrap(), self.libae.li_decode_single_stream().unwrap()))
        } else {
            None
        }
    }
}

impl<T:StorageSystem> AbaeTraits for Abae<T> {
    /// hands set_content calls through to underlying storage system.
    fn set_content(&mut self, bytes: &[u8]) {
        self.libae.set_content(bytes);
    }
    /// hands get_content calls through to underlying storage system.
    /// Maybe problematic with some storage systems.
    fn get_content(&self) -> &[u8] {
        self.libae.get_content()
    }

    ///Returns all the tags in the system.
    ///As per condition each tag should only occur once and each tag should satisfy != null.
    // todo implement
    fn get_tags(&mut self) -> &[String] {
        unimplemented!()
    }

    ///Checks if an entry with the specified tag exists.
    /// Without the unnecessary overhead of actually retrieving the entry.
    fn tag_exists(&mut self, tag: &str) -> bool {
        self.libae.reset_read_pointer();
        let search_tag_as_bytes = tag.as_bytes(); //&str guarantees utf8

        while let Some(decoded_tag) = self.libae.li_decode_single() {
            if self.libae.li_skip_single()==-1 {  //would decode content, if that fails then return false.
                break
            }

            if search_tag_as_bytes==&decoded_tag[..] {
                return true
            }
        }
        return false
    }

    ///Returns the size of the entry in bytes
    /// not counting the encoding bytes
    fn tag_length(&mut self, tag: &str) -> i64 {
        self.libae.reset_read_pointer();
        let search_tag_as_bytes = tag.as_bytes(); //&str guarantees utf8

        while let Some(decoded_tag) = self.libae.li_decode_single() {
            let length_of_skipped_content = self.libae.li_skip_single();

            if search_tag_as_bytes==&decoded_tag[..] {
                return length_of_skipped_content
            }
        }
        return -1
    }

    /// Retrieves the entry with the specified tag as a byte array.
    /// the resulting vec can be altered, without altering the underlying storage.
    ///    if the underlying storage is altered this will not affect this vec
    /// Will return None if the tag does not point to an entry within the system
    fn get_entry(&mut self, tag: &str) -> Option<Vec<u8>> {
        self.libae.reset_read_pointer();
        let search_tag_as_bytes = tag.as_bytes(); //&str guarantees utf8

        while let Some(decoded_tag) = self.libae.li_decode_single() {
            if search_tag_as_bytes==&decoded_tag[..] {
                return self.libae.li_decode_single()
            } else {
                self.libae.li_skip_single();
            }
        }
        return None
    }

    /// Retrieves the entry with the specified tag as a stream.
    ///    if the underlying storage is altered this might affect what can be read from the stream
    /// Will return None if the tag does not point to an entry within the system
    fn get_entry_as_stream(&mut self, tag: &str) -> Option<Substream<File>> {
        self.libae.reset_read_pointer();
        let search_tag_as_bytes = tag.as_bytes(); //&str guarantees utf8

        while let Some(decoded_tag) = self.libae.li_decode_single() {
            if search_tag_as_bytes==&decoded_tag[..] {
                return self.libae.li_decode_single_stream()
            } else {
                self.libae.li_skip_single();
            }
        }
        return None
    }

    /// Same as get_entry, but deletes the specified entry and it's tag.
    fn delete_entry(&mut self, tag: &str) -> Option<Vec<u8>> {
        self.libae.reset_read_pointer();
        let search_tag_as_bytes = tag.as_bytes(); //&str guarantees utf8

        let mut last_read_pointer:i64 = 0;
        while let Some(decoded_tag) = self.libae.li_decode_single() {
            if search_tag_as_bytes==&decoded_tag[..] {
                let toreturn = self.libae.li_decode_single();
                let cur_rp = self.libae.manually_get_read_pointer();
                self.libae.storage_system.delete(last_read_pointer, cur_rp);
                return toreturn
            } else {
                self.libae.li_skip_single();
                last_read_pointer=self.libae.manually_get_read_pointer();
            }
        }
        return None
    }

    /// same as delete entry, but does not return or allocate the entry as a vec.
    fn delete_entry_noreturn(&mut self, tag: &str) {
        self.libae.reset_read_pointer();
        let search_tag_as_bytes = tag.as_bytes(); //&str guarantees utf8

        let mut last_read_pointer:i64 = 0;
        while let Some(decoded_tag) = self.libae.li_decode_single() {
            if search_tag_as_bytes==&decoded_tag[..] {
                self.libae.li_skip_single();
                let cur_rp = self.libae.manually_get_read_pointer();
                self.libae.storage_system.delete(last_read_pointer, cur_rp);
                break
            } else {
                self.libae.li_skip_single();
                last_read_pointer=self.libae.manually_get_read_pointer();
            }
        }
    }

    /// Adds the entry, with it's specified tag to the system.
    /// If an entry with the specified tag is already in the system it is DELETED and replaced.
    ///    To maintain the system condition that each tag is unique within the system.
    fn add_entry(&mut self, tag: &str, content: &[u8]) {
        self.delete_entry_noreturn(tag);

        self.libae.li_encode_single(tag.as_bytes()); //&str guarantees utf8
        self.libae.li_encode_single(content);
    }

    /// Same as add_entry,
    ///   but the caller ensures us that the tag does not yet exist within the system.
    ///     this can provide a considerable speed up, since the system is not searched
    ///   If the caller is wrong decoding the added content may become hard to impossible.
    fn add_entry_nocheck(&mut self, tag: &str, content: &[u8]) {
//        self.delete_entry_noreturn(tag);

        self.libae.li_encode_single(tag.as_bytes()); //&str guarantees utf8
        self.libae.li_encode_single(content);
    }

    /// same as add_entry, but reads the entry from the provided stream.
    ///   if stream is not of stream length behaviour is mostly undefined.
    ///   Though the system will try not to break because of it.
    fn add_entry_from_stream(&mut self, tag: &str, stream: &mut Read, stream_length: i64) {
        self.delete_entry_noreturn(tag);

        self.libae.li_encode_single(tag.as_bytes()); //&str guarantees utf8
        self.libae.li_encode_single_stream(stream, stream_length);
    }

    /// Same as add_entry_from_stream,
    ///   but the caller ensures us that the tag does not yet exist within the system.
    ///     this can provide a considerable speed up, since the system is not searched
    ///   If the caller is wrong decoding the added content may become hard to impossible.
    fn add_entry_from_stream_nocheck(&mut self, tag: &str, stream: &mut Read, stream_length: i64) {
//        self.delete_entry_noreturn(tag);

        self.libae.li_encode_single(tag.as_bytes()); //&str guarantees utf8
        self.libae.li_encode_single_stream(stream, stream_length);
    }
}