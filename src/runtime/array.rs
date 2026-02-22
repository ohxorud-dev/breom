use super::*;

unsafe fn array_elem_ptr(arr_ptr: *mut u8, elem_size: u64, index: u64) -> *mut u8 {
    let data_start = unsafe { arr_ptr.add(ArrayHeader::SIZE) };
    unsafe { data_start.add((index * elem_size) as usize) }
}

fn validate_array_ptr(arr_ptr: *mut u8, op: &str) -> Option<*mut ArrayHeader> {
    if arr_ptr.is_null() {
        return None;
    }

    let arr_addr = arr_ptr as usize;
    if arr_addr < MIN_VALID_HEAP_ADDR {
        eprintln!(
            "runtime warning: {} received invalid array pointer {:p}",
            op, arr_ptr
        );
        return None;
    }

    if !arr_addr.is_multiple_of(std::mem::align_of::<ArrayHeader>()) {
        eprintln!(
            "runtime warning: {} received misaligned array pointer {:p}",
            op, arr_ptr
        );
        return None;
    }

    let header_ptr = unsafe { arc::get_header(arr_ptr) };
    let header_addr = header_ptr as usize;

    if header_addr < MIN_VALID_HEAP_ADDR
        || !header_addr.is_multiple_of(std::mem::align_of::<ArcHeader>())
    {
        eprintln!(
            "runtime warning: {} received invalid array header {:p}",
            op, header_ptr
        );
        return None;
    }

    unsafe {
        if (*header_ptr).type_id != ARRAY_TYPE_ID {
            eprintln!(
                "runtime warning: {} received non-array pointer {:p} (type_id={})",
                op,
                arr_ptr,
                (*header_ptr).type_id
            );
            return None;
        }
    }

    Some(arr_ptr as *mut ArrayHeader)
}

unsafe fn array_header_fast(arr_ptr: *mut u8, _op: &str) -> Option<*mut ArrayHeader> {
    #[cfg(debug_assertions)]
    {
        validate_array_ptr(arr_ptr, _op)
    }

    #[cfg(not(debug_assertions))]
    {
        if arr_ptr.is_null() {
            None
        } else {
            Some(arr_ptr as *mut ArrayHeader)
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_array_new(elem_size: u64, initial_cap: u64) -> *mut u8 {
    let data_size = ArrayHeader::SIZE as u64 + elem_size * initial_cap;
    let ptr = breom_arc_alloc(data_size, ARRAY_TYPE_ID);

    unsafe {
        let arr_header = ptr as *mut ArrayHeader;
        (*arr_header).len = 0;
        (*arr_header).cap = initial_cap;
        (*arr_header).elem_size = elem_size;
    }

    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_array_from_i64_buffer(data_ptr: *const i64, len: u64) -> *mut u8 {
    if data_ptr.is_null() {
        return breom_array_new(8, len.max(1));
    }

    let arr_ptr = breom_array_new(8, len.max(1));
    unsafe {
        let arr_header = arr_ptr as *mut ArrayHeader;
        let dst = arr_ptr.add(ArrayHeader::SIZE) as *mut i64;
        if len > 0 {
            std::ptr::copy_nonoverlapping(data_ptr, dst, len as usize);
        }
        (*arr_header).len = len;
    }
    arr_ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_array_len(arr_ptr: *mut u8) -> u64 {
    let Some(arr_header) = (unsafe { array_header_fast(arr_ptr, "breom_array_len") }) else {
        return 0;
    };

    unsafe { (*arr_header).len }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_array_get_ptr(arr_ptr: *mut u8, index: u64) -> *mut u8 {
    let Some(arr_header) = (unsafe { array_header_fast(arr_ptr, "breom_array_get_ptr") }) else {
        return ptr::null_mut();
    };

    unsafe {
        let elem_size = (*arr_header).elem_size;
        array_elem_ptr(arr_ptr, elem_size, index)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_array_cap(arr_ptr: *mut u8) -> u64 {
    let Some(arr_header) = (unsafe { array_header_fast(arr_ptr, "breom_array_cap") }) else {
        return 0;
    };

    unsafe { (*arr_header).cap }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_array_push(arr_ptr: *mut u8, value: i64) -> *mut u8 {
    let Some(arr_header) = (unsafe { array_header_fast(arr_ptr, "breom_array_push") }) else {
        return ptr::null_mut();
    };

    unsafe {
        let current_len = (*arr_header).len;
        let current_cap = (*arr_header).cap;
        let elem_size = (*arr_header).elem_size;

        let result_ptr = if current_len >= current_cap {
            let new_cap = if current_cap == 0 { 4 } else { current_cap * 2 };
            breom_array_resize(arr_ptr, new_cap)
        } else {
            arr_ptr
        };

        let result_header = result_ptr as *mut ArrayHeader;
        let write_index = (*result_header).len;
        let data_start = result_ptr.add(ArrayHeader::SIZE);
        let slot_ptr = data_start.add((write_index * elem_size) as usize);

        if elem_size == 8 {
            *(slot_ptr as *mut i64) = value;
        }

        (*result_header).len += 1;
        result_ptr
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_array_pop(arr_ptr: *mut u8) -> i64 {
    let Some(arr_header) = (unsafe { array_header_fast(arr_ptr, "breom_array_pop") }) else {
        return 0;
    };

    unsafe {
        let current_len = (*arr_header).len;

        if current_len == 0 {
            return 0;
        }

        (*arr_header).len -= 1;
        let elem_size = (*arr_header).elem_size;
        let elem_ptr = array_elem_ptr(arr_ptr, elem_size, (*arr_header).len);
        *(elem_ptr as *const i64)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_array_resize(arr_ptr: *mut u8, new_cap: u64) -> *mut u8 {
    if arr_ptr.is_null() {
        return breom_array_new(8, new_cap);
    }

    let Some(old_header) = (unsafe { array_header_fast(arr_ptr, "breom_array_resize") }) else {
        return ptr::null_mut();
    };

    unsafe {
        let old_len = (*old_header).len;
        let old_cap = (*old_header).cap;
        let elem_size = (*old_header).elem_size;
        let preserved_len = old_len.min(new_cap);

        if old_cap == new_cap {
            (*old_header).len = preserved_len;
            return arr_ptr;
        }

        let old_data_size = ArrayHeader::SIZE as u64 + (old_cap * elem_size);
        let new_data_size = ArrayHeader::SIZE as u64 + (new_cap * elem_size);

        let old_total_size = ArcHeader::SIZE + old_data_size as usize;
        let new_total_size = ArcHeader::SIZE + new_data_size as usize;

        let old_layout =
            Layout::from_size_align(old_total_size, ArcHeader::ALIGN).expect("Invalid layout");
        let header_ptr = arc::get_header(arr_ptr) as *mut u8;
        let new_header_ptr = realloc(header_ptr, old_layout, new_total_size);
        if new_header_ptr.is_null() {
            panic!("breom_array_resize: out of memory");
        }

        let arc_header = new_header_ptr as *mut ArcHeader;
        (*arc_header).data_size = new_data_size;

        let new_arr_ptr = new_header_ptr.add(ArcHeader::SIZE);
        let new_header = new_arr_ptr as *mut ArrayHeader;
        (*new_header).cap = new_cap;
        (*new_header).len = preserved_len;

        new_arr_ptr
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_array_set(arr_ptr: *mut u8, index: u64, value: i64) {
    let Some(arr_header) = (unsafe { array_header_fast(arr_ptr, "breom_array_set") }) else {
        return;
    };

    unsafe {
        if index >= (*arr_header).len {
            eprintln!(
                "runtime warning: breom_array_set index out of bounds (index={}, len={})",
                index,
                (*arr_header).len
            );
            return;
        }
        let elem_size = (*arr_header).elem_size;
        let elem_ptr = array_elem_ptr(arr_ptr, elem_size, index);
        *(elem_ptr as *mut i64) = value;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_array_get(arr_ptr: *mut u8, index: u64) -> i64 {
    let Some(arr_header) = (unsafe { array_header_fast(arr_ptr, "breom_array_get") }) else {
        return 0;
    };

    unsafe {
        if index >= (*arr_header).len {
            eprintln!(
                "runtime warning: breom_array_get index out of bounds (index={}, len={})",
                index,
                (*arr_header).len
            );
            return 0;
        }
        let elem_size = (*arr_header).elem_size;
        let elem_ptr = array_elem_ptr(arr_ptr, elem_size, index);
        *(elem_ptr as *const i64)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_array_get_checked(arr_ptr: *mut u8, index: u64) -> GetCheckedResult {
    let Some(arr_header) = validate_array_ptr(arr_ptr, "breom_array_get_checked") else {
        let msg = "invalid array pointer";
        let s = breom_string_new(msg.as_ptr(), msg.len() as u64);
        let err = breom_error_new(s);
        breom_arc_release(s);
        return GetCheckedResult {
            err: err as i64,
            value: 0,
        };
    };
    unsafe {
        let len = (*arr_header).len;
        if index >= len {
            let msg = "index out of bounds";
            let s = breom_string_new(msg.as_ptr(), msg.len() as u64);
            let err = breom_error_new(s);
            breom_arc_release(s);
            return GetCheckedResult {
                err: err as i64,
                value: 0,
            };
        }
        let elem_size = (*arr_header).elem_size;
        let elem_ptr = array_elem_ptr(arr_ptr, elem_size, index);
        let value = *(elem_ptr as *const i64);
        GetCheckedResult { err: 0, value }
    }
}
