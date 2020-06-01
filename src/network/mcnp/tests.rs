use std::io::Read;
use std::thread;
use std::time::Duration;

use network::mcnp::mcnp_client::*;
use network::mcnp::mcnp_connection::*;
use network::mcnp::mcnp_server::*;

//Test is standardized:
//

#[test]
fn mcnp_test() {
    const PORT:u16 = 17731;

    let server_thread = thread::spawn(|| {

        let server = McnpServer::new(PORT);
        println!("mcnp_test: ");
        server.run_server_listener_loop(|initial_cause, con| {
            if initial_cause == 1 {
                let b = con.read_fixed_chunk_u8().unwrap();
                con.send_fixed_chunk_u8(b).unwrap();
            }
            DefaultConnectionState{initial_cause}
        }, |typed_cause, con, state| {
            if typed_cause == (1, 12) { //yes, pattern matching :) :)
                con.read_fixed_chunk_u8().unwrap();
                let int32 = con.read_fixed_chunk_i32().unwrap();
                con.read_fixed_chunk_i64().unwrap();
                con.read_fixed_chunk_f64().unwrap();
                con.send_fixed_chunk_i32(int32).unwrap();
            } else if typed_cause == (1, 8) {
                let bytes = con.read_variable_chunk().unwrap();
                con.send_variable_chunk(&bytes).unwrap();
                let str = String::from_utf8(con.read_variable_chunk().unwrap()).unwrap();
                con.send_variable_chunk(str.as_bytes()).unwrap();
                match con.read_variable_chunk() {
                    Ok(_) => {},
                    Err(_) => {}, // will throw error on unwrap - is "unsupported" none type
                };
            }
            return state;
        });

    });

    thread::sleep(Duration::from_millis(2000));

    run_client_test(PORT);

//    server_thread.join().expect("woah. this never failed before");//if left out the thread is automatically dropped. Which can be cool, but also annoying.
}

fn run_client_test(port: u16) {
    let mut client = McnpClient::new("127.0.0.1", port);

    client.send_cause(1).unwrap();
    let send_b = 133u8;
    client.send_fixed_chunk_u8(send_b).unwrap();

    let read_b = client.read_fixed_chunk_u8().unwrap();
    assert_eq!(send_b, read_b);


    client.send_cause(12).unwrap();
    client.send_fixed_chunk_u8(1).unwrap();

    let send_i32:i32 = 9122303;
    client.send_fixed_chunk_i32(send_i32).unwrap();

    client.send_fixed_chunk_i64(1238234594375345).unwrap();
    client.send_fixed_chunk_f64(1238234594375345.072342834728341).unwrap();


    let read_i32 = client.read_fixed_chunk_i32().unwrap();
    assert_eq!(send_i32, read_i32);


    client.send_cause(8).unwrap();

    let send_vec_stream = vec![2u8;100000];
    client.send_variable_chunk(&send_vec_stream).expect("send err");
    let mut read_vec_stream_back = client.read_variable_chunk_as_stream().unwrap();
    let mut stream_buf = vec![0u8; read_vec_stream_back.1 as usize];
    read_vec_stream_back.0.read_exact(&mut stream_buf).unwrap();
    assert_eq!(send_vec_stream, stream_buf);

    let send_string = "";
    client.send_variable_chunk(send_string.as_bytes()).unwrap();
    let read_string = String::from_utf8(client.read_variable_chunk().unwrap()).unwrap();
    assert_eq!(send_string, &read_string);

    client.start_variable_chunk(-1).unwrap();
}