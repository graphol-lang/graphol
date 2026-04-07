use std::any::Any;

use super::super::host::ExecutionHost;
use super::super::value::{ObjectRef, Value};
use super::{GrapholObject, object_ref};

pub fn new_list() -> ObjectRef {
    object_ref(ListObject { items: Vec::new() })
}

pub fn list_target(value: &Value) -> Option<ObjectRef> {
    let Value::Obj(obj) = value else {
        return None;
    };

    if obj.borrow().list_len().is_some() {
        return Some(obj.clone());
    }

    None
}

struct ListObject {
    items: Vec<Value>,
}

impl GrapholObject for ListObject {
    fn receive(&mut self, value: Value, _host: &mut dyn ExecutionHost) {
        self.items.push(value);
    }

    fn tonumber(&self) -> f64 {
        self.items.len() as f64
    }

    fn tostring(&self) -> String {
        let values = self
            .items
            .iter()
            .map(format_list_value)
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{}]", values)
    }

    fn toboolean(&self) -> bool {
        !self.items.is_empty()
    }

    fn get_type(&self) -> &'static str {
        "list"
    }

    fn list_len(&self) -> Option<usize> {
        Some(self.items.len())
    }

    fn list_get(&self, index: usize) -> Option<Value> {
        self.items.get(index).cloned()
    }

    fn list_set(&mut self, index: usize, value: Value) -> bool {
        if index >= self.items.len() {
            return false;
        }
        self.items[index] = value;
        true
    }

    fn list_pop(&mut self) -> Option<Value> {
        self.items.pop()
    }

    fn list_push(&mut self, value: Value) -> bool {
        self.items.push(value);
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn format_list_value(value: &Value) -> String {
    match value {
        Value::Number(number) => number.to_string(),
        Value::Text(text) => format!("{:?}", text),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Null => "null".to_string(),
        Value::Obj(obj) => obj.borrow().tostring(),
    }
}
