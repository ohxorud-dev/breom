use anyhow::{anyhow, Result};
use cranelift_codegen::ir::{types, Block, Value};
use cranelift_frontend::{FunctionBuilder, Variable};
use std::collections::HashMap;

use crate::ast::statements::DeferBody;
use crate::codegen::types::VarType;

#[derive(Debug, Clone)]
pub struct LoopContext {
    pub header_block: Block,
    pub exit_block: Block,
}

pub struct FunctionContext {
    pub variables: HashMap<String, Variable>,
    pub var_types: HashMap<String, VarType>,
    pub loop_stack: Vec<LoopContext>,
    pub defer_stack: Vec<DeferBody>,
    pub heap_vars: Vec<String>,
    pub scope_stack: Vec<usize>,
    pub struct_fields: HashMap<String, (String, u64)>,
    pub result_error: Option<Value>,
    pub result_value: Option<Value>,
    pub is_error_result: bool,
    pub expected_return_type: Option<VarType>,
    pub in_result_context: bool,
    pub current_struct: Option<String>,
}

impl FunctionContext {
    pub fn new() -> Self {
        FunctionContext {
            variables: HashMap::new(),
            var_types: HashMap::new(),
            loop_stack: Vec::new(),
            defer_stack: Vec::new(),
            heap_vars: Vec::new(),
            scope_stack: Vec::new(),
            struct_fields: HashMap::new(),
            result_error: None,
            result_value: None,
            is_error_result: false,
            expected_return_type: None,
            in_result_context: false,
            current_struct: None,
        }
    }

    pub fn push_defer(&mut self, body: DeferBody) {
        self.defer_stack.push(body);
    }

    #[cfg(test)]
    pub fn get_defers(&self) -> Vec<&DeferBody> {
        self.defer_stack.iter().rev().collect()
    }

    pub fn register_heap_var(&mut self, name: &str) {
        self.heap_vars.push(name.to_string());
    }

    pub fn enter_scope(&mut self) {
        self.scope_stack.push(self.heap_vars.len());
    }

    pub fn exit_scope(&mut self) -> Vec<String> {
        let start_idx = self.scope_stack.pop().unwrap_or(0);
        self.heap_vars.split_off(start_idx)
    }

    pub fn get_all_heap_vars(&self) -> Vec<String> {
        self.heap_vars.clone()
    }

    pub fn remove_heap_var(&mut self, name: &str) {
        if let Some(pos) = self.heap_vars.iter().position(|v| v == name) {
            self.heap_vars.remove(pos);
        }
    }

    pub fn create_variable(
        &mut self,
        builder: &mut FunctionBuilder,
        name: &str,
        ty: types::Type,
    ) -> Variable {
        let var = builder.declare_var(ty);
        self.variables.insert(name.to_string(), var);
        var
    }

    pub fn set_var_type(&mut self, name: &str, var_type: VarType) {
        self.var_types.insert(name.to_string(), var_type);
    }

    pub fn get_var_type(&self, name: &str) -> VarType {
        self.var_types
            .get(name)
            .cloned()
            .unwrap_or(VarType::Unknown)
    }

    pub fn register_struct_field(&mut self, field_name: &str, struct_name: &str, offset: u64) {
        self.struct_fields
            .insert(field_name.to_string(), (struct_name.to_string(), offset));
    }

    pub fn get_struct_field(&self, field_name: &str) -> Option<&(String, u64)> {
        self.struct_fields.get(field_name)
    }

    pub fn set_variable(
        &self,
        builder: &mut FunctionBuilder,
        name: &str,
        value: Value,
    ) -> Result<()> {
        let var = self
            .variables
            .get(name)
            .ok_or_else(|| anyhow!("Undefined variable: {}", name))?;
        builder.def_var(*var, value);
        Ok(())
    }

    pub fn get_variable(&self, builder: &mut FunctionBuilder, name: &str) -> Result<Value> {
        let var = self
            .variables
            .get(name)
            .ok_or_else(|| anyhow!("Undefined variable: {}", name))?;
        Ok(builder.use_var(*var))
    }

    pub fn push_loop(&mut self, header: Block, exit: Block) {
        self.loop_stack.push(LoopContext {
            header_block: header,
            exit_block: exit,
        });
    }

    pub fn pop_loop(&mut self) {
        self.loop_stack.pop();
    }

    pub fn current_loop_header(&self) -> Option<Block> {
        self.loop_stack.last().map(|l| l.header_block)
    }

    pub fn current_loop_exit(&self) -> Option<Block> {
        self.loop_stack.last().map(|l| l.exit_block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::common::Span;
    use crate::ast::expressions::Expression;
    use crate::codegen::types::VarType;

    #[test]
    fn defer_stack_is_lifo() {
        let mut ctx = FunctionContext::new();
        ctx.push_defer(DeferBody::Expression(Expression::Identifier(
            "first".to_string(),
            Span { start: 0, end: 0 },
        )));
        ctx.push_defer(DeferBody::Expression(Expression::Identifier(
            "second".to_string(),
            Span { start: 0, end: 0 },
        )));

        let defers = ctx.get_defers();
        assert_eq!(defers.len(), 2);

        match defers[0] {
            DeferBody::Expression(Expression::Identifier(name, _)) => assert_eq!(name, "second"),
            _ => panic!("unexpected defer shape"),
        }
        match defers[1] {
            DeferBody::Expression(Expression::Identifier(name, _)) => assert_eq!(name, "first"),
            _ => panic!("unexpected defer shape"),
        }
    }

    #[test]
    fn heap_vars_respect_scope_boundaries() {
        let mut ctx = FunctionContext::new();
        ctx.register_heap_var("a");
        ctx.enter_scope();
        ctx.register_heap_var("b");
        ctx.register_heap_var("c");

        let released = ctx.exit_scope();
        assert_eq!(released, vec!["b".to_string(), "c".to_string()]);
        assert_eq!(ctx.get_all_heap_vars(), vec!["a".to_string()]);
    }

    #[test]
    fn remove_heap_var_and_var_type_lookup_work() {
        let mut ctx = FunctionContext::new();
        ctx.register_heap_var("temp");
        ctx.remove_heap_var("temp");
        assert!(ctx.get_all_heap_vars().is_empty());

        ctx.set_var_type("value", VarType::Int);
        assert_eq!(ctx.get_var_type("value"), VarType::Int);
        assert_eq!(ctx.get_var_type("missing"), VarType::Unknown);
    }

    #[test]
    fn struct_field_registry_roundtrip() {
        let mut ctx = FunctionContext::new();
        ctx.register_struct_field("x", "Point", 16);
        assert_eq!(
            ctx.get_struct_field("x").cloned(),
            Some(("Point".to_string(), 16))
        );
        assert!(ctx.get_struct_field("y").is_none());
    }
}
