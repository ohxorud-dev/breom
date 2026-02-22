use cranelift_jit::JITBuilder;

use crate::runtime as native;

pub(super) fn bind_native_symbols(builder: &mut JITBuilder) {
    builder.symbol("breom_arc_alloc", native::arc::breom_arc_alloc as *const u8);
    builder.symbol(
        "breom_arc_retain",
        native::arc::breom_arc_retain as *const u8,
    );
    builder.symbol(
        "breom_arc_release",
        native::arc::breom_arc_release as *const u8,
    );

    builder.symbol(
        "breom_string_new",
        native::string::breom_string_new as *const u8,
    );
    builder.symbol(
        "breom_string_empty",
        native::string::breom_string_empty as *const u8,
    );
    builder.symbol(
        "breom_string_concat",
        native::string::breom_string_concat as *const u8,
    );
    builder.symbol(
        "breom_string_len",
        native::string::breom_string_len as *const u8,
    );
    builder.symbol(
        "breom_string_eq",
        native::string::breom_string_eq as *const u8,
    );
    builder.symbol(
        "breom_int_to_string",
        native::string::breom_int_to_string as *const u8,
    );
    builder.symbol(
        "breom_float_to_string",
        native::string::breom_float_to_string as *const u8,
    );
    builder.symbol(
        "breom_string_print",
        native::string::breom_string_print as *const u8,
    );
    builder.symbol(
        "breom_string_println",
        native::string::breom_string_println as *const u8,
    );

    builder.symbol(
        "breom_test_begin",
        native::test_api::breom_test_begin as *const u8,
    );
    builder.symbol(
        "breom_test_end",
        native::test_api::breom_test_end as *const u8,
    );
    builder.symbol(
        "breom_test_assert",
        native::test_api::breom_test_assert as *const u8,
    );
    builder.symbol(
        "breom_test_fail",
        native::test_api::breom_test_fail as *const u8,
    );

    builder.symbol(
        "breom_array_new",
        native::array::breom_array_new as *const u8,
    );
    builder.symbol(
        "breom_array_from_i64_buffer",
        native::array::breom_array_from_i64_buffer as *const u8,
    );
    builder.symbol(
        "breom_array_len",
        native::array::breom_array_len as *const u8,
    );
    builder.symbol(
        "breom_array_push",
        native::array::breom_array_push as *const u8,
    );
    builder.symbol(
        "breom_array_pop",
        native::array::breom_array_pop as *const u8,
    );
    builder.symbol(
        "breom_array_get",
        native::array::breom_array_get as *const u8,
    );
    builder.symbol(
        "breom_array_set",
        native::array::breom_array_set as *const u8,
    );
    builder.symbol(
        "breom_array_get_checked",
        native::array::breom_array_get_checked as *const u8,
    );

    builder.symbol(
        "breom_error_new",
        native::error::breom_error_new as *const u8,
    );
    builder.symbol("breom_panic", native::error::breom_panic as *const u8);

    builder.symbol("breom_map_new", native::map::breom_map_new as *const u8);
    builder.symbol("breom_map_len", native::map::breom_map_len as *const u8);
    builder.symbol("breom_map_get", native::map::breom_map_get as *const u8);
    builder.symbol(
        "breom_map_get_checked",
        native::map::breom_map_get_checked as *const u8,
    );
    builder.symbol("breom_map_set", native::map::breom_map_set as *const u8);
    builder.symbol(
        "breom_map_contains",
        native::map::breom_map_contains as *const u8,
    );
    builder.symbol(
        "breom_map_delete",
        native::map::breom_map_delete as *const u8,
    );

    builder.symbol("breom_set_new", native::set::breom_set_new as *const u8);
    builder.symbol("breom_set_len", native::set::breom_set_len as *const u8);
    builder.symbol("breom_set_add", native::set::breom_set_add as *const u8);
    builder.symbol(
        "breom_set_contains",
        native::set::breom_set_contains as *const u8,
    );
    builder.symbol(
        "breom_set_remove",
        native::set::breom_set_remove as *const u8,
    );

    builder.symbol("breom_chan_new", native::chan::breom_chan_new as *const u8);
    builder.symbol(
        "breom_chan_send",
        native::chan::breom_chan_send as *const u8,
    );
    builder.symbol(
        "breom_chan_recv",
        native::chan::breom_chan_recv as *const u8,
    );
    builder.symbol(
        "breom_chan_try_recv",
        native::chan::breom_chan_try_recv as *const u8,
    );
    builder.symbol("breom_spawn", native::sched::breom_spawn as *const u8);
    builder.symbol(
        "breom_thread_yield",
        native::sched::breom_thread_yield as *const u8,
    );
    builder.symbol(
        "breom_thread_sleep",
        native::sched::breom_thread_sleep as *const u8,
    );
    builder.symbol(
        "breom_select_epoch",
        native::sched::breom_select_epoch as *const u8,
    );
    builder.symbol(
        "breom_select_wait",
        native::sched::breom_select_wait as *const u8,
    );

    builder.symbol(
        "breom_net_bind",
        native::net::udp::breom_net_bind as *const u8,
    );
    builder.symbol(
        "breom_net_tcp_bind",
        native::net::tcp::breom_net_tcp_bind as *const u8,
    );
    builder.symbol(
        "breom_net_tcp_connect",
        native::net::tcp::breom_net_tcp_connect as *const u8,
    );
    builder.symbol(
        "breom_net_send",
        native::net::udp::breom_net_send as *const u8,
    );
    builder.symbol(
        "breom_net_tcp_send",
        native::net::tcp::breom_net_tcp_send as *const u8,
    );
    builder.symbol(
        "breom_net_tcp_recv",
        native::net::tcp::breom_net_tcp_recv as *const u8,
    );
    builder.symbol(
        "breom_net_http_listen",
        native::net::http::breom_net_http_listen as *const u8,
    );
    builder.symbol(
        "breom_net_http_response_status",
        native::net::http::breom_net_http_response_status as *const u8,
    );
    builder.symbol(
        "breom_net_http_response_headers",
        native::net::http::breom_net_http_response_headers as *const u8,
    );
    builder.symbol(
        "breom_net_http_response_body",
        native::net::http::breom_net_http_response_body as *const u8,
    );

    builder.symbol(
        "breom_file_read",
        native::file::ops::breom_file_read as *const u8,
    );
    builder.symbol(
        "breom_file_read_byte_sum",
        native::file::ops::breom_file_read_byte_sum as *const u8,
    );
    builder.symbol(
        "breom_file_write",
        native::file::ops::breom_file_write as *const u8,
    );
    builder.symbol(
        "breom_file_append",
        native::file::ops::breom_file_append as *const u8,
    );
    builder.symbol(
        "breom_file_exists",
        native::file::ops::breom_file_exists as *const u8,
    );
    builder.symbol(
        "breom_file_remove",
        native::file::ops::breom_file_remove as *const u8,
    );
    builder.symbol(
        "breom_file_mkdir",
        native::file::ops::breom_file_mkdir as *const u8,
    );
    builder.symbol(
        "breom_file_reader_open",
        native::file::reader::breom_file_reader_open as *const u8,
    );
    builder.symbol(
        "breom_file_reader_read_all",
        native::file::reader::breom_file_reader_read_all as *const u8,
    );
    builder.symbol(
        "breom_file_reader_close",
        native::file::reader::breom_file_reader_close as *const u8,
    );
    builder.symbol(
        "breom_file_scanner_open",
        native::file::scanner::breom_file_scanner_open as *const u8,
    );
    builder.symbol(
        "breom_file_scanner_has_next",
        native::file::scanner::breom_file_scanner_has_next as *const u8,
    );
    builder.symbol(
        "breom_file_scanner_next_line",
        native::file::scanner::breom_file_scanner_next_line as *const u8,
    );
    builder.symbol(
        "breom_file_scanner_close",
        native::file::scanner::breom_file_scanner_close as *const u8,
    );
}
