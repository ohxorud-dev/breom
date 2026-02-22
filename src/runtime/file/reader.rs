use super::*;

#[unsafe(no_mangle)]
pub extern "C" fn breom_file_reader_open(path_ptr: *mut u8) -> *mut u8 {
    let path = string_to_rust(path_ptr);
    let data_size = std::mem::size_of::<BreomReader>() as u64;
    let ptr = breom_arc_alloc(data_size, FILE_READER_TYPE_ID);
    unsafe {
        let reader = ptr as *mut BreomReader;
        ptr::write(reader, BreomReader { path, closed: 0 });
    }
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_file_reader_read_all(reader_ptr: *mut u8) -> *mut u8 {
    if reader_ptr.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        let reader = reader_ptr as *mut BreomReader;
        if (*reader).closed != 0 {
            return ptr::null_mut();
        }
        match fs::read(&(*reader).path) {
            Ok(bytes) => breom_string_new(bytes.as_ptr(), bytes.len() as u64),
            Err(_) => ptr::null_mut(),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_file_reader_close(reader_ptr: *mut u8) -> i64 {
    if reader_ptr.is_null() {
        return 0;
    }
    unsafe {
        let reader = reader_ptr as *mut BreomReader;
        (*reader).closed = 1;
    }
    1
}
