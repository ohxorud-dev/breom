use super::*;

#[unsafe(no_mangle)]
pub extern "C" fn breom_set_new(initial_cap: u64) -> *mut u8 {
    breom_map_new(initial_cap)
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_set_len(set_ptr: *mut u8) -> u64 {
    breom_map_len(set_ptr)
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_set_add(set_ptr: *mut u8, value: i64) {
    breom_map_set(set_ptr, value, 1);
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_set_contains(set_ptr: *mut u8, value: i64) -> i64 {
    breom_map_contains(set_ptr, value)
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_set_remove(set_ptr: *mut u8, value: i64) -> i64 {
    breom_map_delete(set_ptr, value)
}
