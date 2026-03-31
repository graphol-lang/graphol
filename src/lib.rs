pub mod ast;
pub mod parser;
pub mod runtime;
pub mod source_loader;

use std::fmt::{Display, Formatter};
use std::path::Path;

use parser::{ParseError, parse_program};
use runtime::{OutputEvent, RuntimeIo, Vm, VmError};
use source_loader::{IncludeError, load_entry_source, resolve_source};

pub fn run_graphol(source: &str, io: Box<dyn RuntimeIo>) -> Result<Vec<OutputEvent>, GrapholError> {
    let resolved_source = resolve_source(source, None)?;
    run_resolved_graphol(&resolved_source, io)
}

pub fn run_graphol_with_base(
    source: &str,
    base_dir: &Path,
    io: Box<dyn RuntimeIo>,
) -> Result<Vec<OutputEvent>, GrapholError> {
    let resolved_source = resolve_source(source, Some(base_dir))?;
    run_resolved_graphol(&resolved_source, io)
}

pub fn run_graphol_file(
    path: &Path,
    io: Box<dyn RuntimeIo>,
) -> Result<Vec<OutputEvent>, GrapholError> {
    let resolved_source = load_entry_source(path)?;
    run_resolved_graphol(&resolved_source, io)
}

fn run_resolved_graphol(
    source: &str,
    io: Box<dyn RuntimeIo>,
) -> Result<Vec<OutputEvent>, GrapholError> {
    let program = parse_program(source)?;
    let mut vm = Vm::new(program, io);
    vm.run()?;
    Ok(vm.outputs().to_vec())
}

#[derive(Debug)]
pub enum GrapholError {
    Include(IncludeError),
    Parse(ParseError),
    Vm(VmError),
}

impl Display for GrapholError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Include(err) => write!(f, "{err}"),
            Self::Parse(err) => write!(f, "{err}"),
            Self::Vm(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for GrapholError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Include(err) => Some(err),
            Self::Parse(err) => Some(err),
            Self::Vm(err) => Some(err),
        }
    }
}

impl From<IncludeError> for GrapholError {
    fn from(value: IncludeError) -> Self {
        Self::Include(value)
    }
}

impl From<ParseError> for GrapholError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<VmError> for GrapholError {
    fn from(value: VmError) -> Self {
        Self::Vm(value)
    }
}
