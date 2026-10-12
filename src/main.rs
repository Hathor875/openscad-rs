use eframe::egui;
use eframe::Frame;
use openscad_rust::{Parser, Executor, Environment};
use std::rc::Rc;

struct OpenScadApp {
    source: String,
    errors: Vec<String>,
    model_mesh: Option<Vec<u8>>,
}

impl OpenScadApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            source: String::new(),
            errors: Vec::new(),
            model_mesh: None,
        }
    }

    fn load_file(&mut self, path: &str) {
        self.source = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                self.errors.push(format!("Failed to read file: {e}"));
                return;
            }
        };
        self.errors.clear();
        self.model_mesh = None;
        self.parse_and_eval();
    }

    fn parse_and_eval(&mut self) {
        match Parser::parse_file(&self.source) {
            Ok(parsed) => {
                let env = Rc::new(Environment::new());
                let mut exec = Executor::new(env);
                match exec.execute_source(&parsed) {
                    Ok(()) => {
                        let mesh = exec.build_mesh();
                        self.errors.clear();
                        if let Some(mesh) = mesh {
                            if !mesh.is_empty() {
                                self.model_mesh = Some(mesh.to_stl().into_bytes());
                            }
                        }
                    }
                    Err(e) => {
                        self.errors.push(format!("Evaluation error: {e}"));
                    }
                }
            }
            Err(e) => {
                self.errors.push(format!("Parse error: {e}"));
            }
        }
    }
}

impl eframe::App for OpenScadApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.button("Open").clicked() {}
                ui.separator();
                if ui.button("Run").clicked() {
                    self.parse_and_eval();
                }
                ui.separator();
                if ui.button("Export STL").clicked() {}
                ui.separator();
                if ui.button("Clear").clicked() {
                    self.errors.clear();
                    self.model_mesh = None;
                }
            });
        });

        egui::SidePanel::left("side").show(ctx, |ui| {
            ui.heading("OpenSCAD Rust");
            ui.label("Rust-based OpenSCAD interpreter");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Parse").clicked() {
                    self.parse_and_eval();
                }
                if ui.button("Run").clicked() {
                    self.parse_and_eval();
                }
            });
            ui.separator();
            ui.label("SCAD Source Area");
            if !self.errors.is_empty() {
                ui.colored_label(egui::Color32::RED, "Errors:");
                for err in &self.errors {
                    ui.label(err);
                }
            }
            if self.model_mesh.is_some() {
                ui.label("Model generated successfully!");
            }
        });
    }
}

fn run_cli(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Usage: openscad-rust <file.scad> [-o|--output <file.stl|file.obj>] [--binary]");
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
        std::fs::write(output, contents)
            .map_err(|error| format!("failed to write '{output}': {error}"))?;
    }
    Ok(())
}

fn usage_error(message: &str) -> String {
    format!("{message}\n\nUsage: openscad-rust <file.scad> [-o|--output <file.stl|file.obj>] [--binary]")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // If CLI arguments provided, run in CLI mode
    if !args.is_empty() {
        match run_cli(&args) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    // Otherwise, launch GUI
    let mut options = eframe::NativeOptions::default();
    options.viewport.inner_size = Some(egui::vec2(1280.0, 800.0));
    options.viewport.resizable = Some(true);

    eframe::run_native(
        "OpenSCAD - Rust",
        options,
        Box::new(|_cc| Box::new(OpenScadApp::new(_cc))),
    )
    .expect("failed to run eframe");
}