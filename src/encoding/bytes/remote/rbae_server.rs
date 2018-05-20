use network::mcnp::mcnp_server::McnpServer;
use encoding::bytes::ubae::Ubae;
use encoding::bytes::file_storage_system::FileStorageSystem;
use std::thread;
use network::mcnp::mcnp_connection::McnpConnection;
use network::mcnp::mcnp_connection::McnpConnectionTraits;
use std::sync::Mutex;
use std::sync::Arc;
use encoding::bytes::remote::rbae_mcnp_causes;
use encoding::bytes::ubae::UbaeTraits;
use encoding::bytes::libae::LIbae;
use encoding::bytes::vec_storage_system::VecStorageSystem;
use encoding::bytes::libae::LIbaeTraits;
use std::sync::atomic::{AtomicUsize, Ordering};
use encoding::bytes::libae_storage_system::StorageSystemError;
use std::io::Read;
use encoding::bytes::Substream;
use std::fs::File;
use std::sync::MutexGuard;
use std;

pub struct ObserverConnection {
    con:Arc<Mutex<McnpConnection>>,
    con_id:usize
}
impl ObserverConnection {
    pub fn new(con:McnpConnection, con_id:usize) -> ObserverConnection {
        ObserverConnection {
            con:Arc::new(Mutex::new(con)),
            con_id
        }
    }

    pub fn get_uid(observers:&[ObserverConnection]) -> usize {
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
impl PartialEq<ObserverConnection> for ObserverConnection {
    fn eq(&self, other: &ObserverConnection) -> bool {
        self.con_id == other.con_id
    }
}
impl Clone for ObserverConnection {
    fn clone(&self) -> Self {
        ObserverConnection {
            con:self.con.clone(),
            con_id:self.con_id
        }
    }
}

pub struct RbaeServer<O>
    where O: std::marker::Send + Clone + PartialEq<O> {
    port:u16,
    ubae:Arc<Mutex<Ubae<FileStorageSystem>>>,

    observers:Arc<Mutex<Vec<O>>>  //it looks ugly, but it actually is rather nice
}
impl <O> RbaeServer<O>
    where O: std::marker::Send + Clone + PartialEq<O> {
    pub fn new(port:u16, ubae:Ubae<FileStorageSystem>) -> RbaeServer<O> {
        RbaeServer {
            port,
            ubae:Arc::new(Mutex::new(ubae)),
            observers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn ubae_clone_lock(&mut self) -> MutexGuard<Ubae<FileStorageSystem>> {
        return self.ubae.lock().expect("obtaining lock failed");
    }
    pub fn get_observers_cloned(&self) -> Arc<Mutex<Vec<O>>> {
        self.observers.clone()
    }
    pub fn get_observers_locked(&self) -> MutexGuard<Vec<O>> {
        self.observers.lock().expect("locking observer failed")
    }
}

impl RbaeServer<ObserverConnection> {
    pub fn add_observing_client(&mut self, connection:McnpConnection) {
        let mut obs = self.observers.lock().unwrap();
        let con_id = ObserverConnection::get_uid(&obs);
        obs.push(ObserverConnection::new(connection, con_id));
    }
}

impl<O> Clone for RbaeServer<O>
    where O: std::marker::Send + Clone + PartialEq<O> {
    fn clone(&self) -> Self {
        RbaeServer {
            port:self.port,
            ubae:self.ubae.clone(),
            observers:self.observers.clone()
        }
    }
}

pub fn general_run_logic_loop<O: 'static>(rbae_server:RbaeServer<O>, handle_new_connection:fn(&mut RbaeServer<O>, McnpConnection))
    where O: std::marker::Send + Clone + PartialEq<O> {
    let mcnp_server = McnpServer::new(rbae_server.port);

    let mut total_number_of_connections = 0;
    let connection_count = Arc::new(AtomicUsize::new(0));
    loop {
        let new_con = mcnp_server.server_socket.accept();
        match new_con {
            Err(e) => println!("couldn't get client: {:?}", e),
            Ok((stream, addr)) => {
                total_number_of_connections+=1;
                println!("new connection(number {}) from: {} - spawning thread to handle", total_number_of_connections, addr);

                let mut alias = rbae_server.clone();
                let mut connection_count_clone = connection_count.clone();
                thread::Builder::new().name(format!("thread <connecting to: {}>", addr.to_string())).spawn(move || {
                    connection_count_clone.fetch_add(1, Ordering::Relaxed);
                    println!("spawned thread. number of currently connected clients: {}", connection_count_clone.load(Ordering::Relaxed));

                    //important bit
                    handle_new_connection(&mut alias, McnpConnection::new_from_stream(stream));

                    connection_count_clone.fetch_sub(1, Ordering::Relaxed);
                    println!("thread finished. number of currently connected clients: {}", connection_count_clone.load(Ordering::Relaxed));

                }).expect("spawing thread failed. wow this is really going downhill now.....");
            }
        }
    }
}

pub fn run_logic_loop(rbae_server:RbaeServer<ObserverConnection>) {
    general_run_logic_loop(rbae_server, handle_new_connection)
}





pub fn handle_new_connection(server:&mut RbaeServer<ObserverConnection>, mut connection:McnpConnection) {
    match connection.read_cause() {
        Ok(rbae_mcnp_causes::INITIAL_CONNECTION_CAUSE__IS_OBSERVER) => {

            server.add_observing_client(connection);

        },
        Ok(rbae_mcnp_causes::INITIAL_CONNECTION_CAUSE__IS_CLIENT) => {

            loop {
                match connection.read_cause() {
                    Err(_) => break, //indicates eof or error, hopefully eof
                    Ok(cause) => {
//                println!("cause: {}", cause);
                        match handle_request_by(server, cause, &mut connection) {
                            Err(e) => {
                                println!("Error reading cause: {:?}", e)
//                        break
                            }, //todo to break or not to break
                            Ok(_) => {}
                        }
                    }
                }
            }

        },
        _ => println!("client send unrecognised initial connection cause")
    }

}

pub fn handle_request_by(server:&mut RbaeServer<ObserverConnection>, cause:i32, connection:&mut McnpConnection) -> Result<(), StorageSystemError> {
    match cause {
        rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR => {
            handle_add_entry_byte_arr_by(server, connection)
        },
        rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR_NOCHECK => {
            handle_add_entry_byte_arr_nocheck_by(server, connection)
        },
        rbae_mcnp_causes::GET_ENTRY_BYTE_ARR => {
            handle_get_entry_byte_arr_by(server, connection)
        },
        rbae_mcnp_causes::DELETE_ENTRY_BYTE_ARR => {
            handle_delete_entry_byte_arr_by(server, connection)
        },

        rbae_mcnp_causes::DELETE_NO_RETURN => {
            handle_delete_entry_noreturn_by(server, connection)
        },
        rbae_mcnp_causes::EXISTS => {
            handle_exists_by(server, connection)
        },
        rbae_mcnp_causes::GET_TAGS => {
            handle_get_tags_by(server, connection)
        },
        rbae_mcnp_causes::LENGTH => {
            handle_length_by(server, connection)
        },
        rbae_mcnp_causes::SET_CONTENT => {
            handle_set_content_by(server, connection)
        },
        rbae_mcnp_causes::GET_CONTENT => {
            handle_get_content_by(server, connection)
        },

        _ => {
            Err(StorageSystemError::new("unknown cause received"))
        }
    }
}


fn handle_add_entry_byte_arr_by(server:&mut RbaeServer<ObserverConnection>, connection:&mut McnpConnection) -> Result<(), StorageSystemError> {
    let tag = String::from_utf8(connection.read_variable_chunk()?).unwrap();

    //todo, this doesn't work for unknown reasons:
    //interestingly it works in the nocheck version below
//    let mut entry_to_add = connection.read_variable_chunk_as_stream()?;
//    match ubae.add_entry_from_stream(&tag, &mut entry_to_add.0, entry_to_add.1) {
//        Ok(_) => {
//            connection.send_fixed_chunk_u8(rbae_mcnp_causes::NO_ERROR as u8)?;
//            Ok(())
//        },
//        Err(e) => {
//            connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
//            Err(e)
//        }
//    }

    let entry_to_add = connection.read_variable_chunk()?;
    match server.add_entry(&tag, &entry_to_add) {
        Ok(_) => {
            connection.send_fixed_chunk_u8(rbae_mcnp_causes::NO_ERROR as u8)?;
            send_update_callback(server, rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR, Some(tag))?;
            Ok(())
        },
        Err(e) => {
            connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
            Err(e)
        }
    }
}

//todo succeptible to a denial of service attack.
//todo The server can be endlessly blocked should someone send a stream with mcnp length indication n, but only supply m(where m < n) bytes without closing the stream
fn handle_add_entry_byte_arr_nocheck_by(server:&mut RbaeServer<ObserverConnection>, connection:&mut McnpConnection) -> Result<(), StorageSystemError> {
    let tag = String::from_utf8(connection.read_variable_chunk()?).unwrap();


//    let entry_to_add = connection.read_variable_chunk()?;
//    match ubae.add_entry_nocheck(&tag, &entry_to_add) {
//        Ok(_) => {
//            connection.send_fixed_chunk_u8(rbae_mcnp_causes::NO_ERROR as u8)?;
//            Ok(())
//        },
//        Err(e) => {
//            connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
//            Err(e)
//        }
//    }

    let mut entry_to_add = connection.read_variable_chunk_as_stream()?;
    match server.add_entry_from_stream_nocheck(&tag, &mut entry_to_add.0, entry_to_add.1) {
        Ok(_) => {
            connection.send_fixed_chunk_u8(rbae_mcnp_causes::NO_ERROR as u8)?;
            send_update_callback(server, rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR_NOCHECK, Some(tag))?;
            Ok(())
        },
        Err(e) => {
            connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
            Err(e)
        }
    }
}

fn handle_get_entry_byte_arr_by(server:&mut RbaeServer<ObserverConnection>, connection:&mut McnpConnection) -> Result<(), StorageSystemError> {
    let tag = String::from_utf8(connection.read_variable_chunk()?).unwrap();

    match server.get_entry_as_stream(&tag) {
        Ok(Some(mut stream)) => connection.send_variable_chunk_from_stream(&mut stream.0, stream.1)?,
        Ok(None)             => connection.start_variable_chunk(-1)?,
        Err(e)  => {
            connection.start_variable_chunk(-1)?;
            return Err(e)
        }
    }
    Ok(())
}

fn handle_delete_entry_byte_arr_by(server:&mut RbaeServer<ObserverConnection>, connection:&mut McnpConnection) -> Result<(), StorageSystemError> {
    let tag = String::from_utf8(connection.read_variable_chunk()?).unwrap();

    match server.delete_entry(&tag) {
        Ok(Some(deleted_entry)) => {
            connection.send_variable_chunk(&deleted_entry)?;
            send_update_callback(server, rbae_mcnp_causes::DELETE_ENTRY_BYTE_ARR, Some(tag))?;
        },
        Ok(None)                => connection.start_variable_chunk(-1)?,
        Err(e)  => {
            connection.start_variable_chunk(-1)?;
            return Err(e)
        }
    }
    Ok(())
}

fn handle_delete_entry_noreturn_by(server:&mut RbaeServer<ObserverConnection>, connection:&mut McnpConnection) -> Result<(), StorageSystemError> {
    let tag = String::from_utf8(connection.read_variable_chunk()?).unwrap();

    match server.delete_entry_noreturn(&tag) {
        Ok(true)  => {
            connection.send_fixed_chunk_u8(rbae_mcnp_causes::TRUE as u8)?;
            send_update_callback(server, rbae_mcnp_causes::DELETE_NO_RETURN, Some(tag))?;
        },
        Ok(false) => connection.send_fixed_chunk_u8(rbae_mcnp_causes::FALSE as u8)?,
        Err(e) => {
            connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
            return Err(e)
        }
    }
    Ok(())
}

fn handle_exists_by(server:&mut RbaeServer<ObserverConnection>, connection:&mut McnpConnection) -> Result<(), StorageSystemError> {
    let tag = String::from_utf8(connection.read_variable_chunk()?).unwrap();

    match server.tag_exists(&tag) {
        Ok(true)  => connection.send_fixed_chunk_u8(rbae_mcnp_causes::TRUE as u8)?,
        Ok(false) => connection.send_fixed_chunk_u8(rbae_mcnp_causes::FALSE as u8)?,
        Err(e) => {
            connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
            return Err(e)
        }
    }
    Ok(())
}

fn handle_get_tags_by(server:&mut RbaeServer<ObserverConnection>, connection:&mut McnpConnection) -> Result<(), StorageSystemError> {
    let mut libae = LIbae::new(VecStorageSystem::new_empty());

    let get_tags = server.get_tags()?;
    for tag in get_tags {
        libae.li_encode_single(tag.as_bytes()).expect("wow. now vec storage has issues now... it literally can't, but fine. fine. I am ok with this. Kind of. Not really. WHAT THE HELL? HOW IT DOESN'T EVEN RETURN ERROR EVER!");
    }
    let encoded_tags = libae.get_content()?;
    connection.send_variable_chunk(&encoded_tags)?;
    Ok(())
}

fn handle_length_by(server:&mut RbaeServer<ObserverConnection>, connection:&mut McnpConnection) -> Result<(), StorageSystemError> {
    let tag = String::from_utf8(connection.read_variable_chunk()?).unwrap();

    match server.tag_length(&tag) {
        Ok(tag_length) => {
            connection.send_fixed_chunk_i64(tag_length)?;
            Ok(())
        },
        Err(e) => {
            connection.send_fixed_chunk_i64(rbae_mcnp_causes::ERROR as i64)?;
            Err(e)
        }
    }
}

fn handle_set_content_by(server:&mut RbaeServer<ObserverConnection>, connection:&mut McnpConnection) -> Result<(), StorageSystemError> {
    let new_content = connection.read_variable_chunk()?;

    match server.set_content(&new_content) {
        Ok(_) => {
            connection.send_fixed_chunk_u8(rbae_mcnp_causes::NO_ERROR as u8)?;
            send_update_callback(server, rbae_mcnp_causes::SET_CONTENT, None)?;
            Ok(())
        },
        Err(e) => {
            connection.send_fixed_chunk_u8(rbae_mcnp_causes::ERROR as u8)?;
            Err(e)
        }
    }
}

fn handle_get_content_by(server:&mut RbaeServer<ObserverConnection>, connection:&mut McnpConnection) -> Result<(), StorageSystemError> {
    let content = server.get_content()?;
    connection.send_variable_chunk(&content)?;
    Ok(())
}


fn send_update_callback(server:&mut RbaeServer<ObserverConnection>, operation_completed_cause:i32, tag_altered:Option<String>) -> Result<(), StorageSystemError> {
    let cloned_server_outer = server.clone();
    let cloned_observers = cloned_server_outer.observers.clone();
    let locked_observers_outer = cloned_observers.lock().unwrap();
    for con in locked_observers_outer.iter() {
        let cloned_server = server.clone();
        let con = con.clone();
        let tag_altered = tag_altered.clone();
        thread::spawn(move || {
            let mut locked_con = con.lock();
            match locked_con.send_cause(operation_completed_cause) {
                Ok(_) => {
                    if let Some(tag_altered) = tag_altered {
                        match locked_con.send_variable_chunk(tag_altered.as_bytes()) {
                            Ok(_) => {
//                                println!("send update callback with tag: {}", tag_altered);
                            },
                            Err(e) => {
                                println!("error sending var chunk: {:?}   --- assuming socket was closed on client side, removing observer", e);
//                                remove_observer(cloned_server, index, occupied_observer_spots);
                                let mut locked_observers = cloned_server.observers.lock().expect("locking observer failed");
                                for i in 0..locked_observers.len() {
                                    if locked_observers[i] == con {
                                        locked_observers.remove(i);
                                        break;
                                    }
                                }
                            }
                        }
                    }
//                    else {
//                        println!("send update callback without tag");
//                    }
                },
                Err(e) => {
                    println!("error sending cause: {:?}   --- assuming socket was closed on client side, removing observer", e);
                    let mut locked_observers = cloned_server.observers.lock().expect("locking observer failed");
                    for i in 0..locked_observers.len() {
                        if locked_observers[i] == con {
                            locked_observers.remove(i);
                            break;
                        }
                    }
                }
            }
        });
    }
    Ok(())
}

//fn remove_observer(cloned_server:RbaeServer, con_index:usize, occupied_observer_spots:Arc<Mutex<Vec<bool>>>) {
//    let mut locked_observers = cloned_server.observers.lock().expect("locking observer failed");
//    let mut actual_index = 0;
//    let mut occupied_spots = occupied_observer_spots.lock().unwrap();
//    for i in 0..occupied_spots.len() {
//        let occupied = occupied_spots[i];
//        if occupied {
//            if i == con_index {
//                locked_observers.remove(actual_index);
//                occupied_spots[i] = false;
//            }
//            actual_index+=1;
//        }
//    }
//}


//a wrapper to use the ubae traits directly on the server.
//it is absolutly required because it additionally provides the thread safety needed.
//     itself required by the consistency constraint of libae and ubae itself
impl<O> UbaeTraits<File> for RbaeServer<O>
    where O: std::marker::Send + Clone + PartialEq<O> {
    fn set_content(&mut self, bytes: &[u8]) -> Result<(), StorageSystemError> {
        let ubae = self.ubae.clone();
        let mut locked_ubae = ubae.lock().unwrap();
        locked_ubae.set_content(bytes)
    }

    fn get_content(&mut self) -> Result<Vec<u8>, StorageSystemError> {
        let ubae = self.ubae.clone();
        let mut locked_ubae = ubae.lock().unwrap();
        locked_ubae.get_content()
    }

    fn get_tags(&mut self) -> Result<Vec<String>, StorageSystemError> {
        let ubae = self.ubae.clone();
        let mut locked_ubae = ubae.lock().unwrap();
        locked_ubae.get_tags()
    }

    fn tag_exists(&mut self, tag: &str) -> Result<bool, StorageSystemError> {
        let ubae = self.ubae.clone();
        let mut locked_ubae = ubae.lock().unwrap();
        locked_ubae.tag_exists(tag)
    }

    fn tag_length(&mut self, tag: &str) -> Result<i64, StorageSystemError> {
        let ubae = self.ubae.clone();
        let mut locked_ubae = ubae.lock().unwrap();
        locked_ubae.tag_length(tag)
    }

    fn get_entry(&mut self, tag: &str) -> Result<Option<Vec<u8>>, StorageSystemError> {
        let ubae = self.ubae.clone();
        let mut locked_ubae = ubae.lock().unwrap();
        locked_ubae.get_entry(tag)
    }

    fn get_entry_as_stream(&mut self, tag: &str) -> Result<Option<(Substream<File>, i64)>, StorageSystemError> {
        let ubae = self.ubae.clone();
        let mut locked_ubae = ubae.lock().unwrap();
        locked_ubae.get_entry_as_stream(tag)
    }

    fn delete_entry(&mut self, tag: &str) -> Result<Option<Vec<u8>>, StorageSystemError> {
        let ubae = self.ubae.clone();
        let mut locked_ubae = ubae.lock().unwrap();
        locked_ubae.delete_entry(tag)
    }

    fn delete_entry_noreturn(&mut self, tag: &str) -> Result<bool, StorageSystemError> {
        let ubae = self.ubae.clone();
        let mut locked_ubae = ubae.lock().unwrap();
        locked_ubae.delete_entry_noreturn(tag)
    }

    fn add_entry(&mut self, tag: &str, content: &[u8]) -> Result<(), StorageSystemError> {
        let ubae = self.ubae.clone();
        let mut locked_ubae = ubae.lock().unwrap();
        locked_ubae.add_entry(tag, content)
    }

    fn add_entry_nocheck(&mut self, tag: &str, content: &[u8]) -> Result<(), StorageSystemError> {
        let ubae = self.ubae.clone();
        let mut locked_ubae = ubae.lock().unwrap();
        locked_ubae.add_entry_nocheck(tag, content)
    }

    fn add_entry_from_stream(&mut self, tag: &str, stream: &mut Read, stream_length: i64) -> Result<(), StorageSystemError> {
        let ubae = self.ubae.clone();
        let mut locked_ubae = ubae.lock().unwrap();
        locked_ubae.add_entry_from_stream(tag, stream, stream_length)
    }

    fn add_entry_from_stream_nocheck(&mut self, tag: &str, stream: &mut Read, stream_length: i64) -> Result<(), StorageSystemError> {
        let ubae = self.ubae.clone();
        let mut locked_ubae = ubae.lock().unwrap();
        locked_ubae.add_entry_from_stream_nocheck(tag, stream, stream_length)
    }
}