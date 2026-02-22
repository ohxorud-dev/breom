use super::*;

#[unsafe(no_mangle)]
pub extern "C" fn breom_net_tcp_bind(port: i64) -> *mut u8 {
    let addr = format!("0.0.0.0:{}", port);
    match TcpListener::bind(addr) {
        Ok(listener) => {
            let socket_id = TCP_NEXT_SOCKET_ID.fetch_add(1, Ordering::Relaxed);
            let chan_ptr = breom_chan_new(1024);
            let chan_ptr_usize = chan_ptr as usize;
            breom_arc_retain(chan_ptr);

            let state = Arc::new(Mutex::new(TcpListenerState {
                connections: HashMap::new(),
            }));
            {
                let mut listeners = TCP_LISTENERS.lock().unwrap();
                listeners.insert(socket_id, state.clone());
            }

            thread::spawn(move || {
                for incoming in listener.incoming() {
                    let stream = match incoming {
                        Ok(s) => s,
                        Err(_) => continue,
                    };

                    let writer = match stream.try_clone() {
                        Ok(s) => Arc::new(Mutex::new(s)),
                        Err(_) => continue,
                    };

                    let conn_id = TCP_NEXT_CONN_ID.fetch_add(1, Ordering::Relaxed);
                    {
                        let mut guard = state.lock().unwrap();
                        guard.connections.insert(conn_id, writer);
                    }

                    let chan_ptr_usize_local = chan_ptr_usize;
                    thread::spawn(move || {
                        let chan_ptr = chan_ptr_usize_local as *mut u8;
                        let mut stream = stream;
                        let mut buf = [0u8; 65535];
                        let len = match stream.read(&mut buf) {
                            Ok(n) if n > 0 => n,
                            _ => return,
                        };

                        let data_ptr = breom_string_new(buf.as_ptr(), len as u64);
                        let packet_ptr = breom_arc_alloc(
                            std::mem::size_of::<TcpPacketData>() as u64,
                            TCP_PACKET_TYPE_ID,
                        );
                        unsafe {
                            let packet = packet_ptr as *mut TcpPacketData;
                            (*packet).conn = conn_id;
                            (*packet).data = data_ptr;
                        }
                        breom_chan_send(chan_ptr, packet_ptr as i64);
                    });
                }

                let chan_ptr = chan_ptr_usize as *mut u8;
                breom_arc_release(chan_ptr);
            });

            let socket_ptr = breom_arc_alloc(
                std::mem::size_of::<BreomTcpSocket>() as u64,
                TCP_SOCKET_TYPE_ID,
            );
            unsafe {
                ptr::write(
                    socket_ptr as *mut BreomTcpSocket,
                    BreomTcpSocket { id: socket_id },
                );
            }

            let res_ptr = breom_arc_alloc(
                std::mem::size_of::<BindResultData>() as u64,
                BIND_RESULT_TYPE_ID,
            );
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
pub extern "C" fn breom_net_tcp_connect(address_ptr: *mut u8, port: i64) -> *mut u8 {
    if address_ptr.is_null() {
        return ptr::null_mut();
    }

    let address = string_to_rust(address_ptr);
    let addr = format!("{}:{}", address, port);
    let targets: Vec<_> = match addr.to_socket_addrs() {
        Ok(addrs) => addrs.collect(),
        Err(_) => return ptr::null_mut(),
    };
    if targets.is_empty() {
        return ptr::null_mut();
    }

    let mut stream_opt = None;
    for _ in 0..20 {
        for target in &targets {
            if let Ok(stream) = TcpStream::connect_timeout(target, Duration::from_millis(80)) {
                stream_opt = Some(stream);
                break;
            }
        }
        if stream_opt.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(15));
    }

    let Some(stream) = stream_opt else {
        return ptr::null_mut();
    };

    let socket_id = TCP_NEXT_SOCKET_ID.fetch_add(1, Ordering::Relaxed);
    {
        let mut clients = TCP_CLIENT_STREAMS.lock().unwrap();
        clients.insert(socket_id, Arc::new(Mutex::new(stream)));
    }

    let socket_ptr = breom_arc_alloc(
        std::mem::size_of::<BreomTcpSocket>() as u64,
        TCP_SOCKET_TYPE_ID,
    );
    unsafe {
        ptr::write(
            socket_ptr as *mut BreomTcpSocket,
            BreomTcpSocket { id: socket_id },
        );
    }
    socket_ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_net_tcp_send(socket_ptr: *mut u8, conn: i64, data_ptr: *mut u8) -> i64 {
    if socket_ptr.is_null() || data_ptr.is_null() {
        return -1;
    }

    let socket = unsafe { &*(socket_ptr as *const BreomTcpSocket) };
    let data = string_to_rust(data_ptr);

    let state = {
        let listeners = TCP_LISTENERS.lock().unwrap();
        listeners.get(&socket.id).cloned()
    };
    if let Some(state) = state {
        let stream = {
            let guard = state.lock().unwrap();
            guard.connections.get(&conn).cloned()
        };
        let Some(stream) = stream else {
            return -1;
        };

        let written = {
            let mut stream = stream.lock().unwrap();
            match stream.write(data.as_bytes()) {
                Ok(n) => {
                    let _ = stream.flush();
                    n as i64
                }
                Err(_) => -1,
            }
        };

        if written >= 0 {
            let mut guard = state.lock().unwrap();
            guard.connections.remove(&conn);
        }

        return written;
    }

    let client_stream = {
        let clients = TCP_CLIENT_STREAMS.lock().unwrap();
        clients.get(&socket.id).cloned()
    };
    let Some(client_stream) = client_stream else {
        return -1;
    };

    let mut client_stream = client_stream.lock().unwrap();
    match client_stream.write(data.as_bytes()) {
        Ok(n) => {
            let _ = client_stream.flush();
            n as i64
        }
        Err(_) => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_net_tcp_recv(socket_ptr: *mut u8) -> *mut u8 {
    if socket_ptr.is_null() {
        return ptr::null_mut();
    }

    let socket = unsafe { &*(socket_ptr as *const BreomTcpSocket) };
    let client_stream = {
        let clients = TCP_CLIENT_STREAMS.lock().unwrap();
        clients.get(&socket.id).cloned()
    };
    let Some(client_stream) = client_stream else {
        return ptr::null_mut();
    };

    let mut stream = client_stream.lock().unwrap();
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];

    let first_read = match stream.read(&mut chunk) {
        Ok(0) => return breom_string_empty(),
        Ok(n) => n,
        Err(_) => return ptr::null_mut(),
    };
    buf.extend_from_slice(&chunk[..first_read]);

    let _ = stream.set_read_timeout(Some(Duration::from_millis(20)));
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut =>
            {
                break;
            }
            Err(_) => {
                let _ = stream.set_read_timeout(None);
                return ptr::null_mut();
            }
        }
    }
    let _ = stream.set_read_timeout(None);

    breom_string_new(buf.as_ptr(), buf.len() as u64)
}
