extern crate core;

use encoding::tag_based::bytes::ubae::Ubae;
use transparent_storage::bytes::file_storage_system::FileStorageSystem;
use encoding::tag_based::bytes::ubae::UbaeTraits;
use std::path::PathBuf;
use std::fs;
use std::fs::File;
use transparent_storage::StorageSystem;
use std::path::Path;
use std::io;


///About half the performance of a java program
///   SOO...... WTF??
/// But just sometimes and just when copying from USB flash drive to ssd
/// todo speed up

///
///Writes every file into the created target file
///
///Please note that empty directories are not supported and will therefore silently disappear during encoding.
///
///
pub fn encode(source_directory_path:&str, target_file_path:&str) -> u64 {
    let source_directory = Path::new(source_directory_path);
    let target_file = Path::new(target_file_path);
    if !source_directory.exists() || !source_directory.is_dir() {
        panic!("Provided directory file(arg0={}) is not a valid directory", source_directory_path);
    }
    if target_file.exists() {
        panic!("Provided target_file(arg1={}) already exists", target_file_path);
    }

    let storage = FileStorageSystem::create_leave_source_intact_with_custom_buf_size(target_file_path, 16384);
    let mut ubae = Ubae::new(storage);
    ubae.set_content(&vec![0u8;0][..]).expect("setting content failed");

    let errors = add_directory_to_system(&mut ubae, source_directory_path,
                                         source_directory.to_owned());

    return errors;
}

//allowing up to u64 errors is not necessary, but funny.
fn add_directory_to_system<T:StorageSystem> (ubae: &mut Ubae<T>, original_directory_path:&str, directory:PathBuf) -> u64 {
    if !directory.exists() || !directory.is_dir() {
        panic!("invalid directory supplied");
    }

    let mut errors =0u64;
    if let Ok(paths) = fs::read_dir(&directory) {
        for path in paths {
            if let Ok(subfilepath) = path {
                let f = subfilepath.path();
                if f.is_dir() {
                    errors+=add_directory_to_system(ubae, original_directory_path, f);
                } else {
                    if let Ok(mut file) = File::open(&f) {
                        if let Ok(metadata) = fs::metadata(&f) {
                            let internal_path_opt = get_inner_path(original_directory_path, &f.to_str().expect("what?? how was this not valid unicode?"));
                            if let Some(internal_path) = internal_path_opt {
                                if let Ok(()) = ubae.add_entry_from_stream_nocheck(&internal_path, &mut file, metadata.len() as i64) {//added untested, should definitly work though
                                    continue
                                }
                            }
                        }
                    }
                    errors+=1;//jumped by continue
                }
            }
        }
    }
    return errors;
}
fn get_inner_path(dir_root_path:&str, path:&str) -> Option<String> {
    if (&path).contains(&dir_root_path) {
        let sub = &path.replace("\\", "/")[dir_root_path.len()..];
        if sub.starts_with("/") {
            return Some(String::from(&sub[1..]));
        } else {
            return Some(String::from(sub));
        }
    }
    return None
}





///
///Decodes the content of the file into the target directory.
///If it was previously encoded using this software, then the directory should be identical to the one that it was encoded from.
///    Except for the missing empty directories.
///
pub fn decode(source_file_path:&str, target_directory_path:&str) -> u64 {
    let source_file = Path::new(source_file_path);
    let target_directory = Path::new(target_directory_path);
    if !source_file.exists() || source_file.is_dir() {
        panic!("Provided directory source_file(arg0={}) is not a valid source file", source_file_path);
    }
    if target_directory.exists() {
        panic!("Provided target_directory(arg1={}) already exists", target_directory_path);
    }

    let storage = FileStorageSystem::create_leave_source_intact_with_custom_buf_size(source_file_path, 16384);
    let ubae_iter = Ubae::new_tag_stream_iterator(storage);

    let mut errors = 0u64;//yes we do need a u64 for this. (honestly we do not actually, but I think it's funny)
    for (tag, mut stream) in ubae_iter {
        let target_file = target_directory.join(Path::new(&tag));
        fs::create_dir_all(target_file.parent().expect("parent file could not be found or something like that i don't even anymore.")).expect("create parent dir failed");
        if let Ok(mut target_stream) = File::create(target_file) {
            io::copy(&mut stream.0, &mut target_stream).expect("copy failed");
        } else {
            println!("{} failed to be restored", tag);
            errors+=1;
        }
    }

    return errors;
}