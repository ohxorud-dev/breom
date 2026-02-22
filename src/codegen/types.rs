use crate::runtime;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: String,
    pub type_name: String,
    pub offset: u64,
    #[allow(dead_code)]
    pub size: u64,
    pub is_public: bool,
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub size: u64,
    #[allow(dead_code)]
    pub alignment: u64,
    #[allow(dead_code)]
    pub is_heap: bool,
    pub fields: Vec<FieldInfo>,
    pub type_id: u64,
}

impl TypeInfo {
    pub fn primitive(name: &str, size: u64, type_id: u64) -> Self {
        TypeInfo {
            name: name.to_string(),
            size,
            alignment: size.min(8),
            is_heap: false,
            fields: Vec::new(),
            type_id,
        }
    }

    pub fn heap_type(name: &str, type_id: u64) -> Self {
        TypeInfo {
            name: name.to_string(),
            size: 8,
            alignment: 8,
            is_heap: true,
            fields: Vec::new(),
            type_id,
        }
    }

    pub fn get_field(&self, name: &str) -> Option<&FieldInfo> {
        self.fields.iter().find(|f| f.name == name)
    }
}

pub struct TypeRegistry {
    pub types: HashMap<String, TypeInfo>,
    next_type_id: u64,
}

impl TypeRegistry {
    pub fn new() -> Self {
        let mut registry = TypeRegistry {
            types: HashMap::new(),
            next_type_id: 100,
        };

        registry.register(TypeInfo::primitive("Int", 8, 0));
        registry.register(TypeInfo::primitive("Int64", 8, 0));
        registry.register(TypeInfo::primitive("Int32", 4, 1));
        registry.register(TypeInfo::primitive("Int16", 2, 2));
        registry.register(TypeInfo::primitive("Int8", 1, 3));
        registry.register(TypeInfo::primitive("UInt", 8, 4));
        registry.register(TypeInfo::primitive("UInt64", 8, 4));
        registry.register(TypeInfo::primitive("UInt32", 4, 5));
        registry.register(TypeInfo::primitive("UInt16", 2, 6));
        registry.register(TypeInfo::primitive("UInt8", 1, 7));
        registry.register(TypeInfo::primitive("Byte", 1, 7));
        registry.register(TypeInfo::primitive("Float", 8, 8));
        registry.register(TypeInfo::primitive("Float64", 8, 8));
        registry.register(TypeInfo::primitive("Float32", 4, 9));
        registry.register(TypeInfo::primitive("Bool", 8, 10));

        registry.register(TypeInfo::heap_type(
            "String",
            runtime::common::STRING_TYPE_ID,
        ));

        let mut error_info = TypeInfo::heap_type("Error", runtime::common::ERROR_TYPE_ID);
        error_info.fields.push(FieldInfo {
            name: "message".to_string(),
            type_name: "String".to_string(),
            offset: 0,
            size: 8,
            is_public: true,
        });
        registry.register(error_info);

        registry
    }

    pub fn register(&mut self, type_info: TypeInfo) {
        self.types.insert(type_info.name.clone(), type_info);
    }

    pub fn get(&self, name: &str) -> Option<&TypeInfo> {
        self.types.get(name)
    }

    pub fn allocate_type_id(&mut self) -> u64 {
        let id = self.next_type_id;
        self.next_type_id += 1;
        id
    }

    pub fn register_struct(&mut self, name: &str, fields: Vec<(String, String, bool)>) -> TypeInfo {
        let type_id = self.allocate_type_id();
        let mut offset = 0u64;
        let mut field_infos = Vec::new();

        for (field_name, field_type, is_public) in fields {
            let field_size = self.get(&field_type).map(|t| t.size).unwrap_or(8);

            offset = (offset + 7) & !7;

            field_infos.push(FieldInfo {
                name: field_name,
                type_name: field_type,
                offset,
                size: field_size,
                is_public,
            });

            offset += field_size;
        }

        let total_size = (offset + 7) & !7;

        let type_info = TypeInfo {
            name: name.to_string(),
            size: total_size,
            alignment: 8,
            is_heap: true,
            fields: field_infos,
            type_id,
        };

        self.register(type_info.clone());
        type_info
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VarType {
    Int,
    Float,
    Bool,
    String,
    StaticArray(Box<VarType>, usize),
    DynamicArray(Box<VarType>),
    Map(Box<VarType>, Box<VarType>),
    Set(Box<VarType>),
    Struct(std::string::String),
    Lambda {
        params: Vec<VarType>,
        return_type: Box<VarType>,
    },
    Chan(Box<VarType>),
    Tuple(Vec<VarType>),
    Error,
    Unknown,
}

impl VarType {
    pub fn is_heap_type(&self) -> bool {
        matches!(
            self,
            VarType::String
                | VarType::StaticArray(_, _)
                | VarType::DynamicArray(_)
                | VarType::Map(_, _)
                | VarType::Set(_)
                | VarType::Struct(_)
                | VarType::Chan(_)
                | VarType::Tuple(_)
                | VarType::Error
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DefineValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_builtin_types() {
        let registry = TypeRegistry::new();

        assert!(registry.get("Int").is_some());
        assert!(registry.get("String").is_some());

        let error = registry.get("Error").unwrap();
        assert_eq!(error.type_id, runtime::common::ERROR_TYPE_ID);
        assert_eq!(error.fields.len(), 1);
        assert_eq!(error.fields[0].name, "message");
        assert!(error.fields[0].is_public);
    }

    #[test]
    fn register_struct_aligns_fields_to_eight_bytes() {
        let mut registry = TypeRegistry::new();
        let info = registry.register_struct(
            "Pair",
            vec![
                ("a".to_string(), "UInt8".to_string(), true),
                ("b".to_string(), "Int64".to_string(), false),
            ],
        );

        assert_eq!(info.fields.len(), 2);
        assert_eq!(info.fields[0].offset, 0);
        assert_eq!(info.fields[1].offset, 8);
        assert_eq!(info.size, 16);
        assert!(info.is_heap);
    }

    #[test]
    fn allocate_type_id_is_monotonic() {
        let mut registry = TypeRegistry::new();
        let id1 = registry.allocate_type_id();
        let id2 = registry.allocate_type_id();
        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn var_type_heap_detection_matches_runtime_model() {
        assert!(VarType::String.is_heap_type());
        assert!(VarType::DynamicArray(Box::new(VarType::Int)).is_heap_type());
        assert!(!VarType::Int.is_heap_type());
    }
}
