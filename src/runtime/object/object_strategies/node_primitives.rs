use std::any::Any;

use super::numeric_ops::new_number;
use crate::runtime::host::ExecutionHost;
use crate::runtime::object::{GrapholObject, object_ref, receive_object};
use crate::runtime::value::{ObjectRef, ScalarValue, Value};

pub fn new_node() -> ObjectRef {
    object_ref(NodeObject { strategy: None })
}

pub fn new_string(initial: String) -> ObjectRef {
    object_ref(StringStrategy { value: initial })
}

struct NullStrategy;

impl GrapholObject for NullStrategy {
    fn receive(&mut self, _value: Value, _host: &mut dyn ExecutionHost) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct NodeObject {
    strategy: Option<ObjectRef>,
}

impl GrapholObject for NodeObject {
    fn receive(&mut self, value: Value, host: &mut dyn ExecutionHost) {
        if let Some(strategy) = &self.strategy {
            receive_object(strategy, value, host);
            return;
        }
        self.strategy = Some(strategy_factory(value, host));
    }

    fn exec(&mut self, host: &mut dyn ExecutionHost) {
        if let Some(strategy) = &self.strategy {
            strategy.borrow_mut().exec(host);
        }
    }

    fn end(&mut self) {
        if let Some(strategy) = &self.strategy {
            strategy.borrow_mut().end();
        }
    }

    fn tonumber(&self) -> f64 {
        self.strategy
            .as_ref()
            .map(|strategy| strategy.borrow().tonumber())
            .unwrap_or(0.0)
    }

    fn tostring(&self) -> String {
        self.strategy
            .as_ref()
            .map(|strategy| strategy.borrow().tostring())
            .unwrap_or_default()
    }

    fn toboolean(&self) -> bool {
        self.strategy
            .as_ref()
            .map(|strategy| strategy.borrow().toboolean())
            .unwrap_or(false)
    }

    fn get_type(&self) -> &'static str {
        self.strategy
            .as_ref()
            .map(|strategy| strategy.borrow().get_type())
            .unwrap_or("null")
    }

    fn get_value(&self) -> ScalarValue {
        self.strategy
            .as_ref()
            .map(|strategy| strategy.borrow().get_value())
            .unwrap_or(ScalarValue::Null)
    }

    fn list_len(&self) -> Option<usize> {
        self.strategy
            .as_ref()
            .and_then(|strategy| strategy.borrow().list_len())
    }

    fn list_get(&self, index: usize) -> Option<Value> {
        self.strategy
            .as_ref()
            .and_then(|strategy| strategy.borrow().list_get(index))
    }

    fn list_set(&mut self, index: usize, value: Value) -> bool {
        self.strategy
            .as_ref()
            .map(|strategy| strategy.borrow_mut().list_set(index, value))
            .unwrap_or(false)
    }

    fn list_pop(&mut self) -> Option<Value> {
        self.strategy
            .as_ref()
            .and_then(|strategy| strategy.borrow_mut().list_pop())
    }

    fn list_push(&mut self, value: Value) -> bool {
        self.strategy
            .as_ref()
            .map(|strategy| strategy.borrow_mut().list_push(value))
            .unwrap_or(false)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct StringStrategy {
    value: String,
}

impl GrapholObject for StringStrategy {
    fn receive(&mut self, value: Value, _host: &mut dyn ExecutionHost) {
        self.value.push_str(&value.as_text());
    }

    fn tonumber(&self) -> f64 {
        self.value.parse::<f64>().unwrap_or(0.0)
    }

    fn tostring(&self) -> String {
        self.value.clone()
    }

    fn get_type(&self) -> &'static str {
        "text"
    }

    fn get_value(&self) -> ScalarValue {
        ScalarValue::Text(self.value.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct BooleanStrategy {
    value: bool,
}

impl GrapholObject for BooleanStrategy {
    fn receive(&mut self, value: Value, _host: &mut dyn ExecutionHost) {
        self.value = self.value && value.as_bool();
    }

    fn toboolean(&self) -> bool {
        self.value
    }

    fn get_type(&self) -> &'static str {
        "boolean"
    }

    fn get_value(&self) -> ScalarValue {
        ScalarValue::Bool(self.value)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn strategy_factory(value: Value, host: &mut dyn ExecutionHost) -> ObjectRef {
    match value {
        Value::Obj(obj) => {
            let kind = obj.borrow().get_type().to_string();
            match kind.as_str() {
                "block" => obj,
                "list" => obj,
                "boolean" => object_ref(BooleanStrategy {
                    value: obj.borrow().toboolean(),
                }),
                "number" => new_number(Some(obj.borrow().tonumber())),
                "operator" => {
                    let number = new_number(None);
                    receive_object(&number, Value::Obj(obj), host);
                    number
                }
                "logicOperator" | "booleanOperator" => obj,
                _ => new_string(obj.borrow().tostring()),
            }
        }
        Value::Text(v) => new_string(v),
        Value::Bool(v) => object_ref(BooleanStrategy { value: v }),
        Value::Number(v) => new_number(Some(v)),
        Value::Null => object_ref(NullStrategy),
    }
}
