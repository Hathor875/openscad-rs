use eframe::egui;
use eframe::Frame;
use openscad_rust::{Parser, Executor, Environment};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver};


#[derive(Clone, Debug)]
struct MeshData {
    vertices: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

enum EvalResult {
    Log(String),
    Mesh(MeshData, [f32; 3], f32),
    Timing(u128),
}

#[derive(Clone, Copy)]
struct Camera {
    yaw: f32,
    pitch: f32,
    distance: f32,
    target: [f32; 3],
}

impl Default for Camera {
    fn default() -> Self {
        Self { yaw: -0.785, pitch: 0.5, distance: 40.0, target: [0.0, 0.0, 0.0] }
    }
}

struct OpenScadApp {
    source: String,
    console: Vec<String>,
    model_stl: Option<Vec<u8>>,
    current_file: Option<String>,
    dirty: bool,
    evaluating: bool,
    scroll_to_bottom: bool,
    rx: Option<Receiver<EvalResult>>,
    mesh: Option<MeshData>,
    camera: Camera,
    dragging: bool,
    last_pointer_pos: Option<egui::Pos2>,
    auto_preview: bool,
    last_edit: Option<std::time::Instant>,
    eval_ms: Option<u128>,
    error_line: Option<usize>,
    jump_line: Option<usize>,
    show_grid: bool,
    orthographic: bool,
    cursor_world: Option<[f32; 3]>,
}

impl OpenScadApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let demo_source = "// OpenSCAD Rust GUI\n// Write your code here and press Run\n\ncube([10, 10, 10]);\ntranslate([15, 0, 0])\n    sphere(r=5);\n".to_string();
        Self {
            source: demo_source,
            console: vec!["OpenSCAD Rust GUI ready.".to_string()],
            model_stl: None,
            current_file: None,
            dirty: true,
            evaluating: false,
            scroll_to_bottom: false,
            rx: None,
            mesh: None,
            camera: Camera::default(),
            dragging: false,
            last_pointer_pos: None,
            auto_preview: true,
            last_edit: None,
            eval_ms: None,
            error_line: None,
            jump_line: None,
            show_grid: true,
            orthographic: false,
            cursor_world: None,
        }
    }

    fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("OpenSCAD", &["scad"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    self.source = content;
                    self.current_file = Some(path.display().to_string());
                    self.dirty = true;
                    self.console.push(format!("Opened: {}", path.display()));
                }
                Err(e) => {
                    self.console.push(format!("Error reading file: {e}"));
                }
            }
        }
    }

    fn save_file(&mut self) {
        let path = if let Some(ref p) = self.current_file {
            Some(p.clone())
        } else {
            rfd::FileDialog::new()
                .add_filter("OpenSCAD", &["scad"])
                .save_file()
                .map(|p| p.display().to_string())
        };

        if let Some(p) = path {
            match std::fs::write(&p, &self.source) {
                Ok(()) => {
                    self.current_file = Some(p.clone());
                    self.dirty = false;
                    self.console.push(format!("Saved: {p}"));
                }
                Err(e) => {
                    self.console.push(format!("Error saving file: {e}"));
                }
            }
        }
    }

    fn evaluate(&mut self) {
        if self.evaluating {
            return;
        }
        self.evaluating = true;
        self.dirty = false;
        self.console.clear();
        self.model_stl = None;
        self.error_line = None;
        // The previous mesh stays visible until the new result arrives.

        let (tx, rx) = mpsc::channel();
        let source = self.source.clone();
        let path = self.current_file.clone();

        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let _ = tx.send(EvalResult::Log("Evaluating...".to_string()));

            let parsed = match &path {
                Some(p) => Parser::parse_with_path(&source, p.clone()),
                None => Parser::parse(&source),
            };
            let parsed = match parsed {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(EvalResult::Log(format!("Parse error: {e}")));
                    let _ = tx.send(EvalResult::Timing(started.elapsed().as_millis()));
                    let _ = tx.send(EvalResult::Log("DONE".to_string()));
                    return;
                }
            };

            let _ = tx.send(EvalResult::Log("Parsed OK, evaluating...".to_string()));

            let env = Rc::new(Environment::new());
            let mut exec = Executor::new(env);

            match exec.execute_source(&parsed) {
                Ok(()) => {
                    let _ = tx.send(EvalResult::Log("Evaluation complete.".to_string()));
                    let mesh = exec.build_mesh();
                    if let Some(mesh) = mesh {
                        let stl = mesh.to_stl();
                        let vertices: Vec<[f32; 3]> = mesh.vertices.iter()
                            .map(|v| [v[0] as f32, v[1] as f32, v[2] as f32])
                            .collect();
                        let mut indices: Vec<u32> = Vec::new();
                        for face in &mesh.faces {
                            if face[0] != face[1] && face[1] != face[2] && face[0] != face[2] {
                                indices.push(face[0] as u32);
                                indices.push(face[1] as u32);
                                indices.push(face[2] as u32);
                            }
                        }

                        let mut bounds_min = [f32::INFINITY; 3];
                        let mut bounds_max = [f32::NEG_INFINITY; 3];
                        for v in &vertices {
                            for i in 0..3 {
                                bounds_min[i] = bounds_min[i].min(v[i]);
                                bounds_max[i] = bounds_max[i].max(v[i]);
                            }
                        }
                        let center = [
                            (bounds_min[0] + bounds_max[0]) / 2.0,
                            (bounds_min[1] + bounds_max[1]) / 2.0,
                            (bounds_min[2] + bounds_max[2]) / 2.0,
                        ];
                        let extent = (bounds_max[0] - bounds_min[0])
                            .max(bounds_max[1] - bounds_min[1])
                            .max(bounds_max[2] - bounds_min[2]);
                        let distance = (extent * 2.5).max(5.0);

                        let vertex_count = vertices.len();
                        let face_count = mesh.faces.len();
                        let _ = tx.send(EvalResult::Log(
                            format!("Mesh: {vertex_count} vertices, {face_count} faces")
                        ));
                        let _ = tx.send(EvalResult::Log(format!(
                            "Bounds: [{:.2}, {:.2}, {:.2}] to [{:.2}, {:.2}, {:.2}]",
                            bounds_min[0] as f64, bounds_min[1] as f64, bounds_min[2] as f64,
                            bounds_max[0] as f64, bounds_max[1] as f64, bounds_max[2] as f64
                        )));
                        let _ = tx.send(EvalResult::Log(format!("STL size: {} bytes", stl.len())));

                        let mesh_data = MeshData { vertices, indices };
                        let _ = tx.send(EvalResult::Mesh(mesh_data, center, distance));
                        let _ = tx.send(EvalResult::Timing(started.elapsed().as_millis()));
                        let _ = tx.send(EvalResult::Log("DONE".to_string()));
                    } else {
                        let _ = tx.send(EvalResult::Log("No geometry generated.".to_string()));
                        let _ = tx.send(EvalResult::Timing(started.elapsed().as_millis()));
                        let _ = tx.send(EvalResult::Log("DONE".to_string()));
                    }
                }
                Err(e) => {
                    let _ = tx.send(EvalResult::Log(format!("Evaluation error: {e}")));
                    let _ = tx.send(EvalResult::Timing(started.elapsed().as_millis()));
                    let _ = tx.send(EvalResult::Log("DONE".to_string()));
                }
            }
        });

        self.rx = Some(rx);
    }

    fn fit_camera(&mut self) {
        let frame = self.mesh.as_ref().and_then(|mesh| {
            let mut min = [f32::INFINITY; 3];
            let mut max = [f32::NEG_INFINITY; 3];
            for v in &mesh.vertices {
                for i in 0..3 {
                    min[i] = min[i].min(v[i]);
                    max[i] = max[i].max(v[i]);
                }
            }
            if !min[0].is_finite() {
                return None;
            }
            let extent = (max[0] - min[0])
                .max(max[1] - min[1])
                .max(max[2] - min[2]);
            Some((
                [
                    (min[0] + max[0]) / 2.0,
                    (min[1] + max[1]) / 2.0,
                    (min[2] + max[2]) / 2.0,
                ],
                extent,
            ))
        });
        if let Some((center, extent)) = frame {
            self.camera.target = center;
            self.camera.distance = (extent * 2.5).max(5.0);
        }
    }

    fn export_stl(&mut self) {
        if let Some(ref data) = self.model_stl {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("STL", &["stl"])
                .set_file_name("model.stl")
                .save_file()
            {
                match std::fs::write(&path, data) {
                    Ok(()) => {
                        self.console.push(format!("Exported STL: {}", path.display()));
                    }
                    Err(e) => {
                        self.console.push(format!("Error exporting STL: {e}"));
                    }
                }
            }
        } else {
            self.console.push("No model to export. Run evaluation first.".to_string());
        }
    }
}

impl eframe::App for OpenScadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let mut received = false;
        if let Some(ref rx) = self.rx {
            while let Ok(result) = rx.try_recv() {
                received = true;
                match result {
                    EvalResult::Log(msg) => {
                        if msg.starts_with("Parse error:") {
                            self.error_line = extract_error_line(&msg);
                        }
                        self.console.push(msg.clone());
                        if msg == "DONE" {
                            self.evaluating = false;
                        }
                    }
                    EvalResult::Mesh(mesh, center, distance) => {
                        self.mesh = Some(mesh);
                        self.camera.target = center;
                        self.camera.distance = distance;
                    }
                    EvalResult::Timing(ms) => {
                        self.eval_ms = Some(ms);
                        self.console.push(format!("Evaluated in {ms} ms"));
                    }
                }
                self.scroll_to_bottom = true;
            }
        }
        if received || self.evaluating {
            ctx.request_repaint();
        }

        // --- Keyboard shortcuts -------------------------------------------------
        let mut open = false;
        let mut save = false;
        let mut run = false;
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::O) {
                open = true;
            }
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::S) {
                save = true;
            }
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::F5)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::F6)
            {
                run = true;
            }
        });
        if open {
            self.open_file();
        }
        if save {
            self.save_file();
        }
        if run {
            self.evaluate();
        }

        // --- Debounced auto-preview after edits ---------------------------------
        if self.auto_preview && self.dirty && !self.evaluating {
            let ready = self
                .last_edit
                .map(|t| t.elapsed() >= std::time::Duration::from_millis(600))
                .unwrap_or(true);
            if ready {
                self.evaluate();
            }
        }

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.button("Open…").on_hover_text("Ctrl+O").clicked() {
                    self.open_file();
                }
                if ui.button("Save").on_hover_text("Ctrl+S").clicked() {
                    self.save_file();
                }
                ui.separator();
                if ui.button("Run").on_hover_text("F5 / Ctrl+Enter").clicked() {
                    self.evaluate();
                }
                ui.checkbox(&mut self.auto_preview, "Auto-preview")
                    .on_hover_text("Re-render 600 ms after you stop typing (F5 runs now)");
                ui.separator();
                if ui.button("Export STL").clicked() {
                    self.export_stl();
                }
                ui.separator();
                if ui.button("Clear Console").clicked() {
                    self.console.clear();
                }
            });
        });

        egui::SidePanel::left("file_panel").default_width(200.0).show(ctx, |ui| {
            ui.heading("OpenSCAD Rust");
            ui.separator();
            if let Some(ref path) = self.current_file {
                ui.label(format!("File: {}", path));
            } else {
                ui.label("File: <untitled>");
            }
            ui.separator();
            if self.evaluating {
                ui.colored_label(egui::Color32::YELLOW, "Evaluating...");
            }
            if let Some(ref mesh) = self.mesh {
                ui.separator();
                ui.heading("Mesh Info");
                ui.label(format!("Vertices: {}", mesh.vertices.len()));
                ui.label(format!("Triangles: {}", mesh.indices.len() / 3));
            }
            ui.separator();
            ui.collapsing("Camera", |ui| {
                ui.label(format!("Distance: {:.1}", self.camera.distance));
                ui.label(format!("Yaw: {:.2}", self.camera.yaw));
                ui.label(format!("Pitch: {:.2}", self.camera.pitch));
            });
        });

        egui::TopBottomPanel::bottom("status").exact_height(22.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                match self.eval_ms {
                    Some(ms) => ui.label(format!("Eval: {ms} ms")),
                    None => ui.label("Eval: –"),
                };
                ui.separator();
                match &self.mesh {
                    Some(m) => {
                        ui.label(format!("Tris: {}", m.indices.len() / 3));
                    }
                    None => {
                        ui.label("Tris: –");
                    }
                }
                ui.separator();
                match self.cursor_world {
                    Some(p) => ui.label(format!("X: {:.1}  Y: {:.1}  Z: {:.1}", p[0], p[1], p[2])),
                    None => ui.label("X: –  Y: –  Z: –"),
                };
                ui.separator();
                ui.label(format!("Zoom: {:.1}", self.camera.distance));
                ui.separator();
                ui.label(if self.orthographic { "Ortho" } else { "Persp" });
                ui.separator();
                ui.label(if self.auto_preview { "Preview: auto" } else { "Preview: off" });
            });
        });

        egui::TopBottomPanel::bottom("console").default_height(150.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Console");
                if let Some(line) = self.error_line {
                    if ui.small_button(format!("→ jump to line {line}")).clicked() {
                        self.jump_line = Some(line);
                    }
                }
            });
            egui::ScrollArea::new([true, true]).show(ui, |ui| {
                for line in &self.console {
                    let color = if line.contains("Error") || line.contains("error") {
                        egui::Color32::RED
                    } else if line == "DONE" {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::GRAY
                    };
                    ui.colored_label(color, line);
                }
                if self.scroll_to_bottom {
                    ui.scroll_to_cursor(Some(egui::Align::Max));
                    self.scroll_to_bottom = false;
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let avail = ui.available_rect_before_wrap();
            let editor_width = avail.width() * 0.5;

            let editor_rect = egui::Rect::from_min_size(
                avail.min,
                egui::vec2(editor_width, avail.height()),
            );
            let viewport_rect = egui::Rect::from_min_size(
                egui::pos2(avail.min.x + editor_width, avail.min.y),
                egui::vec2(avail.width() - editor_width, avail.height()),
            );

            ui.allocate_ui_at_rect(editor_rect, |ui| {
                ui.heading("Editor");
                egui::ScrollArea::new([false, true]).show(ui, |ui| {
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.source)
                            .code_editor()
                            .desired_rows(25)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace),
                    );
                    if response.changed() {
                        self.dirty = true;
                        self.last_edit = Some(std::time::Instant::now());
                        self.error_line = None;
                    }
                    if let Some(line) = self.jump_line.take() {
                        let start = char_offset_for_line(&self.source, line);
                        let end = start
                            + self.source[start..]
                                .find('\n')
                                .unwrap_or(self.source.len() - start);
                        response.request_focus();
                        if let Some(mut state) =
                            egui::text_edit::TextEditState::load(ui.ctx(), response.id)
                        {
                            state.set_ccursor_range(Some(
                                egui::text_edit::CCursorRange::two(
                                    egui::text::CCursor::new(start),
                                    egui::text::CCursor::new(end),
                                ),
                            ));
                            state.store(ui.ctx(), response.id);
                        }
                    }
                });
            });

            ui.allocate_ui_at_rect(viewport_rect, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("3D Viewport");
                    ui.separator();
                    if ui
                        .selectable_label(self.orthographic, "Ortho")
                        .on_hover_text("Toggle orthographic / perspective projection")
                        .clicked()
                    {
                        self.orthographic = !self.orthographic;
                    }
                    if ui
                        .selectable_label(self.show_grid, "Grid")
                        .on_hover_text("Toggle ground grid and scale")
                        .clicked()
                    {
                        self.show_grid = !self.show_grid;
                    }
                    ui.separator();
                    if ui.button("Iso").clicked() {
                        self.camera.yaw = -0.785;
                        self.camera.pitch = 0.5;
                    }
                    if ui.button("Front").clicked() {
                        self.camera.yaw = -std::f32::consts::FRAC_PI_2;
                        self.camera.pitch = 0.0;
                    }
                    if ui.button("Right").clicked() {
                        self.camera.yaw = 0.0;
                        self.camera.pitch = 0.0;
                    }
                    if ui.button("Top").clicked() {
                        self.camera.yaw = -std::f32::consts::FRAC_PI_2;
                        self.camera.pitch = 1.5;
                    }
                    if ui.button("Fit").clicked() {
                        self.fit_camera();
                    }
                });

                let size = ui.available_size();
                let (response, painter) = ui.allocate_painter(size, egui::Sense::drag());

                if let Some(ref mesh) = self.mesh {
                    render_mesh_2d(
                        &painter,
                        mesh,
                        self.camera,
                        self.show_grid,
                        self.orthographic,
                        response.rect,
                    );
                } else {
                    painter.rect_filled(
                        response.rect,
                        0.0,
                        egui::Color32::from_rgb(40, 44, 52),
                    );
                    painter.text(
                        response.rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Press Run (F5) to evaluate",
                        egui::FontId::proportional(16.0),
                        egui::Color32::GRAY,
                    );
                }

                self.cursor_world = response.hover_pos().and_then(|pos| {
                    world_at_screen_point(pos, response.rect, &self.camera, self.orthographic)
                });
                if !response.hovered() {
                    self.cursor_world = None;
                }

                if response.drag_started() {
                    self.dragging = true;
                }
                if response.drag_released() {
                    self.dragging = false;
                    self.last_pointer_pos = None;
                }
                if self.dragging {
                    if let Some(current_pos) = response.hover_pos() {
                        if let Some(last_pos) = self.last_pointer_pos {
                            let delta = current_pos - last_pos;
                            self.camera.yaw += delta.x * 0.01;
                            self.camera.pitch = (self.camera.pitch + delta.y * 0.01)
                                .clamp(-1.5, 1.5);
                        }
                        self.last_pointer_pos = Some(current_pos);
                    }
                }
                if response.hovered() {
                    let scroll = ui.input(|i| i.scroll_delta.y);
                    if scroll != 0.0 {
                        self.camera.distance *= 1.0 - scroll * 0.01;
                        self.camera.distance = self.camera.distance.clamp(1.0, 500.0);
                    }
                }
            });
        });
    }
}

fn render_mesh_2d(
    painter: &egui::Painter,
    mesh: &MeshData,
    camera: Camera,
    show_grid: bool,
    orthographic: bool,
    rect: egui::Rect,
) {
    let aspect = if rect.width() > 0.0 && rect.height() > 0.0 {
        rect.width() / rect.height()
    } else {
        1.0
    };
    let eye = camera_eye(&camera);
    let (proj, view) = build_matrices(eye, camera.target, aspect, camera.distance, orthographic);
    let light_dir = normalize([0.4, 0.5, 0.9]);

    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(40, 44, 52));

    let project = |p: [f32; 3]| -> Option<egui::Pos2> {
        let clip = mat4_mul_vec4(proj, mat4_mul_vec4(view, [p[0], p[1], p[2], 1.0f32]));
        if clip[3] < 0.001 {
            return None;
        }
        let ndc_x = clip[0] / clip[3];
        let ndc_y = clip[1] / clip[3];
        Some(egui::pos2(
            rect.left() + (ndc_x * 0.5 + 0.5) * rect.width(),
            rect.top() + (1.0 - (ndc_y * 0.5 + 0.5)) * rect.height(),
        ))
    };

    let draw_line = |a: [f32; 3], b: [f32; 3], color: egui::Color32, width: f32| {
        if let (Some(pa), Some(pb)) = (project(a), project(b)) {
            let bounds = egui::Rect::from_min_max(
                egui::pos2(pa.x.min(pb.x), pa.y.min(pb.y)),
                egui::pos2(pa.x.max(pb.x), pa.y.max(pb.y)),
            );
            if rect.intersects(bounds) && (pa - pb).length_sq() > 0.25 {
                painter.line_segment([pa, pb], egui::Stroke::new(width, color));
            }
        }
    };

    let indices: Vec<[u32; 3]> = mesh.indices.chunks_exact(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();

    let mut normals = vec![[0.0f32; 3]; mesh.vertices.len()];
    for tri in &indices {
        let a = mesh.vertices[tri[0] as usize];
        let b = mesh.vertices[tri[1] as usize];
        let c = mesh.vertices[tri[2] as usize];
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        for &i in tri {
            normals[i as usize][0] += n[0];
            normals[i as usize][1] += n[1];
            normals[i as usize][2] += n[2];
        }
    }
    for n in &mut normals {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len > 0.0 {
            n[0] /= len;
            n[1] /= len;
            n[2] /= len;
        }
    }

    let mut step = 1.0f32;
    while camera.distance / step > 40.0 {
        step *= match step as i32 {
            1 => 5.0,
            _ => 2.0,
        };
    }
    let extent = step * 16.0;

    if show_grid {
        let minor = egui::Color32::from_gray(58);
        let mut k = -extent;
        while k <= extent + 0.001 {
            let c = k;
            if c.abs() > 0.001 {
                draw_line([-extent, c, 0.0], [extent, c, 0.0], minor, 1.0);
                draw_line([c, -extent, 0.0], [c, extent, 0.0], minor, 1.0);
            }
            k += step;
        }
    }
    draw_line([-extent, 0.0, 0.0], [extent, 0.0, 0.0], egui::Color32::from_rgb(220, 80, 80), 1.5);
    draw_line([0.0, -extent, 0.0], [0.0, extent, 0.0], egui::Color32::from_rgb(80, 200, 80), 1.5);
    draw_line([0.0, 0.0, 0.0], [0.0, 0.0, extent], egui::Color32::from_rgb(90, 140, 240), 1.5);

    if show_grid {
        let tick = step * 0.25;
        let font = egui::FontId::monospace(11.0);
        let mut label_k = -extent;
        while label_k <= extent + 0.001 {
            let c = label_k;
            if c.abs() > 0.001 {
                draw_line([c, -tick, 0.0], [c, tick, 0.0], egui::Color32::from_gray(140), 1.0);
                draw_line([-tick, c, 0.0], [tick, c, 0.0], egui::Color32::from_gray(140), 1.0);
            }
            if let Some(p) = project([c, 0.0, 0.0]) {
                if rect.contains(p) && c.abs() > 0.001 {
                    let text = if step >= 1.0 {
                        format!("{}", c as i64)
                    } else {
                        format!("{c:.2}")
                    };
                    painter.text(
                        p + egui::vec2(3.0, -14.0),
                        egui::Align2::LEFT_TOP,
                        text,
                        font.clone(),
                        egui::Color32::from_gray(150),
                    );
                }
            }
            if let Some(p) = project([0.0, c, 0.0]) {
                if rect.contains(p) && c.abs() > 0.001 {
                    let text = if step >= 1.0 {
                        format!("{}", c as i64)
                    } else {
                        format!("{c:.2}")
                    };
                    painter.text(
                        p + egui::vec2(3.0, 3.0),
                        egui::Align2::LEFT_TOP,
                        text,
                        font.clone(),
                        egui::Color32::from_gray(150),
                    );
                }
            }
            label_k += step;
        }
    }

    let mut triangles: Vec<([egui::epaint::Vertex; 3], f32)> = Vec::with_capacity(indices.len());

    for tri in &indices {
        let mut verts = [egui::epaint::Vertex {
            pos: egui::Pos2::ZERO,
            uv: egui::Pos2::ZERO,
            color: egui::Color32::WHITE,
        }; 3];
        let mut depth = 0.0f32;
        let mut valid = true;
        for (slot, &i) in verts.iter_mut().zip(tri.iter()) {
            let v = mesh.vertices[i as usize];
            let n = normals[i as usize];
            let view_pos = mat4_mul_vec4(view, [v[0], v[1], v[2], 1.0f32]);
            let clip_pos = mat4_mul_vec4(proj, view_pos);
            if clip_pos[3] < 0.001 {
                valid = false;
                break;
            }
            depth += (view_pos[0] * view_pos[0]
                + view_pos[1] * view_pos[1]
                + view_pos[2] * view_pos[2])
                .sqrt();
            let ndc_x = clip_pos[0] / clip_pos[3];
            let ndc_y = clip_pos[1] / clip_pos[3];
            let sx = rect.left() + (ndc_x * 0.5 + 0.5) * rect.width();
            let sy = rect.top() + (1.0 - (ndc_y * 0.5 + 0.5)) * rect.height();
            let dot = (n[0] * light_dir[0] + n[1] * light_dir[1] + n[2] * light_dir[2]).max(0.0);
            let shade = 0.3 + 0.7 * dot;
            *slot = egui::epaint::Vertex {
                pos: egui::pos2(sx, sy),
                uv: egui::Pos2::ZERO,
                color: egui::Color32::from_rgba_unmultiplied(
                    (shade * 180.0) as u8,
                    (shade * 200.0) as u8,
                    (shade * 255.0) as u8,
                    255,
                ),
            };
        }
        if !valid {
            continue;
        }
        triangles.push((verts, depth / 3.0));
    }

    triangles.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut vertices: Vec<egui::epaint::Vertex> = Vec::with_capacity(triangles.len() * 3);
    let mut tri_idx: Vec<u32> = Vec::with_capacity(triangles.len() * 3);
    for (verts, _) in &triangles {
        let base = vertices.len() as u32;
        vertices.extend_from_slice(verts);
        tri_idx.extend_from_slice(&[base, base + 1, base + 2]);
    }

    for start in (0..vertices.len()).step_by(6000) {
        let end = (start + 6000).min(vertices.len());
        let base = start as u32;
        let idx: Vec<u32> = tri_idx[start..end].iter().map(|i| i - base).collect();
        let mesh_shape = egui::Mesh {
            vertices: vertices[start..end].to_vec(),
            indices: idx,
            ..Default::default()
        };
        painter.add(egui::Shape::Mesh(mesh_shape));
    }

    if mesh.vertices.len() < 3 || mesh.indices.len() < 3 {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No geometry to display",
            egui::FontId::proportional(16.0),
            egui::Color32::GRAY,
        );
    }

    let (_fwd, cam_right, cam_true_up) = camera_basis(eye, camera.target);

    let origin = egui::pos2(rect.left() + 40.0, rect.bottom() - 40.0);
    let gizmo_len = 26.0;
    let axes: [([f32; 3], egui::Color32, &str); 3] = [
        ([1.0, 0.0, 0.0], egui::Color32::from_rgb(230, 70, 70), "X"),
        ([0.0, 1.0, 0.0], egui::Color32::from_rgb(70, 220, 70), "Y"),
        ([0.0, 0.0, 1.0], egui::Color32::from_rgb(80, 130, 250), "Z"),
    ];
    for (dir, color, name) in axes {
        let sx = dir[0] * cam_right[0] + dir[1] * cam_right[1] + dir[2] * cam_right[2];
        let sy = -(dir[0] * cam_true_up[0] + dir[1] * cam_true_up[1] + dir[2] * cam_true_up[2]);
        let end = origin + egui::vec2(sx * gizmo_len, sy * gizmo_len);
        painter.line_segment([origin, end], egui::Stroke::new(2.0_f32, color));
        painter.text(
            end + egui::vec2(sx * 7.0, sy * 7.0),
            egui::Align2::CENTER_CENTER,
            name,
            egui::FontId::proportional(12.0),
            color,
        );
    }
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 { [v[0] / len, v[1] / len, v[2] / len] } else { [0.0, 0.0, 1.0] }
}

fn camera_eye(camera: &Camera) -> [f32; 3] {
    [
        camera.target[0] + camera.distance * camera.yaw.cos() * camera.pitch.cos(),
        camera.target[1] + camera.distance * camera.yaw.sin() * camera.pitch.cos(),
        camera.target[2] + camera.distance * camera.pitch.sin(),
    ]
}

/// Camera basis with Z up, matching OpenSCAD's world orientation.
/// Returns `(forward, right, up)`.
fn camera_basis(eye: [f32; 3], target: [f32; 3]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let fwd = normalize([target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]]);
    let world_up = [0.0f32, 0.0, 1.0];
    let right = normalize([
        fwd[1] * world_up[2] - fwd[2] * world_up[1],
        fwd[2] * world_up[0] - fwd[0] * world_up[2],
        fwd[0] * world_up[1] - fwd[1] * world_up[0],
    ]);
    let up = [
        right[1] * fwd[2] - right[2] * fwd[1],
        right[2] * fwd[0] - right[0] * fwd[2],
        right[0] * fwd[1] - right[1] * fwd[0],
    ];
    (fwd, right, up)
}

/// World-space point under `pos`, intersected with the Z=0 ground plane.
fn world_at_screen_point(
    pos: egui::Pos2,
    rect: egui::Rect,
    camera: &Camera,
    orthographic: bool,
) -> Option<[f32; 3]> {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return None;
    }
    let ndc_x = ((pos.x - rect.left()) / rect.width()) * 2.0 - 1.0;
    let ndc_y = 1.0 - ((pos.y - rect.top()) / rect.height()) * 2.0;
    let eye = camera_eye(camera);
    let (fwd, right, up) = camera_basis(eye, camera.target);
    let tan_half = (60.0f32.to_radians() / 2.0).tan();
    let aspect = rect.width() / rect.height();

    let (origin, dir) = if orthographic {
        let half = camera.distance * tan_half;
        let origin = [
            eye[0] + right[0] * ndc_x * half * aspect + up[0] * ndc_y * half,
            eye[1] + right[1] * ndc_x * half * aspect + up[1] * ndc_y * half,
            eye[2] + right[2] * ndc_x * half * aspect + up[2] * ndc_y * half,
        ];
        (origin, fwd)
    } else {
        let dir = normalize([
            fwd[0] + right[0] * ndc_x * tan_half * aspect + up[0] * ndc_y * tan_half,
            fwd[1] + right[1] * ndc_x * tan_half * aspect + up[1] * ndc_y * tan_half,
            fwd[2] + right[2] * ndc_x * tan_half * aspect + up[2] * ndc_y * tan_half,
        ]);
        (eye, dir)
    };

    if dir[2].abs() < 1e-6 {
        return None;
    }
    let t = -origin[2] / dir[2];
    if t < 0.0 {
        return None;
    }
    Some([
        origin[0] + dir[0] * t,
        origin[1] + dir[1] * t,
        origin[2] + dir[2] * t,
    ])
}

/// 0-based char offset of the start of a 1-based line number.
fn char_offset_for_line(source: &str, line: usize) -> usize {
    if line <= 1 {
        return 0;
    }
    let mut offset = 0;
    for _ in 1..line {
        match source[offset..].find('\n') {
            Some(i) => offset += i + 1,
            None => return source.len(),
        }
    }
    offset
}

/// Extract the line number from a pest error message (`--> LINE:COL`).
fn extract_error_line(message: &str) -> Option<usize> {
    let idx = message.find("-->")?;
    let rest = message[idx + 3..].trim_start();
    let colon = rest.find(':')?;
    rest[..colon].trim().parse().ok()
}

fn build_matrices(
    eye: [f32; 3],
    target: [f32; 3],
    aspect: f32,
    distance: f32,
    orthographic: bool,
) -> ([[f32; 4]; 4], [[f32; 4]; 4]) {
    let (fwd, right, true_up) = camera_basis(eye, target);

    let view = [
        [right[0], right[1], right[2], -(right[0] * eye[0] + right[1] * eye[1] + right[2] * eye[2])],
        [true_up[0], true_up[1], true_up[2], -(true_up[0] * eye[0] + true_up[1] * eye[1] + true_up[2] * eye[2])],
        [-fwd[0], -fwd[1], -fwd[2], fwd[0] * eye[0] + fwd[1] * eye[1] + fwd[2] * eye[2]],
        [0.0, 0.0, 0.0, 1.0],
    ];

    let fov = 60.0f32.to_radians();
    let near = 0.1f32;
    let far = (distance * 4.0).max(100.0);
    let nf = 1.0 / (near - far);

    let proj = if orthographic {
        let half = distance * (fov / 2.0).tan();
        [
            [1.0 / (half * aspect), 0.0, 0.0, 0.0],
            [0.0, 1.0 / half, 0.0, 0.0],
            [0.0, 0.0, -2.0 / (far - near), -(far + near) / (far - near)],
            [0.0, 0.0, 0.0, 1.0],
        ]
    } else {
        let f = 1.0 / (fov / 2.0).tan();
        [
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, (far + near) * nf, 2.0 * far * near * nf],
            [0.0, 0.0, -1.0, 0.0],
        ]
    };

    (proj, view)
}

fn mat4_mul_vec4(m: [[f32; 4]; 4], v: [f32; 4]) -> [f32; 4] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2] + m[0][3] * v[3],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2] + m[1][3] * v[3],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2] + m[2][3] * v[3],
        m[3][0] * v[0] + m[3][1] * v[1] + m[3][2] * v[2] + m[3][3] * v[3],
    ]
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