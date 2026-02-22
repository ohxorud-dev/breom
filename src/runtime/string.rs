use super::*;

#[unsafe(no_mangle)]
pub extern "C" fn breom_string_new(data: *const u8, len: u64) -> *mut u8 {
    let data_size = StringHeader::SIZE as u64 + len;
    let ptr = breom_arc_alloc(data_size, STRING_TYPE_ID);

    unsafe {
        let str_header = ptr as *mut StringHeader;
        (*str_header).len = len;

        if len > 0 && !data.is_null() {
            let str_data = ptr.add(StringHeader::SIZE);
            ptr::copy_nonoverlapping(data, str_data, len as usize);
        }
    }

    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_string_empty() -> *mut u8 {
    breom_string_new(ptr::null(), 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_string_len(str_ptr: *mut u8) -> u64 {
    if str_ptr.is_null() {
        return 0;
    }

    unsafe {
        let str_header = str_ptr as *mut StringHeader;
        (*str_header).len
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_string_data(str_ptr: *mut u8) -> *const u8 {
    if str_ptr.is_null() {
        return ptr::null();
    }

    unsafe { str_ptr.add(StringHeader::SIZE) }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_string_concat(a: *mut u8, b: *mut u8) -> *mut u8 {
    let len_a = breom_string_len(a);
    let len_b = breom_string_len(b);
    let total_len = len_a + len_b;

    let data_size = StringHeader::SIZE as u64 + total_len;
    let result = breom_arc_alloc(data_size, STRING_TYPE_ID);

    unsafe {
        let str_header = result as *mut StringHeader;
        (*str_header).len = total_len;

        let result_data = result.add(StringHeader::SIZE);
        if len_a > 0 {
            let data_a = breom_string_data(a);
            ptr::copy_nonoverlapping(data_a, result_data, len_a as usize);
        }

        if len_b > 0 {
            let data_b = breom_string_data(b);
            ptr::copy_nonoverlapping(data_b, result_data.add(len_a as usize), len_b as usize);
        }
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_string_eq(a: *mut u8, b: *mut u8) -> i64 {
    if a == b {
        return 1;
    }
    if a.is_null() || b.is_null() {
        return 0;
    }

    let len_a = breom_string_len(a);
    let len_b = breom_string_len(b);

    if len_a != len_b {
        return 0;
    }

    if len_a == 0 {
        return 1;
    }

    unsafe {
        let data_a = breom_string_data(a);
        let data_b = breom_string_data(b);
        let slice_a = std::slice::from_raw_parts(data_a, len_a as usize);
        let slice_b = std::slice::from_raw_parts(data_b, len_b as usize);
        if slice_a == slice_b {
            1
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_int_to_string(value: i64) -> *mut u8 {
    let s = value.to_string();
    let bytes = s.as_bytes();
    breom_string_new(bytes.as_ptr(), bytes.len() as u64)
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_float_to_string(value: f64) -> *mut u8 {
    let s = value.to_string();
    let bytes = s.as_bytes();
    breom_string_new(bytes.as_ptr(), bytes.len() as u64)
}

pub fn string_to_rust(str_ptr: *mut u8) -> String {
    if str_ptr.is_null() {
        return String::new();
    }

    unsafe {
        let len = breom_string_len(str_ptr) as usize;
        let data = breom_string_data(str_ptr);
        let slice = std::slice::from_raw_parts(data, len);
        String::from_utf8_lossy(slice).into_owned()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_string_print(str_ptr: *mut u8) {
    let s = string_to_rust(str_ptr);
    print!("{}", s);
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_string_println(str_ptr: *mut u8) {
    let s = string_to_rust(str_ptr);
    println!("{}", s);
}
