use openscad_rust::{Parser, Executor, Environment};
use std::rc::Rc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: openscad-rust <file.scad>");
        std::process::exit(1);
    }
    let parsed = Parser::parse_file(&args[1]).expect("Parse error");
    let env = Rc::new(Environment::new());
    let mut exec = Executor::new(env);
    if let Err(e) = exec.execute_source(&parsed) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
