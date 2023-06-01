use alloc::string::ToString;
use alloc::vec::Vec;
use nx::result::*;
use nx::fs;
use alloc::string::String;
use crate::{amiibo, miiext};

#[inline]
pub fn exists_file(path: String) -> bool {
    match fs::get_entry_type(path) {
        Ok(ent_type) => ent_type == fs::DirectoryEntryType::File,
        Err(_) => false
    }
}

pub const BASE_DIR: &'static str = "sdmc:/emuiibo";

pub const FLAGS_DIR: &'static str = "sdmc:/emuiibo/flags";

#[inline(always)]
pub fn make_flag_path(name: &str) -> String {
    format!("{}/{}.flag", FLAGS_DIR, name)
}

pub fn has_flag(name: &str) -> bool {
    exists_file(make_flag_path(name))
}

pub fn set_flag(name: &str, enabled: bool) {
    let flag_path = make_flag_path(name);
    if enabled {
        let _ = fs::create_file(flag_path, 0, fs::FileAttribute::None());
    }
    else {
        let _ = fs::delete_file(flag_path);
    }
}

pub fn get_path_without_extension(path: String) -> String {
    let mut path_items: Vec<&str> = path.split_terminator('.').collect();
    path_items.pop();
    path_items.join(".")
}

pub fn get_path_file_name(path: String) -> String {
    let path_items: Vec<&str> = path.split_terminator('/').collect();
    String::from(*path_items.last().unwrap_or(&""))
}

#[inline]
pub fn get_path_file_name_without_extension(path: String) -> String {
    get_path_file_name(get_path_without_extension(path))
}

#[inline]
pub fn recreate_directory(path: String) -> Result<()> {
    // The directory might not already exist, thus this attempt to delete it could fail
    let _ = fs::delete_directory_recursively(path.clone());
    fs::create_directory(path.clone())?;
    Ok(())
}

// Note: using macros instead of fns to avoid having to deal with serde_json's lifetime stuff

macro_rules! read_deserialize_json {
    ($path:expr => $t:ty) => {{
        let json_data = {
            let mut file = nx::fs::open_file($path, nx::fs::FileOpenOption::Read())?;
            let mut data: alloc::vec::Vec<u8> = vec![0; file.get_size()?];
            file.read_array(&mut data)?;
            data
        };
    
        serde_json::from_slice::<$t>(&json_data).map_err(|_| $crate::rc::ResultInvalidJsonDeserialization::make())
    }};
}

macro_rules! write_serialize_json {
    ($path:expr, $t:expr) => {{
        if let Ok(json_data) = serde_json::to_vec_pretty($t) {
            let _ = nx::fs::delete_file($path.clone());
            let mut file = nx::fs::open_file($path.clone(), nx::fs::FileOpenOption::Create() | nx::fs::FileOpenOption::Write() | nx::fs::FileOpenOption::Append())?;
            file.write_array(&json_data)?;
            Ok(())
        }
        else {
            Err($crate::rc::ResultInvalidJsonSerialization::make())
        }
    }};
}

pub fn ensure_directories() -> Result<()> {
    let _ = fs::create_directory(BASE_DIR.to_string());
    let _ = fs::create_directory(amiibo::VIRTUAL_AMIIBO_DIR.to_string());
    let _ = fs::create_directory(FLAGS_DIR.to_string());
    recreate_directory(miiext::EXPORTED_MIIS_DIR.to_string())?;

    Ok(())
}