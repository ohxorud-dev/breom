use anyhow::{anyhow, Result};
use cranelift_codegen::ir::{types, AbiParam, InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;
use cranelift_jit::JITModule;
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};
use std::collections::HashMap;

use crate::codegen::context::FunctionContext;
use crate::codegen::CodeGen;

pub fn declare_runtime_functions(
    module: &mut JITModule,
    runtime_functions: &mut HashMap<String, FuncId>,
) -> Result<()> {
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_arc_alloc", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_arc_alloc".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_arc_retain", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_arc_retain".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_arc_release", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_arc_release".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_string_new", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_string_new".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_string_empty", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_string_empty".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_string_eq", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_string_eq".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_string_concat", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_string_concat".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_string_len", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_string_len".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_int_to_string", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_int_to_string".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::F64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_float_to_string", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_float_to_string".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_string_print", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_string_print".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_string_println", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_string_println".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_test_assert", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_test_assert".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_test_fail", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_test_fail".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_array_new", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_array_new".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_array_from_i64_buffer", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_array_from_i64_buffer".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_array_len", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_array_len".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_array_push", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_array_push".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_array_pop", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_array_pop".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_array_get", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_array_get".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_array_get_checked", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_array_get_checked".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_error_new", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_error_new".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_panic", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_panic".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_array_set", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_array_set".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_map_new", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_map_new".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_map_len", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_map_len".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_map_get", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_map_get".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_map_get_checked", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_map_get_checked".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_map_set", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_map_set".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_map_contains", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_map_contains".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_map_delete", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_map_delete".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_set_new", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_set_new".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_set_len", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_set_len".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_set_add", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_set_add".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_set_contains", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_set_contains".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_set_remove", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_set_remove".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_chan_new", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_chan_new".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_chan_send", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_chan_send".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_chan_recv", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_chan_recv".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_chan_try_recv", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_chan_try_recv".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_spawn", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_spawn".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_net_bind", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_net_bind".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_net_tcp_bind", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_net_tcp_bind".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_net_send", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_net_send".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_net_tcp_connect", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_net_tcp_connect".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_net_tcp_recv", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_net_tcp_recv".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_net_tcp_send", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_net_tcp_send".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_net_http_listen", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_net_http_listen".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id =
            module.declare_function("breom_net_http_response_status", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_net_http_response_status".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id =
            module.declare_function("breom_net_http_response_headers", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_net_http_response_headers".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_net_http_response_body", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_net_http_response_body".to_string(), id);
    }

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_file_read", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_file_read".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_file_read_byte_sum", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_file_read_byte_sum".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_file_write", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_file_write".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_file_append", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_file_append".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_file_exists", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_file_exists".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_file_remove", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_file_remove".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_file_mkdir", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_file_mkdir".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_file_reader_open", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_file_reader_open".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_file_reader_read_all", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_file_reader_read_all".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_file_reader_close", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_file_reader_close".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_file_scanner_open", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_file_scanner_open".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_file_scanner_has_next", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_file_scanner_has_next".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_file_scanner_next_line", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_file_scanner_next_line".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_file_scanner_close", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_file_scanner_close".to_string(), id);
    }

    {
        let sig = module.make_signature();
        let id = module.declare_function("breom_thread_yield", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_thread_yield".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_thread_sleep", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_thread_sleep".to_string(), id);
    }
    {
        let sig = module.make_signature();
        let id = module.declare_function("breom_select_epoch", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_select_epoch".to_string(), id);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function("breom_select_wait", Linkage::Import, &sig)?;
        runtime_functions.insert("breom_select_wait".to_string(), id);
    }

    Ok(())
}

pub fn call_runtime(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    name: &str,
    args: &[Value],
) -> Result<Value> {
    let func_id = *codegen
        .runtime_functions
        .get(name)
        .ok_or_else(|| anyhow!("Runtime function not found: {}", name))?;
    let func_ref = codegen.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, args);
    let results = builder.inst_results(call);
    if results.is_empty() {
        Ok(builder.ins().iconst(types::I64, 0))
    } else {
        Ok(results[0])
    }
}

pub fn arc_retain(codegen: &mut CodeGen, builder: &mut FunctionBuilder, ptr: Value) -> Result<()> {
    call_runtime(codegen, builder, "breom_arc_retain", &[ptr])?;
    Ok(())
}

pub fn arc_release(codegen: &mut CodeGen, builder: &mut FunctionBuilder, ptr: Value) -> Result<()> {
    call_runtime(codegen, builder, "breom_arc_release", &[ptr])?;
    Ok(())
}

pub fn release_var(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &FunctionContext,
    var_name: &str,
) -> Result<()> {
    if let Some(&var) = ctx.variables.get(var_name) {
        let val = builder.use_var(var);
        arc_release(codegen, builder, val)?;
    }
    Ok(())
}

pub fn release_scope_vars(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
) -> Result<()> {
    let vars_to_release = ctx.exit_scope();
    for var_name in vars_to_release {
        release_var(codegen, builder, ctx, &var_name)?;
    }
    Ok(())
}

pub fn release_all_heap_vars(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &FunctionContext,
) -> Result<()> {
    for var_name in ctx.get_all_heap_vars() {
        release_var(codegen, builder, ctx, &var_name)?;
    }
    Ok(())
}

pub fn intern_string(codegen: &mut CodeGen, s: &str) -> Result<DataId> {
    if let Some(&id) = codegen.string_data.get(s) {
        return Ok(id);
    }

    let name = format!("str_{}", codegen.string_data.len());
    let id = codegen
        .module
        .declare_data(&name, Linkage::Local, false, false)?;

    let mut desc = DataDescription::new();
    desc.define(s.as_bytes().to_vec().into_boxed_slice());
    codegen.module.define_data(id, &desc)?;

    codegen.string_data.insert(s.to_string(), id);
    Ok(id)
}
