use encoding::bytes::ubae::UbaeTraits;
use network::mcnp::mcnp_client::McnpClient;
use network::mcnp::mcnp_connection::McnpConnection;
use network::mcnp::mcnp_connection::McnpConnectionTraits;
use super::rbae_mcnp_causes;
use std::io::Read;
use std;
use std::net::TcpStream;
use encoding::bytes::Substream;
use encoding::bytes::libae_storage_system::StorageSystemError;
use encoding::bytes::libae::LIbae;
use encoding::bytes::vec_storage_system::VecStorageSystem;
use encoding::bytes::libae::LIbaeTraits;

pub struct Rbae {
    client:McnpConnection
}
impl Rbae {
    /// Creates a new ubae system with the provided storage system.
    pub fn new(addr:&str, port:u16) -> Rbae {
        let mut client = McnpClient::new(addr, port);
        client.send_cause(rbae_mcnp_causes::INITIAL_CONNECTION_CAUSE__IS_CLIENT).expect("Initialization failed.");
        Rbae {
            client
        }
    }
}

//note on using StorageSystemError's to propagate mcnp network error's to the user:
//    it is fine, because mcnp is essentially the internal storage system
//    it just works at a different level due to thread and process safety and consistency requirements.
impl UbaeTraits<TcpStream> for Rbae {
    /// hands set_content calls through to underlying storage system.
    fn set_content(&mut self, bytes: &[u8]) -> Result<(), StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::SET_CONTENT)?;
        self.client.send_variable_chunk(bytes)?;

        match self.client.read_fixed_chunk_u8()? as i8 {
            rbae_mcnp_causes::NO_ERROR => Ok(()),
            _ => Err(StorageSystemError::new("server returned unexpected, error implying message"))
        }
    }
    /// hands get_content calls through to underlying storage system.
    /// Maybe problematic with some storage systems.
    fn get_content(&mut self) -> Result<Vec<u8>, StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::GET_CONTENT)?;

        let read = self.client.read_variable_chunk()?;
        return Ok(read);
    }

    ///Returns all the tags in the system.
    ///As per condition each tag should only occur once and each tag should satisfy != null.
    fn get_tags(&mut self) -> Result<Vec<String>, StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::GET_TAGS)?;
        let mut libae = LIbae::new(VecStorageSystem::new_empty());
        libae.set_content(&self.client.read_variable_chunk()?)?;
        let tags_enc = libae.li_decode_all();
        let mut tags = Vec::new();
        for tag_enc in tags_enc {
            tags.push(String::from_utf8(tag_enc).expect("how is this not utf8 though?"));
        }
        return Ok(tags);
    }

    ///Checks if an entry with the specified tag exists.
    /// Without the unnecessary overhead of actually retrieving the entry.
    fn tag_exists(&mut self, tag: &str) -> Result<bool, StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::EXISTS)?;
        self.client.send_variable_chunk(tag.as_bytes())?;

        match self.client.read_fixed_chunk_u8()? as i8 {
            rbae_mcnp_causes::ERROR => Err(StorageSystemError::new("server returned error message.")),
            rbae_mcnp_causes::TRUE => Ok(true),
            rbae_mcnp_causes::FALSE => Ok(false),
            _ => Err(StorageSystemError::new("server returned unknown message."))
        }
    }

    ///Returns the size of the entry in bytes
    /// not counting the encoding bytes
    fn tag_length(&mut self, tag: &str) -> Result<i64, StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::LENGTH)?;
        self.client.send_variable_chunk(tag.as_bytes())?;

        let result = self.client.read_fixed_chunk_i64()?;
        if result == rbae_mcnp_causes::ERROR as i64 {
            return Err(StorageSystemError::new("server returned error message"))
        } else {
            return Ok(result)
        }
    }

    /// Retrieves the entry with the specified tag as a byte array.
    /// the resulting vec can be altered, without altering the underlying storage.
    ///    if the underlying storage is altered this will not affect this vec
    /// Will return None if the tag does not point to an entry within the system
    fn get_entry(&mut self, tag: &str) -> Result<Option<Vec<u8>>, StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::GET_ENTRY_BYTE_ARR)?;
        self.client.send_variable_chunk(tag.as_bytes())?;

        match self.client.read_variable_chunk() {
            Err(ref e) if e.kind() == std::io::ErrorKind::InvalidData => Ok(None),
            Err(e) => { return Err(StorageSystemError::from(e)) }
            Ok(read) => return Ok(Some(read)),
        }
    }

    /// Retrieves the entry with the specified tag as a stream.
    ///    if the underlying storage is altered this might affect what can be read from the stream
    /// Will return None if the tag does not point to an entry within the system
    fn get_entry_as_stream(&mut self, tag: &str) -> Result<Option<(Substream<TcpStream>, i64)>, StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::GET_ENTRY_BYTE_ARR)?;
        self.client.send_variable_chunk(tag.as_bytes())?;
        let entry_stream = self.client.read_variable_chunk_as_stream()?;
        return Ok(Some(entry_stream));
    }

    /// Same as get_entry, but deletes the specified entry and it's tag.
    fn delete_entry(&mut self, tag: &str) -> Result<Option<Vec<u8>>, StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::DELETE_ENTRY_BYTE_ARR)?;
        self.client.send_variable_chunk(tag.as_bytes())?;

        match self.client.read_variable_chunk() {
            Err(ref e) if e.kind() == std::io::ErrorKind::InvalidData => Ok(None),
            Err(e) => { return Err(StorageSystemError::from(e)) }
            Ok(read) => return Ok(Some(read)),
        }
    }

    /// same as delete entry, but does not return or allocate the entry as a vec.
    fn delete_entry_noreturn(&mut self, tag: &str) -> Result<bool, StorageSystemError>  {
        self.client.send_cause(rbae_mcnp_causes::DELETE_NO_RETURN)?;
        self.client.send_variable_chunk(tag.as_bytes())?;

        match self.client.read_fixed_chunk_u8()? as i8 {
            rbae_mcnp_causes::ERROR => Err(StorageSystemError::new("server returned error message.")),
            rbae_mcnp_causes::TRUE => Ok(true),
            rbae_mcnp_causes::FALSE => Ok(false),
            _ => Err(StorageSystemError::new("server returned unknown message."))
        }
    }

    /// Adds the entry, with it's specified tag to the system.
    /// If an entry with the specified tag is already in the system it is DELETED and replaced.
    ///    To maintain the system condition that each tag is unique within the system.
    fn add_entry(&mut self, tag: &str, content: &[u8]) -> Result<(), StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR)?;
        self.client.send_variable_chunk(tag.as_bytes())?;
        self.client.send_variable_chunk(content)?;

        match self.client.read_fixed_chunk_u8()? as i8 {
            rbae_mcnp_causes::NO_ERROR => Ok(()),
            _ => Err(StorageSystemError::new("server returned unexpected, error implying message"))
        }
    }

    /// Same as add_entry,
    ///   but the caller ensures us that the tag does not yet exist within the system.
    ///     this can provide a considerable speed up, since the system is not searched
    ///   If the caller is wrong decoding the added content may become hard to impossible.
    fn add_entry_nocheck(&mut self, tag: &str, content: &[u8]) -> Result<(), StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR_NOCHECK)?;
        self.client.send_variable_chunk(tag.as_bytes())?;
        self.client.send_variable_chunk(content)?;

        match self.client.read_fixed_chunk_u8()? as i8 {
            rbae_mcnp_causes::NO_ERROR => Ok(()),
            _ => Err(StorageSystemError::new("server returned unexpected, error implying message"))
        }
    }

    /// same as add_entry, but reads the entry from the provided stream.
    ///   if stream is not of stream length behaviour is mostly undefined.
    ///   Though the system will try not to break because of it.
    fn add_entry_from_stream(&mut self, tag: &str, stream: &mut Read, stream_length: i64) -> Result<(), StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR)?;
        self.client.send_variable_chunk(tag.as_bytes())?;
        self.client.send_variable_chunk_from_stream(stream, stream_length)?;

        match self.client.read_fixed_chunk_u8()? as i8 {
            rbae_mcnp_causes::NO_ERROR => Ok(()),
            _ => Err(StorageSystemError::new("server returned unexpected, error implying message"))
        }
    }

    /// Same as add_entry_from_stream,
    ///   but the caller ensures us that the tag does not yet exist within the system.
    ///     this can provide a considerable speed up, since the system is not searched
    ///   If the caller is wrong decoding the added content may become hard to impossible.
    fn add_entry_from_stream_nocheck(&mut self, tag: &str, stream: &mut Read, stream_length: i64) -> Result<(), StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR_NOCHECK)?;
        self.client.send_variable_chunk(tag.as_bytes())?;
        self.client.send_variable_chunk_from_stream(stream, stream_length)?;

        match self.client.read_fixed_chunk_u8()? as i8 {
            rbae_mcnp_causes::NO_ERROR => Ok(()),
            _ => Err(StorageSystemError::new("server returned unexpected, error implying message"))
        }
    }
}