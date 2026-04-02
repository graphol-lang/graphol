use std::fmt::Write;

use crate::ast::{ArithmeticOp, BooleanOp, LogicOp, ReservedToken};
use crate::ir::{BlockIr, ExprIr, NodeIr, ProgramIr};

const AST_SOURCE: &str = include_str!("../ast.rs");
const HOST_SOURCE: &str = include_str!("../runtime/host.rs");
const IO_SOURCE: &str = include_str!("../runtime/io.rs");
const VALUE_SOURCE: &str = include_str!("../runtime/value.rs");
const OBJECT_SOURCE: &str = include_str!("../runtime/object.rs");
const OBJECT_COMMANDS_SOURCE: &str = include_str!("../runtime/object/object_commands.rs");
const NODE_PRIMITIVES_SOURCE: &str =
    include_str!("../runtime/object/object_strategies/node_primitives.rs");
const NUMERIC_OPS_SOURCE: &str = include_str!("../runtime/object/object_strategies/numeric_ops.rs");
const STRATEGY_PREDICATES_SOURCE: &str =
    include_str!("../runtime/object/object_strategies/strategy_predicates.rs");
const SCOPE_SOURCE: &str = include_str!("../runtime/scope.rs");
const EXECUTOR_SOURCE: &str = include_str!("../runtime/executor.rs");

pub fn generate_rust_source(program: &ProgramIr) -> String {
    let mut out = String::new();
    out.push_str("#![allow(clippy::all)]\n#![allow(dead_code)]\n\n");

    push_module(&mut out, "ast", AST_SOURCE);
    out.push('\n');
    out.push_str(&runtime_module_source());
    out.push('\n');

    out.push_str("fn build_program() -> ast::Program {\n");
    out.push_str("    ast::Program {\n");
    out.push_str("        expressions: std::rc::Rc::new(vec![\n");
    for expr in &program.expressions {
        let _ = writeln!(out, "            {},", render_expr(expr));
    }
    out.push_str("        ]),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    out.push_str("fn main() {\n");
    out.push_str("    let program = build_program();\n");
    out.push_str(
        "    let mut vm = runtime::RuntimeEngine::new(program, Box::new(runtime::StdIo));\n",
    );
    out.push_str("    if let Err(err) = vm.run() {\n");
    out.push_str("        eprintln!(\"error: {}\", err);\n");
    out.push_str("        std::process::exit(1);\n");
    out.push_str("    }\n");
    out.push_str("}\n");

    out
}

fn runtime_module_source() -> String {
    let mut out = String::new();
    out.push_str("mod runtime {\n");

    push_nested_module(&mut out, "host", HOST_SOURCE, 1);
    push_nested_module(&mut out, "io", IO_SOURCE, 1);
    push_nested_module(&mut out, "value", VALUE_SOURCE, 1);

    out.push_str("    pub mod object {\n");
    push_nested_module(&mut out, "object_commands", OBJECT_COMMANDS_SOURCE, 2);

    out.push_str("        mod object_strategies {\n");
    out.push_str("            mod strategy_core {\n");
    push_nested_module(&mut out, "node_primitives", NODE_PRIMITIVES_SOURCE, 4);
    push_nested_module(&mut out, "numeric_ops", NUMERIC_OPS_SOURCE, 4);
    out.push_str("                pub use node_primitives::{new_node, new_string};\n");
    out.push_str("                pub use numeric_ops::{new_number, new_operator};\n");
    out.push_str("            }\n");
    push_nested_module(
        &mut out,
        "strategy_predicates",
        STRATEGY_PREDICATES_SOURCE,
        3,
    );
    out.push_str(
        "            pub use strategy_core::{new_node, new_number, new_operator, new_string};\n",
    );
    out.push_str(
        "            pub use strategy_predicates::{new_boolean_operator, new_logic_operator};\n",
    );
    out.push_str("        }\n\n");

    let cleaned_object_source = strip_object_module_decls(OBJECT_SOURCE);
    for line in cleaned_object_source.lines() {
        let _ = writeln!(out, "        {}", line);
    }
    out.push_str("    }\n");

    push_nested_module(&mut out, "scope", SCOPE_SOURCE, 1);
    push_nested_module(&mut out, "executor", EXECUTOR_SOURCE, 1);

    out.push_str("    pub use io::{OutputEvent, OutputMode, RuntimeIo, StdIo, TestIo};\n");
    out.push_str("    pub use executor::{RuntimeEngine, RuntimeError};\n");
    out.push_str("}\n");

    out
}

fn push_module(target: &mut String, name: &str, body: &str) {
    let _ = writeln!(target, "mod {} {{", name);
    for line in body.lines() {
        let _ = writeln!(target, "    {}", line);
    }
    target.push_str("}\n");
}

fn push_nested_module(target: &mut String, name: &str, body: &str, indentation_levels: usize) {
    let indentation = "    ".repeat(indentation_levels);
    let _ = writeln!(target, "{}mod {} {{", indentation, name);
    for line in body.lines() {
        let _ = writeln!(target, "{}    {}", indentation, line);
    }
    let _ = writeln!(target, "{}}}", indentation);
}

fn strip_object_module_decls(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed != "mod object_commands;" && trimmed != "mod object_strategies;"
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_expr(expr: &ExprIr) -> String {
    let nodes = expr
        .nodes
        .iter()
        .map(render_node)
        .collect::<Vec<_>>()
        .join(", ");
    format!("ast::Expr {{ nodes: vec![{}] }}", nodes)
}

fn render_block(block: &BlockIr) -> String {
    let expressions = block
        .expressions
        .iter()
        .map(render_expr)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "ast::BlockLiteral {{ id: {}, expressions: std::rc::Rc::new(vec![{}]) }}",
        block.id, expressions
    )
}

fn render_node(node: &NodeIr) -> String {
    match node {
        NodeIr::Identifier(name) => {
            format!(
                "ast::NodeExpr::Identifier({}.to_string())",
                rust_string(name)
            )
        }
        NodeIr::StringLiteral(text) => {
            format!(
                "ast::NodeExpr::StringLiteral({}.to_string())",
                rust_string(text)
            )
        }
        NodeIr::BlockLiteral(block) => {
            format!("ast::NodeExpr::BlockLiteral({})", render_block(block))
        }
        NodeIr::SubExpression(expr) => {
            format!(
                "ast::NodeExpr::SubExpression(Box::new({}))",
                render_expr(expr)
            )
        }
        NodeIr::Reserved(token) => {
            format!("ast::NodeExpr::Reserved({})", render_reserved_token(*token))
        }
    }
}

fn render_reserved_token(token: ReservedToken) -> String {
    match token {
        ReservedToken::Arithmetic(op) => {
            format!(
                "ast::ReservedToken::Arithmetic({})",
                render_arithmetic_op(op)
            )
        }
        ReservedToken::Logic(op) => format!("ast::ReservedToken::Logic({})", render_logic_op(op)),
        ReservedToken::Boolean(op) => {
            format!("ast::ReservedToken::Boolean({})", render_boolean_op(op))
        }
    }
}

fn render_arithmetic_op(op: ArithmeticOp) -> &'static str {
    match op {
        ArithmeticOp::Add => "ast::ArithmeticOp::Add",
        ArithmeticOp::Sub => "ast::ArithmeticOp::Sub",
        ArithmeticOp::Mul => "ast::ArithmeticOp::Mul",
        ArithmeticOp::Div => "ast::ArithmeticOp::Div",
        ArithmeticOp::Xor => "ast::ArithmeticOp::Xor",
    }
}

fn render_logic_op(op: LogicOp) -> &'static str {
    match op {
        LogicOp::Eq => "ast::LogicOp::Eq",
        LogicOp::Ne => "ast::LogicOp::Ne",
        LogicOp::Gt => "ast::LogicOp::Gt",
        LogicOp::Lt => "ast::LogicOp::Lt",
        LogicOp::Ge => "ast::LogicOp::Ge",
        LogicOp::Le => "ast::LogicOp::Le",
    }
}

fn render_boolean_op(op: BooleanOp) -> &'static str {
    match op {
        BooleanOp::And => "ast::BooleanOp::And",
        BooleanOp::Or => "ast::BooleanOp::Or",
        BooleanOp::Not => "ast::BooleanOp::Not",
        BooleanOp::Xor => "ast::BooleanOp::Xor",
    }
}

fn rust_string(input: &str) -> String {
    format!("{:?}", input)
}
