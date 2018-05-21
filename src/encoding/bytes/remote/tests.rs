use encoding::bytes::file_storage_system::FileStorageSystem;
use encoding::bytes::ubae::Ubae;
use encoding::bytes::ubae::UbaeTraits;
use encoding::bytes::remote::rbae::Rbae;
use encoding::bytes::remote::rbae_server;
use encoding::bytes::remote::rbae_mcnp_causes;
use encoding::bytes::remote::rbae_observer;
use std::env;
use std::path::Path;
use std::thread;
use std::time::Duration;
use network::mcnp::mcnp_connection::McnpConnectionTraits;
use std::str;


#[test]
fn rbae_test() {
    const PORT:u16 = rbae_mcnp_causes::DEFAULT_SERVER_PORT;

    let rbae_storage_file = env::home_dir().unwrap().join(Path::new("Desktop/rbae_storage_file.txt"));
    let rbae_storage_path = rbae_storage_file.to_str().unwrap();
    let mut rbae_server = rbae_server::RbaeServer::new_rbae(PORT, Ubae::new(FileStorageSystem::create_leave_source_intact(rbae_storage_path)));

    rbae_server.add_cause_handler(667, |_, state| {
        println!("Client send what is assumed to be some log message: {}", str::from_utf8(&state.read_variable_chunk().expect("server custom receive from client failed")).unwrap());
        Ok(())
    });

    let rbae_clone_for_thread = rbae_server.clone();
    let _joinhandle =
        thread::spawn(|| {
            rbae_clone_for_thread.run_logic_loop();
        });

    thread::sleep(Duration::from_millis(2000));



    { //block so that rbae is dropped
        let mut rbae = Rbae::new("127.0.0.1", PORT);
        rbae.set_content(&[]).expect("set content failed");

        let val1 = vec![1, 2, 3, 4, 5, 32, 56, 21, 6, 7, 7];

        assert_eq!(false, rbae.tag_exists("1").unwrap());
        rbae.add_entry("1", &val1).expect("add entry 1 failed");
        assert_eq!(val1, rbae.get_entry("1").unwrap().unwrap());
        assert_eq!(val1.len() as i64, rbae.tag_length("1").unwrap());
        assert_eq!(true, rbae.tag_exists("1").unwrap());

        let val2 = vec![2u8; 1000];
        rbae.add_entry("2", &val2).expect("add entry 2 failed");
        assert_eq!(val2, rbae.get_entry("2").unwrap().unwrap());

        assert_eq!(val1, rbae.delete_entry("1").unwrap().unwrap());
        assert_eq!(false, rbae.tag_exists("1").unwrap());

        //server sees client
        assert_eq!(true, rbae_server.tag_exists("2").unwrap());
        assert_eq!(false, rbae_server.tag_exists("1").unwrap());

        rbae.delete_entry_noreturn("2").expect("delete entry no return");
        let exists2 = rbae.tag_exists("2").unwrap();
        assert_eq!(false, exists2);
        println!("echo1: {}", exists2);
        let exists_some_bs = rbae.tag_exists("123123").unwrap();
        println!("echo2: {}", exists_some_bs);

        rbae_server.add_entry("test", &val1).expect("adding entry to server failed");
        assert_eq!(val1, rbae_server.get_entry("test").unwrap().unwrap());

        //client sees server
        assert_eq!(val1, rbae.get_entry("test").unwrap().unwrap());

        //client can send independant from server communication:
        rbae.client.send_cause(667).unwrap();
        rbae.client.send_variable_chunk("whats up. I am a client.".as_bytes()).unwrap();
    }


    { //currently the thread is not closable
        let _handler = thread::spawn(move || {
            rbae_observer::new_remote_update_callback_receiver("127.0.0.1", PORT,
                                                               |tag| {
                                                                   println!("update add! tag: {}", tag);
                                                               },
                                                               |tag| {
                                                                   println!("update remove! tag: {}", tag);
                                                               },
                                                               || {
                                                                   println!("update set content");
                                                               }
            ).expect("registering remote callback receiver failed");
        });
        let mut rbae = Rbae::new("127.0.0.1", PORT);
        rbae.set_content(&[]).expect("set_content");
        let data = vec![12,12,13,14,15,16,17,18,19];
        rbae.add_entry("bababedi", &data).expect("add_entry bababedi failed");
        rbae.add_entry("bubedi", &data).expect("add_entry bubedi failed");

        rbae.delete_entry("bababedi").expect("delete_entry bababedi failed");
        rbae.delete_entry_noreturn("bubedi").expect("delete_entry_noreturn bubedi failed");
    }

//    _joinhandle.join().expect("woah. this never failed before");//if left out the thread is automatically dropped. Which can be cool, but also annoying.
}