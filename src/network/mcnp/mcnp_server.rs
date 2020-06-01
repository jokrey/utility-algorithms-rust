use core::str::FromStr;
use std::net::*;
use std::thread;

use network::mcnp::mcnp_connection::McnpConnectionTraits;

use super::mcnp_connection::McnpConnection;

pub struct McnpServer {
    pub server_socket:TcpListener
}
pub trait ConnectionState {
    fn get_initial_cause(self) -> i32;
}
pub struct DefaultConnectionState {
    pub initial_cause:i32
}
impl ConnectionState for DefaultConnectionState {
    fn get_initial_cause(self) -> i32 {self.initial_cause}
}


impl McnpServer {
    pub fn new(port:u16 ) -> McnpServer {
        McnpServer {
            server_socket:TcpListener::bind(SocketAddr::new(IpAddr::from_str("127.0.0.1").unwrap(), port)).unwrap()
        }
    }

    //new_connection_handler:fn(con:&mut McnpConnection)
    pub fn run_server_listener_loop<CT:'static + ConnectionState>(&self, new_connection: fn(initial_cause:i32, con:&mut McnpConnection) -> CT, handle_interaction: fn(typed_cause:(i32, i32), con:&mut McnpConnection, state:CT) -> CT) {
        //shorter and cooler, but doesn't do "error handling" or show the addr
//        for Ok(incoming_stream) in self.server_socket.incoming() {
//          thread::spawn( move || (new_connection_handler) ( &mut McnpConnection::new_from_stream(stream) ));
//        }
        loop {
            let new_con = self.server_socket.accept();
            match new_con {
                Err(e) => println!("couldn't get client: {:?}", e),
                Ok((stream, addr)) => {
                    println!("new connection from: {} - spawning thread to handle", addr);
                    thread::spawn( move || {
                        let mut con = McnpConnection::new_from_stream(stream);
                        let initial_cause = con.read_cause().expect("reading initial cause failed");
                        let mut state = (new_connection)(initial_cause, &mut con);
                        loop {
                            let concrete_cause = match con.read_cause() {
                                Ok(cause) => cause,
                                Err(e) => {
                                    println!("{}",e.to_string());
                                    break;
                                }
                            };
                            state = (handle_interaction)((initial_cause, concrete_cause), &mut con, state);
                        }
                    });
                }
            }
        }
    }
}