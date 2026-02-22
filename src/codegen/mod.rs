use anyhow::{anyhow, Result};
use cranelift_codegen::ir::types as cl_types;
use cranelift_codegen::ir::{InstBuilder, MemFlags};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};
use std::collections::{HashMap, HashSet};

use crate::ast::{common::*, declarations::*, expressions::*, program::*, statements::*, types::*};

mod bindings;
pub mod context;
mod execution;
pub mod expression;
pub mod func;
mod return_types;
pub mod runtime;
pub mod statement;
pub mod structs;
pub mod types;

use self::bindings::bind_native_symbols;
#[cfg(test)]
#[allow(unused_imports)]
pub use self::return_types::is_error_result_type;
pub(crate) use self::return_types::split_generic_args;
pub(crate) use self::return_types::substitute_type_name;
pub use self::return_types::wrap_return_type;
use self::types::{DefineValue, TypeRegistry, VarType};
use context::FunctionContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileMode {
    RunBuild,
    Test,
}

#[derive(Debug, Clone)]
pub struct TestFunction {
    pub display_name: String,
    pub stable_name: String,
    pub function_name: String,
}

#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub conversions: HashSet<String>,
    pub default_conversions: HashMap<String, InterfaceDefaultConversion>,
}

#[derive(Debug, Clone)]
pub struct SynthesizedConversion {
    pub owner: String,
    pub target_type: TypeExpr,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct EnumVariantInfo {
    pub tag: i64,
    pub payload_types: Vec<String>,
}

pub struct CodeGen {
    pub module: JITModule,
    pub functions: HashMap<String, FuncId>,
    pub runtime_functions: HashMap<String, FuncId>,
    pub type_registry: TypeRegistry,
    pub string_data: HashMap<String, DataId>,
    pub lambda_counter: u64,
    pub defines: HashMap<String, DefineValue>,
    pub main_returns_int: bool,
    pub main_throws: bool,
    pub struct_operators: HashMap<(String, String), String>,
    pub struct_conversions: HashMap<(String, String), String>,
    pub synthesized_conversions: HashMap<String, SynthesizedConversion>,
    pub function_return_types: HashMap<String, TypeExpr>,
    pub function_value_types: HashMap<String, VarType>,
    pub function_param_types: HashMap<String, Vec<VarType>>,
    pub function_param_type_exprs: HashMap<String, Vec<TypeExpr>>,
    pub function_generic_params: HashMap<String, Vec<GenericParam>>,
    pub current_package: String,
    pub entry_package: String,
    pub current_imports: HashMap<String, String>,
    pub global_vars: HashMap<String, DataId>,
    pub global_var_types: HashMap<String, VarType>,
    pub package_inits: Vec<FuncId>,
    pub global_var_decls: HashMap<String, Vec<VarDecl>>,
    pub define_error_decls: HashMap<String, Vec<(String, String)>>,
    pub define_error_globals: HashMap<String, DataId>,
    pub declared_inits: HashSet<String>,
    pub function_visibility: HashMap<String, bool>,
    pub struct_visibility: HashMap<String, bool>,
    pub struct_packages: HashMap<String, String>,
    pub generic_struct_params: HashMap<String, Vec<GenericParam>>,
    pub interfaces: HashMap<String, InterfaceInfo>,
    pub enum_variants: HashMap<String, HashMap<String, EnumVariantInfo>>,
    pub struct_parents: HashMap<String, Vec<String>>,
    pub struct_method_resolution: HashMap<(String, String), String>,
    pub struct_conversion_resolution: HashMap<(String, String), String>,
    pub struct_interfaces: HashMap<String, Vec<String>>,
    pub struct_point_fields: HashMap<String, Vec<(String, String)>>,
    pub compile_mode: CompileMode,
    pub tests: Vec<TestFunction>,
}

impl CodeGen {
    pub fn new() -> Result<Self> {
        let mut flag_builder = settings::builder();

        flag_builder.set("opt_level", "speed").unwrap();

        let isa_builder = cranelift_native::builder()
            .map_err(|e| anyhow!("Failed to create ISA builder: {}", e))?;

        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| anyhow!("Failed to build ISA: {}", e))?;

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        bind_native_symbols(&mut builder);

        let mut module = JITModule::new(builder);
        let mut runtime_functions = HashMap::new();

        self::runtime::declare_runtime_functions(&mut module, &mut runtime_functions)?;

        Ok(CodeGen {
            module,
            functions: HashMap::new(),
            runtime_functions,
            type_registry: TypeRegistry::new(),
            string_data: HashMap::new(),
            lambda_counter: 0,
            defines: HashMap::new(),
            main_returns_int: false,
            main_throws: false,
            struct_operators: HashMap::new(),
            struct_conversions: HashMap::new(),
            synthesized_conversions: HashMap::new(),
            function_return_types: HashMap::new(),
            function_value_types: HashMap::new(),
            function_param_types: HashMap::new(),
            function_param_type_exprs: HashMap::new(),
            function_generic_params: HashMap::new(),
            current_package: "main".to_string(),
            entry_package: "main".to_string(),
            current_imports: HashMap::new(),
            global_vars: HashMap::new(),
            global_var_types: HashMap::new(),
            package_inits: Vec::new(),
            global_var_decls: HashMap::new(),
            define_error_decls: HashMap::new(),
            define_error_globals: HashMap::new(),
            declared_inits: HashSet::new(),
            function_visibility: HashMap::new(),
            struct_visibility: HashMap::new(),
            struct_packages: HashMap::new(),
            generic_struct_params: HashMap::new(),
            interfaces: HashMap::new(),
            enum_variants: HashMap::new(),
            struct_parents: HashMap::new(),
            struct_method_resolution: HashMap::new(),
            struct_conversion_resolution: HashMap::new(),
            struct_interfaces: HashMap::new(),
            struct_point_fields: HashMap::new(),
            compile_mode: CompileMode::RunBuild,
            tests: Vec::new(),
        })
    }

    pub fn set_compile_mode(&mut self, mode: CompileMode) {
        self.compile_mode = mode;
    }

    pub fn is_test_mode(&self) -> bool {
        self.compile_mode == CompileMode::Test
    }

    pub fn set_tests(&mut self, tests: Vec<TestFunction>) {
        self.tests = tests;
    }

    pub fn preprocess_program(&mut self, program: &Program) -> Result<()> {
        self.set_current_program_context(program);

        for item in &program.items {
            if let TopLevelItem::Define(define_decl) = item {
                self.process_define(define_decl)?;
            }
        }

        for item in &program.items {
            if let TopLevelItem::Interface(interface_decl) = item {
                self.register_interface(interface_decl)?;
            }
        }

        for item in &program.items {
            if let TopLevelItem::Enum(enum_decl) = item {
                self.register_enum(enum_decl)?;
            }
        }

        for item in &program.items {
            if let TopLevelItem::Struct(struct_decl) = item {
                structs::register_struct(self, struct_decl)?;
            }
        }

        for item in &program.items {
            if let TopLevelItem::Statement(Statement::VarDecl(var_decl)) = item {
                let pkg = self.current_package.clone();
                self.global_var_decls
                    .entry(pkg)
                    .or_default()
                    .push(var_decl.clone());
            }
        }

        Ok(())
    }

    fn register_interface(&mut self, interface_decl: &InterfaceDecl) -> Result<()> {
        let fqcn = self.local_struct_fqcn(&interface_decl.name);
        let mut conversions = HashSet::new();
        let mut default_conversions = HashMap::new();
        for member in &interface_decl.members {
            match member {
                InterfaceMember::Signature(_) => {}
                InterfaceMember::ConversionSignature(conv) => {
                    conversions.insert(self.type_expr_name(&conv.target_type));
                }
                InterfaceMember::DefaultConversion(conv) => {
                    let target_name = self.type_expr_name(&conv.target_type);
                    conversions.insert(target_name.clone());
                    default_conversions.insert(target_name, conv.clone());
                }
                InterfaceMember::DefaultMethod(_) => {}
            }
        }
        self.interfaces.insert(
            fqcn.clone(),
            InterfaceInfo {
                conversions,
                default_conversions,
            },
        );
        Ok(())
    }

    fn register_enum(&mut self, enum_decl: &EnumDecl) -> Result<()> {
        let enum_fqcn = self.local_struct_fqcn(&enum_decl.name);
        let mut variants = HashMap::new();
        let mut max_payload_arity = 0usize;
        let mut payload_types_by_slot: Vec<String> = Vec::new();

        for (tag, variant) in enum_decl.variants.iter().enumerate() {
            if variants.contains_key(&variant.name) {
                return Err(anyhow!(
                    "Duplicate enum variant '{}.{}'",
                    enum_decl.name,
                    variant.name
                ));
            }

            let payload_types: Vec<String> = variant
                .payload_types
                .iter()
                .map(|t| self.type_expr_name(t))
                .collect();

            if payload_types.len() > max_payload_arity {
                max_payload_arity = payload_types.len();
            }

            for (idx, ty) in payload_types.iter().enumerate() {
                if payload_types_by_slot.len() <= idx {
                    payload_types_by_slot.push(ty.clone());
                } else if payload_types_by_slot[idx] != *ty {
                    payload_types_by_slot[idx] = "Int".to_string();
                }
            }

            variants.insert(
                variant.name.clone(),
                EnumVariantInfo {
                    tag: tag as i64,
                    payload_types,
                },
            );
        }

        self.enum_variants.insert(enum_fqcn.clone(), variants);

        let mut fields = vec![("__tag".to_string(), "Int".to_string(), false)];
        for idx in 0..max_payload_arity {
            let ty = payload_types_by_slot
                .get(idx)
                .cloned()
                .unwrap_or_else(|| "Int".to_string());
            fields.push((format!("__payload{}", idx), ty, false));
        }
        self.type_registry.register_struct(&enum_fqcn, fields);
        let is_public = matches!(enum_decl.visibility, Visibility::Public);
        self.struct_visibility.insert(enum_fqcn.clone(), is_public);
        self.struct_packages
            .insert(enum_fqcn.clone(), self.current_package.clone());
        if !enum_decl.generic_params.is_empty() {
            self.generic_struct_params
                .insert(enum_fqcn, enum_decl.generic_params.clone());
        }
        Ok(())
    }

    pub fn resolve_struct_method_name(
        &self,
        struct_name: &str,
        method_name: &str,
    ) -> Option<String> {
        self.try_resolve_struct_method_name(struct_name, method_name)
            .ok()
            .flatten()
    }

    pub fn try_resolve_struct_method_name(
        &self,
        struct_name: &str,
        method_name: &str,
    ) -> Result<Option<String>> {
        let resolved_struct = self.resolve_struct_type_name(struct_name);
        let direct = format!("{}__{}", resolved_struct, method_name);
        if self.functions.contains_key(&direct) {
            return Ok(Some(direct));
        }

        let mut candidates: Vec<(String, String)> = Vec::new();
        let mut stack: Vec<String> = self
            .struct_parents
            .get(&resolved_struct)
            .cloned()
            .unwrap_or_default();
        let mut visited = HashSet::new();

        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            let symbol = format!("{}__{}", current, method_name);
            if self.functions.contains_key(&symbol) {
                candidates.push((current.clone(), symbol));
            }

            if let Some(parents) = self.struct_parents.get(&current) {
                for parent in parents.iter().rev() {
                    stack.push(parent.clone());
                }
            }
        }

        if candidates.is_empty() {
            return Ok(None);
        }
        if candidates.len() == 1 {
            return Ok(Some(candidates[0].1.clone()));
        }

        if let Some(selected_parent) = self
            .struct_method_resolution
            .get(&(resolved_struct.clone(), method_name.to_string()))
        {
            if let Some((_, symbol)) = candidates
                .iter()
                .find(|(owner, _)| owner == selected_parent)
            {
                return Ok(Some(symbol.clone()));
            }
            return Err(anyhow!(
                "Invalid @resolve_inherit for '{}.{}': parent '{}' does not provide the method",
                resolved_struct,
                method_name,
                selected_parent
            ));
        }

        let owners = candidates
            .iter()
            .map(|(owner, _)| owner.clone())
            .collect::<Vec<_>>()
            .join(", ");
        Err(anyhow!(
            "Ambiguous inherited method '{}.{}' from parents: {}. Add @resolve_inherit(\"method:{}\", \"Parent\") on struct.",
            resolved_struct,
            method_name,
            owners,
            method_name
        ))
    }

    #[allow(dead_code)]
    pub fn resolve_struct_conversion_name(
        &self,
        struct_name: &str,
        target_name: &str,
    ) -> Option<String> {
        self.try_resolve_struct_conversion_name(struct_name, target_name)
            .ok()
            .flatten()
    }

    pub fn try_resolve_struct_conversion_name(
        &self,
        struct_name: &str,
        target_name: &str,
    ) -> Result<Option<String>> {
        let resolved_struct = self.resolve_struct_type_name(struct_name);
        let resolved_target = self.resolve_struct_type_name(target_name);

        if let Some(name) = self
            .struct_conversions
            .get(&(resolved_struct.clone(), resolved_target.clone()))
        {
            return Ok(Some(name.clone()));
        }

        let mut candidates: Vec<(String, String)> = Vec::new();
        let mut stack: Vec<String> = self
            .struct_parents
            .get(&resolved_struct)
            .cloned()
            .unwrap_or_default();
        let mut visited = HashSet::new();

        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            if let Some(symbol) = self
                .struct_conversions
                .get(&(current.clone(), resolved_target.clone()))
            {
                candidates.push((current.clone(), symbol.clone()));
            }

            if let Some(parents) = self.struct_parents.get(&current) {
                for parent in parents.iter().rev() {
                    stack.push(parent.clone());
                }
            }
        }

        if candidates.is_empty() {
            return Ok(None);
        }
        if candidates.len() == 1 {
            return Ok(Some(candidates[0].1.clone()));
        }

        if let Some(selected_parent) = self
            .struct_conversion_resolution
            .get(&(resolved_struct.clone(), resolved_target.clone()))
        {
            if let Some((_, symbol)) = candidates
                .iter()
                .find(|(owner, _)| owner == selected_parent)
            {
                return Ok(Some(symbol.clone()));
            }
            return Err(anyhow!(
                "Invalid @resolve_inherit for '{} to {}': parent '{}' does not provide the conversion",
                resolved_struct,
                resolved_target,
                selected_parent
            ));
        }

        let owners = candidates
            .iter()
            .map(|(owner, _)| owner.clone())
            .collect::<Vec<_>>()
            .join(", ");
        Err(anyhow!(
            "Ambiguous inherited conversion '{} -> {}' from parents: {}. Add @resolve_inherit(\"conv:{}\", \"Parent\") on struct.",
            resolved_struct,
            resolved_target,
            owners,
            resolved_target
        ))
    }

    pub(crate) fn struct_implements_interface(
        &self,
        struct_name: &str,
        interface_name: &str,
    ) -> bool {
        let resolved_struct = self.resolve_struct_type_name(struct_name);
        let resolved_iface = self.resolve_struct_type_name(interface_name);
        let mut stack = vec![resolved_struct];
        let mut visited = HashSet::new();

        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            if self
                .struct_interfaces
                .get(&current)
                .map(|ifs| ifs.iter().any(|name| name == &resolved_iface))
                .unwrap_or(false)
            {
                return true;
            }

            if let Some(parents) = self.struct_parents.get(&current) {
                for parent in parents {
                    stack.push(parent.clone());
                }
            }
        }

        false
    }

    fn set_current_program_context(&mut self, program: &Program) {
        self.current_package = program
            .module
            .as_ref()
            .map(|m| m.path.segments.join("."))
            .unwrap_or_else(|| self.entry_package.clone());

        self.current_imports.clear();
        for dep in &program.depends {
            let full_path = dep.path.segments.join(".");
            let alias = dep
                .alias
                .clone()
                .unwrap_or_else(|| dep.path.segments.last().unwrap().clone());
            self.current_imports.insert(alias, full_path);
        }
    }

    pub fn mangle_name(&self, name: &str) -> String {
        if name == "main" && self.current_package == self.entry_package {
            "main".to_string()
        } else {
            format!("{}.{}", self.current_package, name)
        }
    }

    pub fn is_builtin_type_name(name: &str) -> bool {
        matches!(
            name,
            "Int"
                | "Int8"
                | "Int16"
                | "Int32"
                | "Int64"
                | "UInt"
                | "UInt8"
                | "UInt16"
                | "UInt32"
                | "UInt64"
                | "Byte"
                | "Float"
                | "Float32"
                | "Float64"
                | "Bool"
                | "String"
                | "Char"
                | "Void"
                | "Error"
        )
    }

    pub fn resolve_struct_type_name(&self, name: &str) -> String {
        if Self::is_builtin_type_name(name) || name.starts_with("[]") || name.starts_with('[') {
            return name.to_string();
        }
        if let Some((head, tail)) = name.split_once('.') {
            if let Some(pkg) = self.current_imports.get(head) {
                return format!("{}.{}", pkg, tail);
            }
            return name.to_string();
        }
        format!("{}.{}", self.current_package, name)
    }

    pub fn local_struct_fqcn(&self, local_name: &str) -> String {
        format!("{}.{}", self.current_package, local_name)
    }

    pub fn generic_type_name(&self, generic: &GenericType) -> String {
        let base = self.resolve_struct_type_name(&generic.base);
        let args: Vec<String> = generic
            .type_args
            .iter()
            .map(|arg| self.type_expr_name(&arg.type_expr))
            .collect();
        format!("{}<{}>", base, args.join(","))
    }

    pub fn type_expr_name(&self, ty: &TypeExpr) -> String {
        match ty {
            TypeExpr::Base(base) => self.resolve_struct_type_name(&base.name),
            TypeExpr::DynamicArray(arr) => format!("[]{}", self.type_expr_name(&arr.element_type)),
            TypeExpr::Array(arr) => {
                format!("[{}]{}", arr.size, self.type_expr_name(&arr.element_type))
            }
            TypeExpr::Chan(chan) => format!("Channel<{}>", self.type_expr_name(&chan.element_type)),
            TypeExpr::Generic(generic) => self.generic_type_name(generic),
            _ => "Int".to_string(),
        }
    }

    pub fn ensure_instantiated_struct_type(&mut self, type_name: &str) -> Result<()> {
        if self.type_registry.get(type_name).is_some() {
            return Ok(());
        }

        let Some(lt) = type_name.find('<') else {
            return Ok(());
        };
        let Some(gt) = type_name.rfind('>') else {
            return Ok(());
        };
        if gt <= lt {
            return Ok(());
        }

        let base_name = &type_name[..lt];
        let args_str = &type_name[lt + 1..gt];
        let args = split_generic_args(args_str);

        let params = match self.generic_struct_params.get(base_name) {
            Some(params) => params.clone(),
            None => return Ok(()),
        };

        if params.len() != args.len() {
            return Err(anyhow!(
                "Generic argument arity mismatch for '{}': expected {}, got {}",
                base_name,
                params.len(),
                args.len()
            ));
        }

        for (param, arg) in params.iter().zip(args.iter()) {
            if param.constraints.is_empty() {
                continue;
            }

            let satisfies = param
                .constraints
                .iter()
                .any(|constraint| self.constraint_matches_arg(constraint, arg));

            if !satisfies {
                let expected = param
                    .constraints
                    .iter()
                    .map(|c| self.type_expr_name(c))
                    .collect::<Vec<_>>()
                    .join(" | ");
                return Err(anyhow!(
                    "Generic argument '{}' does not satisfy constraint '{}' for '{}'",
                    arg,
                    expected,
                    param.name
                ));
            }
        }

        let base_type = self
            .type_registry
            .get(base_name)
            .ok_or_else(|| anyhow!("Unknown generic base type: {}", base_name))?
            .clone();

        let subst: HashMap<String, String> = params.into_iter().map(|p| p.name).zip(args).collect();

        let mut fields = Vec::new();
        for field in &base_type.fields {
            let resolved_type = substitute_type_name(&field.type_name, &subst);
            fields.push((field.name.clone(), resolved_type, field.is_public));
        }

        self.type_registry.register_struct(type_name, fields);

        if let Some(pkg) = self.struct_packages.get(base_name).cloned() {
            self.struct_packages.insert(type_name.to_string(), pkg);
        }
        if let Some(vis) = self.struct_visibility.get(base_name).copied() {
            self.struct_visibility.insert(type_name.to_string(), vis);
        }
        Ok(())
    }

    pub(crate) fn constraint_matches_arg(&self, constraint: &TypeExpr, arg: &str) -> bool {
        let expected = self.type_expr_name(constraint);
        if expected == arg {
            return true;
        }

        if self.interfaces.contains_key(&expected)
            && self.struct_implements_interface(arg, &expected)
        {
            return true;
        }

        matches!(
            (expected.as_str(), arg),
            ("Int", "Int64" | "Int32" | "Int16" | "Int8")
                | ("UInt", "UInt64" | "UInt32" | "UInt16" | "UInt8" | "Byte")
                | ("Float", "Float64" | "Float32")
        )
    }

    pub fn set_entry_package(&mut self, package: &str) {
        self.entry_package = package.to_string();
    }

    pub fn check_function_access(&self, mangled_name: &str) -> Result<()> {
        if let Some(&is_public) = self.function_visibility.get(mangled_name) {
            if !is_public {
                let func_pkg = mangled_name.rsplitn(2, '.').last().unwrap_or("");
                if func_pkg != self.current_package {
                    return Err(anyhow!(
                        "Cannot access private function '{}' from package '{}'",
                        mangled_name,
                        self.current_package
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn check_field_access(&self, struct_name: &str, field_name: &str) -> Result<()> {
        let resolved = self.resolve_struct_type_name(struct_name);
        let candidates = if resolved == struct_name {
            vec![resolved]
        } else {
            vec![resolved, struct_name.to_string()]
        };

        for candidate in candidates {
            if let Some(struct_pkg) = self.struct_packages.get(&candidate) {
                if struct_pkg != &self.current_package {
                    if let Some(type_info) = self.type_registry.get(&candidate) {
                        if let Some(field) = type_info.get_field(field_name) {
                            if !field.is_public {
                                return Err(anyhow!(
                                    "Cannot access private field '{}.{}' from package '{}'",
                                    candidate,
                                    field_name,
                                    self.current_package
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn declare_program(&mut self, program: &Program) -> Result<()> {
        self.set_current_program_context(program);

        for item in &program.items {
            if let TopLevelItem::Function(func) = item {
                func::declare_function(self, func)?;
            }
        }

        for item in &program.items {
            if let TopLevelItem::Struct(struct_decl) = item {
                structs::declare_struct_methods(self, struct_decl)?;
            }
        }

        let pkg = self.current_package.clone();
        if !self.declared_inits.contains(&pkg) {
            let decls = self.global_var_decls.get(&pkg).cloned().unwrap_or_default();
            let error_defines = self
                .define_error_decls
                .get(&pkg)
                .cloned()
                .unwrap_or_default();

            for decl in &decls {
                let mangled = format!("{}.{}", pkg, decl.name);
                let mut data_desc = DataDescription::new();
                data_desc.define_zeroinit(8);
                let data_id = self
                    .module
                    .declare_data(&mangled, Linkage::Export, true, false)?;
                self.module.define_data(data_id, &data_desc)?;
                self.global_vars.insert(mangled.clone(), data_id);

                let dummy_ctx = FunctionContext::new();
                let ty = expression::typing::infer_expr_type(self, &dummy_ctx, &decl.value);
                self.global_var_types.insert(mangled, ty);
            }

            for (name, _) in &error_defines {
                let mangled = format!("{}.{}", pkg, name);
                let mut data_desc = DataDescription::new();
                data_desc.define_zeroinit(8);
                let data_id = self
                    .module
                    .declare_data(&mangled, Linkage::Export, true, false)?;
                self.module.define_data(data_id, &data_desc)?;
                self.define_error_globals.insert(mangled, data_id);
            }

            if !decls.is_empty() || !error_defines.is_empty() {
                let init_name = format!("{}__init", pkg);
                let sig = self.module.make_signature();
                let func_id = self
                    .module
                    .declare_function(&init_name, Linkage::Export, &sig)?;
                self.functions.insert(init_name, func_id);
                self.package_inits.push(func_id);
            }
            self.declared_inits.insert(pkg);
        }

        Ok(())
    }

    pub fn compile_program_bodies(&mut self, program: &Program) -> Result<()> {
        self.set_current_program_context(program);

        for item in &program.items {
            if let TopLevelItem::Function(func) = item {
                func::compile_function(self, func)?;
            }
        }

        for item in &program.items {
            if let TopLevelItem::Struct(struct_decl) = item {
                structs::compile_struct_methods(self, struct_decl)?;
            }
        }

        let pkg = self.current_package.clone();
        let init_name = format!("{}__init", pkg);
        if let Some(&func_id) = self.functions.get(&init_name) {
            let decls = self.global_var_decls.get(&pkg).cloned().unwrap_or_default();
            let error_defines = self
                .define_error_decls
                .get(&pkg)
                .cloned()
                .unwrap_or_default();
            self.compile_init_function(pkg, func_id, &decls, &error_defines)?;
        }

        Ok(())
    }

    fn compile_init_function(
        &mut self,
        pkg: String,
        func_id: FuncId,
        decls: &[VarDecl],
        error_defines: &[(String, String)],
    ) -> Result<()> {
        let mut ctx = self.module.make_context();
        let mut builder_ctx = cranelift_frontend::FunctionBuilderContext::new();
        let mut builder = cranelift_frontend::FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);

        let entry_block = builder.create_block();
        builder.switch_to_block(entry_block);

        let mut func_ctx = FunctionContext::new();

        for decl in decls {
            let expected_type = if let Some(type_expr) = &decl.type_annotation {
                expression::typing::infer_type_expr_to_var_type_with_codegen(self, type_expr)
            } else {
                expression::typing::infer_expr_type(self, &func_ctx, &decl.value)
            };

            let val = expression::compile_expression_with_type_hint(
                self,
                &mut builder,
                &mut func_ctx,
                &decl.value,
                Some(&expected_type),
            )?;
            let mangled = format!("{}.{}", pkg, decl.name);
            let data_id = *self
                .global_vars
                .get(&mangled)
                .ok_or_else(|| anyhow!("Global not found"))?;
            let data_ref = self.module.declare_data_in_func(data_id, builder.func);
            let addr = builder.ins().symbol_value(cl_types::I64, data_ref);
            builder.ins().store(MemFlags::new(), val, addr, 0);
        }

        for (name, message) in error_defines {
            let msg_val =
                expression::literals::compile_string_literal(self, &mut builder, message)?;
            let err_val = runtime::call_runtime(self, &mut builder, "breom_error_new", &[msg_val])?;

            let mangled = format!("{}.{}", pkg, name);
            let data_id = *self
                .define_error_globals
                .get(&mangled)
                .ok_or_else(|| anyhow!("Define error global not found"))?;
            let data_ref = self.module.declare_data_in_func(data_id, builder.func);
            let addr = builder.ins().symbol_value(cl_types::I64, data_ref);
            builder.ins().store(MemFlags::new(), err_val, addr, 0);
        }

        builder.ins().return_(&[]);
        builder.seal_block(entry_block);
        builder.finalize();

        self.module.define_function(func_id, &mut ctx)?;
        self.module.clear_context(&mut ctx);
        Ok(())
    }

    pub fn convert_type(&self, type_expr: &TypeExpr) -> Option<cl_types::Type> {
        match type_expr {
            TypeExpr::Base(base) => match base.name.as_str() {
                "Int" | "Int64" => Some(cl_types::I64),
                "Int32" => Some(cl_types::I32),
                "Int16" => Some(cl_types::I16),
                "Int8" => Some(cl_types::I8),
                "UInt" | "UInt64" => Some(cl_types::I64),
                "UInt32" => Some(cl_types::I32),
                "UInt16" => Some(cl_types::I16),
                "UInt8" | "Byte" => Some(cl_types::I8),
                "Float" | "Float64" => Some(cl_types::F64),
                "Float32" => Some(cl_types::F32),
                "Bool" => Some(cl_types::I64),
                "String" | "Chan" | "Error" => Some(cl_types::I64),
                _ => Some(cl_types::I64),
            },
            _ => Some(cl_types::I64),
        }
    }

    fn process_define(&mut self, define: &DefineDecl) -> Result<()> {
        let value = expression::const_eval::evaluate_const_expr(self, &define.value)?;

        let final_value = if let Some(ref type_annotation) = define.type_annotation {
            self.convert_define_value(value, type_annotation)?
        } else {
            value
        };

        if let DefineValue::Error(message) = &final_value {
            let pkg = self.current_package.clone();
            self.define_error_decls
                .entry(pkg)
                .or_default()
                .push((define.name.clone(), message.clone()));
        }

        self.defines.insert(define.name.clone(), final_value);
        Ok(())
    }

    fn convert_define_value(
        &self,
        value: DefineValue,
        type_expr: &TypeExpr,
    ) -> Result<DefineValue> {
        let type_name = match type_expr {
            TypeExpr::Base(base) => base.name.as_str(),
            _ => return Ok(value),
        };

        match (type_name, value) {
            ("Int" | "Int64" | "Int32" | "Int16" | "Int8", DefineValue::Int(v)) => {
                Ok(DefineValue::Int(v))
            }
            ("Int" | "Int64" | "Int32" | "Int16" | "Int8", DefineValue::Float(v)) => {
                Ok(DefineValue::Int(v as i64))
            }
            ("Int" | "Int64" | "Int32" | "Int16" | "Int8", DefineValue::Bool(v)) => {
                Ok(DefineValue::Int(if v { 1 } else { 0 }))
            }

            ("Float" | "Float64" | "Float32", DefineValue::Float(v)) => Ok(DefineValue::Float(v)),
            ("Float" | "Float64" | "Float32", DefineValue::Int(v)) => {
                Ok(DefineValue::Float(v as f64))
            }

            ("Bool", DefineValue::Bool(v)) => Ok(DefineValue::Bool(v)),
            ("Bool", DefineValue::Int(v)) => Ok(DefineValue::Bool(v != 0)),

            ("String", DefineValue::String(v)) => Ok(DefineValue::String(v)),
            ("Error", DefineValue::Error(v)) => Ok(DefineValue::Error(v)),

            (_, v) => Ok(v),
        }
    }
}
