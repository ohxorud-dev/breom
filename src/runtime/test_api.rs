use super::*;

fn with_test_state<R>(f: impl FnOnce(&mut TestRuntimeState) -> R) -> R {
    let state = TEST_STATE.get_or_init(|| Mutex::new(TestRuntimeState::default()));
    let mut guard = state.lock().unwrap();
    f(&mut guard)
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_test_begin(name_ptr: *const u8, name_len: u64) {
    let name = if name_ptr.is_null() || name_len == 0 {
        String::new()
    } else {
        let bytes = unsafe { std::slice::from_raw_parts(name_ptr, name_len as usize) };
        String::from_utf8_lossy(bytes).into_owned()
    };

    with_test_state(|state| {
        state.current_name = name;
        state.failures = 0;
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_test_end() -> i64 {
    with_test_state(|state| state.failures)
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_test_assert(cond: i64) {
    if cond != 0 {
        return;
    }
    with_test_state(|state| {
        state.failures += 1;
        println!("[ASSERT] {}: expected true, got false", state.current_name);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_test_fail(message_ptr: *mut u8) {
    let message = string_to_rust(message_ptr);
    with_test_state(|state| {
        state.failures += 1;
        if message.is_empty() {
            println!("[FAIL] {}", state.current_name);
        } else {
            println!("[FAIL] {}: {}", state.current_name, message);
        }
    });
}
