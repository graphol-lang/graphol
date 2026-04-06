use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Program {
    pub expressions: Rc<Vec<Expr>>,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub nodes: Vec<NodeExpr>,
}

#[derive(Debug, Clone)]
pub enum NodeExpr {
    Identifier(String),
    StringLiteral(String),
    BlockLiteral(BlockLiteral),
    SubExpression(Box<Expr>),
    Reserved(ReservedToken),
}

#[derive(Debug, Clone)]
pub struct BlockLiteral {
    pub id: usize,
    pub expressions: Rc<Vec<Expr>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservedToken {
    Arithmetic(ArithmeticOp),
    Logic(LogicOp),
    Boolean(BooleanOp),
    Control(ControlOp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
    Xor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    And,
    Or,
    Not,
    Xor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlOp {
    Break,
    Continue,
}

impl ControlOp {
    pub fn keyword(&self) -> &'static str {
        match self {
            Self::Break => "break",
            Self::Continue => "continue",
        }
    }
}
