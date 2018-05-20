///author: jokrey

use encoding::bytes::file_storage_system::FileStorageSystem;
use encoding::bytes::ubae::Ubae;
use encoding::bytes::ubae::UbaeTraits;
use std::env;
use std::path::Path;
use std::thread;
use std::time::Duration;
use encoding::bytes::remote::authenticated::arbae::Arbae;
use encoding::bytes::remote::authenticated::arbae_server;
use encoding::bytes::remote::authenticated::arbae_server::ArbaeServer;
use encoding::bytes::remote::authenticated::arbae_mcnp_causes;
use std::vec::Vec;
use encoding::bytes::remote::authenticated::arbae_observer;

#[test]
fn arbae_test() {
    const PORT:u16 = arbae_mcnp_causes::DEFAULT_SERVER_PORT;

    let arbae_storage_file = env::home_dir().unwrap().join(Path::new("Desktop/authenticated_rbae_storage_file.txt"));
    let arbae_storage_path = arbae_storage_file.to_str().unwrap();
    let mut arbae_server = ArbaeServer::new(PORT, Ubae::new(FileStorageSystem::create_leave_source_intact(arbae_storage_path)));
    arbae_server.set_content(&[]).expect("clearing content failed");

    let arbae_clone_for_thread = arbae_server.clone();
    let _joinhandle =
        thread::spawn(|| {
            arbae_server::run_logic_loop(arbae_clone_for_thread);
        });

    thread::sleep(Duration::from_millis(2000));

    let pers_test_val = vec![111, 2, 13, 94, 5, 32, 56, 21, 69, 97, 78,76,34,12,32];
    { //block so that rbae is dropped
        let mut arbae = Arbae::register("127.0.0.1", PORT, "larry", "larrydoesn'tknowgoodpasswords").expect("arbae register failed");
        assert!(arbae.set_content(&[]).is_err());//illegal operation fails
        assert!(arbae.get_content().is_err());//illegal operation fails

        let val1 = vec![1, 2, 3, 4, 5, 32, 56, 21, 6, 7, 7];
        assert_eq!(false, arbae.tag_exists("1").expect("tag exists 1"));
        arbae.add_entry("1", &val1).expect("add entry 1 failed");
        assert_eq!(val1, arbae.get_entry("1").expect("get entry 1").unwrap());
        assert_eq!(val1.len() as i64, arbae.tag_length("1").expect("tag_length 1"));
        assert_eq!(true, arbae.tag_exists("1").expect("tag_exists 1"));

        let val2 = vec![2u8; 1000];
        arbae.add_entry("2", &val2).expect("add entry 2 failed");
        assert_eq!(val2, arbae.get_entry("2").expect("get_entry(2").unwrap());

        assert_eq!(val1, arbae.delete_entry("1").expect("delete_entry(1").unwrap());
        assert_eq!(false, arbae.tag_exists("1").unwrap());

        //server sees client
        assert_eq!(true, arbae_server.user_tag_exists("larry", "2").unwrap());
        assert_eq!(false, arbae_server.user_tag_exists("larry", "1").unwrap());

        arbae.delete_entry_noreturn("2").expect("delete entry no return");
        let exists2 = arbae.tag_exists("2").unwrap();
        assert_eq!(false, exists2);
        println!("echo1: {}", exists2);
        let exists_some_bs = arbae.tag_exists("123123").unwrap();
        println!("echo2: {}", exists_some_bs);

        arbae_server.add_user_entry("larry", "test", &val1).expect("adding entry to server failed");
        assert_eq!(val1, arbae_server.get_user_entry("larry", "test").unwrap().unwrap());

        //client sees server
        assert_eq!(val1, arbae.get_entry("test").unwrap().unwrap());

        arbae.add_entry("persistence test", &pers_test_val).expect("add_entry(persistence");
        assert_eq!(pers_test_val, arbae.get_entry("persistence test").unwrap().unwrap());
    }

    assert!(Arbae::login("127.0.0.1", PORT, "larry", "clearly the wrong password").is_err()); //login with wrong password fails
    assert!(Arbae::login("127.0.0.1", PORT, "not_registered_user", "doesn't even matter").is_err()); //login with unregistered user name fails
    assert!(Arbae::register("127.0.0.1", PORT, "bob", "doesn't even matter").is_ok()); //second register works
    let empty_string_vec:Vec<String> = vec![];
    assert_eq!(empty_string_vec, arbae_server.get_user_tags("bob").unwrap()); //bob doesn't have any tags set...

    println!("tags: {:?}", arbae_server.get_tags());

    {
        let mut arbae = Arbae::login("127.0.0.1", PORT, "larry", "larrydoesn'tknowgoodpasswords").expect("arbae login failed");
        assert!(arbae.set_content(&[0,1,2,3,4,11,5,6,12,6,7,4,53,45,34,245,34,5,34,5,34,5,3,45,235,245,245,2,245]).is_err());//illegal operation fails, and doesn't screw anything up

        assert_eq!(pers_test_val, arbae.get_entry("persistence test").unwrap().unwrap());
        assert_eq!(vec![String::from("test"), String::from("persistence test")], arbae.get_tags().unwrap());
        assert_eq!(vec![String::from("test"), String::from("persistence test")], arbae_server.get_user_tags("larry").unwrap());
    }



    {
        let mut bob = Arbae::login("127.0.0.1", PORT, "bob", "doesn't even matter").expect("arbae login failed");
        bob.add_entry("hello my name is ... ", "bob".as_bytes()).unwrap();
        assert_eq!(Some(Vec::from("bob".as_bytes())), bob.get_entry("hello my name is ... ").unwrap());
        bob.unregister().expect("unregister failed");
    }
    {
        assert!(Arbae::login("127.0.0.1", PORT, "bob", "doesn't even matter").is_err());//login in after unregister fails
    }
    {
        let mut bob = Arbae::register("127.0.0.1", PORT, "bob", "doesn't even matter").expect("arbae register failed");
        assert_eq!(None, bob.get_entry("hello my name is ... ").unwrap());//entry doesn't exist after unregister
    }




    { //currently the thread is not closable
        let mut bobby = Arbae::register("127.0.0.1", PORT, "bobby", "doesn't even matter").expect("arbae register failed");
        let _handler = thread::spawn(move || {
            arbae_observer::new_remote_update_callback_receiver("127.0.0.1", PORT,"bobby", "doesn't even matter",
               |tag| {
                    println!("update add! tag: {}", tag);
                    assert_ne!(tag, "not_update_called_tag1");
                    assert_ne!(tag, "not_update_called_tag2");
                },
               |tag| {
                    println!("update remove! tag: {}", tag);
                    assert_ne!(tag, "not_update_called_tag1");
                    assert_ne!(tag, "not_update_called_tag2");
                },
                || {
                    println!("update client_unregistered!");
                }
            ).expect("registering remote callback receiver failed");
        });
        let data = vec![12,12,13,14,15,16,17,18,19];
        bobby.add_entry("bababedi", &data).expect("add_entry bababedi failed");
        bobby.add_entry("bubedi", &data).expect("add_entry bubedi failed");

        bobby.delete_entry("bababedi").expect("delete_entry bababedi failed");
        bobby.delete_entry_noreturn("bubedi").expect("delete_entry_noreturn bubedi failed");

        bobby.unregister().expect("unregister for bobby failed");

        let mut bob = Arbae::login("127.0.0.1", PORT, "bob", "doesn't even matter").expect("arbae login failed");
        bob.add_entry("not_update_called_tag1", &data).expect("add entry failed");;
        bob.delete_entry("not_update_called_tag1").expect("delete_entry entry failed");;
        bob.add_entry("not_update_called_tag2", &data).expect("add entry failed");;
        bob.delete_entry_noreturn("not_update_called_tag1").expect("delete_entry failed");;
    }


//    _joinhandle.join().expect("woah. this never failed before");//if left out the thread is automatically dropped. Which can be cool, but also annoying should one not want that...
}