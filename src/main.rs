use std::env;
use std::io::{self, Read};

mod cli;

use cli::{compile_file, parse_cli_args};
use graphol_rs::runtime::StdIo;
use graphol_rs::{run_graphol, run_graphol_file};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_cli_args(env::args_os().skip(1))?;

    match (options.input, options.output) {
        (None, None) => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            run_source(&buffer)?;
        }
        (Some(input), None) => {
            run_file(&input)?;
        }
        (Some(input), Some(output)) => {
            compile_file(&input, &output)?;
            println!("generated executable: {}", output.display());
        }
        (None, Some(_)) => unreachable!("output without input is prevented by parse_cli_args"),
    }

    Ok(())
}

fn run_source(source: &str) -> Result<(), Box<dyn std::error::Error>> {
    run_graphol(source, Box::new(StdIo))?;
    Ok(())
}

fn run_file(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    run_graphol_file(path, Box::new(StdIo))?;
    Ok(())
}
