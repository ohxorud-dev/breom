use super::*;

#[unsafe(no_mangle)]
pub extern "C" fn breom_error_new(message: *mut u8) -> *mut u8 {
    if message.is_null() {
        return ptr::null_mut();
    }
    let data_size = std::mem::size_of::<ErrorData>() as u64;
    let ptr = breom_arc_alloc(data_size, ERROR_TYPE_ID);
    unsafe {
        breom_arc_retain(message);
        let err = ptr as *mut ErrorData;
        (*err).message = message;
    }
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_panic(err_ptr: *mut u8) {
    if err_ptr.is_null() {
        eprintln!("panic: (null error)");
    } else {
        unsafe {
            let err = err_ptr as *const ErrorData;
            let msg_ptr = (*err).message;
            if msg_ptr.is_null() {
                eprintln!("panic: error with no message");
            } else {
                let len = breom_string_len(msg_ptr);
                let data = breom_string_data(msg_ptr);
                let s =
                    std::str::from_utf8_unchecked(std::slice::from_raw_parts(data, len as usize));
                eprintln!("panic: {}", s);
            }
        }
    }
    std::process::abort();
}
