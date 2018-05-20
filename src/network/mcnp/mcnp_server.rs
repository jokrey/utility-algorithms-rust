use std::net::*;
use std::thread;
use core::str::FromStr;
use super::mcnp_connection::McnpConnection;

pub struct McnpServer {
    pub server_socket:TcpListener
}

impl McnpServer {
    pub fn new(port:u16 ) -> McnpServer {
        McnpServer {
            server_socket:TcpListener::bind(SocketAddr::new(IpAddr::from_str("127.0.0.1").unwrap(), port)).unwrap()
        }
    }

    //new_connection_handler:fn(con:&mut McnpConnection)
    pub fn run_server_listener_loop(&self, new_connection_handler:fn(&mut McnpConnection)) {
        //shorter, but doesn't do "error handling" or show the addr
//        for Ok(incoming_stream) in self.server_socket.incoming() {
//          thread::spawn( move || (new_connection_handler) ( &mut McnpConnection::new_from_stream(stream) ));
//        }
        loop {
            let new_con = self.server_socket.accept();
            match new_con {
                Err(e) => println!("couldn't get client: {:?}", e),
                Ok((stream, addr)) => {
                    println!("new connection from: {} - spawning thread to handle", addr);
                    thread::spawn( move || (new_connection_handler)( &mut McnpConnection::new_from_stream(stream) ));
                }
            }
        }
    }
}