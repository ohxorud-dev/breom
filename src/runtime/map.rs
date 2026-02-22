use super::*;

unsafe fn map_entries_layout(cap: u64) -> Option<Layout> {
    Layout::array::<MapEntry>(cap as usize).ok()
}

unsafe fn map_alloc_entries(cap: u64) -> Option<*mut MapEntry> {
    let layout = map_entries_layout(cap)?;
    let ptr = alloc(layout) as *mut MapEntry;
    if ptr.is_null() {
        return None;
    }
    for i in 0..cap {
        let entry = ptr.add(i as usize);
        (*entry).state = MAP_ENTRY_EMPTY;
    }
    Some(ptr)
}

pub(crate) unsafe fn map_dealloc_entries(entries: *mut MapEntry, cap: u64) {
    if entries.is_null() {
        return;
    }
    if let Some(layout) = map_entries_layout(cap) {
        dealloc(entries as *mut u8, layout);
    }
}

unsafe fn map_grow(map_ptr: *mut u8, new_cap: u64) -> bool {
    let header = map_ptr as *mut MapHeader;
    let old_cap = (*header).cap;
    let old_entries = (*header).entries;
    let Some(new_entries) = map_alloc_entries(new_cap) else {
        return false;
    };

    for i in 0..old_cap {
        let old_entry = old_entries.add(i as usize);
        if (*old_entry).state != MAP_ENTRY_OCCUPIED {
            continue;
        }
        let hash = (*old_entry).key_hash;
        let mut index = (hash % new_cap) as usize;
        loop {
            let entry = new_entries.add(index);
            if (*entry).state == MAP_ENTRY_EMPTY {
                (*entry).key_hash = hash;
                (*entry).key = (*old_entry).key;
                (*entry).value = (*old_entry).value;
                (*entry).state = MAP_ENTRY_OCCUPIED;
                break;
            }
            index = (index + 1) % new_cap as usize;
        }
    }

    map_dealloc_entries(old_entries, old_cap);
    (*header).entries = new_entries;
    (*header).cap = new_cap;
    (*header).tombstones = 0;
    true
}

unsafe fn map_rehash_in_place(map_ptr: *mut u8) {
    let header = map_ptr as *mut MapHeader;
    let cap = (*header).cap;
    let entries = (*header).entries;

    let mut occupied_entries: Vec<(u64, i64, i64)> = Vec::with_capacity((*header).len as usize);
    for i in 0..cap {
        let entry = entries.add(i as usize);
        if (*entry).state == MAP_ENTRY_OCCUPIED {
            occupied_entries.push(((*entry).key_hash, (*entry).key, (*entry).value));
        }
        (*entry).state = MAP_ENTRY_EMPTY;
    }

    (*header).len = 0;
    (*header).tombstones = 0;

    for (hash, key, value) in occupied_entries {
        let mut index = (hash % cap) as usize;
        loop {
            let entry = entries.add(index);
            if (*entry).state == MAP_ENTRY_EMPTY {
                (*entry).key_hash = hash;
                (*entry).key = key;
                (*entry).value = value;
                (*entry).state = MAP_ENTRY_OCCUPIED;
                (*header).len += 1;
                break;
            }
            index = (index + 1) % cap as usize;
        }
    }
}

fn hash_i64(value: i64) -> u64 {
    let mut h = value as u64;
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
    h ^= h >> 33;
    h
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_map_new(initial_cap: u64) -> *mut u8 {
    let requested = if initial_cap < 8 { 8 } else { initial_cap };
    let cap = requested.saturating_mul(2).next_power_of_two().max(8);
    let data_size = std::mem::size_of::<MapHeader>() as u64;
    let ptr = breom_arc_alloc(data_size, MAP_TYPE_ID);

    unsafe {
        let header = ptr as *mut MapHeader;
        let Some(entries) = map_alloc_entries(cap) else {
            breom_arc_release(ptr);
            return ptr::null_mut();
        };
        (*header).len = 0;
        (*header).cap = cap;
        (*header).tombstones = 0;
        (*header).entries = entries;
    }

    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_map_len(map_ptr: *mut u8) -> u64 {
    if map_ptr.is_null() {
        return 0;
    }

    unsafe {
        let header = map_ptr as *mut MapHeader;
        (*header).len
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_map_get(map_ptr: *mut u8, key: i64) -> i64 {
    if map_ptr.is_null() {
        return 0;
    }

    unsafe {
        let header = map_ptr as *mut MapHeader;
        let cap = (*header).cap;
        let entries = (*header).entries;

        let hash = hash_i64(key);
        let mut index = (hash % cap) as usize;

        for _ in 0..cap {
            let entry = entries.add(index);
            if (*entry).state == MAP_ENTRY_EMPTY {
                return 0;
            }
            if (*entry).state == MAP_ENTRY_OCCUPIED && (*entry).key == key {
                return (*entry).value;
            }
            index = (index + 1) % cap as usize;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_map_get_checked(map_ptr: *mut u8, key: i64) -> GetCheckedResult {
    if map_ptr.is_null() {
        let msg = "map is null";
        let s = breom_string_new(msg.as_ptr(), msg.len() as u64);
        let err = breom_error_new(s);
        breom_arc_release(s);
        return GetCheckedResult {
            err: err as i64,
            value: 0,
        };
    }
    unsafe {
        let header = map_ptr as *mut MapHeader;
        let cap = (*header).cap;
        let entries = (*header).entries;
        let hash = hash_i64(key);
        let mut index = (hash % cap) as usize;
        for _ in 0..cap {
            let entry = entries.add(index);
            if (*entry).state == MAP_ENTRY_EMPTY {
                let msg = "key not found";
                let s = breom_string_new(msg.as_ptr(), msg.len() as u64);
                let err = breom_error_new(s);
                breom_arc_release(s);
                return GetCheckedResult {
                    err: err as i64,
                    value: 0,
                };
            }
            if (*entry).state == MAP_ENTRY_OCCUPIED && (*entry).key == key {
                return GetCheckedResult {
                    err: 0,
                    value: (*entry).value,
                };
            }
            index = (index + 1) % cap as usize;
        }
        let msg = "key not found";
        let s = breom_string_new(msg.as_ptr(), msg.len() as u64);
        let err = breom_error_new(s);
        breom_arc_release(s);
        GetCheckedResult {
            err: err as i64,
            value: 0,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_map_set(map_ptr: *mut u8, key: i64, value: i64) {
    if map_ptr.is_null() {
        return;
    }

    unsafe {
        let header = map_ptr as *mut MapHeader;
        if (*header).len + (*header).tombstones + 1 >= (*header).cap {
            let mut new_cap = (*header).cap.saturating_mul(2).max(8);
            if new_cap == (*header).cap {
                new_cap = (*header).cap + 1;
            }
            if !map_grow(map_ptr, new_cap) {
                return;
            }
        } else if (*header).tombstones > (*header).cap / 3 {
            map_rehash_in_place(map_ptr);
        }

        let cap = (*header).cap;
        let entries = (*header).entries;

        let hash = hash_i64(key);
        let mut index = (hash % cap) as usize;
        let mut first_deleted: Option<usize> = None;

        for _ in 0..cap {
            let entry = entries.add(index);
            match (*entry).state {
                MAP_ENTRY_EMPTY => {
                    let insert_index = first_deleted.unwrap_or(index);
                    let insert_entry = entries.add(insert_index);
                    (*insert_entry).key_hash = hash;
                    (*insert_entry).key = key;
                    (*insert_entry).value = value;
                    if (*insert_entry).state == MAP_ENTRY_DELETED {
                        (*header).tombstones -= 1;
                    }
                    (*insert_entry).state = MAP_ENTRY_OCCUPIED;
                    (*header).len += 1;
                    return;
                }
                MAP_ENTRY_OCCUPIED => {
                    if (*entry).key == key {
                        (*entry).value = value;
                        return;
                    }
                }
                MAP_ENTRY_DELETED => {
                    if first_deleted.is_none() {
                        first_deleted = Some(index);
                    }
                }
                _ => {}
            }
            index = (index + 1) % cap as usize;
        }

        if let Some(insert_index) = first_deleted {
            let insert_entry = entries.add(insert_index);
            (*insert_entry).key_hash = hash;
            (*insert_entry).key = key;
            (*insert_entry).value = value;
            (*insert_entry).state = MAP_ENTRY_OCCUPIED;
            (*header).tombstones -= 1;
            (*header).len += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_map_contains(map_ptr: *mut u8, key: i64) -> i64 {
    if map_ptr.is_null() {
        return 0;
    }

    unsafe {
        let header = map_ptr as *mut MapHeader;
        let cap = (*header).cap;
        let entries = (*header).entries;

        let hash = hash_i64(key);
        let mut index = (hash % cap) as usize;

        for _ in 0..cap {
            let entry = entries.add(index);
            if (*entry).state == MAP_ENTRY_EMPTY {
                return 0;
            }
            if (*entry).state == MAP_ENTRY_OCCUPIED && (*entry).key == key {
                return 1;
            }
            index = (index + 1) % cap as usize;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_map_delete(map_ptr: *mut u8, key: i64) -> i64 {
    if map_ptr.is_null() {
        return 0;
    }

    unsafe {
        let header = map_ptr as *mut MapHeader;
        let cap = (*header).cap;
        let entries = (*header).entries;

        let hash = hash_i64(key);
        let mut index = (hash % cap) as usize;

        for _ in 0..cap {
            let entry = entries.add(index);
            if (*entry).state == MAP_ENTRY_EMPTY {
                return 0;
            }
            if (*entry).state == MAP_ENTRY_OCCUPIED && (*entry).key == key {
                (*entry).state = MAP_ENTRY_DELETED;
                (*header).len -= 1;
                (*header).tombstones += 1;
                return 1;
            }
            index = (index + 1) % cap as usize;
        }

        0
    }
}
