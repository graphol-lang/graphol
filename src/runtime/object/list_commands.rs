use std::any::Any;

use super::super::host::ExecutionHost;
use super::super::value::{ObjectRef, ScalarValue, Value};
use super::list_object::list_target;
use super::{GrapholObject, object_ref};

pub fn new_list_push() -> ObjectRef {
    object_ref(ListPushCommand { target: None })
}

pub fn new_list_pop() -> ObjectRef {
    object_ref(ListPopCommand {
        target: None,
        result: Value::Null,
    })
}

pub fn new_list_get() -> ObjectRef {
    object_ref(ListGetCommand {
        target: None,
        index: None,
        result: Value::Null,
    })
}

pub fn new_list_set() -> ObjectRef {
    object_ref(ListSetCommand {
        target: None,
        index: None,
        has_index: false,
    })
}

pub fn new_list_len() -> ObjectRef {
    object_ref(ListLenCommand {
        consumed_target: false,
        result: 0,
    })
}

struct ListPushCommand {
    target: Option<ObjectRef>,
}

impl GrapholObject for ListPushCommand {
    fn receive(&mut self, value: Value, _host: &mut dyn ExecutionHost) {
        if self.target.is_none() {
            self.target = list_target(&value);
            return;
        }

        if let Some(target) = &self.target {
            target.borrow_mut().list_push(value);
        }
    }

    fn end(&mut self) {
        self.target = None;
    }

    fn get_type(&self) -> &'static str {
        "command"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct ListPopCommand {
    target: Option<ObjectRef>,
    result: Value,
}

impl GrapholObject for ListPopCommand {
    fn receive(&mut self, value: Value, _host: &mut dyn ExecutionHost) {
        if self.target.is_some() {
            return;
        }

        self.target = list_target(&value);
        self.result = self
            .target
            .as_ref()
            .and_then(|target| target.borrow_mut().list_pop())
            .unwrap_or(Value::Null);
    }

    fn end(&mut self) {
        self.target = None;
    }

    fn tonumber(&self) -> f64 {
        self.result.as_number().unwrap_or(0.0)
    }

    fn tostring(&self) -> String {
        self.result.as_text()
    }

    fn toboolean(&self) -> bool {
        self.result.as_bool()
    }

    fn get_type(&self) -> &'static str {
        "command"
    }

    fn get_value(&self) -> ScalarValue {
        self.result.to_scalar()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct ListGetCommand {
    target: Option<ObjectRef>,
    index: Option<usize>,
    result: Value,
}

impl GrapholObject for ListGetCommand {
    fn receive(&mut self, value: Value, _host: &mut dyn ExecutionHost) {
        if self.target.is_none() {
            self.target = list_target(&value);
            return;
        }

        if self.index.is_none() {
            self.index = value.as_number().and_then(float_to_index);
            self.result = self
                .target
                .as_ref()
                .and_then(|target| self.index.and_then(|index| target.borrow().list_get(index)))
                .unwrap_or(Value::Null);
        }
    }

    fn end(&mut self) {
        self.target = None;
        self.index = None;
    }

    fn tonumber(&self) -> f64 {
        self.result.as_number().unwrap_or(0.0)
    }

    fn tostring(&self) -> String {
        self.result.as_text()
    }

    fn toboolean(&self) -> bool {
        self.result.as_bool()
    }

    fn get_type(&self) -> &'static str {
        "command"
    }

    fn get_value(&self) -> ScalarValue {
        self.result.to_scalar()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct ListSetCommand {
    target: Option<ObjectRef>,
    index: Option<usize>,
    has_index: bool,
}

impl GrapholObject for ListSetCommand {
    fn receive(&mut self, value: Value, _host: &mut dyn ExecutionHost) {
        if self.target.is_none() {
            self.target = list_target(&value);
            return;
        }

        if !self.has_index {
            self.index = value.as_number().and_then(float_to_index);
            self.has_index = true;
            return;
        }

        if let (Some(target), Some(index)) = (&self.target, self.index) {
            target.borrow_mut().list_set(index, value);
        }
    }

    fn end(&mut self) {
        self.target = None;
        self.index = None;
        self.has_index = false;
    }

    fn get_type(&self) -> &'static str {
        "command"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct ListLenCommand {
    consumed_target: bool,
    result: usize,
}

impl GrapholObject for ListLenCommand {
    fn receive(&mut self, value: Value, _host: &mut dyn ExecutionHost) {
        if self.consumed_target {
            return;
        }

        self.result = list_target(&value)
            .and_then(|target| target.borrow().list_len())
            .unwrap_or(0);
        self.consumed_target = true;
    }

    fn end(&mut self) {
        self.consumed_target = false;
    }

    fn tonumber(&self) -> f64 {
        self.result as f64
    }

    fn tostring(&self) -> String {
        self.result.to_string()
    }

    fn toboolean(&self) -> bool {
        self.result > 0
    }

    fn get_type(&self) -> &'static str {
        "command"
    }

    fn get_value(&self) -> ScalarValue {
        ScalarValue::Number(self.result as f64)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn float_to_index(value: f64) -> Option<usize> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 {
        return None;
    }
    Some(value as usize)
}
