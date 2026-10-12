use openscad_rust::{Parser, Executor, Environment};
use std::fs;
use std::rc::Rc;

fn main() {
    if let Err(error) = run(std::env::args().skip(1).collect()) {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(());
    }
    let input = args.first()
        .ok_or_else(|| usage_error("missing input file"))?;
    if input.starts_with('-') {
        return Err(usage_error("input file must be the first argument"));
    }

    let mut output = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "-o" | "--output" => {
                index += 1;
                let path = args.get(index)
                    .ok_or_else(|| usage_error("missing path after --output"))?;
                output = Some(path);
            }
            option => return Err(usage_error(&format!("unknown option '{option}'"))),
        }
        index += 1;
    }

    let parsed = Parser::parse_file(input)
        .map_err(|error| format!("failed to parse '{input}': {error}"))?;
    let env = Rc::new(Environment::new());
    let mut exec = Executor::new(env);
    exec.execute_source(&parsed)?;

    if let Some(output) = output {
        let Some(mesh) = exec.build_mesh() else {
            return Err("model produced no geometry".into());
        };
        fs::write(output, mesh.to_stl())
            .map_err(|error| format!("failed to write '{output}': {error}"))?;
    }
    Ok(())
}

fn print_usage() {
    println!("Usage: openscad-rust <file.scad> [-o|--output <file.stl>]");
}

fn usage_error(message: &str) -> String {
    format!("{message}\n\nUsage: openscad-rust <file.scad> [-o|--output <file.stl>]")
}
