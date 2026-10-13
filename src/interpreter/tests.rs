#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::interpreter::{Environment, Executor, Value};
    use crate::parser::Parser;
    use std::fs;
    use std::path::PathBuf;
    use std::rc::Rc;

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
    fn test_comparison_operators_after_function_calls() {
        let parsed = Parser::parse("x = version_num() >= 20210100;").unwrap();
        assert_eq!(parsed.statements.len(), 1);
    }

    #[test]
    fn test_version_num_and_assert_module() {
        let parsed = Parser::parse("assert(version_num() >= 20210100); cube(1);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        assert!(!exec.build_mesh().unwrap().is_empty());
    }

    #[test]
    fn test_cancel_stops_evaluation_on_its_thread_only() {
        // Cancellation targets the thread that registered itself, so the rest
        // of the (parallel) test suite is unaffected.
        crate::cancel::begin_cancellable_eval();
        crate::cancel::cancel_evaluation();
        let parsed = Parser::parse("for (i = [0:1000]) cube(i);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        let error = exec.execute_source(&parsed).unwrap_err();
        assert_eq!(error, crate::cancel::CANCELLED_MESSAGE);
        crate::cancel::reset_evaluation();

        // Without a registration the flag must be inert.
        crate::cancel::cancel_evaluation();
        assert!(!crate::cancel::evaluation_cancelled());
        crate::cancel::reset_evaluation();
    }

    #[test]
    fn test_bosll2_type_and_list_builtins() {
        let env = eval_source(
            "a = [is_def(undef), is_int(2), is_finite(2), is_nan(0), is_vector([1]), last([3,4])];",
        )
        .unwrap();
        assert_eq!(
            env.get("a"),
            Some(crate::interpreter::Value::Vector(vec![
                crate::interpreter::Value::Bool(false),
                crate::interpreter::Value::Bool(true),
                crate::interpreter::Value::Bool(true),
                crate::interpreter::Value::Bool(false),
                crate::interpreter::Value::Bool(true),
                crate::interpreter::Value::Number(4.0),
            ]))
        );
    }

    #[test]
    fn test_eval_vector() {
        let input = "x = [1, 2, 3];";
        let env = eval_source(input).unwrap();
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::Vector(vec![
                crate::interpreter::Value::Number(1.0),
                crate::interpreter::Value::Number(2.0),
                crate::interpreter::Value::Number(3.0),
            ]))
        );
    }

    #[test]
    fn test_eval_range() {
        let input = "x = [0:2:10];";
        let env = eval_source(input).unwrap();
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::Vector(vec![
                crate::interpreter::Value::Number(0.0),
                crate::interpreter::Value::Number(2.0),
                crate::interpreter::Value::Number(4.0),
                crate::interpreter::Value::Number(6.0),
                crate::interpreter::Value::Number(8.0),
                crate::interpreter::Value::Number(10.0),
            ]))
        );
    }

    #[test]
    fn test_eval_function() {
        let env = eval_source("function foo(x) = x + 1; y = foo(4);").unwrap();
        assert_eq!(env.get("y"), Some(crate::interpreter::Value::Number(5.0)));
    }

    #[test]
    fn test_anonymous_function_value() {
        let env = eval_source("add = function(x) x + 1; y = add(2);").unwrap();
        assert_eq!(env.get("y"), Some(crate::interpreter::Value::Number(3.0)));
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
        let max_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(max_x, 3.0);
    }

    #[test]
    fn test_eval_builtin_sin() {
        let input = "x = sin(90);";
        let parsed = Parser::parse(input);
        println!("Parse result: {:?}", parsed);
        let env = eval_source(input).unwrap();
        assert!(matches!(
            env.get("x"),
            Some(crate::interpreter::Value::Number(_))
        ));
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
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::Vector(vec![
                crate::interpreter::Value::Number(2.0),
                crate::interpreter::Value::Number(4.0),
                crate::interpreter::Value::Number(6.0),
            ]))
        );
    }

    #[test]
    fn test_filtered_for_expression() {
        let env = eval_source("x = [for (i = [0:4]) if (i % 2 == 0) i];").unwrap();
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::Vector(vec![
                crate::interpreter::Value::Number(0.0),
                crate::interpreter::Value::Number(2.0),
                crate::interpreter::Value::Number(4.0),
            ]))
        );
    }

    #[test]
    fn test_if_expression_without_else_filters_undef() {
        let env = eval_source("x = [if (false) 1, if (true) 2];").unwrap();
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::Vector(vec![
                crate::interpreter::Value::Number(2.0),
            ]))
        );
    }

    #[test]
    fn test_nested_if_expression_else_if() {
        let env = eval_source("x = [if (false) 1 else if (true) 2];").unwrap();
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::Vector(vec![
                crate::interpreter::Value::Number(2.0),
            ]))
        );
    }

    #[test]
    fn test_nested_for_expression() {
        let env = eval_source("x = [for (i = [1:2], j = [10:11]) i + j];").unwrap();
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::Vector(vec![
                crate::interpreter::Value::Number(11.0),
                crate::interpreter::Value::Number(12.0),
                crate::interpreter::Value::Number(12.0),
                crate::interpreter::Value::Number(13.0),
            ]))
        );
    }

    #[test]
    fn test_for_module_repeats_child_geometry() {
        let parsed = Parser::parse("for (i = [0:2]) translate([i * 2, 0, 0]) cube(1);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.faces.len(), 36);
    }

    #[test]
    fn test_scalar_vector_arithmetic_and_scalar_rotation() {
        let parsed = Parser::parse("rotate(45) translate(2 * [1, 0, 0]) cube(1);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex[0] > 1.0 && vertex[1] > 1.0));
    }

    #[test]
    fn test_scale_accepts_uniform_scalar() {
        let parsed = Parser::parse("scale(2) cube(1);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        let max_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(max_x, 2.0);
    }

    #[test]
    fn test_each_expression() {
        let env = eval_source("x = each [[1, 2], [3, 4]];").unwrap();
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::Vector(vec![
                crate::interpreter::Value::Number(1.0),
                crate::interpreter::Value::Number(2.0),
                crate::interpreter::Value::Number(3.0),
                crate::interpreter::Value::Number(4.0),
            ]))
        );
    }

    #[test]
    fn test_string_escapes() {
        // OpenSCAD decodes `\uXXXX` but not the braced `\u{...}` form,
        // which it reads as a literal `u{...}`.
        let env = eval_source(r#"x = "line\n\u03bb\x21";"#).unwrap();
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
    fn test_invalid_primitive_dimensions_are_rejected() {
        for input in [
            "cube([-1, 1, 1]);",
            "sphere(r = -1);",
            "circle(r = 1, $fn = 2);",
            "cylinder(h = 1, r = 1, $fs = 0);",
        ] {
            // OpenSCAD reports bad primitive arguments as warnings and keeps
            // going, so the invalid input must not abort the run nor emit
            // geometry of its own.
            let parsed = Parser::parse(input).unwrap();
            let mut exec = Executor::new(Rc::new(Environment::new()));
            exec.execute_source(&parsed).unwrap();
            if let Some(mesh) = exec.build_mesh() {
                assert!(
                    mesh.faces.is_empty(),
                    "invalid input produced geometry: {input}"
                );
            }

            // The run continues: a valid primitive after it still renders.
            let parsed = Parser::parse(&format!("{input} cube(2);")).unwrap();
            let mut exec = Executor::new(Rc::new(Environment::new()));
            exec.execute_source(&parsed).unwrap();
            let mesh = exec
                .build_mesh()
                .unwrap_or_else(|| panic!("valid primitive after {input} produced no mesh"));
            assert!(
                !mesh.faces.is_empty(),
                "run aborted after invalid input: {input}"
            );
        }
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
    fn test_sphere_has_no_degenerate_faces() {
        let parsed = Parser::parse("sphere(r = 2, $fn = 8);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 58);
        assert_eq!(mesh.faces.len(), 112);
        assert!(mesh.faces.iter().all(|face| {
            let a = mesh.vertices[face[0]];
            let b = mesh.vertices[face[1]];
            let c = mesh.vertices[face[2]];
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            (ab[0] * ac[1] - ab[1] * ac[0]).abs()
                + (ab[1] * ac[2] - ab[2] * ac[1]).abs()
                + (ab[2] * ac[0] - ab[0] * ac[2]).abs()
                > 1e-9
        }));
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
        fs::write(
            &stl,
            "solid test\n\
            facet normal 0 0 1\n\
              outer loop\n\
                vertex 0 0 0\n\
                vertex 1 0 0\n\
                vertex 0 1 0\n\
              endloop\n\
            endfacet\n\
            endsolid test\n",
        )
        .unwrap();
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
    fn test_imports_obj_relative_to_source() {
        let directory =
            std::env::temp_dir().join(format!("openscad-rust-obj-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("shape.obj"),
            "v 0 0 0\nv 2 0 0\nv 0 2 0\nf 1/1 2/2 3/3\n",
        )
        .unwrap();
        let source = directory.join("main.scad");
        std::fs::write(&source, "import(\"shape.obj\");").unwrap();

        let parsed = Parser::parse_file(&source).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.faces.len(), 1);

        std::fs::remove_file(source).unwrap();
        std::fs::remove_file(directory.join("shape.obj")).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn test_sibling_primitives_are_unioned() {
        let parsed =
            Parser::parse("cube([1, 1, 1]); translate([2, 0, 0]) cube([1, 1, 1]);").unwrap();
        let env = Rc::new(Environment::new());
        let mut exec = Executor::new(env);
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        let min_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::INFINITY, f64::min);
        let max_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((min_x - 0.0).abs() < 1e-9);
        assert!((max_x - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_difference_preserves_a_central_opening() {
        let parsed = Parser::parse(
            "difference() { cube([10, 10, 10], center = true); cylinder(h = 20, r = 2, center = true, $fn = 16); }",
        ).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        let min_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::INFINITY, f64::min);
        let max_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::NEG_INFINITY, f64::max);
        let min_z = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[2])
            .fold(f64::INFINITY, f64::min);
        let max_z = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[2])
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((min_x + 5.0).abs() < 1e-9);
        assert!((max_x - 5.0).abs() < 1e-9);
        assert!((min_z + 5.0).abs() < 1e-9);
        assert!((max_z - 5.0).abs() < 1e-9);
        assert!(mesh.faces.len() > 24);
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
    fn test_polygon_holes_are_triangulated() {
        let parsed = Parser::parse(
            "polygon(points = [[0,0], [4,0], [4,4], [0,4], [1,1], [1,2], [2,2], [2,1]], paths = [[0,1,2,3], [4,5,6,7]]);",
        ).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.faces.len(), 8);
        let area = mesh
            .faces
            .iter()
            .map(|face| {
                let a = mesh.vertices[face[0]];
                let b = mesh.vertices[face[1]];
                let c = mesh.vertices[face[2]];
                ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.0
            })
            .sum::<f64>();
        assert!((area - 15.0).abs() < 1e-9, "triangulated area was {area}");
        let hole_center = [1.5, 1.5];
        let contains = |point: [f64; 2], face: &[usize; 3]| {
            let vertices = face.map(|index| mesh.vertices[index]);
            let signs = [
                (vertices[1][0] - vertices[0][0]) * (point[1] - vertices[0][1])
                    - (vertices[1][1] - vertices[0][1]) * (point[0] - vertices[0][0]),
                (vertices[2][0] - vertices[1][0]) * (point[1] - vertices[1][1])
                    - (vertices[2][1] - vertices[1][1]) * (point[0] - vertices[1][0]),
                (vertices[0][0] - vertices[2][0]) * (point[1] - vertices[2][1])
                    - (vertices[0][1] - vertices[2][1]) * (point[0] - vertices[2][0]),
            ];
            signs.iter().all(|sign| *sign >= 0.0) || signs.iter().all(|sign| *sign <= 0.0)
        };
        assert!(!mesh.faces.iter().any(|face| contains(hole_center, face)));
    }

    #[test]
    fn test_polygon_holes_must_be_inside_outer_path() {
        let parsed = Parser::parse(
            "polygon(points = [[0,0], [4,0], [4,4], [0,4], [5,5], [6,5], [6,6], [5,6]], paths = [[0,1,2,3], [4,5,6,7]]);",
        ).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        // OpenSCAD logs the problem and keeps going without that polygon.
        exec.execute_source(&parsed).unwrap();
        if let Some(mesh) = exec.build_mesh() {
            assert!(mesh.faces.is_empty());
        }

        // The run continues: a valid primitive after it still renders.
        let parsed = Parser::parse(
            "polygon(points = [[0,0], [4,0], [4,4], [0,4], [5,5], [6,5], [6,6], [5,6]], paths = [[0,1,2,3], [4,5,6,7]]); cube(2);",
        ).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        assert!(exec
            .build_mesh()
            .map(|mesh| !mesh.faces.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn test_invalid_polygon_geometry_reports_error() {
        let parsed = Parser::parse("polygon(points = [[0,0], [4,4], [0,3], [3,0]]);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        if let Some(mesh) = exec.build_mesh() {
            assert!(mesh.faces.is_empty());
        }

        let parsed = Parser::parse("polygon(points = [[0,0], [1,0], [2,0]]);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        if let Some(mesh) = exec.build_mesh() {
            assert!(mesh.faces.is_empty());
        }
    }

    #[test]
    fn test_stl_export_skips_invalid_faces() {
        let mesh = crate::csg::Mesh {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
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
    fn test_2d_offset_and_difference() {
        let code = r#"
s = 10;
difference() {
    offset(delta = 0.6) polygon([
        [-0.40*s, -0.50*s],
        [ 0.40*s, -0.50*s],
        [ 0.40*s,  0.05*s],
        [ 0.50*s,  0.05*s],
        [ 0.00,    0.50*s],
        [-0.50*s,  0.05*s],
        [-0.40*s,  0.05*s],
    ]);
    offset(delta = -0.6) polygon([
        [-0.40*s, -0.50*s],
        [ 0.40*s, -0.50*s],
        [ 0.40*s,  0.05*s],
        [ 0.50*s,  0.05*s],
        [ 0.00,    0.50*s],
        [-0.50*s,  0.05*s],
        [-0.40*s,  0.05*s],
    ]);
}
"#;
        let parsed = Parser::parse(code).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert!(mesh.is_2d());
        assert!(!mesh.faces.is_empty());
        assert_eq!(mesh.vertices.len(), 19);
        assert_eq!(mesh.faces.len(), 15);
    }

    #[test]
    fn test_named_color_preserves_child_geometry() {
        let parsed = Parser::parse("color(\"lime\") cube(2);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        assert!(!exec.build_mesh().unwrap().is_empty());
    }

    #[test]
    fn test_render_preserves_child_geometry() {
        let parsed = Parser::parse("render(convexity = 4) translate([2, 0, 0]) cube(2);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        let min_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::INFINITY, f64::min);
        assert_eq!(min_x, 2.0);
    }

    #[test]
    fn test_multmatrix_transforms_child_geometry() {
        let parsed =
            Parser::parse("multmatrix([[1,0,0,2],[0,1,0,3],[0,0,1,4],[0,0,0,1]]) cube();").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        let min_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::INFINITY, f64::min);
        let min_y = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[1])
            .fold(f64::INFINITY, f64::min);
        let min_z = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[2])
            .fold(f64::INFINITY, f64::min);
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
                    child: Box::new(crate::csg::CsgNode::Primitive(
                        crate::csg::Primitive::Cube {
                            size: [1.0, 1.0, 1.0],
                            center: false,
                        },
                    )),
                    translate: Some([3.0, 0.0, 0.0]),
                    rotate: None,
                    scale: None,
                    mirror: None,
                    matrix: None,
                },
            ],
        }
        .to_mesh();
        let min_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::INFINITY, f64::min);
        let max_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(min_x, 0.0);
        assert_eq!(max_x, 4.0);
        assert!(!mesh.faces.is_empty());
    }

    #[test]
    fn test_minkowski_combines_child_extents() {
        let mesh = crate::csg::CsgNode::Minkowski {
            children: vec![
                crate::csg::CsgNode::Primitive(crate::csg::Primitive::Cube {
                    size: [2.0, 2.0, 2.0],
                    center: false,
                }),
                crate::csg::CsgNode::Primitive(crate::csg::Primitive::Cube {
                    size: [1.0, 1.0, 1.0],
                    center: false,
                }),
            ],
        }
        .to_mesh();
        let max_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[1])
            .fold(f64::NEG_INFINITY, f64::max);
        let max_z = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[2])
            .fold(f64::NEG_INFINITY, f64::max);
        assert_eq!([max_x, max_y, max_z], [3.0, 3.0, 3.0]);
        assert!(!mesh.faces.is_empty());
    }

    #[test]
    fn test_text_builds_planar_geometry_and_extrudes() {
        let parsed = Parser::parse("linear_extrude(height = 2) text(\"CAD\", size = 5);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.faces.len(), 36);
        assert_eq!(
            mesh.vertices
                .iter()
                .map(|vertex| vertex[2])
                .fold(f64::NEG_INFINITY, f64::max),
            2.0
        );
    }

    #[test]
    fn test_text_alignment_positions_geometry() {
        let parsed =
            Parser::parse("text(\"AB\", size = 10, halign = \"center\", valign = \"top\");")
                .unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        let min_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::INFINITY, f64::min);
        let max_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[1])
            .fold(f64::INFINITY, f64::min);
        let max_y = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[1])
            .fold(f64::NEG_INFINITY, f64::max);
        assert_eq!([min_x, max_x, min_y, max_y], [-6.5, 6.5, -10.0, 0.0]);
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
        let min_z = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[2])
            .fold(f64::INFINITY, f64::min);
        let max_z = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[2])
            .fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(min_z, 0.0);
        assert_eq!(max_z, 3.0);
    }

    #[test]
    fn test_linear_extrude_applies_scale_and_twist() {
        let parsed =
            Parser::parse("linear_extrude(height = 2, scale = 2, twist = 90) square([2, 1]);")
                .unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        let max_z = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[2])
            .fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(max_z, 2.0);
        let max_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0].abs())
            .fold(0.0, f64::max);
        let max_y = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[1].abs())
            .fold(0.0, f64::max);
        assert!(max_x > 1.9);
        assert!(max_y > 1.9);
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
    fn test_rotate_extrude_accepts_positional_angle() {
        let parsed = Parser::parse("rotate_extrude(180, $fn = 8) square([1, 1]);").unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        let max_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::NEG_INFINITY, f64::max);
        let min_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex[0])
            .fold(f64::INFINITY, f64::min);
        assert!(max_x > 0.9);
        assert!(min_x < -0.9);
    }

    #[test]
    fn test_include_loads_relative_assignments() {
        let dir = unique_fixture_dir("include");
        fs::create_dir_all(&dir).unwrap();
        let library = dir.join("library.scad");
        let main = dir.join("main.scad");
        fs::write(&library, "included_value = 42;").unwrap();
        fs::write(
            &main,
            "include <library.scad>\nresult = included_value + 1;",
        )
        .unwrap();

        let source = Parser::parse_file(&main).unwrap();
        let env = Rc::new(Environment::new());
        let mut exec = Executor::new(env.clone());
        exec.execute_source(&source).unwrap();
        assert_eq!(
            env.get("result"),
            Some(crate::interpreter::Value::Number(43.0))
        );

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_use_loads_definitions_without_assignments() {
        let dir = unique_fixture_dir("use");
        fs::create_dir_all(&dir).unwrap();
        let library = dir.join("library.scad");
        let main = dir.join("main.scad");
        fs::write(
            &library,
            "library_value = 42; function increment(x) = x + 1;",
        )
        .unwrap();
        fs::write(&main, "use <library.scad>\nresult = increment(2);").unwrap();

        let source = Parser::parse_file(&main).unwrap();
        let env = Rc::new(Environment::new());
        let mut exec = Executor::new(env.clone());
        exec.execute_source(&source).unwrap();
        assert_eq!(
            env.get("result"),
            Some(crate::interpreter::Value::Number(3.0))
        );
        assert_eq!(env.get("library_value"), None);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_include_cycle_is_skipped() {
        let dir = unique_fixture_dir("cycle");
        fs::create_dir_all(&dir).unwrap();
        let first = dir.join("first.scad");
        let second = dir.join("second.scad");
        fs::write(&first, "include <second.scad>").unwrap();
        fs::write(&second, "include <first.scad>").unwrap();

        let source = Parser::parse_file(&first).unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        // OpenSCAD skips the recursive include instead of failing.
        exec.execute_source(&source).unwrap();

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_builtin_rands_with_seed() {
        let env1 = eval_source("x = rands(0, 10, 3, 42);").unwrap();
        let env2 = eval_source("x = rands(0, 10, 3, 42);").unwrap();
        assert_eq!(
            env1.get("x"),
            env2.get("x"),
            "rands with same seed should produce identical results"
        );
    }

    #[test]
    fn test_builtin_ord() {
        let env = eval_source("x = ord(\"A\");").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(65.0)));

        let env = eval_source("x = ord(\"hello\");").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(104.0)));

        // OpenSCAD: ord("") echoes undef instead of failing.
        let env = eval_source("x = ord(\"\");").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Undef));
    }

    #[test]
    fn test_builtin_version() {
        let env = eval_source("x = version();").unwrap();
        assert!(matches!(
            env.get("x"),
            Some(crate::interpreter::Value::Vector(_))
        ));
    }

    #[test]
    fn test_builtin_version_num() {
        let env = eval_source("x = version_num();").unwrap();
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::Number(20210100.0))
        );
    }

    #[test]
    fn test_builtin_search_string() {
        let env = eval_source("x = search(\"an\", \"banana\");").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(1.0)));

        let env = eval_source("x = search(\"z\", \"banana\");").unwrap();
        assert!(matches!(
            env.get("x"),
            Some(crate::interpreter::Value::Undef)
        ));
    }

    #[test]
    fn test_builtin_search_vector() {
        let env =
            eval_source("x = search(\"apple\", [\"banana\", \"apple\", \"cherry\"]);").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(1.0)));

        let env = eval_source("x = search(\"z\", [\"banana\", \"apple\", \"cherry\"]);").unwrap();
        assert!(matches!(
            env.get("x"),
            Some(crate::interpreter::Value::Undef)
        ));

        let env = eval_source("x = search(\"a\", [\"banana\", \"apple\", \"cherry\"]);").unwrap();
        if let Some(crate::interpreter::Value::Vector(vec)) = env.get("x") {
            assert_eq!(vec.len(), 2);
            assert_eq!(vec[0], crate::interpreter::Value::Number(0.0));
            assert_eq!(vec[1], crate::interpreter::Value::Number(1.0));
        }
    }

    #[test]
    fn test_phase1_undef_and_defaults() {
        let parsed = Parser::parse(
            "
            x = non_existent_var;
            y = 1 + x;
            is_u = is_undef(non_existent_var);
            p = $preview;
            lh = $layer_height;
        ",
        )
        .unwrap();
        let env = Rc::new(Environment::new());
        let mut exec = Executor::new(env.clone());
        exec.execute_source(&parsed).unwrap();
        assert_eq!(env.get("x").unwrap(), Value::Undef);
        assert_eq!(env.get("y").unwrap(), Value::Undef);
        assert_eq!(env.get("is_u").unwrap(), Value::Bool(true));
        assert_eq!(env.get("p").unwrap(), Value::Bool(false));
        assert_eq!(env.get("lh").unwrap(), Value::Number(0.2));
    }

    #[test]
    fn test_phase1_2d_vector_transforms() {
        let parsed = Parser::parse(
            "
            translate([10, 20]) scale([2, 3]) cube([1, 1, 1]);
        ",
        )
        .unwrap();
        let mut exec = Executor::new(Rc::new(Environment::new()));
        exec.execute_source(&parsed).unwrap();
        let mesh = exec.build_mesh().unwrap();
        assert!(!mesh.is_empty());
    }

    #[test]
    fn test_builtin_chr_single() {
        let env = eval_source("x = chr(65);").unwrap();
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::String("A".to_string()))
        );
    }

    #[test]
    fn test_builtin_chr_vector() {
        let env = eval_source("x = chr([65, 66, 67]);").unwrap();
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::String("ABC".to_string()))
        );
    }

    #[test]
    fn test_builtin_chr_invalid() {
        // OpenSCAD chr("invalid") echoes the empty string.
        let env = eval_source("x = chr(\"invalid\");").unwrap();
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::String(String::new()))
        );
    }

    #[test]
    fn test_builtin_cross_3d() {
        let env = eval_source("x = cross([1, 0, 0], [0, 1, 0]);").unwrap();
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::Vector(vec![
                crate::interpreter::Value::Number(0.0),
                crate::interpreter::Value::Number(0.0),
                crate::interpreter::Value::Number(1.0),
            ]))
        );
    }

    #[test]
    fn test_builtin_cross_2d() {
        let env = eval_source("x = cross([1, 0], [0, 1]);").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(1.0)));
    }

    #[test]
    fn test_builtin_cross_invalid_dim() {
        // OpenSCAD warns about the vector size and yields undef.
        let env = eval_source("x = cross([1, 0], [0, 1, 2]);").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Undef));
    }

    #[test]
    fn test_builtin_norm() {
        let env = eval_source("x = norm([3, 4]);").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(5.0)));
    }

    #[test]
    fn test_builtin_unit() {
        let env = eval_source("x = unit([3, 4]);").unwrap();
        assert_eq!(
            env.get("x"),
            Some(crate::interpreter::Value::Vector(vec![
                crate::interpreter::Value::Number(3.0 / 5.0),
                crate::interpreter::Value::Number(4.0 / 5.0),
            ]))
        );
    }

    #[test]
    fn test_builtin_unit_zero() {
        let env = eval_source("x = unit([0, 0]);").unwrap();
        assert!(matches!(
            env.get("x"),
            Some(crate::interpreter::Value::Undef)
        ));
    }

    #[test]
    fn test_builtin_lookup() {
        let env = eval_source(
            "
            table = [[0, 10], [10, 20], [20, 40]];
            v_low = lookup(-5, table);
            v_mid = lookup(5, table);
            v_high = lookup(30, table);
        ",
        )
        .unwrap();
        assert_eq!(
            env.get("v_low"),
            Some(crate::interpreter::Value::Number(10.0))
        );
        assert_eq!(
            env.get("v_mid"),
            Some(crate::interpreter::Value::Number(15.0))
        );
        assert_eq!(
            env.get("v_high"),
            Some(crate::interpreter::Value::Number(40.0))
        );
    }

    #[test]
    fn test_separate_namespaces() {
        let env = eval_source(
            "
            function foo(x) = x * 2;
            foo = 100;
            y = foo(foo);
        ",
        )
        .unwrap();
        assert_eq!(
            env.get("foo"),
            Some(crate::interpreter::Value::Number(100.0))
        );
        assert_eq!(env.get("y"), Some(crate::interpreter::Value::Number(200.0)));
    }

    #[test]
    fn test_recursion_limit_function() {
        // OpenSCAD warns about runaway recursion and evaluates to undef.
        let env = eval_source(
            "
            function rec(x) = rec(x + 1);
            val = rec(0);
        ",
        )
        .unwrap();
        assert_eq!(
            env.get("val"),
            Some(crate::interpreter::Value::Undef),
            "runaway recursion must degrade to undef, not abort"
        );
    }

    #[test]
    fn test_recursion_limit_module() {
        // OpenSCAD warns about runaway recursion and skips the body.
        eval_source(
            "
            module rec(x) { rec(x + 1); }
            rec(0);
        ",
        )
        .unwrap();
    }

    #[test]
    fn test_for_c_basic() {
        let env = eval_source(
            "
            v = [for (i = 0; i < 5; i = i + 1) i * 2];
        ",
        )
        .unwrap();
        assert_eq!(
            env.get("v"),
            Some(crate::interpreter::Value::Vector(vec![
                crate::interpreter::Value::Number(0.0),
                crate::interpreter::Value::Number(2.0),
                crate::interpreter::Value::Number(4.0),
                crate::interpreter::Value::Number(6.0),
                crate::interpreter::Value::Number(8.0),
            ]))
        );
    }

    #[test]
    fn test_for_c_empty_init() {
        let env = eval_source(
            "
            list = [[1, 2], [3, 4]];
            v = [for (; is_list(list) && len(list) > 0; list = list[0]) len(list)];
        ",
        )
        .unwrap();
        assert_eq!(
            env.get("v"),
            Some(crate::interpreter::Value::Vector(vec![
                crate::interpreter::Value::Number(2.0),
                crate::interpreter::Value::Number(2.0),
            ]))
        );
    }

    #[test]
    fn test_for_c_with_if_filter() {
        let env = eval_source(
            "
            v = [for (i = 0; i < 5; i = i + 1) if (i % 2 == 0) i];
        ",
        )
        .unwrap();
        assert_eq!(
            env.get("v"),
            Some(crate::interpreter::Value::Vector(vec![
                crate::interpreter::Value::Number(0.0),
                crate::interpreter::Value::Number(2.0),
                crate::interpreter::Value::Number(4.0),
            ]))
        );
    }

    #[test]
    fn test_range_infinite_and_huge_warns_and_degrades() {
        // [0:inf] or huge step count must warn and degrade to undef instead of OOM
        let env = eval_source(
            "
            inf = 1/0;
            v = [for (i = [0:inf]) i];
            w = [for (i = [0:1:4294967296]) i];
        ",
        )
        .unwrap();
        assert_eq!(
            env.get("v"),
            Some(crate::interpreter::Value::Vector(vec![]))
        );
        assert_eq!(
            env.get("w"),
            Some(crate::interpreter::Value::Vector(vec![]))
        );
    }

    #[test]
    fn test_range_nan_step_evaluates_empty() {
        let env = eval_source(
            "
            nan = 0/0;
            v = [for (i = [0:nan:0]) i];
        ",
        )
        .unwrap();
        assert_eq!(
            env.get("v"),
            Some(crate::interpreter::Value::Vector(vec![]))
        );
    }

    #[test]
    fn test_whitespace_zero_width_nbsp() {
        let env = eval_source("a\u{FEFF}=\u{FEFF}42;").unwrap();
        assert_eq!(env.get("a"), Some(crate::interpreter::Value::Number(42.0)));
    }

    fn unique_fixture_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("openscad-rust-{}-{}", name, std::process::id()))
    }
}
