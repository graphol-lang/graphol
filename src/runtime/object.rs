mod object_commands;
mod list_commands;
mod list_object;
mod object_strategies;

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::ast::Expr;

use super::host::ExecutionHost;
use super::io::OutputMode;
use super::scope::ScopeRef;
use super::value::{ObjectRef, ScalarValue, Value};

pub use object_commands::{
    new_block, new_echo, new_if, new_input, new_message_async, new_message_else, new_message_run,
    new_stdout, new_while,
};
pub use list_commands::{new_list_get, new_list_len, new_list_pop, new_list_push, new_list_set};
pub use list_object::new_list;
pub use object_strategies::{
    new_boolean_operator, new_logic_operator, new_node, new_number, new_operator, new_string,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Run,
    Async,
    Else,
}

#[derive(Clone)]
pub struct BlockSnapshot {
    pub id: usize,
    pub expressions: Rc<Vec<Expr>>,
    pub parent_scope: ScopeRef,
    pub inbox: ObjectRef,
    pub is_async: bool,
}

pub trait GrapholObject {
    fn receive(&mut self, value: Value, host: &mut dyn ExecutionHost);

    fn exec(&mut self, _host: &mut dyn ExecutionHost) {}

    fn end(&mut self) {}

    fn tonumber(&self) -> f64 {
        0.0
    }

    fn tostring(&self) -> String {
        String::new()
    }

    fn toboolean(&self) -> bool {
        false
    }

    fn get_type(&self) -> &'static str {
        "null"
    }

    fn get_value(&self) -> ScalarValue {
        ScalarValue::Null
    }

    fn get_message(&self) -> Option<MessageKind> {
        None
    }

    fn block_snapshot(&self) -> Option<BlockSnapshot> {
        None
    }

    fn list_len(&self) -> Option<usize> {
        None
    }

    fn list_get(&self, _index: usize) -> Option<Value> {
        None
    }

    fn list_set(&mut self, _index: usize, _value: Value) -> bool {
        false
    }

    fn list_pop(&mut self) -> Option<Value> {
        None
    }

    fn list_push(&mut self, _value: Value) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any;
}

#[derive(Clone)]
pub struct StdoutState {
    mode: Rc<RefCell<OutputMode>>,
}

impl StdoutState {
    pub fn new() -> Self {
        Self {
            mode: Rc::new(RefCell::new(OutputMode::Alert)),
        }
    }

    pub fn mode(&self) -> OutputMode {
        *self.mode.borrow()
    }

    pub fn set_mode(&self, mode: OutputMode) {
        *self.mode.borrow_mut() = mode;
    }

    pub fn clone_ref(&self) -> Self {
        Self {
            mode: self.mode.clone(),
        }
    }
}

pub fn object_ref<T>(value: T) -> ObjectRef
where
    T: GrapholObject + 'static,
{
    Rc::new(RefCell::new(value))
}

pub fn receive_object(receiver: &ObjectRef, value: Value, host: &mut dyn ExecutionHost) {
    receiver.borrow_mut().receive(value, host);
}

pub fn exec_object(receiver: &ObjectRef, host: &mut dyn ExecutionHost) {
    receiver.borrow_mut().exec(host);
}

pub fn end_object(receiver: &ObjectRef) {
    receiver.borrow_mut().end();
}

pub fn message_kind(value: &Value) -> Option<MessageKind> {
    if let Value::Obj(obj) = value {
        return obj.borrow().get_message();
    }
    None
}

pub fn ensure_node_value(value: Value, host: &mut dyn ExecutionHost) -> ObjectRef {
    let node = new_node();
    receive_object(&node, value, host);
    node
}
