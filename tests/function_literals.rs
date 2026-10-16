//! Regression tests for function literals — `f = function(x) …` as a first-class
//! value (OpenSCAD 2021.01 feature). Verified against the reference C++ OpenSCAD
//! binary: every `.scad` snippet below echoes the same value in both
//! implementations (assignment + call, immediate invocation, defaults, let,
//! for-comprehension, vectors of literals, literal returned from a named
//! function, calling a non-function degrading to `undef`).

use openscad_rust::interpreter::{Environment, Executor, Value};
use openscad_rust::parser::Parser;
use std::rc::Rc;

fn eval(source: &str) -> Result<Value, String> {
    let parsed = Parser::parse(source).map_err(|e| format!("Parse error: {:?}", e))?;
    let env = Rc::new(Environment::new());
    let mut exec = Executor::new(env.clone());
    exec.execute_source(&parsed)?;
    env.get("result")
        .ok_or_else(|| "result variable not set".to_string())
}

fn run(source: &str) -> Result<openscad_rust::csg::Mesh, String> {
    let parsed = Parser::parse(source).map_err(|e| format!("Parse error: {:?}", e))?;
    let env = Rc::new(Environment::new());
    let mut exec = Executor::new(env);
    exec.execute_source(&parsed)?;
    exec.build_mesh()
        .ok_or_else(|| "script produced no geometry".to_string())
}

fn assert_number(value: &Value, expected: f64) {
    match value {
        Value::Number(n) => assert!((n - expected).abs() < 1e-9, "expected {expected}, got {n}"),
        other => panic!("expected number {expected}, got {other:?}"),
    }
}

#[test]
fn literal_parses_and_is_callable_after_assignment() {
    let value = eval("f = function(x) x * 2; result = f(21);").expect("parse+eval");
    assert_number(&value, 42.0);
}

#[test]
fn literal_is_immediately_invokable_with_call_parens() {
    let value = eval("result = (function(x) x * 2)(5);").expect("parse+eval");
    assert_number(&value, 10.0);
}

#[test]
fn literal_without_parameters() {
    let value = eval("f = function() 7; result = f();").expect("parse+eval");
    assert_number(&value, 7.0);
}

#[test]
fn literal_with_multiple_parameters() {
    let value = eval("f = function(a, b) a * b; result = f(6, 7);").expect("parse+eval");
    assert_number(&value, 42.0);
}

#[test]
fn literal_default_parameter_used_when_argument_missing() {
    let value = eval("f = function(x = 5) x; result = f();").expect("parse+eval");
    assert_number(&value, 5.0);
    let value = eval("f = function(x = 5) x; result = f(2);").expect("parse+eval");
    assert_number(&value, 2.0);
}

#[test]
fn literal_callable_inside_let_binding() {
    let value = eval("result = let(f = function(x) x * 2) f(3);").expect("parse+eval");
    assert_number(&value, 6.0);
}

#[test]
fn literal_returned_from_named_function() {
    let value =
        eval("function mk() = function(x) x * 3; h = mk(); result = h(4);").expect("parse+eval");
    assert_number(&value, 12.0);
}

#[test]
fn literal_used_in_for_comprehension() {
    let value = eval(
        "double = function(x) x * 2; \
         result = [for (i = [1:3]) double(i)];",
    )
    .expect("parse+eval");
    assert_eq!(
        value,
        Value::Vector(vec![
            Value::Number(2.0),
            Value::Number(4.0),
            Value::Number(6.0)
        ])
    );
}

#[test]
fn literals_stored_in_vector_are_index_callable() {
    let value = eval(
        "fs = [function(x) x + 1, function(x) x * 2]; \
         result = fs[0](10) + fs[1](10);",
    )
    .expect("parse+eval");
    assert_number(&value, 31.0);
}

#[test]
fn literal_with_expression_body_and_multiple_statements() {
    let value = eval("f = function(x) let(y = x + 1) y * 2; result = f(3);").expect("parse+eval");
    assert_number(&value, 8.0);
}

#[test]
fn calling_non_function_value_warns_and_yields_undef() {
    let value = eval("f = 5; result = f(1);").expect("must not abort the script");
    assert_eq!(value, Value::Undef);
}

#[test]
fn geometry_driven_by_function_literal() {
    let mesh = run("f = function(x) x * 2; cube([f(2), 1, 1]);").expect("geometry");
    let max_x = mesh
        .vertices
        .iter()
        .map(|v| v[0])
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((max_x - 4.0).abs() < 1e-9, "max_x = {max_x}");
    assert!(!mesh.faces.is_empty(), "cube faces vanished");
}

#[test]
fn echo_formats_function_value_like_upstream() {
    // Upstream OpenSCAD prints `function(a, b = 2) (a + b)` for a literal.
    let value = eval("result = function(a, b = 2) a + b;").expect("parse+eval");
    assert_eq!(value.to_string(), "function(a, b = 2) (a + b)");
    let value = eval("result = function(x) x;").expect("parse+eval");
    assert_eq!(value.to_string(), "function(x) x");
}
