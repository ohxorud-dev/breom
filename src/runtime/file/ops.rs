use super::*;

#[unsafe(no_mangle)]
pub extern "C" fn breom_file_read(path_ptr: *mut u8) -> *mut u8 {
    let path = string_to_rust(path_ptr);
    match fs::read(path) {
        Ok(bytes) => breom_string_new(bytes.as_ptr(), bytes.len() as u64),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_file_read_byte_sum(path_ptr: *mut u8) -> i64 {
    let path = string_to_rust(path_ptr);
    match fs::read(path) {
        Ok(bytes) => bytes.iter().map(|b| *b as i64).sum::<i64>(),
        Err(_) => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_file_write(path_ptr: *mut u8, data_ptr: *mut u8) -> i64 {
    let path = string_to_rust(path_ptr);
    let data = string_to_rust(data_ptr);
    match fs::write(path, data.as_bytes()) {
        Ok(()) => data.len() as i64,
        Err(_) => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_file_append(path_ptr: *mut u8, data_ptr: *mut u8) -> i64 {
    let path = string_to_rust(path_ptr);
    let data = string_to_rust(data_ptr);
    let file = fs::OpenOptions::new().create(true).append(true).open(path);

    match file {
        Ok(mut f) => match f.write_all(data.as_bytes()) {
            Ok(()) => data.len() as i64,
            Err(_) => -1,
        },
        Err(_) => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_file_exists(path_ptr: *mut u8) -> i64 {
    let path = string_to_rust(path_ptr);
    if Path::new(&path).exists() {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_file_remove(path_ptr: *mut u8) -> i64 {
    let path = string_to_rust(path_ptr);
    let file_path = Path::new(&path);

    let res = if file_path.is_dir() {
        fs::remove_dir_all(file_path)
    } else {
        fs::remove_file(file_path)
    };

    if res.is_ok() {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_file_mkdir(path_ptr: *mut u8) -> i64 {
    let path = string_to_rust(path_ptr);
    if fs::create_dir_all(path).is_ok() {
        1
    } else {
        0
    }
}
