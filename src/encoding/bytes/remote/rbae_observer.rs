use std::io;
use encoding::bytes::remote::rbae_mcnp_causes;
use network::mcnp::mcnp_client::McnpClient;
use network::mcnp::mcnp_connection::McnpConnectionTraits;

pub fn new_remote_update_callback_receiver(addr:&str, port:u16,
                                           update_add:fn(tag:String), update_remove:fn(tag:String), update_set_content:fn()) -> Result<(), io::Error> {
    let mut client = McnpClient::new(addr, port);
    client.send_cause(rbae_mcnp_causes::INITIAL_CONNECTION_CAUSE__IS_OBSERVER)?;

    loop {
        match client.read_cause()? {
//            rbae_mcnp_causes::UPDATE_CALLBACK => {
//                match client.read_fixed_chunk_i32()? {
                    rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR | rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR_NOCHECK => {
                        if let Ok(tag) = String::from_utf8(client.read_variable_chunk()?) {
                            update_add(tag);
                        } else {
                            return Err(io::Error::new(io::ErrorKind::Other, "received tag bytes were not valid utf8"));
                        }
                    },
                    rbae_mcnp_causes::DELETE_ENTRY_BYTE_ARR | rbae_mcnp_causes::DELETE_NO_RETURN => {
                        if let Ok(tag) = String::from_utf8(client.read_variable_chunk()?) {
                            update_remove(tag);
                        } else {
                            return Err(io::Error::new(io::ErrorKind::Other, "received tag bytes were not valid utf8"));
                        }
                    },
                    rbae_mcnp_causes::SET_CONTENT => update_set_content(),
                    _ => println!("unrecognised update kind detected")
//                }
//            },
//            _ => {
//
//            }
        }
    }
}