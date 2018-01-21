use encoding::bytes::libae::LIbae;
use encoding::bytes::libae::LIbaeTraits;
use encoding::bytes::vec_storage_system::VecStorageSystem;
use encoding::bytes::file_storage_system::FileStorageSystem;
use encoding::bytes::libae_storage_system::StorageSystem;
use encoding::bytes::abae::Abae;
use encoding::bytes::abae::AbaeTraits;
use std::fs::File;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::io;
use std::io::BufReader;
use time_keeper::TimeKeeper;
use std::env;
use std::path::Path;
use encoding::bytes::abae_directory_encoder;

#[test]
fn test_li_encoding() {
    let mut time_keeper = TimeKeeper::init();

    let orig_vec0:Vec<u8> = vec![10;10_000_000];  //should not actually be decoded, takes forever. Just a test that it doesn't slow down everything else.
    let orig_vec1:Vec<u8> = vec![10,11];
    let orig_vec2:Vec<u8> = vec![20;100000];
    let orig_vec3:Vec<u8> = vec![30,33,33,33,33,33,33,33,33,31];
    let orig_vec4:Vec<u8> = vec![40,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,41];

    time_keeper.println_set_mark("li created test values in");

    let storage = VecStorageSystem::new_empty();
    let mut libae = LIbae::new(storage);

    time_keeper.println_set_mark("li created storage and libae in");

    libae.li_encode_single(&orig_vec0[..]);
    libae.li_encode_single(&orig_vec1[..]);
    libae.li_encode_single(&orig_vec2[..]);
    libae.li_encode_single(&orig_vec3[..]);
    libae.li_encode_single(&orig_vec4[..]);

    time_keeper.println_set_mark("li encoding took");

    libae.li_skip_single();
    assert_eq!(orig_vec1, libae.li_decode_single().unwrap());
    assert_eq!(orig_vec2, libae.li_decode_single().unwrap());
    assert_eq!(orig_vec3, libae.li_decode_single().unwrap());
    assert_eq!(orig_vec4, libae.li_decode_single().unwrap());
    assert_eq!(None, libae.li_decode_single());
    libae.reset_read_pointer();
    libae.li_skip_single();
    assert_eq!(orig_vec1, libae.li_decode_single().unwrap());

    let size_before_delete = libae.storage_system.content_size();
    assert_eq!(orig_vec2, libae.li_delete_single().unwrap());
    let size_after_delete = libae.storage_system.content_size();
    assert!( size_after_delete < size_before_delete );

    time_keeper.println_set_mark("li decoding and assertions took");
}
#[ignore]
#[test]
fn test_stream_li_encoding() {
    let mut time_keeper = TimeKeeper::init();

    let fp1 = env::home_dir().unwrap().join(Path::new("Desktop/li_stream_test.txt"));
    let fp2 = env::home_dir().unwrap().join(Path::new("Desktop/li_stream_test2.txt"));
    let file_path = fp1.to_str().unwrap();
    let file_path2 = fp2.to_str().unwrap();
    let buf_size:i64 = 8192;
    let write_iterations:i64 = 1000;
    let file_size_after_write:i64 = buf_size*write_iterations;

    let mut file = File::create(file_path).expect("file not found");
    let write_buf = vec![106;buf_size as usize];
    for _ in 0..write_iterations {
        file.write(&write_buf[..]).expect("file write failed");
    }

    time_keeper.println_set_mark("writing test file took: ");

    let mut file = File::open(file_path).expect("file not found");
    let file_length = fs::metadata(file_path).expect("file not found").len();

         //we will require at least(file_size) storage to encode the file. So we preallocate that plus a bit more.
    let storage = VecStorageSystem::new_with_prealloc_cap((file_size_after_write+10) as usize);
    let mut libae = LIbae::new(storage);
    libae.li_encode_single_stream(&mut file, file_length as i64);

    time_keeper.println_set_mark("stream encoding took(from file): ");

    assert!(libae.storage_system.content_size() > file_size_after_write);
    assert!(libae.storage_system.content_size() < file_size_after_write+9);//9 is the max number of bytes used for li


    let mut file2 = File::create(file_path2).expect("file not found");
    let decoded = libae.li_decode_single().expect("internally something went wrong");
    let mut reader = BufReader::new(&decoded[..]);

    io::copy(&mut reader, &mut file2).unwrap();  //pretty cool

    let file1 = File::open(file_path).expect("file not found");
    let file2 = File::open(file_path2).expect("file not found");
    let mut readf1 = BufReader::new(file1);
    let mut readf2 = BufReader::new(file2);
    let mut resf1 = vec![0;file_size_after_write as usize];
    let mut resf2 = vec![0;file_size_after_write as usize];
    readf1.read_to_end(&mut resf1).unwrap();
    readf2.read_to_end(&mut resf2).unwrap();
    assert_eq!(resf1, resf2);

    time_keeper.println_set_mark("decoding and comparing(inefficient using this Storage System) took: ");
}

#[test]
fn test_abae_encoder() {
    let mut time_keeper = TimeKeeper::init();

    let storage = VecStorageSystem::new_empty();
    let mut abae = Abae::new(storage);

    let orig_vec0:Vec<u8> = vec![10;10_000_000];
    let orig_vec1:Vec<u8> = vec![10,11];
    let orig_vec2:Vec<u8> = vec![20;20000];
    let orig_vec3:Vec<u8> = vec![30,33,33,33,33,33,33,33,33,31];
    let orig_vec4:Vec<u8> = vec![40,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,41];

    time_keeper.println_set_mark("abae creating test values took:");

    abae.add_entry("0\n", &orig_vec0[..]);
    abae.add_entry("1", &orig_vec1[..]);
    abae.add_entry("2", &orig_vec2[..]);
    abae.add_entry("3", &orig_vec3[..]);
    abae.add_entry("4", &orig_vec4[..]);

    time_keeper.println_set_mark("abae encoding took:");

    assert_eq!(None, abae.get_entry("5"));//never added
    assert_eq!(orig_vec2, abae.get_entry("2").unwrap());
    assert_eq!(orig_vec1, abae.get_entry("1").unwrap());
    assert_eq!(orig_vec3, abae.get_entry("3").unwrap());
    assert_eq!(orig_vec1, abae.delete_entry("1").unwrap());
    assert_eq!(orig_vec4, abae.get_entry("4").unwrap());
    assert_eq!(None, abae.get_entry("1"));
    abae.delete_entry_noreturn("0\n");
    assert_eq!(None, abae.get_entry("0\n"));

    time_keeper.println_set_mark("abae decoding, deleting and asserting took:");
}
#[test]
fn test_file_storage_system() {
    let mut time_keeper = TimeKeeper::init();

    let fp1 = env::home_dir().unwrap().join(Path::new("Desktop/test_file_storage.txt"));
    let path = fp1.to_str().unwrap();

    let mut storage = FileStorageSystem::create_leave_source_intact(path);

    time_keeper.println_set_mark("file_storage init:");
    let test = storage.content_size();
    time_keeper.println_set_mark("file_storage get content size time:");
    println!("file_storage content_size_bef {}", test);

    let setcontenttest = vec![107,108,109,110,111];
    storage.set_content(&setcontenttest[..]);
    let appendtest = vec![112,113,114,115,116];
    storage.append(&appendtest[..]);

    assert_eq!(storage.subarray(0,setcontenttest.len() as i64).unwrap(), setcontenttest);

    //should yield the same written line again.
    storage.append("\n".as_bytes());
    let mut file = File::open(path).expect("file not found");
    let file_length = fs::metadata(path).expect("file not found").len();
    let start_of_new_line = storage.content_size();
    storage.append_stream(&mut file, file_length as i64);

    assert_eq!(storage.subarray(start_of_new_line,
                                start_of_new_line+setcontenttest.len() as i64).unwrap(), setcontenttest);


    time_keeper.println_set_mark("file_storage assertions took:");
    let orig_vec0:Vec<u8> = vec![10;10_000_000];
    let orig_vec2:Vec<u8> = vec![30,33,33,33,33,33,33,33,33,31];
    time_keeper.println_set_mark("file_storage creating huge test vec took:");
    let size_bef = storage.content_size();
    storage.append(&orig_vec0[..]);
    let size_after = storage.content_size();
    storage.append(&orig_vec2[..]);
    storage.delete(size_bef, size_after);
    time_keeper.println_set_mark("file_storage huge test took:");

    let mut subreader = storage.substream(0,setcontenttest.len() as i64).unwrap();
    let mut buf = vec![0u8;setcontenttest.len()];
    subreader.read(&mut buf).expect("read failed");

    assert_eq!(storage.subarray(0,setcontenttest.len() as i64).unwrap(), setcontenttest);
    time_keeper.println_set_mark("file_storage substream test took:");
}

#[test]
fn test_li_on_file() {
    let mut time_keeper = TimeKeeper::init();

    let orig_vec0:Vec<u8> = vec![10;10_000_000];  //should not actually be decoded, takes forever. Just a test that it doesn't slow down everything else.
    let orig_vec1:Vec<u8> = vec![10,11];
    let orig_vec2:Vec<u8> = vec![20;100000];
    let orig_vec3:Vec<u8> = vec![30,33,33,33,33,33,33,33,33,31];
    let orig_vec4:Vec<u8> = vec![40,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,41];

    time_keeper.println_set_mark("li on file created test values in");

    let fp1 = env::home_dir().unwrap().join(Path::new("Desktop/test_file_storage_libae.txt"));
    let path = fp1.to_str().unwrap();

    let mut storage = FileStorageSystem::create_leave_source_intact(path);
    storage.set_content(&vec![0u8;0][..]);
    let mut libae = LIbae::new(storage);

    time_keeper.println_set_mark("li on file created storage and libae in");

    libae.li_encode_single(&orig_vec0[..]);
    libae.li_encode_single(&orig_vec1[..]);
    libae.li_encode_single(&orig_vec2[..]);
    libae.li_encode_single(&orig_vec3[..]);
    libae.li_encode_single(&orig_vec4[..]);

    time_keeper.println_set_mark("li on file encoding took");

    libae.li_skip_single();
    assert_eq!(orig_vec1, libae.li_decode_single().unwrap());
    assert_eq!(orig_vec2, libae.li_decode_single().unwrap());
    assert_eq!(orig_vec3, libae.li_decode_single().unwrap());
    assert_eq!(orig_vec4, libae.li_decode_single().unwrap());
    assert_eq!(None, libae.li_decode_single());
    libae.reset_read_pointer();
    libae.li_skip_single();
    assert_eq!(orig_vec1, libae.li_decode_single().unwrap());

    let size_before_delete = libae.storage_system.content_size();
    assert_eq!(orig_vec2, libae.li_delete_single().unwrap());
    let size_after_delete = libae.storage_system.content_size();
    assert!( size_after_delete < size_before_delete );

    time_keeper.println_set_mark("li on file decoding and assertions took");
}

#[test]
fn test_abae_on_file() {
    let mut time_keeper = TimeKeeper::init();

    let fp1 = env::home_dir().unwrap().join(Path::new("Desktop/test_file_storage_abae.txt"));
    let path = fp1.to_str().unwrap();

    let mut storage = FileStorageSystem::create_leave_source_intact(path);
    storage.set_content(&vec![0u8;0][..]);
    let mut abae = Abae::new(storage);

    let orig_vec0:Vec<u8> = vec![10;10_000_000];
    let orig_vec1:Vec<u8> = vec![10,11];
    let orig_vec2:Vec<u8> = vec![20;20000];
    let orig_vec3:Vec<u8> = vec![30,33,33,33,33,33,33,33,33,31];
    let orig_vec4:Vec<u8> = vec![40,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,44,41];

    time_keeper.println_set_mark("abae on file creating test values took:");

    abae.add_entry("0\n", &orig_vec0[..]);
    abae.add_entry("1", &orig_vec1[..]);
    abae.add_entry("2", &orig_vec2[..]);
    abae.add_entry("3", &orig_vec3[..]);
    abae.add_entry("4", &orig_vec4[..]);

    time_keeper.println_set_mark("abae on file encoding took:");

    assert_eq!(None, abae.get_entry("5"));//never added
    assert_eq!(orig_vec2, abae.get_entry("2").unwrap());
    assert_eq!(orig_vec1, abae.get_entry("1").unwrap());
    assert_eq!(orig_vec3, abae.get_entry("3").unwrap());
    assert_eq!(orig_vec1, abae.delete_entry("1").unwrap());
    assert_eq!(orig_vec4, abae.get_entry("4").unwrap());
    assert_eq!(None, abae.get_entry("1"));
    abae.delete_entry_noreturn("0\n");
    assert_eq!(None, abae.get_entry("0\n"));

    time_keeper.println_set_mark("abae on file decoding, deleting and asserting took:");
}




#[ignore]
#[test]
fn directory_encoder_test() {
    let test_dir_orig_path = "*******";
    let target_file_path = "*********\\test_rust.abae";
    let test_dir_out_path = "************";

    let mut time_keeper_total = TimeKeeper::init();
    let mut time_keeper = TimeKeeper::init();
    let error_count = abae_directory_encoder::encode(test_dir_orig_path, target_file_path);
    println!("Encoding finished(with {} errors)!", error_count);
    time_keeper.println_set_mark("encoding took");
    let error_count = abae_directory_encoder::decode(target_file_path, test_dir_out_path);
    println!("Decoding finished(with {} errors)!", error_count);
    time_keeper.println_set_mark("decoding took");
    time_keeper_total.println_set_mark("Complete dir encoder test took");
}