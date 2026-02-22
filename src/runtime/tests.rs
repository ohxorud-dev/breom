use super::arc::*;
use super::array::*;
use super::chan::*;
use super::map::*;
use super::net::tcp::*;
use super::sched::*;
use super::string::*;
use super::*;
use std::io::Write;
use std::sync::mpsc;
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn test_arc_alloc_release() {
    let ptr = breom_arc_alloc(64, 0);
    assert!(!ptr.is_null());
    assert_eq!(breom_arc_get_ref_count(ptr), 1);

    breom_arc_retain(ptr);
    assert_eq!(breom_arc_get_ref_count(ptr), 2);

    breom_arc_release(ptr);
    assert_eq!(breom_arc_get_ref_count(ptr), 1);

    breom_arc_release(ptr);
}

#[test]
fn test_string_new() {
    let data = b"Hello";
    let str_ptr = breom_string_new(data.as_ptr(), data.len() as u64);

    assert_eq!(breom_string_len(str_ptr), 5);
    assert_eq!(string_to_rust(str_ptr), "Hello");

    breom_arc_release(str_ptr);
}

#[test]
fn test_string_concat() {
    let a = breom_string_new(b"Hello, ".as_ptr(), 7);
    let b = breom_string_new(b"World!".as_ptr(), 6);
    let c = breom_string_concat(a, b);

    assert_eq!(string_to_rust(c), "Hello, World!");

    breom_arc_release(a);
    breom_arc_release(b);
    breom_arc_release(c);
}

#[test]
fn test_array_from_i64_buffer() {
    let values = [10_i64, 20_i64, 30_i64];
    let arr = breom_array_from_i64_buffer(values.as_ptr(), values.len() as u64);

    assert_eq!(breom_array_len(arr), 3);
    assert_eq!(breom_array_get(arr, 0), 10);
    assert_eq!(breom_array_get(arr, 1), 20);
    assert_eq!(breom_array_get(arr, 2), 30);

    breom_arc_release(arr);
}

#[test]
fn test_wait_does_not_lose_wakeup() {
    let chan_ptr = breom_chan_new(2);
    assert!(!chan_ptr.is_null());

    let chan_usize = chan_ptr as usize;
    let (phase_tx, phase_rx) = mpsc::channel::<&'static str>();
    let (done_tx, done_rx) = mpsc::channel::<()>();

    let handle = thread::spawn(move || {
        let chan_ptr = chan_usize as *mut u8;
        let mut out = 0i64;
        let got = breom_chan_try_recv(chan_ptr, &mut out as *mut i64);
        assert_eq!(got, 0);
        let epoch = breom_select_epoch();

        phase_tx.send("checked").unwrap();
        thread::sleep(Duration::from_millis(40));
        phase_tx.send("about_to_wait").unwrap();

        let _ = breom_select_wait(epoch);
        done_tx.send(()).unwrap();
    });

    let phase = phase_rx.recv_timeout(Duration::from_millis(200)).unwrap();
    assert_eq!(phase, "checked");

    breom_chan_send(chan_ptr, 1);

    let phase = phase_rx.recv_timeout(Duration::from_millis(200)).unwrap();
    assert_eq!(phase, "about_to_wait");

    done_rx.recv_timeout(Duration::from_millis(150)).unwrap();

    handle.join().unwrap();
    breom_arc_release(chan_ptr);
}

#[test]
fn test_map_set_preserves_entries_beyond_initial_capacity() {
    let map_ptr = breom_map_new(1);
    assert!(!map_ptr.is_null());

    let total = 40i64;
    for key in 0..total {
        breom_map_set(map_ptr, key, key * 10);
    }

    assert_eq!(breom_map_len(map_ptr), total as u64);
    for key in 0..total {
        assert_eq!(breom_map_contains(map_ptr, key), 1, "missing key {key}");
        assert_eq!(
            breom_map_get(map_ptr, key),
            key * 10,
            "wrong value for key {key}"
        );
    }

    breom_arc_release(map_ptr);
}

#[test]
fn test_tcp_recv_returns_available_data_without_peer_close() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.write_all(b"hello").unwrap();
        stream.flush().unwrap();
        thread::sleep(Duration::from_millis(500));
    });

    let host = "127.0.0.1";
    let host_ptr = breom_string_new(host.as_ptr(), host.len() as u64);
    let socket_ptr = breom_net_tcp_connect(host_ptr, port as i64);
    breom_arc_release(host_ptr);
    assert!(!socket_ptr.is_null());

    let socket_usize = socket_ptr as usize;
    let (done_tx, done_rx) = mpsc::channel::<usize>();
    let recv_thread = thread::spawn(move || {
        let socket_ptr = socket_usize as *mut u8;
        let data_ptr = breom_net_tcp_recv(socket_ptr);
        done_tx.send(data_ptr as usize).unwrap();
    });

    let result_ptr = done_rx.recv_timeout(Duration::from_millis(150)).unwrap() as *mut u8;
    assert!(!result_ptr.is_null());
    let payload = string_to_rust(result_ptr);
    assert_eq!(payload, "hello");

    breom_arc_release(result_ptr);
    recv_thread.join().unwrap();
    breom_arc_release(socket_ptr);
    server.join().unwrap();
}

#[test]
fn test_arc_refcount_atomic_under_concurrency() {
    for _ in 0..20_000 {
        let ptr = breom_arc_alloc(8, 0);
        let ptr_usize = ptr as usize;
        let gate = Arc::new(Barrier::new(3));

        let gate_a = gate.clone();
        let t1 = thread::spawn(move || {
            gate_a.wait();
            let ptr = ptr_usize as *mut u8;
            breom_arc_retain(ptr);
        });

        let ptr_usize = ptr as usize;
        let gate_b = gate.clone();
        let t2 = thread::spawn(move || {
            gate_b.wait();
            let ptr = ptr_usize as *mut u8;
            breom_arc_retain(ptr);
        });

        gate.wait();
        t1.join().unwrap();
        t2.join().unwrap();

        let rc = breom_arc_get_ref_count(ptr);
        assert_eq!(rc, 3);

        breom_arc_release(ptr);
        breom_arc_release(ptr);
        breom_arc_release(ptr);
    }
}
