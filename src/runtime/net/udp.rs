use super::*;

#[unsafe(no_mangle)]
pub extern "C" fn breom_net_bind(port: i64) -> *mut u8 {
    let addr = format!("0.0.0.0:{}", port);
    match UdpSocket::bind(addr) {
        Ok(socket) => {
            let socket_clone = socket.try_clone().expect("Failed to clone socket");

            let chan_ptr = breom_chan_new(1024);
            let chan_ptr_usize = chan_ptr as usize;

            thread::spawn(move || {
                let chan_ptr = chan_ptr_usize as *mut u8;
                let mut buf = [0u8; 65535];
                while let Ok((len, _addr)) = socket_clone.recv_from(&mut buf) {
                    let s_ptr = breom_string_new(buf.as_ptr(), len as u64);
                    breom_chan_send(chan_ptr, s_ptr as i64);
                }
            });

            let socket_data_size = std::mem::size_of::<BreomSocket>() as u64;
            let socket_ptr = breom_arc_alloc(socket_data_size, SOCKET_TYPE_ID);
            unsafe {
                ptr::write(socket_ptr as *mut BreomSocket, BreomSocket { socket });
            }

            let res_data_size = std::mem::size_of::<BindResultData>() as u64;
            let res_ptr = breom_arc_alloc(res_data_size, BIND_RESULT_TYPE_ID);
            unsafe {
                let data = res_ptr as *mut BindResultData;
                (*data).socket = socket_ptr;
                (*data).rx = chan_ptr;
            }
            res_ptr
        }
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_net_send(
    socket_ptr: *mut u8,
    address_ptr: *mut u8,
    port: i64,
    data_ptr: *mut u8,
) -> i64 {
    if socket_ptr.is_null() || address_ptr.is_null() || data_ptr.is_null() {
        return -1;
    }

    let b_socket = unsafe { &*(socket_ptr as *const BreomSocket) };
    let address = string_to_rust(address_ptr);
    let data = string_to_rust(data_ptr);

    let dest = format!("{}:{}", address, port);
    match b_socket.socket.send_to(data.as_bytes(), dest) {
        Ok(bytes) => bytes as i64,
        Err(_) => -1,
    }
}
