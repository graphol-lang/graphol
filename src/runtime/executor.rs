use std::fmt::{Display, Formatter};

use crate::ast::{BlockLiteral, Expr, NodeExpr, Program, ReservedToken};

use super::host::ExecutionHost;
use super::io::{OutputEvent, OutputMode, RuntimeIo};
use super::object::{
    BlockSnapshot, StdoutState, end_object, new_block, new_boolean_operator, new_logic_operator,
    new_node, new_operator, receive_object,
};
use super::scope::{Scope, ScopeRef};
use super::value::{ObjectRef, Value};

#[derive(Debug)]
pub struct RuntimeError {
    message: String,
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RuntimeError {}

struct Frame {
    scope: ScopeRef,
    expressions: std::rc::Rc<Vec<Expr>>,
    pc: usize,
}

struct Thread {
    frames: Vec<Frame>,
}

pub struct RuntimeEngine {
    program: Program,
    stdout: StdoutState,
    io: Box<dyn RuntimeIo>,
    threads: Vec<Thread>,
    current_thread: usize,
    active_thread: usize,
    outputs: Vec<OutputEvent>,
}

impl RuntimeEngine {
    pub fn new(program: Program, io: Box<dyn RuntimeIo>) -> Self {
        Self {
            program,
            stdout: StdoutState::new(),
            io,
            threads: Vec::new(),
            current_thread: 0,
            active_thread: 0,
            outputs: Vec::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), RuntimeError> {
        self.threads.clear();
        self.current_thread = 0;
        self.outputs.clear();

        let root_scope = Scope::new(None, self.stdout.clone_ref());
        self.threads.push(Thread {
            frames: vec![Frame {
                scope: root_scope,
                expressions: self.program.expressions.clone(),
                pc: 0,
            }],
        });

        while !self.threads.is_empty() {
            if self.current_thread >= self.threads.len() {
                self.current_thread = 0;
            }
            let thread_index = self.current_thread;
            self.current_thread += 1;
            self.active_thread = thread_index;
            self.step_thread(thread_index)?;
        }

        Ok(())
    }

    pub fn outputs(&self) -> &[OutputEvent] {
        &self.outputs
    }

    fn step_thread(&mut self, index: usize) -> Result<(), RuntimeError> {
        if index >= self.threads.len() {
            return Ok(());
        }

        enum Step {
            Remove,
            Idle,
            Exec(Expr, ScopeRef),
        }

        let step = {
            let thread = &mut self.threads[index];
            if thread.frames.is_empty() {
                Step::Remove
            } else {
                let top = thread.frames.len() - 1;
                let frame = &mut thread.frames[top];
                if frame.pc >= frame.expressions.len() {
                    thread.frames.pop();
                    if thread.frames.is_empty() {
                        Step::Remove
                    } else {
                        Step::Idle
                    }
                } else {
                    let expr = frame.expressions[frame.pc].clone();
                    let scope = frame.scope.clone();
                    frame.pc += 1;
                    Step::Exec(expr, scope)
                }
            }
        };

        match step {
            Step::Remove => self.remove_thread(index),
            Step::Idle => {}
            Step::Exec(expr, scope) => {
                self.eval_expression(&expr, &scope)?;
            }
        }

        Ok(())
    }

    fn remove_thread(&mut self, index: usize) {
        self.threads.remove(index);
        if self.current_thread > 0 && index < self.current_thread {
            self.current_thread -= 1;
        }
        if self.active_thread > 0 && index < self.active_thread {
            self.active_thread -= 1;
        }
    }

    fn eval_expression(&mut self, expr: &Expr, scope: &ScopeRef) -> Result<ObjectRef, RuntimeError> {
        if expr.nodes.is_empty() {
            let result = new_node();
            end_object(&result);
            return Ok(result);
        }

        let receiver = self.eval_root(&expr.nodes[0], scope)?;
        let message_nodes = &expr.nodes[1..];
        let is_while_receiver = receiver.borrow().get_type() == "whileCommand";
        let is_input_receiver = receiver.borrow().get_type() == "input";

        if is_while_receiver {
            self.eval_while(&receiver, message_nodes, scope)?;
        } else if is_input_receiver {
            if !message_nodes.is_empty() {
                let mut prompt = String::new();
                for node in message_nodes {
                    let message = self.eval_message(node, scope)?;
                    prompt.push_str(&message.as_text());
                }
                receive_object(&receiver, Value::Text(prompt), self);
            }
        } else {
            for node in message_nodes {
                let message = self.eval_message(node, scope)?;
                receive_object(&receiver, message, self);
            }
        }

        end_object(&receiver);
        Ok(receiver)
    }

    fn eval_while(
        &mut self,
        receiver: &ObjectRef,
        message_nodes: &[NodeExpr],
        scope: &ScopeRef,
    ) -> Result<(), RuntimeError> {
        if message_nodes.len() < 2 {
            return Ok(());
        }

        let condition = &message_nodes[0];
        let body = &message_nodes[1];

        loop {
            let evaluated_condition = self.eval_message(condition, scope)?;
            let should_continue = evaluated_condition.as_bool();
            receive_object(receiver, evaluated_condition, self);

            if !should_continue {
                break;
            }

            let evaluated_body = self.eval_message(body, scope)?;
            receive_object(receiver, evaluated_body, self);
        }

        Ok(())
    }

    fn eval_root(&mut self, node: &NodeExpr, scope: &ScopeRef) -> Result<ObjectRef, RuntimeError> {
        match node {
            NodeExpr::Identifier(name) => {
                if let Some(literal) = parse_literal(name) {
                    let node_ref = new_node();
                    receive_object(&node_ref, literal, self);
                    Ok(node_ref)
                } else {
                    Ok(Scope::get(scope, name))
                }
            }
            NodeExpr::StringLiteral(text) => {
                let node_ref = new_node();
                receive_object(&node_ref, Value::Text(text.clone()), self);
                Ok(node_ref)
            }
            NodeExpr::Reserved(token) => {
                let node_ref = new_node();
                let op = reserved_to_object(*token);
                receive_object(&node_ref, Value::Obj(op), self);
                Ok(node_ref)
            }
            NodeExpr::SubExpression(sub) => self.eval_expression(sub, scope),
            NodeExpr::BlockLiteral(block) => Ok(self.block_to_object(block, scope)),
        }
    }

    fn eval_message(&mut self, node: &NodeExpr, scope: &ScopeRef) -> Result<Value, RuntimeError> {
        let value = match node {
            NodeExpr::Identifier(name) => {
                parse_literal(name).unwrap_or_else(|| Value::Obj(Scope::get(scope, name)))
            }
            NodeExpr::StringLiteral(text) => Value::Text(text.clone()),
            NodeExpr::Reserved(token) => Value::Obj(reserved_to_object(*token)),
            NodeExpr::SubExpression(sub) => Value::Obj(self.eval_expression(sub, scope)?),
            NodeExpr::BlockLiteral(block) => Value::Obj(self.block_to_object(block, scope)),
        };
        Ok(value)
    }

    fn block_to_object(&self, block: &BlockLiteral, scope: &ScopeRef) -> ObjectRef {
        new_block(block.id, block.expressions.clone(), scope.clone())
    }
}

impl ExecutionHost for RuntimeEngine {
    fn read_input(&mut self, prompt: &str) -> String {
        self.io.read_input(prompt)
    }

    fn emit_output(&mut self, mode: OutputMode, value: &str) {
        self.outputs.push(OutputEvent {
            mode,
            value: value.to_string(),
        });
        self.io.on_output(mode, value);
    }

    fn call_block(&mut self, block: BlockSnapshot) {
        let child_scope = Scope::new(Some(block.parent_scope), self.stdout.clone_ref());
        Scope::set(&child_scope, "inbox", block.inbox);

        let frame = Frame {
            scope: child_scope,
            expressions: block.expressions,
            pc: 0,
        };

        if block.is_async {
            self.threads.push(Thread {
                frames: vec![frame],
            });
            return;
        }

        if let Some(thread) = self.threads.get_mut(self.active_thread) {
            let depth_before = thread.frames.len();
            thread.frames.push(frame);

            while self
                .threads
                .get(self.active_thread)
                .map(|t| t.frames.len() > depth_before)
                .unwrap_or(false)
            {
                if self.step_thread(self.active_thread).is_err() {
                    break;
                }
            }
        }
    }
}

fn parse_literal(token: &str) -> Option<Value> {
    match token {
        "true" => Some(Value::Bool(true)),
        "false" => Some(Value::Bool(false)),
        _ => token.parse::<f64>().ok().map(Value::Number),
    }
}

fn reserved_to_object(token: ReservedToken) -> ObjectRef {
    match token {
        ReservedToken::Arithmetic(op) => new_operator(op),
        ReservedToken::Logic(op) => new_logic_operator(op),
        ReservedToken::Boolean(op) => new_boolean_operator(op),
    }
}
