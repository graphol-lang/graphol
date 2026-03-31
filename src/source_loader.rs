use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct IncludeError {
    pub message: String,
}

impl IncludeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for IncludeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for IncludeError {}

pub fn load_entry_source(path: &Path) -> Result<String, IncludeError> {
    let mut resolver = IncludeResolver::new();
    resolver.load_file(path)
}

pub fn resolve_source(source: &str, base_dir: Option<&Path>) -> Result<String, IncludeError> {
    let mut resolver = IncludeResolver::new();
    resolver.expand_source(source, base_dir)
}

struct IncludeResolver {
    included_files: HashSet<PathBuf>,
    stack: Vec<PathBuf>,
}

impl IncludeResolver {
    fn new() -> Self {
        Self {
            included_files: HashSet::new(),
            stack: Vec::new(),
        }
    }

    fn load_file(&mut self, path: &Path) -> Result<String, IncludeError> {
        let canonical_path = fs::canonicalize(path).map_err(|err| {
            IncludeError::new(format!(
                "failed to resolve include file '{}': {}",
                path.display(),
                err
            ))
        })?;

        if let Some(cycle_at) = self
            .stack
            .iter()
            .position(|current| current == &canonical_path)
        {
            let mut chain: Vec<String> = self.stack[cycle_at..]
                .iter()
                .map(|entry| entry.display().to_string())
                .collect();
            chain.push(canonical_path.display().to_string());
            return Err(IncludeError::new(format!(
                "include cycle detected: {}",
                chain.join(" -> ")
            )));
        }

        if self.included_files.contains(&canonical_path) {
            return Ok(String::new());
        }

        let source = fs::read_to_string(&canonical_path).map_err(|err| {
            IncludeError::new(format!(
                "failed to read include file '{}': {}",
                canonical_path.display(),
                err
            ))
        })?;

        self.stack.push(canonical_path.clone());
        let base_dir = canonical_path
            .parent()
            .map_or(Path::new("."), |parent| parent);
        let expanded = self.expand_source(&source, Some(base_dir));
        self.stack.pop();

        let expanded = expanded?;
        self.included_files.insert(canonical_path);
        Ok(expanded)
    }

    fn expand_source(
        &mut self,
        source: &str,
        base_dir: Option<&Path>,
    ) -> Result<String, IncludeError> {
        let mut out = String::new();

        for (line_no, line) in source.lines().enumerate() {
            let include_target = match parse_include_directive(line) {
                Ok(target) => target,
                Err(message) => {
                    return Err(self.line_error(line_no + 1, &message));
                }
            };

            if let Some(relative_target) = include_target {
                let base_dir = base_dir.ok_or_else(|| {
                    self.line_error(
                        line_no + 1,
                        "include requires file-based execution context to resolve paths",
                    )
                })?;
                let include_path = base_dir.join(&relative_target);
                let expanded = self.load_file(&include_path)?;
                if expanded.is_empty() || expanded.ends_with('\n') {
                    out.push_str(&expanded);
                } else {
                    out.push_str(&expanded);
                    out.push('\n');
                }
                continue;
            }

            out.push_str(line);
            out.push('\n');
        }

        if !source.ends_with('\n') && out.ends_with('\n') {
            out.pop();
        }

        Ok(out)
    }

    fn line_error(&self, line_no: usize, message: &str) -> IncludeError {
        if let Some(current_file) = self.stack.last() {
            return IncludeError::new(format!(
                "{} at {}:{}",
                message,
                current_file.display(),
                line_no
            ));
        }

        IncludeError::new(format!("{} at line {}", message, line_no))
    }
}

fn parse_include_directive(line: &str) -> Result<Option<String>, String> {
    if !contains_include_keyword(line) {
        return Ok(None);
    }

    let trimmed = line.trim();
    if !trimmed.starts_with("include") {
        return Err("invalid use of reserved keyword 'include'".to_string());
    }

    let rest = &trimmed["include".len()..];
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return Err("invalid use of reserved keyword 'include'".to_string());
    }

    let mut chars = rest.chars().peekable();
    while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
        chars.next();
    }

    if chars.peek().is_none() {
        return Err("invalid include directive, expected: include \"path.graphol\"".to_string());
    }

    if chars.next() != Some('"') {
        return Err("invalid include directive, expected: include \"path.graphol\"".to_string());
    }

    let mut target = String::new();
    let mut escaped = false;
    let mut closed = false;
    for ch in chars.by_ref() {
        if escaped {
            target.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            closed = true;
            break;
        }
        target.push(ch);
    }

    if escaped || !closed {
        return Err("invalid include directive, unclosed string literal".to_string());
    }

    while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
        chars.next();
    }

    if chars.peek().is_some() {
        return Err("invalid include directive, expected: include \"path.graphol\"".to_string());
    }

    Ok(Some(target))
}

fn contains_include_keyword(line: &str) -> bool {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let current = chars[i];
        if current == '"' {
            i += 1;
            let mut escaped = false;
            while i < chars.len() {
                let ch = chars[i];
                if escaped {
                    escaped = false;
                    i += 1;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                    i += 1;
                    continue;
                }
                if ch == '"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        if is_name_terminator(current) || current.is_whitespace() {
            i += 1;
            continue;
        }

        let start = i;
        while i < chars.len()
            && !is_name_terminator(chars[i])
            && !chars[i].is_whitespace()
            && chars[i] != '"'
        {
            i += 1;
        }

        let token: String = chars[start..i].iter().collect();
        if token == "include" {
            return true;
        }
    }

    false
}

fn is_name_terminator(c: char) -> bool {
    matches!(c, '+' | '-' | '*' | '/' | '^' | ')' | '(' | '{' | '}')
}
