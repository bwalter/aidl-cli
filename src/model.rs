use serde_derive::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct Aidl {
    pub root: String,
    pub items: HashMap<String, Item>,
}

#[derive(Serialize, Clone)]
pub struct Item {
    pub path: String,
    #[serde(rename = "itemType")]
    pub item_type: ItemType,
    pub name: String,
    pub elements: HashMap<String, Element>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ItemType {
    Interface,
    Parcelable,
    Enum,
}

#[derive(Serialize, Clone)]
#[serde(tag = "elementType")]
#[serde(rename_all = "camelCase")]
#[allow(clippy::enum_variant_names)]
pub enum Element {
    Method {
        oneway: bool,
        name: String,
        #[serde(rename = "returnType")]
        return_type: String,
        args: Vec<Arg>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        doc: Option<String>,
    },
    Const {
        name: String,
        #[serde(rename = "type")]
        const_type: String,
        value: String,
    },
    Field {
        name: String,
        #[serde(rename = "type")]
        field_type: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        doc: Option<String>,
    },
    EnumElement {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        doc: Option<String>,
    },
}

#[derive(Serialize, Clone)]
pub struct Arg {
    #[serde(default, skip_serializing_if = "Direction::is_unspecified")]
    pub direction: Direction,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub arg_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    In,
    Out,
    InOut,
    Unspecified,
}

impl Direction {
    fn is_unspecified(&self) -> bool {
        matches!(self, &Direction::Unspecified)
    }
}
pub fn ast_type_to_string(t: &aidl_parser::ast::Type) -> String {
    match &t.kind {
        aidl_parser::ast::TypeKind::Array => {
            if t.generic_types.is_empty() {
                t.name.clone()
            } else {
                format!("{}<{}>", t.name, ast_type_to_string(&t.generic_types[0]))
            }
        }
        aidl_parser::ast::TypeKind::Map => {
            if t.generic_types.len() < 2 {
                t.name.clone()
            } else {
                format!(
                    "{}<{}, {}>",
                    t.name,
                    ast_type_to_string(&t.generic_types[0]),
                    ast_type_to_string(&t.generic_types[1])
                )
            }
        }
        aidl_parser::ast::TypeKind::List => {
            if t.generic_types.is_empty() {
                t.name.clone()
            } else {
                format!("{}<{}>", t.name, ast_type_to_string(&t.generic_types[0]))
            }
        }
        aidl_parser::ast::TypeKind::Resolved(qualified_name, _) => qualified_name.clone(), // TODO?
        aidl_parser::ast::TypeKind::Unresolved => t.name.clone(),                          // TODO?
        _ => t.name.clone(),
    }
}

pub fn ast_arg_direction_to_direction(d: &aidl_parser::ast::Direction) -> Direction {
    match d {
        aidl_parser::ast::Direction::In(_) => Direction::In,
        aidl_parser::ast::Direction::Out(_) => Direction::Out,
        aidl_parser::ast::Direction::InOut(_) => Direction::InOut,
        aidl_parser::ast::Direction::Unspecified => Direction::Unspecified,
    }
}
