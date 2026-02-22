use anyhow::{anyhow, Result};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    types, Block, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module};

use crate::ast::{expressions::*, statements::*};
use crate::codegen::context::FunctionContext;
use crate::codegen::expression::{literals, typing};
use crate::codegen::types::VarType;
use crate::codegen::CodeGen;
use crate::codegen::{expression as expr, runtime};

use self::defer::execute_defers;
use self::flows::{compile_for, compile_if};
use self::handlers::{
    compile_assignment_stmt, compile_break_stmt, compile_continue_stmt, compile_defer_stmt,
    compile_expression_stmt, compile_instead_stmt, compile_return_stmt, compile_throw_stmt,
    compile_var_decl_stmt,
};
use self::matching::compile_match;
use self::spawn::compile_spawn;
use self::wait::compile_wait;

pub mod defer;
pub mod flows;
pub mod handlers;
pub mod matching;
pub mod spawn;
pub mod wait;

const ARRAY_LEN_OFFSET: i32 = 0;
const ARRAY_DATA_OFFSET: i64 = 24;
const ARRAY_ELEM_SIZE: i64 = 8;

#[inline]
pub(super) fn compile_array_index_ptr(
    builder: &mut FunctionBuilder,
    array_ptr: Value,
    index: Value,
) -> Value {
    let data_ptr = builder.ins().iadd_imm(array_ptr, ARRAY_DATA_OFFSET);
    let byte_offset = builder.ins().ishl_imm(index, 3);
    builder.ins().iadd(data_ptr, byte_offset)
}

pub(super) fn is_borrowed_heap_expression(expr: &Expression) -> bool {
    match expr {
        Expression::Identifier(..) => true,
        Expression::Grouped(inner, _) => is_borrowed_heap_expression(inner),
        _ => false,
    }
}

pub fn compile_statement(
    codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    ctx: &mut FunctionContext,
    stmt: &Statement,
) -> Result<Option<Value>> {
    match stmt {
        Statement::Return(ret_stmt) => compile_return_stmt(codegen, builder, ctx, ret_stmt),
        Statement::Throw(expr, _) => compile_throw_stmt(codegen, builder, ctx, expr),
        Statement::VarDecl(var_decl) => compile_var_decl_stmt(codegen, builder, ctx, var_decl),
        Statement::Expression(expr_stmt) => {
            compile_expression_stmt(codegen, builder, ctx, expr_stmt)
        }
        Statement::Assignment(assign) => compile_assignment_stmt(codegen, builder, ctx, assign),

        Statement::If(if_stmt) => {
            compile_if(codegen, builder, ctx, if_stmt)?;
            Ok(None)
        }

        Statement::For(for_stmt) => {
            compile_for(codegen, builder, ctx, for_stmt)?;
            Ok(None)
        }

        Statement::Break(_) => compile_break_stmt(builder, ctx),
        Statement::Continue(_) => compile_continue_stmt(builder, ctx),

        Statement::Match(match_stmt) => {
            compile_match(codegen, builder, ctx, match_stmt)?;
            Ok(None)
        }
        Statement::Spawn(spawn_stmt) => {
            compile_spawn(codegen, builder, ctx, spawn_stmt)?;
            Ok(None)
        }
        Statement::Wait(wait_stmt) => {
            compile_wait(codegen, builder, ctx, wait_stmt)?;
            Ok(None)
        }
        Statement::Defer(defer_stmt) => compile_defer_stmt(ctx, defer_stmt),
        Statement::Instead(expr, _) => compile_instead_stmt(codegen, builder, ctx, expr),
    }
}

pub(super) fn apply_assign_op(
    _codegen: &mut CodeGen,
    builder: &mut FunctionBuilder,
    op: AssignOp,
    lhs: Value,
    rhs: Value,
) -> Result<Value> {
    let is_float = builder.func.dfg.value_type(lhs) == types::F64;

    let value = match op {
        AssignOp::AddAssign => {
            if is_float {
                builder.ins().fadd(lhs, rhs)
            } else {
                builder.ins().iadd(lhs, rhs)
            }
        }
        AssignOp::SubAssign => {
            if is_float {
                builder.ins().fsub(lhs, rhs)
            } else {
                builder.ins().isub(lhs, rhs)
            }
        }
        AssignOp::MulAssign => {
            if is_float {
                builder.ins().fmul(lhs, rhs)
            } else {
                builder.ins().imul(lhs, rhs)
            }
        }
        AssignOp::DivAssign => {
            if is_float {
                builder.ins().fdiv(lhs, rhs)
            } else {
                builder.ins().sdiv(lhs, rhs)
            }
        }
        AssignOp::ModAssign => {
            if is_float {
                return Err(anyhow!("%= is not supported for Float"));
            }
            builder.ins().srem(lhs, rhs)
        }
        AssignOp::AndAssign => builder.ins().band(lhs, rhs),
        AssignOp::OrAssign => builder.ins().bor(lhs, rhs),
        AssignOp::XorAssign => builder.ins().bxor(lhs, rhs),
        AssignOp::Assign => return Ok(rhs),
    };

    Ok(value)
}

pub(super) fn var_type_from_type_name(type_name: &str) -> VarType {
    match type_name {
        "Int" | "Int64" | "Int32" | "Int16" | "Int8" | "UInt" | "UInt64" | "UInt32" | "UInt16"
        | "UInt8" | "Byte" | "Char" => VarType::Int,
        "Float" | "Float64" | "Float32" => VarType::Float,
        "Bool" => VarType::Bool,
        "String" => VarType::String,
        "Error" => VarType::Error,
        _ if type_name.contains('.') => VarType::Struct(type_name.to_string()),
        _ => VarType::Unknown,
    }
}

pub(super) fn is_type_assignable(codegen: &CodeGen, expected: &VarType, actual: &VarType) -> bool {
    if matches!(expected, VarType::Unknown) || matches!(actual, VarType::Unknown) {
        return true;
    }

    match (expected, actual) {
        (VarType::Int, VarType::Int)
        | (VarType::Float, VarType::Float)
        | (VarType::Bool, VarType::Bool)
        | (VarType::String, VarType::String)
        | (VarType::Error, VarType::Error) => true,
        (VarType::Float, VarType::Int) => true,
        (VarType::Struct(a), VarType::Struct(b)) => {
            codegen.resolve_struct_type_name(a) == codegen.resolve_struct_type_name(b)
        }
        (VarType::DynamicArray(ae), VarType::DynamicArray(be)) => {
            is_type_assignable(codegen, ae, be)
        }
        (VarType::StaticArray(ae, al), VarType::StaticArray(be, bl)) => {
            al == bl && is_type_assignable(codegen, ae, be)
        }
        (VarType::Map(ak, av), VarType::Map(bk, bv)) => {
            is_type_assignable(codegen, ak, bk) && is_type_assignable(codegen, av, bv)
        }
        (VarType::Set(ae), VarType::Set(be)) => is_type_assignable(codegen, ae, be),
        (VarType::Chan(ae), VarType::Chan(be)) => is_type_assignable(codegen, ae, be),
        (VarType::Tuple(ae), VarType::Tuple(be)) => {
            ae.len() == be.len()
                && ae
                    .iter()
                    .zip(be.iter())
                    .all(|(a, b)| is_type_assignable(codegen, a, b))
        }
        (
            VarType::Lambda {
                params: ap,
                return_type: ar,
            },
            VarType::Lambda {
                params: bp,
                return_type: br,
            },
        ) => {
            ap.len() == bp.len()
                && ap
                    .iter()
                    .zip(bp.iter())
                    .all(|(a, b)| is_type_assignable(codegen, a, b))
                && is_type_assignable(codegen, ar, br)
        }
        _ => false,
    }
}

pub(super) fn clif_type_from_var_type(var_type: &VarType) -> types::Type {
    match var_type {
        VarType::Float => types::F64,
        _ => types::I64,
    }
}
