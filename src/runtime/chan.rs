use super::*;

#[unsafe(no_mangle)]
pub extern "C" fn breom_chan_new(buffer_size: u64) -> *mut u8 {
    let cap = if buffer_size == 0 {
        1
    } else {
        buffer_size as usize
    };
    let (tx, rx) = mpsc::sync_channel(cap);
    let chan = Box::new(BreomChannel {
        sender: tx,
        receiver: Mutex::new(rx),
    });

    let data_size = std::mem::size_of::<BreomChannel>() as u64;
    let ptr = breom_arc_alloc(data_size, CHAN_TYPE_ID);

    unsafe {
        ptr::write(ptr as *mut BreomChannel, *chan);
    }

    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_chan_send(chan_ptr: *mut u8, value: i64) {
    if chan_ptr.is_null() {
        return;
    }
    let chan = unsafe { &*(chan_ptr as *const BreomChannel) };
    let _ = chan.sender.send(value);

    {
        let _guard = SELECT_MUTEX.lock().unwrap();
        SELECT_EPOCH.fetch_add(1, Ordering::SeqCst);
        SELECT_CONDVAR.notify_all();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_chan_recv(chan_ptr: *mut u8) -> i64 {
    if chan_ptr.is_null() {
        return 0;
    }

    let chan = unsafe { &*(chan_ptr as *const BreomChannel) };
    let rx = chan.receiver.lock().unwrap();
    rx.recv().unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_chan_pending(chan_ptr: *mut u8) -> i64 {
    if chan_ptr.is_null() {
        return 0;
    }

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_chan_try_recv(chan_ptr: *mut u8, out_val: *mut i64) -> i64 {
    if chan_ptr.is_null() {
        return 0;
    }

    let chan = unsafe { &*(chan_ptr as *const BreomChannel) };
    let rx = chan.receiver.lock().unwrap();
    match rx.try_recv() {
        Ok(val) => {
            if !out_val.is_null() {
                unsafe {
                    *out_val = val;
                }
            }
            1
        }
        Err(_) => 0,
    }
}
