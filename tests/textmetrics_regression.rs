//! Regression tests for the `textmetrics()` builtin (upstream OpenSCAD
//! API: object with `position`, `size`, `ascent`, `descent`, `offset`,
//! `advance`). The reference C++ OpenSCAD 2021.01 binary does not ship
//! `textmetrics` (it landed later as an experimental feature), so the
//! geometric proof here is internal consistency: a square driven by
//! `textmetrics()` must cover exactly the bounding box that the `text()`
//! module emits for the same arguments, and the reported numbers must
//! match the documented placeholder layout (0.6*size glyph, 0.1*size gap).

use openscad_rust::interpreter::{Environment, Executor, Value};
use openscad_rust::parser::Parser;
use std::rc::Rc;

fn run(source: &str) -> Result<openscad_rust::csg::Mesh, String> {
    let parsed = Parser::parse(source).map_err(|e| format!("Parse error: {:?}", e))?;
    let env = Rc::new(Environment::new());
    let mut exec = Executor::new(env);
    exec.execute_source(&parsed)?;
    exec.build_mesh()
        .ok_or_else(|| "script produced no geometry".to_string())
}

fn eval(source: &str) -> Result<Value, String> {
    let parsed = Parser::parse(source).map_err(|e| format!("Parse error: {:?}", e))?;
    let env = Rc::new(Environment::new());
    let mut exec = Executor::new(env.clone());
    exec.execute_source(&parsed)?;
    env.get("result")
        .ok_or_else(|| "result variable not set".to_string())
}

fn bounds(mesh: &openscad_rust::csg::Mesh) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for v in &mesh.vertices {
        min_x = min_x.min(v[0]);
        min_y = min_y.min(v[1]);
        max_x = max_x.max(v[0]);
        max_y = max_y.max(v[1]);
    }
    (min_x, min_y, max_x, max_y)
}

fn assert_bounds_eq(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) {
    assert!((a.0 - b.0).abs() < 1e-9, "min_x mismatch: {a:?} vs {b:?}");
    assert!((a.1 - b.1).abs() < 1e-9, "min_y mismatch: {a:?} vs {b:?}");
    assert!((a.2 - b.2).abs() < 1e-9, "max_x mismatch: {a:?} vs {b:?}");
    assert!((a.3 - b.3).abs() < 1e-9, "max_y mismatch: {a:?} vs {b:?}");
}

#[test]
fn square_driven_by_textmetrics_covers_text_bbox() {
    let driven = run("tm = textmetrics(\"Hi\", size = 10); \
         translate(tm.position) square(tm.size);")
    .expect("textmetrics-driven square");
    let direct = run("text(\"Hi\", size = 10);").expect("text() geometry");
    assert_bounds_eq(bounds(&driven), bounds(&direct));
}

#[test]
fn square_driven_by_textmetrics_covers_aligned_text_bbox() {
    let driven = run(
        "tm = textmetrics(\"AB\", size = 10, halign = \"center\", valign = \"top\"); \
         translate(tm.position) square(tm.size);",
    )
    .expect("aligned textmetrics-driven square");
    let direct = run("text(\"AB\", size = 10, halign = \"center\", valign = \"top\");")
        .expect("aligned text() geometry");
    assert_bounds_eq(bounds(&driven), bounds(&direct));
}

#[test]
fn textmetrics_values_follow_documented_placeholder_layout() {
    let value = eval("result = textmetrics(\"Hello\", size = 10);").expect("eval");
    let Value::Object(map) = &value else {
        panic!("textmetrics must return an object, got {value:?}");
    };
    let size = map.get("size").expect("size key");
    let advance = map.get("advance").expect("advance key");
    assert_eq!(
        size,
        &Value::Vector(vec![Value::Number(34.0), Value::Number(10.0)])
    );
    assert_eq!(
        advance,
        &Value::Vector(vec![Value::Number(34.0), Value::Number(0.0)])
    );
    assert_eq!(map.get("ascent"), Some(&Value::Number(10.0)));
    assert_eq!(map.get("descent"), Some(&Value::Number(0.0)));
    assert_eq!(
        map.get("position"),
        Some(&Value::Vector(vec![Value::Number(0.0), Value::Number(0.0)]))
    );
    assert_eq!(
        map.get("offset"),
        Some(&Value::Vector(vec![Value::Number(0.0), Value::Number(0.0)]))
    );
}

#[test]
fn textmetrics_spacing_scales_glyph_gap() {
    let value = eval("result = textmetrics(\"AB\", size = 10, spacing = 3);").expect("eval");
    let Value::Object(map) = &value else {
        panic!("textmetrics must return an object");
    };
    // 2 glyphs * 6 + 1 gap * (0.1 * 10 * 3) = 12 + 3 = 15.
    assert_eq!(
        map.get("size"),
        Some(&Value::Vector(vec![
            Value::Number(15.0),
            Value::Number(10.0)
        ]))
    );
}

#[test]
fn textmetrics_dot_access_feeds_geometry() {
    let mesh = run("tm = textmetrics(\"Hello\", size = 10); \
         translate([0, 0]) square([tm.size.x, tm.size.y]);")
    .expect("geometry from dot access");
    let (_, _, max_x, max_y) = bounds(&mesh);
    assert!((max_x - 34.0).abs() < 1e-9, "max_x = {max_x}");
    assert!((max_y - 10.0).abs() < 1e-9, "max_y = {max_y}");
}
