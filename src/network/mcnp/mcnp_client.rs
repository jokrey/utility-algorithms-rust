use core::str::FromStr;
use std::net::*;

use super::mcnp_connection::McnpConnection;

//semantic stuff

pub struct McnpClient {}

impl McnpClient {
    pub fn new(addr:&str, port:u16) -> McnpConnection {
        return McnpConnection::new_from_stream(TcpStream::connect(SocketAddr::new(IpAddr::from_str(addr).unwrap(), port)).expect("cannot connect to server"));
    }
}