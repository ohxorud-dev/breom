use super::*;

#[unsafe(no_mangle)]
pub extern "C" fn breom_spawn(func: extern "C" fn(*mut u8), env: *mut u8) {
    let env_ptr = env as usize;
    thread::spawn(move || {
        let env = env_ptr as *mut u8;
        func(env);
        if !env.is_null() {
            breom_arc_release(env);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_thread_yield() {
    thread::yield_now();
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_thread_sleep(ms: i64) {
    if ms > 0 {
        thread::sleep(std::time::Duration::from_millis(ms as u64));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_select_epoch() -> i64 {
    SELECT_EPOCH.load(Ordering::SeqCst) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_select_wait(last_epoch: i64) -> i64 {
    let mut guard = SELECT_MUTEX.lock().unwrap();
    loop {
        let epoch = SELECT_EPOCH.load(Ordering::SeqCst) as i64;
        if epoch != last_epoch {
            return epoch;
        }
        guard = SELECT_CONDVAR.wait(guard).unwrap();
    }
}
