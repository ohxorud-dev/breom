use super::*;

fn trim_line_ending_in_place(line: &mut String) {
    if line.ends_with('\n') {
        line.pop();
    }
    if line.ends_with('\r') {
        line.pop();
    }
}

fn scanner_fill_next_line(scanner: &mut BreomScanner) -> bool {
    if scanner.closed != 0 || scanner.eof != 0 || scanner.has_buffered_line != 0 {
        return scanner.has_buffered_line != 0;
    }

    let Some(reader) = scanner.reader.as_mut() else {
        scanner.eof = 1;
        return false;
    };

    scanner.buffered_line.clear();
    match reader.read_line(&mut scanner.buffered_line) {
        Ok(0) => {
            scanner.eof = 1;
            false
        }
        Ok(_) => {
            trim_line_ending_in_place(&mut scanner.buffered_line);
            scanner.has_buffered_line = 1;
            true
        }
        Err(_) => {
            scanner.eof = 1;
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_file_scanner_open(path_ptr: *mut u8) -> *mut u8 {
    let path = string_to_rust(path_ptr);
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return ptr::null_mut(),
    };

    let data_size = std::mem::size_of::<BreomScanner>() as u64;
    let ptr = breom_arc_alloc(data_size, FILE_SCANNER_TYPE_ID);
    unsafe {
        let scanner = ptr as *mut BreomScanner;
        ptr::write(
            scanner,
            BreomScanner {
                reader: Some(BufReader::new(file)),
                buffered_line: String::new(),
                has_buffered_line: 0,
                eof: 0,
                closed: 0,
            },
        );
    }
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_file_scanner_has_next(scanner_ptr: *mut u8) -> i64 {
    if scanner_ptr.is_null() {
        return 0;
    }
    unsafe {
        let scanner = scanner_ptr as *mut BreomScanner;
        let scanner_ref = &mut *scanner;
        if scanner_ref.closed != 0 {
            return 0;
        }
        if scanner_fill_next_line(scanner_ref) {
            1
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_file_scanner_next_line(scanner_ptr: *mut u8) -> *mut u8 {
    if scanner_ptr.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        let scanner = scanner_ptr as *mut BreomScanner;
        let scanner_ref = &mut *scanner;
        if scanner_ref.closed != 0 || !scanner_fill_next_line(scanner_ref) {
            return ptr::null_mut();
        }

        scanner_ref.has_buffered_line = 0;
        breom_string_new(
            scanner_ref.buffered_line.as_ptr(),
            scanner_ref.buffered_line.len() as u64,
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_file_scanner_close(scanner_ptr: *mut u8) -> i64 {
    if scanner_ptr.is_null() {
        return 0;
    }
    unsafe {
        let scanner = scanner_ptr as *mut BreomScanner;
        (*scanner).closed = 1;
        (*scanner).reader = None;
        (*scanner).buffered_line.clear();
        (*scanner).has_buffered_line = 0;
        (*scanner).eof = 1;
    }
    1
}
