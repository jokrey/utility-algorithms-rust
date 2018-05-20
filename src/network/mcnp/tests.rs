use std::io::Read;
use network::mcnp::mcnp_connection::*;
use network::mcnp::mcnp_server::*;
use network::mcnp::mcnp_client::*;
use std::thread;
use std::time::Duration;


#[test]
fn mcnp_test() {
    const PORT:u16 = 44431;

    /*let joinhandle = */thread::spawn(|| {

        let server = McnpServer::new(PORT);
        println!("mcnp_test: ");
        server.run_server_listener_loop(
            |mcnp_connection| {
                println!("woah function pointers");
                let read_i32 = mcnp_connection.read_fixed_chunk_i32().unwrap();
                println!("read_i32: {}", read_i32);
                mcnp_connection.send_fixed_chunk_i32(read_i32).expect("send err");

                match mcnp_connection.read_cause().unwrap() {
                    1 => {
                        let rec = mcnp_connection.read_variable_chunk().unwrap();
                        println!("rec on server: {:?}", rec);
                        mcnp_connection.send_variable_chunk(&rec).expect("send err");

                        let mut stream = mcnp_connection.read_variable_chunk_as_stream().unwrap();
                        let mut stream_buf = vec![0u8; stream.1 as usize];
                        stream.0.read_exact(&mut stream_buf).unwrap();
                        println!("rec on server stream_buf: {:?}", stream_buf);
                        mcnp_connection.send_variable_chunk(&stream_buf).expect("send err");

                        let send_i64 = 9235123666;
                        mcnp_connection.send_fixed_chunk_i64(send_i64).unwrap();
                        let read_i64 = mcnp_connection.read_fixed_chunk_i64().expect("read i64");
                        assert_eq!(send_i64, read_i64);
                    },
                    _ => {}
                }
            }
        );
        println!("mcnp_test done");

    });

    thread::sleep(Duration::from_millis(2000));

    let mut client = McnpClient::new("127.0.0.1", PORT);
    let send_i32:i32 = 9122303;
    client.send_fixed_chunk_i32(send_i32).expect("send err");
    let read_i32:i32 = client.read_fixed_chunk_i32().unwrap();
    assert_eq!(send_i32, read_i32);
    client.send_cause(1).expect("send err");
    let send_chunk = vec![1,2,3,4,5,6,7];
    client.send_variable_chunk(&send_chunk).expect("send err");
    let read_chunk = client.read_variable_chunk().unwrap();
    assert_eq!(send_chunk, read_chunk);

    let send_vec_stream = vec![2u8;100000];
    client.send_variable_chunk(&send_vec_stream).expect("send err");
    let mut read_vec_stream_back = client.read_variable_chunk_as_stream().unwrap();
    let mut stream_buf = vec![0u8; read_vec_stream_back.1 as usize];
    read_vec_stream_back.0.read_exact(&mut stream_buf).unwrap();
    assert_eq!(send_vec_stream, stream_buf);

    let let_i64 = client.read_fixed_chunk_i64().unwrap();
    client.send_fixed_chunk_i64(let_i64).unwrap();


//    joinhandle.join().expect("woah. this never failed before");//if left out the thread is automatically dropped. Which can be cool, but also annoying.
}