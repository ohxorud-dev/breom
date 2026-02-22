use super::*;

#[unsafe(no_mangle)]
pub extern "C" fn breom_arc_alloc(data_size: u64, type_id: u64) -> *mut u8 {
    let total_size = ArcHeader::SIZE + data_size as usize;
    let layout = Layout::from_size_align(total_size, ArcHeader::ALIGN).expect("Invalid layout");

    unsafe {
        let ptr = alloc(layout);
        if ptr.is_null() {
            panic!("breom_arc_alloc: out of memory");
        }

        let header = ptr as *mut ArcHeader;
        (*header).ref_count = AtomicU64::new(1);
        (*header).type_id = type_id;
        (*header).data_size = data_size;

        ptr.add(ArcHeader::SIZE)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_arc_retain(data_ptr: *mut u8) {
    if data_ptr.is_null() {
        return;
    }

    unsafe {
        let header = get_header(data_ptr);
        (*header).ref_count.fetch_add(1, Ordering::Relaxed);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_arc_release(data_ptr: *mut u8) {
    if data_ptr.is_null() {
        return;
    }

    unsafe {
        let header = get_header(data_ptr);
        if (*header).ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            match (*header).type_id {
                CHAN_TYPE_ID => {
                    let chan = data_ptr as *mut BreomChannel;
                    ptr::drop_in_place(chan);
                }
                SOCKET_TYPE_ID => {
                    let socket = data_ptr as *mut BreomSocket;
                    ptr::drop_in_place(socket);
                }
                TCP_SOCKET_TYPE_ID => {
                    let socket = data_ptr as *mut BreomTcpSocket;
                    let id = (*socket).id;
                    {
                        let mut listeners = TCP_LISTENERS.lock().unwrap();
                        listeners.remove(&id);
                    }
                    {
                        let mut clients = TCP_CLIENT_STREAMS.lock().unwrap();
                        clients.remove(&id);
                    }
                    ptr::drop_in_place(socket);
                }
                BIND_RESULT_TYPE_ID => {
                    let res = data_ptr as *mut BindResultData;
                    if !(*res).socket.is_null() {
                        breom_arc_release((*res).socket);
                    }
                    if !(*res).rx.is_null() {
                        breom_arc_release((*res).rx);
                    }
                }
                ERROR_TYPE_ID => {
                    let err = data_ptr as *mut ErrorData;
                    if !(*err).message.is_null() {
                        breom_arc_release((*err).message);
                    }
                }
                TCP_PACKET_TYPE_ID => {
                    let packet = data_ptr as *mut TcpPacketData;
                    if !(*packet).data.is_null() {
                        breom_arc_release((*packet).data);
                    }
                }
                FILE_READER_TYPE_ID => {
                    let reader = data_ptr as *mut BreomReader;
                    ptr::drop_in_place(reader);
                }
                FILE_SCANNER_TYPE_ID => {
                    let scanner = data_ptr as *mut BreomScanner;
                    ptr::drop_in_place(scanner);
                }
                MAP_TYPE_ID => {
                    let map = data_ptr as *mut MapHeader;
                    map::map_dealloc_entries((*map).entries, (*map).cap);
                    ptr::drop_in_place(map);
                }
                _ => {}
            }

            let total_size = ArcHeader::SIZE + (*header).data_size as usize;
            let layout =
                Layout::from_size_align(total_size, ArcHeader::ALIGN).expect("Invalid layout");
            dealloc(header as *mut u8, layout);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_arc_get_ref_count(data_ptr: *mut u8) -> u64 {
    if data_ptr.is_null() {
        return 0;
    }

    unsafe {
        let header = get_header(data_ptr);
        (*header).ref_count.load(Ordering::Acquire)
    }
}

pub(crate) unsafe fn get_header(data_ptr: *mut u8) -> *mut ArcHeader {
    data_ptr.sub(ArcHeader::SIZE) as *mut ArcHeader
}
