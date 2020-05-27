use encoding::tag_based::bytes::ubae::UbaeTraits;
use network::mcnp::mcnp_client::McnpClient;
use network::mcnp::mcnp_connection::McnpConnection;
use network::mcnp::mcnp_connection::McnpConnectionTraits;
use std::io::Read;
use std;
use std::net::TcpStream;
use transparent_storage::Substream;
use transparent_storage::StorageSystemError;
use encoding::tag_based::bytes::libae::LIbae;
use transparent_storage::bytes::vec_storage_system::VecStorageSystem;
use encoding::tag_based::bytes::libae::LIbaeTraits;
use encoding::tag_based::bytes::remote::rbae_mcnp_causes;
use encoding::tag_based::bytes::remote::authenticated::arbae_mcnp_causes;
use encoding::tag_based::bytes::remote::authenticated::authentication_helper;

//todo. maybe create buffers for nonce, and encrypted_message buffer, though it very much is fast enough for now...

pub struct Arbae {
    client:McnpConnection,
    session_key:Vec<u8>
}
impl Arbae {
    pub fn login(addr:&str, port:u16, user_name:&str, password:&str) -> Result<Arbae, StorageSystemError> {
        match initialize_connection(addr, port, arbae_mcnp_causes::LOGIN_CAUSE, user_name, password) {
            Ok((con, sess_key)) => Ok(Arbae {client:con, session_key:sess_key}),
            Err(e) => Err(e)
        }
    }

    pub fn register(addr:&str, port:u16, user_name:&str, password:&str) -> Result<Arbae, StorageSystemError> {
        match initialize_connection(addr, port, arbae_mcnp_causes::REGISTER_CAUSE, user_name, password) {
            Ok((con, sess_key)) => Ok(Arbae {client:con, session_key:sess_key}),
            Err(e) => Err(e)
        }
    }

    pub fn unregister(mut self) -> Result<(), StorageSystemError> {
        self.client.send_cause(arbae_mcnp_causes::UNREGISTER_CAUSE)?;
        match self.client.read_fixed_chunk_u8()? as i8 {
            rbae_mcnp_causes::NO_ERROR => Ok(()),
            _ => Err(StorageSystemError::new("server returned unexpected, error implying message"))
        }
    }
}

pub fn initialize_connection(addr:&str, port:u16, cause:i32, user_name:&str, password:&str) -> Result<(McnpConnection,Vec<u8>), StorageSystemError> {
    let mut client = McnpClient::new(addr, port);

    client.send_cause(cause)?;
    client.send_variable_chunk(user_name.as_bytes())?;

    let server_public_key = client.read_variable_chunk()?;

    let my_private_key = authentication_helper::generate_private_key().expect("priv gen failed");
    let my_public_key = authentication_helper::compute_public_key(&my_private_key).expect("priv gen failed");
    client.send_variable_chunk(&my_public_key)?;

    if let Ok(exchanged_key) = authentication_helper::do_key_exchange(my_private_key, &my_public_key, &server_public_key) {
        let key = exchanged_key;
        let nonce = authentication_helper::generate_128bit_nonce();

        client.send_fixed_chunk_u8_arr(&nonce)?;
        let encrypted_password = authentication_helper::aes_crt_np_128_encrypt(&authentication_helper::sha256(password.as_bytes()), &key, &nonce);
        client.send_variable_chunk(&encrypted_password)?;

        let result = client.read_fixed_chunk_u8()? as i8;
        match result {
            arbae_mcnp_causes::LOGIN_SUCCESSFUL => {
                Ok((client, key))
            },
            arbae_mcnp_causes::REGISTER_SUCCESSFUL => {
                Ok((client, key))
            },
            arbae_mcnp_causes::REGISTER_FAILED_USER_NAME_TAKEN => Err(StorageSystemError::new("name taken")),
            arbae_mcnp_causes::LOGIN_FAILED_WRONG_NAME => Err(StorageSystemError::new("wrong name")),
            arbae_mcnp_causes::LOGIN_FAILED_WRONG_PASSWORD => Err(StorageSystemError::new("wrong pw")),
            _ => Err(StorageSystemError::new("server error")),
        }
    } else {
        Err(StorageSystemError::new("key exchange failed"))
    }
} 

//note on using StorageSystemError's to propagate mcnp network error's to the user:
//    it is fine, because mcnp is essentially the internal storage system here
//    it just works at a different level due to thread and process safety and consistency requirements.
impl UbaeTraits<TcpStream> for Arbae {
    /// technically not defined for Arbae
    fn set_content(&mut self, _bytes: &[u8]) -> Result<(), StorageSystemError> {
        Err(StorageSystemError::new("undefined"))
    }
    /// technically not defined for Arbae
    fn get_content(&mut self) -> Result<Vec<u8>, StorageSystemError> {
        Err(StorageSystemError::new("undefined"))
    }

    ///Returns all the tags the user defined in the system.
    ///As per condition each tag should only occur once and each tag should satisfy != null.
    fn get_tags(&mut self) -> Result<Vec<String>, StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::GET_TAGS)?;
        let nonce = self.client.read_fixed_chunk_u8_arr(16)?;
        let mut libae = LIbae::new(VecStorageSystem::new_empty());
        libae.set_content(&authentication_helper::aes_crt_np_128_decrypt(&self.client.read_variable_chunk()?, &self.session_key, &nonce))?;
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

        authentication_helper::send_tag(&mut self.client, tag, &self.session_key)?;

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

        authentication_helper::send_tag(&mut self.client, tag, &self.session_key)?;

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
        self.client.send_cause(rbae_mcnp_causes::GET_ENTRY_BYTE_ARR).expect("error sending cause");authentication_helper::send_tag(&mut self.client, tag, &self.session_key)?;

        match self.client.read_variable_chunk() {
            Err(ref e) if e.kind() == std::io::ErrorKind::InvalidData => Ok(None),
            Err(e) => Err(StorageSystemError::from(e)),
            Ok(read_bytes) => Ok(Some(read_bytes)),
        }
    }

    /// Retrieves the entry with the specified tag as a stream.
    ///    if the underlying storage is altered this might affect what can be read from the stream
    /// Will return None if the tag does not point to an entry within the system
    fn get_entry_as_stream(&mut self, tag: &str) -> Result<Option<(Substream<TcpStream>, i64)>, StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::GET_ENTRY_BYTE_ARR)?;

        authentication_helper::send_tag(&mut self.client, tag, &self.session_key)?;

        let entry_stream = self.client.read_variable_chunk_as_stream()?;
        return Ok(Some(entry_stream));
    }

    /// Same as get_entry, but deletes the specified entry and it's tag.
    fn delete_entry(&mut self, tag: &str) -> Result<Option<Vec<u8>>, StorageSystemError> {
        self.client.send_cause(rbae_mcnp_causes::DELETE_ENTRY_BYTE_ARR)?;

        authentication_helper::send_tag(&mut self.client, tag, &self.session_key)?;

        let entry_chunk = self.client.read_variable_chunk()?;
        return Ok(Some(entry_chunk));
    }

    /// same as delete entry, but does not return or allocate the entry as a vec.
    fn delete_entry_noreturn(&mut self, tag: &str) -> Result<bool, StorageSystemError>  {
        self.client.send_cause(rbae_mcnp_causes::DELETE_NO_RETURN)?;

        authentication_helper::send_tag(&mut self.client, tag, &self.session_key)?;

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

        authentication_helper::send_tag(&mut self.client, tag, &self.session_key)?;
        self.client.send_variable_chunk(&content)?;

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

        authentication_helper::send_tag(&mut self.client, tag, &self.session_key)?;

        self.client.send_variable_chunk(&content)?;

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

        authentication_helper::send_tag(&mut self.client, tag, &self.session_key)?;

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

        authentication_helper::send_tag(&mut self.client, tag, &self.session_key)?;

        self.client.send_variable_chunk_from_stream(stream, stream_length)?;

        match self.client.read_fixed_chunk_u8()? as i8 {
            rbae_mcnp_causes::NO_ERROR => Ok(()),
            _ => Err(StorageSystemError::new("server returned unexpected, error implying message"))
        }
    }
}