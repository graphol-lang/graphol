use std::env;

mod cli;
mod runtime_artifacts;

use cli::{compile_file, parse_cli_args};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_cli_args(env::args_os().skip(1))?;
    compile_file(&options.input, &options.output)?;
    println!("generated executable: {}", options.output.display());
    Ok(())
}
