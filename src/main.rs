use openscad_rust::{Parser, Executor, Environment};
use std::rc::Rc;
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: openscad-rust <file.scad>");
        std::process::exit(1);
    }
    let input = fs::read_to_string(&args[1]).expect("Failed to read file");
    let parsed = Parser::parse(&input).expect("Parse error");
    let env = Rc::new(Environment::new());
    let mut exec = Executor::new(env);
    for stmt in &parsed.statements {
        if let Err(e) = exec.execute(stmt) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
