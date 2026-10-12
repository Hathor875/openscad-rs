use openscad_rust::{Parser, Executor, Environment};
use std::rc::Rc;
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: openscad-rust <file.scad> [-o output.stl]");
        std::process::exit(1);
    }
    let parsed = Parser::parse_file(&args[1]).expect("Parse error");
    let env = Rc::new(Environment::new());
    let mut exec = Executor::new(env);
    if let Err(e) = exec.execute_source(&parsed) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
    if let Some(output) = args.iter().position(|arg| arg == "-o" || arg == "--output")
        .and_then(|index| args.get(index + 1))
    {
        let Some(mesh) = exec.build_mesh() else {
            eprintln!("Error: model produced no geometry");
            std::process::exit(1);
        };
        if let Err(e) = fs::write(output, mesh.to_stl()) {
            eprintln!("Error writing '{}': {}", output, e);
            std::process::exit(1);
        }
    }
}
