use super::*;

fn parse_http_request(raw: &[u8]) -> (String, String, String) {
    let req = String::from_utf8_lossy(raw);
    let mut method = String::new();
    let mut path = String::new();

    if let Some(first_line) = req.lines().next() {
        let mut parts = first_line.split_whitespace();
        method = parts.next().unwrap_or("GET").to_string();
        path = parts.next().unwrap_or("/").to_string();
    }

    let mut body = String::new();
    if let Some((_, b)) = req.split_once("\r\n\r\n") {
        body = b.to_string();
    }

    (method, path, body)
}

fn parse_http_response(raw: &str) -> (i64, String, String) {
    let (head, body) = if let Some((h, b)) = raw.split_once("\r\n\r\n") {
        (h, b)
    } else if let Some((h, b)) = raw.split_once("\n\n") {
        (h, b)
    } else {
        (raw, "")
    };

    let mut status = 0;
    if let Some(first_line) = head.lines().next() {
        let mut parts = first_line.split_whitespace();
        let _ = parts.next();
        status = parts
            .next()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);
    }

    let headers = if let Some((_, rest)) = head.split_once("\r\n") {
        rest.to_string()
    } else if let Some((_, rest)) = head.split_once('\n') {
        rest.to_string()
    } else {
        String::new()
    };

    (status, headers, body.to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_net_http_response_status(raw_ptr: *mut u8) -> i64 {
    if raw_ptr.is_null() {
        return 0;
    }
    let raw = string_to_rust(raw_ptr);
    let (status, _, _) = parse_http_response(&raw);
    status
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_net_http_response_headers(raw_ptr: *mut u8) -> *mut u8 {
    if raw_ptr.is_null() {
        return breom_string_empty();
    }
    let raw = string_to_rust(raw_ptr);
    let (_, headers, _) = parse_http_response(&raw);
    breom_string_new(headers.as_ptr(), headers.len() as u64)
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_net_http_response_body(raw_ptr: *mut u8) -> *mut u8 {
    if raw_ptr.is_null() {
        return breom_string_empty();
    }
    let raw = string_to_rust(raw_ptr);
    let (_, _, body) = parse_http_response(&raw);
    breom_string_new(body.as_ptr(), body.len() as u64)
}

fn status_text(status: i64) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn breom_net_http_listen(port: i64, handler_ptr: i64) -> i64 {
    if handler_ptr == 0 {
        return -1;
    }

    let addr = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(addr) {
        Ok(listener) => listener,
        Err(_) => return -1,
    };

    let handler: extern "C" fn(*mut u8) -> *mut u8 = unsafe { std::mem::transmute(handler_ptr) };

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };

        let mut buf = [0u8; 8192];
        let read_len = match stream.read(&mut buf) {
            Ok(n) if n > 0 => n,
            _ => continue,
        };

        let (method, path, body) = parse_http_request(&buf[..read_len]);

        let method_ptr = breom_string_new(method.as_ptr(), method.len() as u64);
        let path_ptr = breom_string_new(path.as_ptr(), path.len() as u64);
        let body_ptr = breom_string_new(body.as_ptr(), body.len() as u64);

        let req_data_size = 24u64;
        let req_ptr = breom_arc_alloc(req_data_size, HTTP_REQUEST_TYPE_ID);
        unsafe {
            *(req_ptr as *mut *mut u8) = method_ptr;
            *((req_ptr as *mut *mut u8).add(1)) = path_ptr;
            *((req_ptr as *mut *mut u8).add(2)) = body_ptr;
        }

        let response_ptr = handler(req_ptr);

        let (status, response_body) = if response_ptr.is_null() {
            (500, String::from("internal server error"))
        } else {
            let status = unsafe { *(response_ptr as *const i64) };
            let body_ptr = unsafe { *((response_ptr as *const *mut u8).add(1)) };
            (status, string_to_rust(body_ptr))
        };

        let status_line = status_text(status);
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status,
            status_line,
            response_body.len(),
            response_body
        );

        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();

        breom_arc_release(method_ptr);
        breom_arc_release(path_ptr);
        breom_arc_release(body_ptr);
        breom_arc_release(req_ptr);
        if !response_ptr.is_null() {
            breom_arc_release(response_ptr);
        }
    }

    0
}
