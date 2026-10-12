#[cfg(test)]
mod tests {
    use crate::parser::Parser;
    use crate::interpreter::{Executor, Environment};
    use std::rc::Rc;
    use std::fs;
    use std::path::PathBuf;

    fn eval_source(input: &str) -> Result<Rc<Environment>, String> {
        let parsed = Parser::parse(input).map_err(|e| format!("Parse error: {:?}", e))?;
        let env = Rc::new(Environment::new());
        let mut exec = Executor::new(env.clone());
        for stmt in &parsed.statements {
            exec.execute(stmt)?;
        }
        Ok(env)
    }

    #[test]
    fn test_eval_assignment() {
        let input = "x = 10;";
        let env = eval_source(input).unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(10.0)));
    }

    #[test]
    fn test_eval_expression() {
        let input = "x = 1 + 2 * 3;";
        let env = eval_source(input).unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(7.0)));
    }

    #[test]
    fn test_eval_vector() {
        let input = "x = [1, 2, 3];";
        let env = eval_source(input).unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Vector(vec![
            crate::interpreter::Value::Number(1.0),
            crate::interpreter::Value::Number(2.0),
            crate::interpreter::Value::Number(3.0),
        ])));
    }

    #[test]
    fn test_eval_range() {
        let input = "x = [0:2:10];";
        let env = eval_source(input).unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Vector(vec![
            crate::interpreter::Value::Number(0.0),
            crate::interpreter::Value::Number(2.0),
            crate::interpreter::Value::Number(4.0),
            crate::interpreter::Value::Number(6.0),
            crate::interpreter::Value::Number(8.0),
            crate::interpreter::Value::Number(10.0),
        ])));
    }

    #[test]
    fn test_eval_function() {
        let env = eval_source("function foo(x) = x + 1; y = foo(4);").unwrap();
        assert_eq!(env.get("y"), Some(crate::interpreter::Value::Number(5.0)));
    }

    #[test]
    fn test_eval_module() {
        let input = "module foo() { cube(10); }";
        assert!(eval_source(input).is_ok());
    }

    #[test]
    fn test_module_arguments_and_geometry() {
        let input = "module plate(size = 2) { cube(size = size); } plate(3);";
        let parsed = Parser::parse(input).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        let max_x = mesh.vertices.iter().map(|vertex| vertex[0]).fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(max_x, 3.0);
    }

    #[test]
    fn test_eval_builtin_sin() {
        let input = "x = sin(90);";
        let parsed = Parser::parse(input);
        println!("Parse result: {:?}", parsed);
        let env = eval_source(input).unwrap();
        assert!(matches!(env.get("x"), Some(crate::interpreter::Value::Number(_))));
    }

    #[test]
    fn test_eval_if() {
        let input = "if (true) { x = 1; }";
        let parsed = Parser::parse(input);
        println!("Parse result: {:?}", parsed);
        let env = eval_source(input).unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(1.0)));
    }

    #[test]
    fn test_eval_let() {
        let input = "x = let(a = 1, b = 2) a + b;";
        let parsed = Parser::parse(input);
        println!("Parse result: {:?}", parsed);
        let env = eval_source(input).unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(3.0)));
    }

    #[test]
    fn test_unary_and_exponent() {
        let env = eval_source("x = -2 ^ 3; y = !false;").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(-8.0)));
        assert_eq!(env.get("y"), Some(crate::interpreter::Value::Bool(true)));
    }

    #[test]
    fn test_named_function_argument() {
        let env = eval_source("function add(a, b) = a + b; x = add(b = 4, a = 3);").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(7.0)));
    }

    #[test]
    fn test_for_expression() {
        let env = eval_source("x = [for (i = [1:3]) i * 2];").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Vector(vec![
            crate::interpreter::Value::Number(2.0),
            crate::interpreter::Value::Number(4.0),
            crate::interpreter::Value::Number(6.0),
        ])));
    }

    #[test]
    fn test_nested_for_expression() {
        let env = eval_source("x = [for (i = [1:2], j = [10:11]) i + j];").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Vector(vec![
            crate::interpreter::Value::Number(11.0),
            crate::interpreter::Value::Number(12.0),
            crate::interpreter::Value::Number(12.0),
            crate::interpreter::Value::Number(13.0),
        ])));
    }

    #[test]
    fn test_each_expression() {
        let env = eval_source("x = each [[1, 2], [3, 4]];").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Vector(vec![
            crate::interpreter::Value::Number(1.0),
            crate::interpreter::Value::Number(2.0),
            crate::interpreter::Value::Number(3.0),
            crate::interpreter::Value::Number(4.0),
        ])));
    }

    #[test]
    fn test_string_escapes() {
        let env = eval_source(r#"x = "line\n\u{03bb}\x21";"#).unwrap();
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::String("line\nλ!".to_string()))
        );
    }

    #[test]
    fn test_cylinder_builds_closed_mesh() {
        let parsed = Parser::parse("cylinder(h = 4, r = 2, $fn = 12);").unwrap();
        let env = Rc::new(Environment::new());
        let mut exec = Executor::new(env);
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 26);
        assert_eq!(mesh.faces.len(), 48);
    }

    #[test]
    fn test_sibling_primitives_are_unioned() {
        let parsed = Parser::parse("cube([1, 1, 1]); translate([2, 0, 0]) cube([1, 1, 1]);").unwrap();
        let env = Rc::new(Environment::new());
        let mut exec = Executor::new(env);
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        let min_x = mesh.vertices.iter().map(|vertex| vertex[0]).fold(f64::INFINITY, f64::min);
        let max_x = mesh.vertices.iter().map(|vertex| vertex[0]).fold(f64::NEG_INFINITY, f64::max);
        assert!((min_x - 0.0).abs() < 1e-9);
        assert!((max_x - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_polyhedron_builds_mesh() {
        let input = "polyhedron(points = [[0,0,0], [1,0,0], [0,1,0], [0,0,1]], faces = [[0,2,1], [0,1,3], [0,3,2], [1,2,3]]);";
        let parsed = Parser::parse(input).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.faces.len(), 4);
    }

    #[test]
    fn test_polygon_builds_triangulated_mesh() {
        let input = "polygon(points = [[0,0], [2,0], [2,1], [0,1]]);";
        let parsed = Parser::parse(input).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.faces.len(), 2);
        assert!(mesh.vertices.iter().all(|vertex| vertex[2] == 0.0));
    }

    #[test]
    fn test_linear_extrude_builds_solid_mesh() {
        let input = "linear_extrude(height = 3) polygon(points = [[0,0], [2,0], [2,1], [0,1]]);";
        let parsed = Parser::parse(input).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 8);
        assert_eq!(mesh.faces.len(), 12);
        let min_z = mesh.vertices.iter().map(|vertex| vertex[2]).fold(f64::INFINITY, f64::min);
        let max_z = mesh.vertices.iter().map(|vertex| vertex[2]).fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(min_z, 0.0);
        assert_eq!(max_z, 3.0);
    }

    #[test]
    fn test_rotate_extrude_builds_revolved_mesh() {
        let input = "rotate_extrude($fn = 8) polygon(points = [[1,0], [2,0], [2,1], [1,1]]);";
        let parsed = Parser::parse(input).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 36);
        assert_eq!(mesh.faces.len(), 96);
        assert!(mesh.vertices.iter().any(|vertex| vertex[0] > 1.9));
        assert!(mesh.vertices.iter().any(|vertex| vertex[0] < -1.9));
    }

    #[test]
    fn test_include_loads_relative_assignments() {
        let dir = unique_fixture_dir("include");
        fs::create_dir_all(&dir).unwrap();
        let library = dir.join("library.scad");
        let main = dir.join("main.scad");
        fs::write(&library, "included_value = 42;").unwrap();
        fs::write(&main, "include <library.scad>\nresult = included_value + 1;").unwrap();

        let source = Parser::parse_file(&main).unwrap();
        let env = Rc::new(Environment::new());
        let mut exec = Executor::new(env.clone());
        exec.execute_source(&source).unwrap();
        assert_eq!(env.get("result"), Some(crate::interpreter::Value::Number(43.0)));

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_use_loads_definitions_without_assignments() {
        let dir = unique_fixture_dir("use");
        fs::create_dir_all(&dir).unwrap();
        let library = dir.join("library.scad");
        let main = dir.join("main.scad");
        fs::write(&library, "library_value = 42; function increment(x) = x + 1;").unwrap();
        fs::write(&main, "use <library.scad>\nresult = increment(2);").unwrap();

        let source = Parser::parse_file(&main).unwrap();
        let env = Rc::new(Environment::new());
        let mut exec = Executor::new(env.clone());
        exec.execute_source(&source).unwrap();
        assert_eq!(env.get("result"), Some(crate::interpreter::Value::Number(3.0)));
        assert_eq!(env.get("library_value"), None);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_include_cycle_is_rejected() {
        let dir = unique_fixture_dir("cycle");
        fs::create_dir_all(&dir).unwrap();
        let first = dir.join("first.scad");
        let second = dir.join("second.scad");
        fs::write(&first, "include <second.scad>").unwrap();
        fs::write(&second, "include <first.scad>").unwrap();

        let source = Parser::parse_file(&first).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        let error = exec.execute_source(&source).unwrap_err();
        assert!(error.contains("Cyclic source include"));

        fs::remove_dir_all(dir).unwrap();
    }

    fn unique_fixture_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "openscad-rust-{}-{}",
            name,
            std::process::id()
        ))
    }
}