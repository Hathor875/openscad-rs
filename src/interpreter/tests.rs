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
    fn test_2d_primitives_build_planar_meshes() {
        let parsed = Parser::parse("square([2, 3], center = true);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.faces.len(), 2);
        assert!(mesh.vertices.iter().all(|vertex| vertex[2] == 0.0));

        let parsed = Parser::parse("circle(r = 2, $fn = 8);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 9);
        assert_eq!(mesh.faces.len(), 8);
        assert!(mesh.vertices.iter().all(|vertex| vertex[2] == 0.0));
    }

    #[test]
    fn test_2d_primitives_can_be_extruded() {
        let parsed = Parser::parse("linear_extrude(height = 2) circle(r = 1, $fn = 8);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 18);
        assert_eq!(mesh.faces.len(), 32);
    }

    #[test]
    fn test_fa_and_fs_control_radial_resolution() {
        let parsed = Parser::parse("circle(r = 2, $fa = 90);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 5);
        assert_eq!(mesh.faces.len(), 4);

        let parsed = Parser::parse("cylinder(h = 1, r = 2, $fs = 1);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 28);
        assert_eq!(mesh.faces.len(), 52);
    }

    #[test]
    fn test_imports_ascii_stl_relative_to_source() {
        let dir = unique_fixture_dir("import");
        fs::create_dir_all(&dir).unwrap();
        let stl = dir.join("triangle.stl");
        let main = dir.join("main.scad");
        fs::write(&stl, "solid test\n\
            facet normal 0 0 1\n\
              outer loop\n\
                vertex 0 0 0\n\
                vertex 1 0 0\n\
                vertex 0 1 0\n\
              endloop\n\
            endfacet\n\
            endsolid test\n").unwrap();
        fs::write(&main, "import(\"triangle.stl\");").unwrap();

        let source = Parser::parse_file(&main).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&source).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.faces.len(), 1);

        fs::remove_dir_all(dir).unwrap();
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
    fn test_concave_polygon_uses_ear_clipping() {
        let input = "polygon(points = [[0,0], [3,0], [3,1], [1,1], [1,3], [0,3]]);";
        let parsed = Parser::parse(input).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.faces.len(), 4);
        assert!(mesh.faces.iter().all(|face| {
            let a = mesh.vertices[face[0]];
            let b = mesh.vertices[face[1]];
            let c = mesh.vertices[face[2]];
            ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() > 0.0
        }));
    }

    #[test]
    fn test_polygon_holes_report_unsupported_geometry() {
        let parsed = Parser::parse(
            "polygon(points = [[0,0], [4,0], [4,4], [0,4], [1,1], [1,2], [2,2], [2,1]], paths = [[0,1,2,3], [4,5,6,7]]);",
        ).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        let error = exec.execute_source(&parsed).unwrap_err();
        assert!(error.contains("holes are not supported"));
    }

    #[test]
    fn test_invalid_polygon_geometry_reports_error() {
        let parsed = Parser::parse(
            "polygon(points = [[0,0], [4,4], [0,3], [3,0]]);",
        ).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        let error = exec.execute_source(&parsed).unwrap_err();
        assert!(error.contains("self-intersect"));

        let parsed = Parser::parse(
            "polygon(points = [[0,0], [1,0], [2,0]]);",
        ).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        let error = exec.execute_source(&parsed).unwrap_err();
        assert!(error.contains("non-zero area"));
    }

    #[test]
    fn test_stl_export_skips_invalid_faces() {
        let mesh = crate::csg::Mesh {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            faces: vec![[0, 1, 2], [0, 2, 9], [0, 0, 1]],
            normals: Vec::new(),
        };
        let stl = mesh.to_stl();
        assert_eq!(stl.matches("facet normal").count(), 1);
    }

    #[test]
    fn test_color_preserves_child_geometry() {
        let parsed = Parser::parse("color([1, 0, 0]) cube([2, 3, 4]);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 8);
        assert_eq!(mesh.faces.len(), 12);
    }

    #[test]
    fn test_multmatrix_transforms_child_geometry() {
        let parsed = Parser::parse(
            "multmatrix([[1,0,0,2],[0,1,0,3],[0,0,1,4],[0,0,0,1]]) cube();",
        ).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        let min_x = mesh.vertices.iter().map(|vertex| vertex[0]).fold(f64::INFINITY, f64::min);
        let min_y = mesh.vertices.iter().map(|vertex| vertex[1]).fold(f64::INFINITY, f64::min);
        let min_z = mesh.vertices.iter().map(|vertex| vertex[2]).fold(f64::INFINITY, f64::min);
        assert_eq!([min_x, min_y, min_z], [2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_hull_contains_all_child_geometry() {
        let mesh = crate::csg::CsgNode::Hull {
            children: vec![
                crate::csg::CsgNode::Primitive(crate::csg::Primitive::Cube {
                    size: [1.0, 1.0, 1.0],
                    center: false,
                }),
                crate::csg::CsgNode::Transform {
                    child: Box::new(crate::csg::CsgNode::Primitive(crate::csg::Primitive::Cube {
                        size: [1.0, 1.0, 1.0],
                        center: false,
                    })),
                    translate: Some([3.0, 0.0, 0.0]),
                    rotate: None,
                    scale: None,
                    mirror: None,
                    matrix: None,
                },
            ],
        }.to_mesh();
        let min_x = mesh.vertices.iter().map(|vertex| vertex[0]).fold(f64::INFINITY, f64::min);
        let max_x = mesh.vertices.iter().map(|vertex| vertex[0]).fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(min_x, 0.0);
        assert_eq!(max_x, 4.0);
        assert!(!mesh.faces.is_empty());
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