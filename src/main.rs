use eframe::egui;
use eframe::Frame;
use openscad_rust::cancel;
use openscad_rust::{Environment, Executor, Parser};
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum RenderMode {
    Shaded,
    Wireframe,
}

impl RenderMode {
    fn toggle(self) -> Self {
        match self {
            RenderMode::Shaded => RenderMode::Wireframe,
            RenderMode::Wireframe => RenderMode::Shaded,
        }
    }
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
        Self {
            yaw: -0.785,
            pitch: 0.5,
            distance: 40.0,
            target: [0.0, 0.0, 0.0],
        }
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
    render_mode: RenderMode,
    crease_angle: f32,
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
            render_mode: RenderMode::Shaded,
            crease_angle: DEFAULT_WIREFRAME_CREASE_ANGLE_DEG,
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
        cancel::reset_evaluation();

        std::thread::spawn(move || {
            cancel::begin_cancellable_eval();
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
                    if cancel::evaluation_cancelled() {
                        let _ = tx.send(EvalResult::Log(cancel::CANCELLED_MESSAGE.to_string()));
                        let _ = tx.send(EvalResult::Timing(started.elapsed().as_millis()));
                        let _ = tx.send(EvalResult::Log("DONE".to_string()));
                        return;
                    }
                    let _ = tx.send(EvalResult::Log("Evaluation complete.".to_string()));
                    let mesh = exec.build_mesh();
                    if let Some(mesh) = mesh {
                        let stl = mesh.to_stl();
                        let vertices: Vec<[f32; 3]> = mesh
                            .vertices
                            .iter()
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
                        let _ = tx.send(EvalResult::Log(format!(
                            "Mesh: {vertex_count} vertices, {face_count} faces"
                        )));
                        let _ = tx.send(EvalResult::Log(format!(
                            "Bounds: [{:.2}, {:.2}, {:.2}] to [{:.2}, {:.2}, {:.2}]",
                            bounds_min[0] as f64,
                            bounds_min[1] as f64,
                            bounds_min[2] as f64,
                            bounds_max[0] as f64,
                            bounds_max[1] as f64,
                            bounds_max[2] as f64
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
                Err(e) if e == cancel::CANCELLED_MESSAGE => {
                    let _ = tx.send(EvalResult::Log(cancel::CANCELLED_MESSAGE.to_string()));
                    let _ = tx.send(EvalResult::Timing(started.elapsed().as_millis()));
                    let _ = tx.send(EvalResult::Log("DONE".to_string()));
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
            let extent = (max[0] - min[0]).max(max[1] - min[1]).max(max[2] - min[2]);
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
                        self.console
                            .push(format!("Exported STL: {}", path.display()));
                    }
                    Err(e) => {
                        self.console.push(format!("Error exporting STL: {e}"));
                    }
                }
            }
        } else {
            self.console
                .push("No model to export. Run evaluation first.".to_string());
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
        let mut cancel_requested = false;
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
            if i.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                cancel_requested = true;
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
        if cancel_requested && self.evaluating {
            cancel::cancel_evaluation();
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
                if self.evaluating
                    && ui
                        .button(egui::RichText::new("Cancel").color(egui::Color32::LIGHT_RED))
                        .on_hover_text("Esc — stop the running evaluation")
                        .clicked()
                {
                    cancel::cancel_evaluation();
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

        egui::SidePanel::left("file_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
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

        egui::TopBottomPanel::bottom("status")
            .exact_height(22.0)
            .show(ctx, |ui| {
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
                        Some(p) => {
                            ui.label(format!("X: {:.1}  Y: {:.1}  Z: {:.1}", p[0], p[1], p[2]))
                        }
                        None => ui.label("X: –  Y: –  Z: –"),
                    };
                    ui.separator();
                    ui.label(format!("Zoom: {:.1}", self.camera.distance));
                    ui.separator();
                    ui.label(if self.orthographic { "Ortho" } else { "Persp" });
                    ui.separator();
                    ui.label(if self.auto_preview {
                        "Preview: auto"
                    } else {
                        "Preview: off"
                    });
                });
            });

        egui::TopBottomPanel::bottom("console")
            .default_height(150.0)
            .show(ctx, |ui| {
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

            let editor_rect =
                egui::Rect::from_min_size(avail.min, egui::vec2(editor_width, avail.height()));
            let viewport_rect = egui::Rect::from_min_size(
                egui::pos2(avail.min.x + editor_width, avail.min.y),
                egui::vec2(avail.width() - editor_width, avail.height()),
            );

            ui.allocate_ui_at_rect(editor_rect, |ui| {
                ui.heading("Editor");
                egui::ScrollArea::new([false, true]).show(ui, |ui| {
                    let font_id = ui
                        .style()
                        .text_styles
                        .get(&egui::TextStyle::Monospace)
                        .cloned()
                        .unwrap_or_else(|| egui::FontId::monospace(13.0));
                    let row_h = ui.text_style_height(&egui::TextStyle::Monospace);
                    let default_color = ui.visuals().widgets.inactive.text_color();
                    let colors = scad_colors(ui.visuals());
                    let visuals = ui.visuals().clone();
                    let mut layouter = {
                        let font_id = font_id.clone();
                        move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                            let job = scad_layout_job(
                                text,
                                font_id.clone(),
                                default_color,
                                wrap_width,
                                colors,
                            );
                            ui.fonts(|f| f.layout_job(job))
                        }
                    };
                    ui.horizontal(|ui| {
                        let line_count = self.source.lines().count().max(1);
                        let digits =
                            ((line_count as f32).log10().floor() as i32 + 1).max(2) as usize;
                        let gutter_w = digits as f32 * font_id.size * 0.62 + 22.0;
                        let spacing_x = ui.spacing().item_spacing.x;
                        let wrap_width = (ui.available_width() - gutter_w - spacing_x).max(80.0);
                        let galley = {
                            let job = scad_layout_job(
                                &self.source,
                                font_id.clone(),
                                default_color,
                                wrap_width,
                                colors,
                            );
                            ui.fonts(|f| f.layout_job(job))
                        };
                        let height = galley.size().y.max(25.0 * row_h);
                        let (gutter, gutter_painter) =
                            ui.allocate_painter(egui::vec2(gutter_w, height), egui::Sense::click());
                        let response = ui.add(
                            egui::TextEdit::multiline(&mut self.source)
                                .code_editor()
                                .desired_rows(25)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace)
                                .id_source("scad_code_editor")
                                .margin(egui::vec2(0.0, 0.0))
                                .layouter(&mut layouter),
                        );
                        if response.changed() {
                            self.dirty = true;
                            self.last_edit = Some(std::time::Instant::now());
                            self.error_line = None;
                        }
                        let cursor_line =
                            egui::text_edit::TextEditState::load(ui.ctx(), response.id)
                                .and_then(|s| s.ccursor_range())
                                .map(|r| {
                                    let idx = r.primary.index.min(self.source.len());
                                    self.source[..idx].matches('\n').count() + 1
                                });
                        paint_line_numbers(
                            &gutter_painter,
                            gutter.rect,
                            &galley,
                            cursor_line,
                            self.error_line,
                            &visuals,
                            font_id.clone(),
                        );
                        if gutter.clicked() {
                            if let Some(pos) = gutter.interact_pointer_pos() {
                                let gy = pos.y - gutter.rect.top();
                                let mut line = 1usize;
                                for row in &galley.rows {
                                    if gy >= row.rect.min.y && gy < row.rect.max.y {
                                        break;
                                    }
                                    if row.ends_with_newline {
                                        line += 1;
                                    }
                                }
                                let offset = char_offset_for_line(&self.source, line);
                                response.request_focus();
                                if let Some(mut state) =
                                    egui::text_edit::TextEditState::load(ui.ctx(), response.id)
                                {
                                    state.set_ccursor_range(Some(
                                        egui::text_edit::CCursorRange::two(
                                            egui::text::CCursor::new(offset),
                                            egui::text::CCursor::new(offset),
                                        ),
                                    ));
                                    state.store(ui.ctx(), response.id);
                                }
                            }
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
                                state.set_ccursor_range(Some(egui::text_edit::CCursorRange::two(
                                    egui::text::CCursor::new(start),
                                    egui::text::CCursor::new(end),
                                )));
                                state.store(ui.ctx(), response.id);
                            }
                        }
                    });
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
                    if ui
                        .selectable_label(self.render_mode == RenderMode::Wireframe, "Edges")
                        .on_hover_text(
                            "Draw the model as wireframe (edges only) instead of shaded surfaces",
                        )
                        .clicked()
                    {
                        self.render_mode = self.render_mode.toggle();
                    }
                    if self.render_mode == RenderMode::Wireframe {
                        ui.add(
                            egui::Slider::new(&mut self.crease_angle, 0.0..=60.0)
                                .suffix("°")
                                .text("Edge angle"),
                        )
                        .on_hover_text(
                            "Draw edges sharper than this dihedral angle. 0° draws every edge \
                             that is not perfectly flat; high values hide facets of curved surfaces.",
                        );
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
                        self.render_mode,
                        self.crease_angle,
                        response.rect,
                    );
                } else {
                    painter.rect_filled(response.rect, 0.0, egui::Color32::from_rgb(40, 44, 52));
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
                            self.camera.pitch =
                                (self.camera.pitch + delta.y * 0.01).clamp(-1.5, 1.5);
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

#[allow(clippy::too_many_arguments)]
fn render_mesh_2d(
    painter: &egui::Painter,
    mesh: &MeshData,
    camera: Camera,
    show_grid: bool,
    orthographic: bool,
    render_mode: RenderMode,
    crease_angle: f32,
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

    let indices: Vec<[u32; 3]> = mesh
        .indices
        .as_chunks::<3>()
        .0
        .iter()
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
    draw_line(
        [-extent, 0.0, 0.0],
        [extent, 0.0, 0.0],
        egui::Color32::from_rgb(220, 80, 80),
        1.5,
    );
    draw_line(
        [0.0, -extent, 0.0],
        [0.0, extent, 0.0],
        egui::Color32::from_rgb(80, 200, 80),
        1.5,
    );
    draw_line(
        [0.0, 0.0, 0.0],
        [0.0, 0.0, extent],
        egui::Color32::from_rgb(90, 140, 240),
        1.5,
    );

    if show_grid {
        let tick = step * 0.25;
        let font = egui::FontId::monospace(11.0);
        let tick_color = egui::Color32::from_gray(140);
        let label_color = egui::Color32::from_gray(150);
        let format_tick = |c: f32| -> String {
            if step >= 1.0 {
                format!("{}", c as i64)
            } else {
                format!("{c:.2}")
            }
        };
        let mut label_k = -extent;
        while label_k <= extent + 0.001 {
            let c = label_k;
            if c.abs() > 0.001 {
                draw_line([c, -tick, 0.0], [c, tick, 0.0], tick_color, 1.0);
                draw_line([-tick, c, 0.0], [tick, c, 0.0], tick_color, 1.0);
            }
            if let Some(p) = project([c, 0.0, 0.0]) {
                if rect.contains(p) && c.abs() > 0.001 {
                    painter.text(
                        p + egui::vec2(3.0, -14.0),
                        egui::Align2::LEFT_TOP,
                        format_tick(c),
                        font.clone(),
                        label_color,
                    );
                }
            }
            if let Some(p) = project([0.0, c, 0.0]) {
                if rect.contains(p) && c.abs() > 0.001 {
                    painter.text(
                        p + egui::vec2(3.0, 3.0),
                        egui::Align2::LEFT_TOP,
                        format_tick(c),
                        font.clone(),
                        label_color,
                    );
                }
            }
            label_k += step;
        }

        let mut z_k = step;
        while z_k <= extent + 0.001 {
            let c = z_k;
            draw_line([-tick, 0.0, c], [tick, 0.0, c], tick_color, 1.0);
            draw_line([0.0, -tick, c], [0.0, tick, c], tick_color, 1.0);
            if let Some(p) = project([0.0, 0.0, c]) {
                if rect.contains(p) {
                    painter.text(
                        p + egui::vec2(4.0, -7.0),
                        egui::Align2::LEFT_TOP,
                        format_tick(c),
                        font.clone(),
                        label_color,
                    );
                }
            }
            z_k += step;
        }
    }

    match render_mode {
        RenderMode::Wireframe => {
            draw_wireframe(painter, mesh, &indices, crease_angle, &project, rect)
        }
        RenderMode::Shaded => draw_shaded(
            painter,
            mesh,
            &indices,
            &normals,
            (view, proj),
            light_dir,
            rect,
        ),
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

fn draw_shaded(
    painter: &egui::Painter,
    mesh: &MeshData,
    indices: &[[u32; 3]],
    normals: &[[f32; 3]],
    (view, proj): ([[f32; 4]; 4], [[f32; 4]; 4]),
    light_dir: [f32; 3],
    rect: egui::Rect,
) {
    let mut triangles: Vec<([egui::epaint::Vertex; 3], f32)> = Vec::with_capacity(indices.len());

    for tri in indices {
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
            depth +=
                (view_pos[0] * view_pos[0] + view_pos[1] * view_pos[1] + view_pos[2] * view_pos[2])
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
}

/// Edges with a dihedral angle above this are drawn in wireframe mode.
/// Below it the surface counts as smooth, so triangulation diagonals and
/// facets of curved surfaces stay hidden. `0.0` draws every edge that is
/// not perfectly flat.
const DEFAULT_WIREFRAME_CREASE_ANGLE_DEG: f32 = 8.0;

#[derive(Clone, Copy)]
struct EdgeInfo {
    first: [f32; 3],
    second: [f32; 3],
    count: u8,
    crease: bool,
}

fn face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    normalize([
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ])
}

/// Feature edges of the mesh: sharp creases plus boundaries of open meshes.
/// Flat triangulation edges (dihedral angle ~0) are always skipped.
fn feature_edges(mesh: &MeshData, indices: &[[u32; 3]], crease_angle_deg: f32) -> Vec<(u32, u32)> {
    let vertex_count = mesh.vertices.len();
    let cos_limit = crease_angle_deg.clamp(0.0, 180.0).to_radians().cos();
    let mut edges: std::collections::HashMap<(u32, u32), EdgeInfo> =
        std::collections::HashMap::new();

    for tri in indices {
        if tri.iter().any(|&i| i as usize >= vertex_count) {
            continue;
        }
        let v = [
            mesh.vertices[tri[0] as usize],
            mesh.vertices[tri[1] as usize],
            mesh.vertices[tri[2] as usize],
        ];
        let normal = face_normal(v[0], v[1], v[2]);
        for k in 0..3 {
            let a = tri[k];
            let b = tri[(k + 1) % 3];
            let edge = if a < b { (a, b) } else { (b, a) };
            match edges.get_mut(&edge) {
                Some(info) => {
                    let other = if info.count == 1 {
                        info.first
                    } else {
                        info.second
                    };
                    let dot = (normal[0] * other[0] + normal[1] * other[1] + normal[2] * other[2])
                        .clamp(-1.0, 1.0);
                    if dot < cos_limit {
                        info.crease = true;
                    }
                    if info.count < 2 {
                        info.second = normal;
                    }
                    info.count = info.count.saturating_add(1);
                }
                None => {
                    edges.insert(
                        edge,
                        EdgeInfo {
                            first: normal,
                            second: normal,
                            count: 1,
                            crease: false,
                        },
                    );
                }
            }
        }
    }

    edges
        .into_iter()
        .filter(|(_, info)| info.crease || info.count < 2)
        .map(|(edge, _)| edge)
        .collect()
}

/// Draws only feature edges: sharp creases and boundaries of the mesh.
/// Flat triangulation edges are skipped, so a cuboid shows 12 edges
/// instead of every diagonal its side faces were split into.
fn draw_wireframe<F>(
    painter: &egui::Painter,
    mesh: &MeshData,
    indices: &[[u32; 3]],
    crease_angle_deg: f32,
    project: &F,
    rect: egui::Rect,
) where
    F: Fn([f32; 3]) -> Option<egui::Pos2>,
{
    let vertex_count = mesh.vertices.len();

    let mut segments: Vec<[egui::Pos2; 2]> = Vec::new();
    for (a, b) in feature_edges(mesh, indices, crease_angle_deg) {
        let (a, b) = (a as usize, b as usize);
        if a >= vertex_count || b >= vertex_count {
            continue;
        }
        let (pa, pb) = match (project(mesh.vertices[a]), project(mesh.vertices[b])) {
            (Some(pa), Some(pb)) => (pa, pb),
            _ => continue,
        };
        if (pa - pb).length_sq() <= 0.25 {
            continue;
        }
        if !rect.intersects(egui::Rect::from_min_max(pa.min(pb), pa.max(pb))) {
            continue;
        }
        segments.push([pa, pb]);
    }

    let color = egui::Color32::from_rgb(205, 215, 230);
    let half_width = 0.5f32;
    const SEGMENTS_PER_CHUNK: usize = 1500;

    for chunk in segments.chunks(SEGMENTS_PER_CHUNK) {
        let mut vertices: Vec<egui::epaint::Vertex> = Vec::with_capacity(chunk.len() * 4);
        let mut idx: Vec<u32> = Vec::with_capacity(chunk.len() * 6);
        for seg in chunk {
            let (pa, pb) = (seg[0], seg[1]);
            let dir = pb - pa;
            let len = dir.length();
            if len <= 0.0 {
                continue;
            }
            let normal = egui::vec2(-dir.y, dir.x) * (half_width / len);
            let base = vertices.len() as u32;
            for p in [pa + normal, pb + normal, pb - normal, pa - normal] {
                vertices.push(egui::epaint::Vertex {
                    pos: p,
                    uv: egui::Pos2::ZERO,
                    color,
                });
            }
            idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        if idx.is_empty() {
            continue;
        }
        painter.add(egui::Shape::Mesh(egui::Mesh {
            vertices,
            indices: idx,
            ..Default::default()
        }));
    }
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, 1.0]
    }
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

const KEYWORDS: &[&str] = &[
    "module",
    "function",
    "if",
    "else",
    "for",
    "intersection_for",
    "let",
    "each",
    "return",
    "include",
    "use",
    "true",
    "false",
    "undef",
    "assert",
    "echo",
];

const BUILTINS: &[&str] = &[
    // modules
    "cube",
    "cylinder",
    "sphere",
    "polyhedron",
    "polygon",
    "square",
    "circle",
    "text",
    "import",
    "surface",
    "translate",
    "rotate",
    "scale",
    "mirror",
    "multmatrix",
    "color",
    "offset",
    "hull",
    "minkowski",
    "union",
    "difference",
    "intersection",
    "linear_extrude",
    "rotate_extrude",
    "projection",
    "render",
    "children",
    "resize",
    // functions
    "len",
    "concat",
    "lookup",
    "str",
    "chr",
    "ord",
    "search",
    "version",
    "version_num",
    "parent_module",
    "is_undef",
    "is_defined",
    "is_list",
    "is_num",
    "is_string",
    "is_bool",
    "is_function",
    "min",
    "max",
    "abs",
    "sign",
    "floor",
    "ceil",
    "round",
    "sqrt",
    "exp",
    "ln",
    "log",
    "sin",
    "cos",
    "tan",
    "asin",
    "acos",
    "atan",
    "atan2",
    "pow",
    "rands",
    "norm",
    "cross",
];

#[derive(Clone, Copy)]
struct ScadColors {
    comment: egui::Color32,
    string: egui::Color32,
    number: egui::Color32,
    keyword: egui::Color32,
    builtin: egui::Color32,
    special: egui::Color32,
}

fn scad_colors(visuals: &egui::Visuals) -> ScadColors {
    if visuals.dark_mode {
        ScadColors {
            comment: egui::Color32::from_gray(120),
            string: egui::Color32::from_rgb(206, 153, 107),
            number: egui::Color32::from_rgb(134, 196, 134),
            keyword: egui::Color32::from_rgb(129, 169, 232),
            builtin: egui::Color32::from_rgb(112, 196, 196),
            special: egui::Color32::from_rgb(196, 153, 232),
        }
    } else {
        ScadColors {
            comment: egui::Color32::from_gray(145),
            string: egui::Color32::from_rgb(160, 80, 40),
            number: egui::Color32::from_rgb(20, 120, 60),
            keyword: egui::Color32::from_rgb(0, 80, 190),
            builtin: egui::Color32::from_rgb(0, 120, 120),
            special: egui::Color32::from_rgb(130, 40, 170),
        }
    }
}

fn scan_number(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        if j < bytes.len() && bytes[j].is_ascii_digit() {
            i = j;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
    }
    i
}

/// Tokenize `text` into a syntax-highlighted layout job for the code editor.
fn scad_layout_job(
    text: &str,
    font_id: egui::FontId,
    default_color: egui::Color32,
    wrap_width: f32,
    colors: ScadColors,
) -> egui::text::LayoutJob {
    let bytes = text.as_bytes();
    let mut spans: Vec<(usize, usize, egui::Color32)> = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        let start = i;
        let b = bytes[i];
        let color = if b == b'/' && bytes.get(i + 1) == Some(&b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            colors.comment
        } else if b == b'/' && bytes.get(i + 1) == Some(&b'*') {
            i += 2;
            while i < bytes.len() && !(bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/')) {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            colors.comment
        } else if b == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' && bytes[i] != b'\n' {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'"' {
                i += 1;
            }
            colors.string
        } else if b == b'$' {
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            colors.special
        } else if b.is_ascii_digit() {
            i = scan_number(bytes, i);
            colors.number
        } else if b.is_ascii_alphabetic() || b == b'_' {
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &text[start..i];
            if KEYWORDS.contains(&word) {
                colors.keyword
            } else if BUILTINS.contains(&word) {
                colors.builtin
            } else {
                default_color
            }
        } else {
            i += 1;
            default_color
        };

        match spans.last_mut() {
            Some(last) if last.2 == color && last.1 == start => last.1 = i,
            _ => spans.push((start, i, color)),
        }
    }

    let mut job = egui::text::LayoutJob {
        text: text.to_string(),
        wrap: egui::text::TextWrapping {
            max_width: wrap_width,
            ..Default::default()
        },
        break_on_newline: true,
        ..Default::default()
    };
    for (start, end, color) in spans {
        job.sections.push(egui::text::LayoutSection {
            leading_space: 0.0,
            byte_range: start..end,
            format: egui::text::TextFormat::simple(font_id.clone(), color),
        });
    }
    job
}

/// Paint the line-number gutter aligned with the editor's laid-out text.
fn paint_line_numbers(
    painter: &egui::Painter,
    rect: egui::Rect,
    galley: &egui::text::Galley,
    cursor_line: Option<usize>,
    error_line: Option<usize>,
    visuals: &egui::Visuals,
    font_id: egui::FontId,
) {
    painter.rect_filled(rect, 0.0, visuals.faint_bg_color);
    let border = visuals.widgets.noninteractive.bg_stroke.color;
    painter.line_segment(
        [
            egui::pos2(rect.right() - 0.5, rect.top()),
            egui::pos2(rect.right() - 0.5, rect.bottom()),
        ],
        egui::Stroke::new(1.0_f32, border),
    );

    let total = galley.rows.iter().filter(|r| r.ends_with_newline).count() + 1;
    let digits = ((total as f32).log10().floor() as i32 + 1).max(2) as usize;
    let dim = visuals.weak_text_color();
    let accent = if visuals.dark_mode {
        egui::Color32::from_rgb(130, 180, 255)
    } else {
        egui::Color32::from_rgb(0, 90, 200)
    };
    let error_color = if visuals.dark_mode {
        egui::Color32::LIGHT_RED
    } else {
        egui::Color32::RED
    };
    let right_x = rect.right() - 9.0;

    let mut line = 1usize;
    let mut first_row_of_line = true;
    for row in &galley.rows {
        if first_row_of_line {
            first_row_of_line = false;
            let color = if error_line == Some(line) {
                error_color
            } else if cursor_line == Some(line) {
                accent
            } else {
                dim
            };
            painter.text(
                egui::pos2(right_x, rect.top() + row.rect.center().y),
                egui::Align2::RIGHT_CENTER,
                format!("{line:>width$}", width = digits),
                font_id.clone(),
                color,
            );
        }
        if row.ends_with_newline {
            line += 1;
            first_row_of_line = true;
        }
    }
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
        [
            right[0],
            right[1],
            right[2],
            -(right[0] * eye[0] + right[1] * eye[1] + right[2] * eye[2]),
        ],
        [
            true_up[0],
            true_up[1],
            true_up[2],
            -(true_up[0] * eye[0] + true_up[1] * eye[1] + true_up[2] * eye[2]),
        ],
        [
            -fwd[0],
            -fwd[1],
            -fwd[2],
            fwd[0] * eye[0] + fwd[1] * eye[1] + fwd[2] * eye[2],
        ],
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

use openscad_rust::csg::Mesh;

fn load_mesh_or_scad(path: &str) -> Result<Mesh, String> {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".scad") {
        let parsed = Parser::parse_file(path)
            .map_err(|error| format!("failed to parse '{path}': {error}"))?;
        let env = Rc::new(Environment::new());
        let mut exec = Executor::new(env);
        exec.execute_source(&parsed)
            .map_err(|error| format!("failed to evaluate '{path}': {error}"))?;
        let mesh = exec
            .build_mesh()
            .ok_or_else(|| format!("'{path}' produced no geometry"))?;
        Ok(mesh)
    } else {
        let data =
            std::fs::read(path).map_err(|error| format!("failed to read '{path}': {error}"))?;
        if lower.ends_with(".obj") {
            Mesh::from_obj(&data)
        } else {
            Mesh::from_stl(&data)
        }
    }
}

fn write_ppm(path: &str, width: usize, height: usize, rgb: &[u8]) -> Result<(), String> {
    let header = format!("P6\n{} {}\n255\n", width, height);
    let mut data = header.into_bytes();
    data.extend_from_slice(rgb);
    std::fs::write(path, data).map_err(|error| format!("failed to write PPM '{path}': {error}"))
}

fn run_compare_cli(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") || args.is_empty() {
        println!("Usage: openscad-rust compare <model_a.stl|a.scad> <model_b.stl|b.scad> [--samples <N>] [--json] [--render <output.ppm>]");
        return Ok(());
    }

    let path_a = args
        .first()
        .ok_or_else(|| usage_error("missing first model path"))?;
    let path_b = args
        .get(1)
        .ok_or_else(|| usage_error("missing second model path"))?;

    let mut samples = 20_000;
    let mut json = false;
    let mut render_path = None;

    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--samples" => {
                index += 1;
                let val = args
                    .get(index)
                    .ok_or_else(|| usage_error("missing value for --samples"))?;
                samples = val
                    .parse::<usize>()
                    .map_err(|_| "invalid number for --samples")?;
            }
            "--json" => json = true,
            "--render" => {
                index += 1;
                let path = args
                    .get(index)
                    .ok_or_else(|| usage_error("missing path after --render"))?;
                render_path = Some(path.clone());
            }
            option => return Err(usage_error(&format!("unknown option '{option}'"))),
        }
        index += 1;
    }

    let mesh_a = load_mesh_or_scad(path_a)?;
    let mesh_b = load_mesh_or_scad(path_b)?;

    let report = openscad_rust::geometry::compare_meshes(&mesh_a, &mesh_b, samples);

    if let Some(ref ppm_path) = render_path {
        if !report.views.is_empty() {
            let total_w = report.views[0].width;
            let total_h = report.views[0].height * report.views.len();
            let mut composite_rgb = Vec::with_capacity(total_w * total_h * 3);
            for v in &report.views {
                composite_rgb.extend_from_slice(&v.rgb_image);
            }
            write_ppm(ppm_path, total_w, total_h, &composite_rgb)?;
        }
    }

    if json {
        let json_str = serde_json::to_string_pretty(&report)
            .map_err(|e| format!("failed to serialize JSON: {e}"))?;
        println!("{json_str}");
    } else {
        println!("{}", report.format_text_report(path_a, path_b));
    }

    Ok(())
}

fn run_cli(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Usage:\n  openscad-rust <file.scad> [-o|--output <file.stl|file.obj>] [--binary]\n  openscad-rust compare <model_a.stl> <model_b.stl> [--samples <N>] [--json] [--render <output.ppm>]");
        return Ok(());
    }

    if args.first().map(|s| s.as_str()) == Some("compare") {
        return run_compare_cli(&args[1..]);
    }

    let input = args
        .first()
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
                let path = args
                    .get(index)
                    .ok_or_else(|| usage_error("missing path after --output"))?;
                output = Some(path);
            }
            "--binary" => binary = true,
            option => return Err(usage_error(&format!("unknown option '{option}'"))),
        }
        index += 1;
    }

    let parsed =
        Parser::parse_file(input).map_err(|error| format!("failed to parse '{input}': {error}"))?;
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
        let format = output
            .rsplit('.')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if binary && format != "stl" {
            return Err("--binary is only supported for STL output".into());
        }
        let contents = match format.as_str() {
            "stl" if binary => mesh.to_binary_stl(),
            "stl" => mesh.to_stl().into_bytes(),
            "obj" => mesh.to_obj().into_bytes(),
            _ => {
                return Err(format!(
                    "unsupported output format '.{format}'; use .stl or .obj"
                ))
            }
        };
        std::fs::write(output, contents)
            .map_err(|error| format!("failed to write '{output}': {error}"))?;
    }
    Ok(())
}

fn usage_error(message: &str) -> String {
    format!("{message}\n\nUsage:\n  openscad-rust <file.scad> [-o|--output <file.stl|file.obj>] [--binary]\n  openscad-rust compare <model_a.stl> <model_b.stl> [--samples <N>] [--json] [--render <output.ppm>]")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_mesh() -> MeshData {
        // Every quad is split into two triangles, so each of the six faces
        // contributes one diagonal that must not show up as a feature edge.
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ];
        let faces: [[u32; 4]; 6] = [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ];
        let indices: Vec<u32> = faces
            .iter()
            .flat_map(|f| [f[0], f[1], f[2], f[0], f[2], f[3]])
            .collect();
        MeshData { vertices, indices }
    }

    fn prism(sides: u32) -> MeshData {
        let mut vertices = Vec::new();
        for &z in &[0.0f32, 1.0] {
            for i in 0..sides {
                let a = i as f32 / sides as f32 * std::f32::consts::TAU;
                vertices.push([a.cos(), a.sin(), z]);
            }
        }
        let mut indices = Vec::new();
        for i in 0..sides {
            let a = i;
            let b = (i + 1) % sides;
            let c = b + sides;
            let d = a + sides;
            indices.extend_from_slice(&[a, b, c, a, c, d]);
        }
        for i in 1..sides - 1 {
            indices.extend_from_slice(&[0, i + 1, i]);
            indices.extend_from_slice(&[sides, sides + i, sides + i + 1]);
        }
        MeshData { vertices, indices }
    }

    fn tri_list(mesh: &MeshData) -> Vec<[u32; 3]> {
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|c| [c[0], c[1], c[2]])
            .collect()
    }

    #[test]
    fn cube_shows_only_12_edges() {
        let mesh = cube_mesh();
        let all = tri_list(&mesh);
        assert_eq!(
            all.len() * 3,
            36,
            "cube has 18 unique mesh edges, 6 of them diagonals"
        );
        let edges = feature_edges(&mesh, &all, DEFAULT_WIREFRAME_CREASE_ANGLE_DEG);
        assert_eq!(edges.len(), 12, "only the 12 real cube edges are features");
    }

    #[test]
    fn smooth_cylinder_facets_are_not_features() {
        // 24-gon prism: side facets meet at 15 degrees, i.e. above the
        // default threshold, so the vertical seams are drawn, and the
        // top/bottom rims meet at 90 degrees.
        let mesh = prism(24);
        let all = tri_list(&mesh);
        let edges = feature_edges(&mesh, &all, DEFAULT_WIREFRAME_CREASE_ANGLE_DEG);
        assert_eq!(edges.len(), 24 * 3, "24 vertical seams plus 2 rims");

        let edges = feature_edges(&mesh, &all, 20.0);
        assert_eq!(
            edges.len(),
            48,
            "above the 15 degree facet angle only the two rims are sharp"
        );
    }

    #[test]
    fn low_poly_sphere_is_still_visible() {
        // Default `sphere(r=10)` is tessellated into facets meeting at
        // roughly 11 degrees, so it must not disappear entirely.
        let mesh = prism(32);
        let all = tri_list(&mesh);
        let edges = feature_edges(&mesh, &all, DEFAULT_WIREFRAME_CREASE_ANGLE_DEG);
        assert!(
            edges.len() > 48,
            "a 32-gon prism at the default angle keeps its silhouette"
        );
    }

    #[test]
    fn flat_diagonals_are_never_drawn() {
        // Even at 0 degrees a perfectly flat triangulation edge is skipped.
        let mesh = cube_mesh();
        let all = tri_list(&mesh);
        let edges = feature_edges(&mesh, &all, 0.0);
        assert_eq!(edges.len(), 12);
    }
}
