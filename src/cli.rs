use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use graphol_rs::parser::parse_program;

#[derive(Debug, Default)]
pub struct CliOptions {
    pub input: Option<PathBuf>,
    pub output: Option<PathBuf>,
}

pub fn parse_cli_args(args: impl IntoIterator<Item = OsString>) -> io::Result<CliOptions> {
    let mut options = CliOptions::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "-o" || arg == "--output" {
            let output = args.next().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "missing value after -o/--output",
                )
            })?;
            options.output = Some(PathBuf::from(output));
            continue;
        }

        if let Some(value) = arg.to_string_lossy().strip_prefix("-o=") {
            options.output = Some(PathBuf::from(value));
            continue;
        }

        if arg.to_string_lossy().starts_with('-') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown option: {}", arg.to_string_lossy()),
            ));
        }

        if options.input.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "multiple input files provided",
            ));
        }
        options.input = Some(PathBuf::from(arg));
    }

    if options.output.is_some() && options.input.is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "output option requires an input .graphol file",
        ));
    }

    Ok(options)
}

pub fn compile_file(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(input)?;
    parse_program(&source)?;

    let (rlib_path, deps_path) = find_runtime_artifacts()?;
    let runner_source = build_runner_source(&source);
    let runner_path = write_runner_source(&runner_source)?;
    let compile_result = compile_runner(&runner_path, output, &rlib_path, &deps_path);
    let _ = fs::remove_file(&runner_path);
    compile_result?;

    Ok(())
}

fn find_runtime_artifacts() -> io::Result<(PathBuf, PathBuf)> {
    let exe = env::current_exe()?;
    let exe_dir = exe.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "failed to resolve executable directory",
        )
    })?;
    let deps_path = exe_dir.join("deps");
    if !deps_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "missing runtime artifacts in target dir; run `cargo build` first",
        ));
    }

    let plain_rlib = exe_dir.join("libgraphol_rs.rlib");
    if plain_rlib.exists() {
        return Ok((plain_rlib, deps_path));
    }

    let mut hashed_rlibs = Vec::new();
    for entry in fs::read_dir(&deps_path)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|file| file.to_str()) {
            if name.starts_with("libgraphol_rs-") && name.ends_with(".rlib") {
                hashed_rlibs.push(path);
            }
        }
    }
    hashed_rlibs.sort_unstable();

    let rlib_path = hashed_rlibs.into_iter().next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not find graphol runtime library; run `cargo build` first",
        )
    })?;
    Ok((rlib_path, deps_path))
}

fn compile_runner(
    runner_path: &Path,
    output: &Path,
    rlib_path: &Path,
    deps_path: &Path,
) -> io::Result<()> {
    let output = Command::new("rustc")
        .arg("--edition=2024")
        .arg(runner_path)
        .arg("-o")
        .arg(output)
        .arg("--extern")
        .arg(format!("graphol_rs={}", rlib_path.display()))
        .arg("-L")
        .arg(format!("dependency={}", deps_path.display()))
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = if stderr.trim().is_empty() {
            "rustc failed to compile generated runner".to_string()
        } else {
            format!("rustc failed: {}", stderr.trim())
        };
        Err(io::Error::other(message))
    }
}

fn write_runner_source(runner_source: &str) -> io::Result<PathBuf> {
    let now_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let file_name = format!("graphol_runner_{}_{}.rs", std::process::id(), now_ns);
    let path = env::temp_dir().join(file_name);
    fs::write(&path, runner_source)?;
    Ok(path)
}

fn build_runner_source(source: &str) -> String {
    let source_literal = string_literal(source);
    format!(
        r#"use graphol_rs::runtime::StdIo;

fn main() {{
    const SOURCE: &str = {source_literal};
    if let Err(err) = graphol_rs::run_graphol(SOURCE, Box::new(StdIo)) {{
        eprintln!("error: {{:?}}", err);
        std::process::exit(1);
    }}
}}
"#
    )
}

fn string_literal(input: &str) -> String {
    for hash_count in 1..=32 {
        let hashes = "#".repeat(hash_count);
        let terminator = format!("\"{}", hashes);
        if !input.contains(&terminator) {
            return format!("r{hashes}\"{input}\"{hashes}");
        }
    }
    format!("{:?}", input)
}

#[cfg(test)]
mod tests {
    use super::{parse_cli_args, string_literal};
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn parses_compile_arguments() {
        let options = parse_cli_args([
            OsString::from("examples/program5.graphol"),
            OsString::from("-o"),
            OsString::from("program5"),
        ])
        .expect("args should be valid");

        assert_eq!(
            options.input,
            Some(PathBuf::from("examples/program5.graphol"))
        );
        assert_eq!(options.output, Some(PathBuf::from("program5")));
    }

    #[test]
    fn output_requires_input() {
        let error = parse_cli_args([OsString::from("-o"), OsString::from("program5")])
            .expect_err("missing input should fail");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn creates_valid_raw_string_literal() {
        let literal = string_literal("value with \"quote\" and \"### terminator");
        assert!(literal.starts_with("r"));
        assert!(literal.contains("value with \"quote\""));
    }
}
