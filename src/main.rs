use openscad_rust::{Parser, Executor, Environment};
use std::fs;
use std::rc::Rc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let thread = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || run(args))
        .expect("failed to spawn worker thread");

    match thread.join().unwrap_or_else(|_| Err("fatal runtime error: thread panicked".into())) {
        Ok(()) => {}
        Err(error) => {
            eprintln!("Error: {error}");
            std::process::exit(1);
        }
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
    let mut binary = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "-o" | "--output" => {
                index += 1;
                let path = args.get(index)
                    .ok_or_else(|| usage_error("missing path after --output"))?;
                output = Some(path);
            }
            "--binary" => binary = true,
            option => return Err(usage_error(&format!("unknown option '{option}'"))),
        }
        index += 1;
    }

    let parsed = Parser::parse_file(input)
        .map_err(|error| format!("failed to parse '{input}': {error}"))?;
    let env = Rc::new(Environment::new());
    let mut exec = Executor::new(env);
    exec.execute_source(&parsed)
        .map_err(|error| format!("failed to evaluate '{input}': {error}"))?;

    if let Some(output) = output {
        let Some(mesh) = exec.build_mesh() else {
            return Err(format!("model '{input}' produced no geometry"));
        };
        if mesh.is_empty() {
            return Err(format!("model '{input}' produced no exportable faces"));
        }
        let format = output.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
        if binary && format != "stl" {
            return Err("--binary is only supported for STL output".into());
        }
        let contents = match format.as_str() {
            "stl" if binary => mesh.to_binary_stl(),
            "stl" => mesh.to_stl().into_bytes(),
            "obj" => mesh.to_obj().into_bytes(),
            _ => return Err(format!(
                "unsupported output format '.{format}'; use .stl or .obj"
            )),
        };
        fs::write(output, contents)
            .map_err(|error| format!("failed to write '{output}': {error}"))?;
    }
    Ok(())
}

fn print_usage() {
    println!("Usage: openscad-rust <file.scad> [-o|--output <file.stl|file.obj>] [--binary]");
}

fn usage_error(message: &str) -> String {
    format!("{message}\n\nUsage: openscad-rust <file.scad> [-o|--output <file.stl|file.obj>] [--binary]")
}
