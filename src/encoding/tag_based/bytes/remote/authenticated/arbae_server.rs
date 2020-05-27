extern crate ring;
extern crate untrusted;

use encoding::tag_based::bytes::ubae::Ubae;
use transparent_storage::bytes::file_storage_system::FileStorageSystem;
use network::mcnp::mcnp_connection::McnpConnection;
use network::mcnp::mcnp_connection::McnpConnectionTraits;
use std::str;
use encoding::tag_based::bytes::remote::rbae_mcnp_causes;
use encoding::tag_based::bytes::ubae::UbaeTraits;
use encoding::tag_based::bytes::libae::LIbae;
use encoding::tag_based::bytes::libae::LIbaeTraits;
use transparent_storage::bytes::vec_storage_system::VecStorageSystem;
use transparent_storage::StorageSystemError;
use std::io::Read;
use transparent_storage::Substream;
use std::fs::File;
use encoding::tag_based::bytes::remote::rbae_server::RbaeServer;
use encoding::tag_based::bytes::remote::authenticated::authentication_helper;
use encoding::tag_based::bytes::remote::authenticated::arbae_mcnp_causes;
use std::error::Error;
use std::sync::MutexGuard;
use std::thread;
use std::sync::Arc;
use std::sync::Mutex;

//todo - encrypt incoming and outgoing data  - the stream functions make this a little hard
//todo implement authentication where the pseudo tag authentication doesn't apply(for example in unregister).

pub struct ArbaeConnectionState {
    pub connection:McnpConnection,
    pub user_name:String,
    pub user_name_hash:String,
    pub session_key:Vec<u8>
}
pub type ArbaeServer = RbaeServer<ArbaeObserverConnection, ArbaeConnectionState>;
impl ArbaeServer {
    pub fn new_arbae(port:u16, ubae:Ubae<FileStorageSystem>) -> ArbaeServer {
        let server = RbaeServer::new_without_cause_handlers(port, ubae);
        server.add_cause_handler(rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR, ArbaeServer::handle_add_entry_byte_arr_by);
        server.add_cause_handler(rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR_NOCHECK, ArbaeServer::handle_add_entry_byte_arr_nocheck_by);
        server.add_cause_handler(rbae_mcnp_causes::GET_ENTRY_BYTE_ARR, ArbaeServer::handle_get_entry_byte_arr_by);
        server.add_cause_handler(rbae_mcnp_causes::DELETE_ENTRY_BYTE_ARR, ArbaeServer::handle_delete_entry_byte_arr_by);
        server.add_cause_handler(rbae_mcnp_causes::DELETE_NO_RETURN, ArbaeServer::handle_delete_entry_noreturn_by);
        server.add_cause_handler(rbae_mcnp_causes::EXISTS, ArbaeServer::handle_exists_by);
        server.add_cause_handler(rbae_mcnp_causes::GET_TAGS, ArbaeServer::handle_get_tags_by);
        server.add_cause_handler(rbae_mcnp_causes::LENGTH, ArbaeServer::handle_length_by);
        server.add_cause_handler(arbae_mcnp_causes::UNREGISTER_CAUSE, ArbaeServer::handle_unregister_by);

        server
    }
    
    
    pub fn add_observing_client(&mut self, connection:McnpConnection, user_name_hash:String, session_key:Vec<u8>) {
        let mut obs = self.get_observers_locked();
        let con_id = ArbaeObserverConnection::get_uid(&obs);
        obs.push(ArbaeObserverConnection::new(connection, user_name_hash, session_key, con_id));
    }

    
    
    pub fn get_user_tags(&mut self, user_name:&str) -> Result<Vec<String>, StorageSystemError> {
        let user_name_hash = authentication_helper::hashed(user_name.as_bytes());
        let get_tags = self.get_tags()?;
        let mut user_tags = Vec::new();

        for tag in get_tags {
            if &tag[..user_name_hash.len()] == user_name_hash { // tag starts with username hash
                let user_tag = &tag[user_name_hash.len()..];
                user_tags.push(String::from(user_tag));
            }
        }
        return Ok(user_tags)
    }
    pub fn user_tag_exists(&mut self, user_name:&str, tag:&str) -> Result<bool, StorageSystemError> {
        let actual_tag = authentication_helper::actual_tag_from(user_name, tag);
        self.tag_exists(&actual_tag)
    }
    pub fn user_tag_length(&mut self, user_name:&str, tag:&str) -> Result<i64, StorageSystemError> {
        let actual_tag = authentication_helper::actual_tag_from(user_name, tag);
        self.tag_length(&actual_tag)
    }
    pub fn get_user_entry(&mut self, user_name:&str, tag:&str) -> Result<Option<Vec<u8>>, StorageSystemError> {
        let actual_tag = authentication_helper::actual_tag_from(user_name, tag);
        self.get_entry(&actual_tag)
    }
    pub fn get_user_entry_as_stream(&mut self, user_name:&str, tag:&str) -> Result<Option<(Substream<File>, i64)>, StorageSystemError> {
        let actual_tag = authentication_helper::actual_tag_from(user_name, tag);
        self.get_entry_as_stream(&actual_tag)
    }
    pub fn delete_user_entry(&mut self, user_name:&str, tag:&str) -> Result<Option<Vec<u8>>, StorageSystemError> {
        let actual_tag = authentication_helper::actual_tag_from(user_name, tag);
        self.delete_entry(&actual_tag)
    }
    pub fn delete_user_entry_noreturn(&mut self, user_name:&str, tag:&str) -> Result<bool, StorageSystemError> {
        let actual_tag = authentication_helper::actual_tag_from(user_name, tag);
        self.delete_entry_noreturn(&actual_tag)
    }
    pub fn add_user_entry(&mut self, user_name:&str, tag:&str, content:&[u8]) -> Result<(), StorageSystemError> {
        let actual_tag = authentication_helper::actual_tag_from(user_name, tag);
        self.add_entry(&actual_tag, content)
    }
    pub fn add_user_entry_nocheck(&mut self, user_name:&str, tag:&str, content:&[u8]) -> Result<(), StorageSystemError> {
        let actual_tag = authentication_helper::actual_tag_from(user_name, tag);
        self.add_entry_nocheck(&actual_tag, content)
    }
    pub fn add_user_entry_from_stream(&mut self, user_name:&str, tag:&str, stream : &mut Read, stream_length:i64) -> Result<(), StorageSystemError> {
        let actual_tag = authentication_helper::actual_tag_from(user_name, tag);
        self.add_entry_from_stream(&actual_tag, stream, stream_length)
    }
    pub fn add_user_entry_from_stream_nocheck(&mut self, user_name:&str, tag:&str, stream : &mut Read, stream_length:i64) -> Result<(), StorageSystemError> {
        let actual_tag = authentication_helper::actual_tag_from(user_name, tag);
        self.add_entry_from_stream_nocheck(&actual_tag, stream, stream_length)
    }




    pub fn run_logic_loop(self) {
        self.general_run_logic_loop(ArbaeServer::handle_new_connection);
    }

    fn handle_new_connection(&mut self, connection:McnpConnection) {
        let mut state = match self.handle_connection_authentication(connection) {
            Ok(name_hash_sk) => name_hash_sk,
            Err(e) => {
                println!("Auth err: {}", e.description());
                return;
            },
        };

        println!("new connection authenticated - Auth-User: {}", state.user_name);

        loop {
            match state.connection.read_cause() {
                Err(_) => break, //indicates eof or error, hopefully eof otherwise we should probably print what the error is for debugging purposes
                Ok(cause) => {
                    match self.handle_request_by(cause, &mut state) {
                        Err(e) => {
                            println!("Error handling request: {:?}", e)
                            //don't break here. just because one error occured, does not mean one will always occur from now on
                        },
                        Ok(_) => {}
                    }
                }
            }
        }
    }

    //returns user_name_hash:String and the session key(for aes crt no padding 128 bit)
    //the user_name_hash is effectivly the user name on server side
    //it has to be hashed so it always has the same size
    //this is important, since a valid transaction is checked using this user_name and checking if it starts with the tag to be altered.
    //if user names did not have the same length, then a user with the name "bob" could alter tags of the user "bobby", by starting the request tag with -"by".....
    fn handle_connection_authentication(&mut self, mut connection:McnpConnection) -> Result<ArbaeConnectionState, StorageSystemError> {
        let cause = connection.read_cause()?;


        let user_name_bytes = connection.read_variable_chunk()?;
        let user_name_hash = authentication_helper::hashed(&user_name_bytes);
        let password_store_tag_for_user_name = authentication_helper::password_store_tag_for_user_name_hash(&user_name_hash);

        let my_private_key = authentication_helper::generate_private_key().expect("priv gen failed");
        let my_public_key = authentication_helper::compute_public_key(&my_private_key).expect("priv gen failed");

        connection.send_variable_chunk(&my_public_key)?;
        let received_remote_public_key = connection.read_variable_chunk()?;

        if let Ok(exchanged_key) = authentication_helper::do_key_exchange(my_private_key, &my_public_key, &received_remote_public_key) {
            let nonce = connection.read_fixed_chunk_u8_arr(16)?;
            let encrypted_password = connection.read_variable_chunk()?;

            let password_received = authentication_helper::aes_crt_np_128_decrypt(&encrypted_password, &exchanged_key, &nonce);
            match cause {
                arbae_mcnp_causes::LOGIN_CAUSE => {
                    let mut locked_ubae:MutexGuard<Ubae<FileStorageSystem>> = self.ubae_clone_lock();
                    let password_on_file = locked_ubae.get_entry(&password_store_tag_for_user_name)?;
                    if let Some(password_on_file) = password_on_file {
                        if password_received == password_on_file {
                            connection.send_fixed_chunk_u8(arbae_mcnp_causes::LOGIN_SUCCESSFUL as u8)?;
                            let state = ArbaeConnectionState {
                                connection,
                                user_name:String::from_utf8(user_name_bytes).expect("illegal user name, not utf8"),
                                user_name_hash,
                                session_key:exchanged_key
                            };
                            Ok(state)
                        } else {
                            connection.send_fixed_chunk_u8(arbae_mcnp_causes::LOGIN_FAILED_WRONG_PASSWORD as u8)?;
                            Err(StorageSystemError::new("wrong pw"))
                        }
                    } else {
                        connection.send_fixed_chunk_u8(arbae_mcnp_causes::LOGIN_FAILED_WRONG_NAME as u8)?;
                        Err(StorageSystemError::new("wrong name"))
                    }
                },
                arbae_mcnp_causes::REGISTER_CAUSE => {
                    let mut locked_ubae:MutexGuard<Ubae<FileStorageSystem>> = self.ubae_clone_lock();
                    let password_on_file = locked_ubae.get_entry(&password_store_tag_for_user_name)?;
                    match password_on_file {
                        None => {
                            locked_ubae.add_entry_nocheck(&password_store_tag_for_user_name, &password_received)?; //nocheck because we just checked if it exists...
                            connection.send_fixed_chunk_u8(arbae_mcnp_causes::REGISTER_SUCCESSFUL as u8)?;
                            let state = ArbaeConnectionState {
                                connection,
                                user_name:String::from_utf8(user_name_bytes).expect("illegal user name, not utf8"),
                                user_name_hash,
                                session_key:exchanged_key
                            };
                            Ok(state)
                        },
                        Some(password_on_file) => {
                            if password_received == password_on_file {
                                connection.send_fixed_chunk_u8(arbae_mcnp_causes::LOGIN_SUCCESSFUL as u8)?;
                                let state = ArbaeConnectionState {
                                    connection,
                                    user_name:String::from_utf8(user_name_bytes).expect("illegal user name, not utf8"),
                                    user_name_hash,
                                    session_key:exchanged_key
                                };
                                Ok(state)
                            } else {
                                connection.send_fixed_chunk_u8(arbae_mcnp_causes::REGISTER_FAILED_USER_NAME_TAKEN as u8)?;
                                Err(StorageSystemError::new("name taken"))
                            }
                        }
                    }
                },
                rbae_mcnp_causes::INITIAL_CONNECTION_CAUSE__IS_OBSERVER => {
                    let password_on_file = {
                        let mut locked_ubae:MutexGuard<Ubae<FileStorageSystem>> = self.ubae_clone_lock();
                        locked_ubae.get_entry(&password_store_tag_for_user_name)?
                    };
                    if let Some(password_on_file) = password_on_file {
                        if password_received == password_on_file {
                            connection.send_fixed_chunk_u8(arbae_mcnp_causes::LOGIN_SUCCESSFUL as u8)?;
                            self.add_observing_client(connection, user_name_hash, exchanged_key);
                            Err(StorageSystemError::new("is observer")) //yes returning an error here is semantically pretty weird, but it's simple and works
                        } else {
                            connection.send_fixed_chunk_u8(arbae_mcnp_causes::LOGIN_FAILED_WRONG_PASSWORD as u8)?;
                            Err(StorageSystemError::new("wrong pw"))
                        }
                    } else {
                        connection.send_fixed_chunk_u8(arbae_mcnp_causes::LOGIN_FAILED_WRONG_NAME as u8)?;
                        Err(StorageSystemError::new("wrong name"))
                    }
                },
                _ => {
                    connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
                    Err(StorageSystemError::new("initial cause unrecognised"))
                }
            }
        } else {
            connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
            Err(StorageSystemError::new("other"))
        }
    }


    fn handle_add_entry_byte_arr_by(&mut self, state:&mut ArbaeConnectionState) -> Result<(), StorageSystemError> {
        let actual_tag = authentication_helper::receive_tag(&mut state.connection, &state.user_name_hash, &state.session_key)?;

        let entry_to_add = state.connection.read_variable_chunk()?;
        match self.add_entry(&actual_tag, &entry_to_add) {
            Ok(_) => {
                state.connection.send_fixed_chunk_u8(rbae_mcnp_causes::NO_ERROR as u8)?;
                self.send_update_callback(rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR, &state.user_name_hash, Some(actual_tag))?;
                Ok(())
            },
            Err(e) => {
                state.connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
                Err(e)
            }
        }
    }

    //todo succeptible to a denial of service attack.
    //todo The server can be endlessly blocked should someone send a stream with mcnp length indication n, but only supply m(where m < n) bytes without closing the stream
    fn handle_add_entry_byte_arr_nocheck_by(&mut self, state:&mut ArbaeConnectionState) -> Result<(), StorageSystemError> {
        let actual_tag = authentication_helper::receive_tag(&mut state.connection, &state.user_name_hash, &state.session_key)?;

        let mut entry_to_add = state.connection.read_variable_chunk_as_stream()?;
        match self.add_entry_from_stream_nocheck(&actual_tag, &mut entry_to_add.0, entry_to_add.1) {
            Ok(_) => {
                state.connection.send_fixed_chunk_u8(rbae_mcnp_causes::NO_ERROR as u8)?;
                self.send_update_callback(rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR_NOCHECK, &state.user_name_hash, Some(actual_tag))?;
                Ok(())
            },
            Err(e) => {
                state.connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
                Err(e)
            }
        }
    }

    fn handle_get_entry_byte_arr_by(&mut self, state:&mut ArbaeConnectionState) -> Result<(), StorageSystemError> {
        let actual_tag = authentication_helper::receive_tag(&mut state.connection, &state.user_name_hash, &state.session_key)?;

        match self.get_entry_as_stream(&actual_tag) {
            Ok(Some(mut stream)) => state.connection.send_variable_chunk_from_stream(&mut stream.0, stream.1)?,
            Ok(None)             => state.connection.start_variable_chunk(-1)?,
            Err(e)  => {
                state.connection.start_variable_chunk(-1)?;
                return Err(e)
            }
        }
        Ok(())
    }

    fn handle_delete_entry_byte_arr_by(&mut self, state:&mut ArbaeConnectionState) -> Result<(), StorageSystemError> {
        let actual_tag = authentication_helper::receive_tag(&mut state.connection, &state.user_name_hash, &state.session_key)?;

        match self.delete_entry(&actual_tag) {
            Ok(Some(deleted_entry)) => state.connection.send_variable_chunk(&deleted_entry)?,
            Ok(None)                => state.connection.start_variable_chunk(-1)?,
            Err(e)  => {
                state.connection.start_variable_chunk(-1)?;
                return Err(e)
            }
        }
        self.send_update_callback(rbae_mcnp_causes::DELETE_ENTRY_BYTE_ARR, &state.user_name_hash, Some(actual_tag))?;
        Ok(())
    }

    fn handle_delete_entry_noreturn_by(&mut self, state:&mut ArbaeConnectionState) -> Result<(), StorageSystemError> {
        let actual_tag = authentication_helper::receive_tag(&mut state.connection, &state.user_name_hash, &state.session_key)?;

        match self.delete_entry_noreturn(&actual_tag) {
            Ok(true)  => state.connection.send_fixed_chunk_u8(rbae_mcnp_causes::TRUE as u8)?,
            Ok(false) => state.connection.send_fixed_chunk_u8(rbae_mcnp_causes::FALSE as u8)?,
            Err(e) => {
                state.connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
                return Err(e)
            }
        }
        self.send_update_callback(rbae_mcnp_causes::DELETE_NO_RETURN, &state.user_name_hash, Some(actual_tag))?;
        Ok(())
    }

    fn handle_exists_by(&mut self, state:&mut ArbaeConnectionState) -> Result<(), StorageSystemError> {
        let actual_tag = authentication_helper::receive_tag(&mut state.connection, &state.user_name_hash, &state.session_key)?;

        match self.tag_exists(&actual_tag) {
            Ok(true)  => state.connection.send_fixed_chunk_u8(rbae_mcnp_causes::TRUE as u8)?,
            Ok(false) => state.connection.send_fixed_chunk_u8(rbae_mcnp_causes::FALSE as u8)?,
            Err(e) => {
                state.connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
                return Err(e)
            }
        }
        Ok(())
    }

    fn handle_get_tags_by(&mut self, state:&mut ArbaeConnectionState) -> Result<(), StorageSystemError> {

        let nonce = authentication_helper::generate_128bit_nonce();
        state.connection.send_fixed_chunk_u8_arr(&nonce)?;
        let mut libae = LIbae::new(VecStorageSystem::new_empty());


        let get_tags = self.get_tags()?;
        for tag in get_tags {
            if &tag[..state.user_name_hash.len()] == state.user_name_hash { // tag starts with username hash
                let user_tag = &tag[state.user_name_hash.len()..];
                libae.li_encode_single(&user_tag.as_bytes()).expect("wow. now vec storage has issues now... it literally can't, but fine. fine. I am ok with this. Kind of. Not really. WHAT THE HELL? HOW IT DOESN'T EVEN RETURN ERROR EVER!");
            }
        }
        let encoded_tags = libae.get_content()?;
        state.connection.send_variable_chunk(&authentication_helper::aes_crt_np_128_encrypt(&encoded_tags, &state.session_key, &nonce))?;
        Ok(())
    }

    fn handle_length_by(&mut self, state:&mut ArbaeConnectionState) -> Result<(), StorageSystemError> {
        let actual_tag = authentication_helper::receive_tag(&mut state.connection, &state.user_name_hash, &state.session_key)?;

        match self.tag_length(&actual_tag) {
            Ok(tag_length) => {
                state.connection.send_fixed_chunk_i64(tag_length)?;
                Ok(())
            },
            Err(e) => {
                state.connection.send_fixed_chunk_i64(rbae_mcnp_causes::ERROR as i64)?;
                Err(e)
            }
        }
    }

    fn handle_unregister_by(&mut self, state:&mut ArbaeConnectionState) -> Result<(), StorageSystemError> {
        {
            //delete every tag the user owns.
            let mut locked_ubae = self.ubae_clone_lock();
            let get_tags = locked_ubae.get_tags()?;
            for tag in get_tags {
                if &tag[..state.user_name_hash.len()] == state.user_name_hash { // tag starts with username hash

                    match locked_ubae.delete_entry_noreturn(&tag) {
                        Ok(_) => {},
                        Err(e) => {
                            state.connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
                            return Err(e)
                        }
                    }
                }
            }

            let password_store_tag_for_user_name = authentication_helper::password_store_tag_for_user_name_hash(&state.user_name_hash);
            match locked_ubae.delete_entry_noreturn(&password_store_tag_for_user_name) {
                Ok(_) => state.connection.send_fixed_chunk_u8(rbae_mcnp_causes::NO_ERROR as u8)?,
                Err(e) => {
                    state.connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
                    return Err(e)
                }
            }
        }
        self.send_update_callback(arbae_mcnp_causes::UNREGISTER_CAUSE, &state.user_name_hash, None)?;
        Ok(())
    }

    fn send_update_callback(&mut self, operation_completed_cause:i32, user_name_hash:&str, tag_altered:Option<String>) -> Result<(), StorageSystemError> {
        let cloned_server_outer = self.clone();
        let cloned_observers = cloned_server_outer.get_observers_cloned();
        let locked_observers_outer = cloned_observers.lock().unwrap();
        for con in locked_observers_outer.iter() {
            if con.user_name_hash.as_ref() == user_name_hash {
                let cloned_server = self.clone();//arc clone
                let con = con.clone();//arc clone
                let tag_altered = tag_altered.clone();//costly
                let user_name_hash = String::from(user_name_hash);//costly
                thread::spawn(move || {
                    let mut locked_con = con.lock();
                    match locked_con.send_cause(operation_completed_cause) {
                        Ok(_) => {
                            if let Some(tag_altered) = tag_altered {
                                let nonce = authentication_helper::generate_128bit_nonce();
                                match locked_con.send_fixed_chunk_u8_arr(&nonce) {
                                    Ok(_) => {
                                        let encrypted_tag_bytes = authentication_helper::aes_crt_np_128_encrypt(tag_altered[user_name_hash.len()..].as_bytes(), &con.session_key, &nonce);
                                        match locked_con.send_variable_chunk(&encrypted_tag_bytes) {
                                            Ok(_) => {
    //                                      println!("send update callback with tag: {}", tag_altered);
                                            },
                                            Err(e) => {
                                                println!("error sending var chunk: {:?}   --- assuming socket was closed on client side, removing observer", e);
                                                remove_observer_from(&con, cloned_server.get_observers_locked());
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        println!("error sending nonce: {:?}   --- assuming socket was closed on client side, removing observer", e);
                                        remove_observer_from(&con, cloned_server.get_observers_locked());
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            println!("error sending cause: {:?}   --- assuming socket was closed on client side, removing observer", e);
                            remove_observer_from(&con, cloned_server.get_observers_locked());
                        }
                    }
                });
            }
        }
        Ok(())
    }
}

fn remove_observer_from(to_remove:&ArbaeObserverConnection, mut locked_observers:MutexGuard<Vec<ArbaeObserverConnection>>) {
    for i in 0..locked_observers.len() {
        if &locked_observers[i] == to_remove {
            locked_observers.remove(i);
            break;
        }
    }
}

pub struct ArbaeObserverConnection {
    con:Arc<Mutex<McnpConnection>>,
    user_name_hash:Arc<String>,
    session_key:Arc<Vec<u8>>,
    con_id:usize
}
impl ArbaeObserverConnection {
    pub fn new(con:McnpConnection, user_name_hash:String, session_key:Vec<u8>, con_id:usize) -> ArbaeObserverConnection {
        ArbaeObserverConnection {
            con:Arc::new(Mutex::new(con)),
            user_name_hash:Arc::new(user_name_hash),
            session_key:Arc::new(session_key),
            con_id
        }
    }

    pub fn get_uid(observers:&[ArbaeObserverConnection]) -> usize {
        if observers.len() == 0 {
            0
        } else {
            let mut uid = 0;
            for o in observers {
                if o.con_id == uid {
                    uid += 1;
                } else {
                    return uid;
                }
            }
            return uid;
        }
    }
    pub fn lock(&self) -> MutexGuard<McnpConnection> {
        self.con.lock().expect("obtaining lock for connection failed")
    }
}
impl PartialEq<ArbaeObserverConnection> for ArbaeObserverConnection {
    fn eq(&self, other: &ArbaeObserverConnection) -> bool {
        self.con_id == other.con_id
    }
}
impl Clone for ArbaeObserverConnection {
    fn clone(&self) -> Self {
        ArbaeObserverConnection {
            con:self.con.clone(),
            user_name_hash:self.user_name_hash.clone(),//too costly?
            session_key:self.session_key.clone(),
            con_id:self.con_id
        }
    }
}