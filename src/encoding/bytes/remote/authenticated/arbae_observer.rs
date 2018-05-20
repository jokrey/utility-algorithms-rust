use std::io;
use encoding::bytes::remote::rbae_mcnp_causes;
use encoding::bytes::remote::authenticated::arbae_mcnp_causes;
use encoding::bytes::remote::authenticated::authentication_helper;
use network::mcnp::mcnp_connection::McnpConnectionTraits;
use encoding::bytes::remote::authenticated::arbae;
use std::error::Error;

pub fn new_remote_update_callback_receiver(addr:&str, port:u16, user_name:&str, password:&str,
                                           update_add:fn(tag:String), update_remove:fn(tag:String), update_unregister:fn()) -> Result<(), io::Error> {
    match arbae::initialize_connection(addr, port, rbae_mcnp_causes::INITIAL_CONNECTION_CAUSE__IS_OBSERVER, user_name, password) {
        Ok((mut client, sess_key)) => {

            loop {
                match client.read_cause()? {
                    rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR | rbae_mcnp_causes::ADD_ENTRY_BYTE_ARR_NOCHECK => {
                        let nonce = client.read_fixed_chunk_u8_arr(16)?;
                        let encrypted_tag = client.read_variable_chunk()?;
                        if let Ok(tag) = String::from_utf8(authentication_helper::aes_crt_np_128_decrypt(&encrypted_tag, &sess_key, &nonce)) {
                            update_add(tag);
                        } else {
                            return Err(io::Error::new(io::ErrorKind::Other, "received tag bytes were not valid utf8 | add arbar callback"));
                        }
                    },
                    rbae_mcnp_causes::DELETE_ENTRY_BYTE_ARR | rbae_mcnp_causes::DELETE_NO_RETURN => {
                        let nonce = client.read_fixed_chunk_u8_arr(16)?;
                        let encrypted_tag = client.read_variable_chunk()?;
                        if let Ok(tag) = String::from_utf8(authentication_helper::aes_crt_np_128_decrypt(&encrypted_tag, &sess_key, &nonce)) {
                            update_remove(tag);
                        } else {
                            return Err(io::Error::new(io::ErrorKind::Other, "received tag bytes were not valid utf8 | del arbar callback"));
                        }
                    },
                    arbae_mcnp_causes::UNREGISTER_CAUSE => update_unregister(),
                    _ => println!("unrecognised update kind detected")
                }
            }

        },
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.description()))
    }
}