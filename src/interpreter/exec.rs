use std::rc::Rc;
use crate::ast::{Statement, Assignment, Expression, ModuleModifiers};
use crate::interpreter::value::{Value, Environment, ModuleValue};
use crate::interpreter::eval::Evaluator;
use crate::csg::{Primitive, CsgBuilder};
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct Executor {
    pub env: Rc<Environment>,
    csg_stack: Vec<crate::csg::CsgBuilder>,
    source_stack: Vec<PathBuf>,
    loading: HashSet<PathBuf>,
}

impl Executor {
    pub fn new(env: Rc<Environment>) -> Self {
        Self {
            env,
            csg_stack: vec![crate::csg::CsgBuilder::new()],
            source_stack: Vec::new(),
            loading: HashSet::new(),
        }
    }

    pub fn execute_source(&mut self, source: &crate::ast::SourceFile) -> Result<(), String> {
        let path = if source.path.is_empty() {
            None
        } else {
            Some(Self::canonical_path(Path::new(&source.path))?)
        };
        if let Some(path) = &path {
            if !self.loading.insert(path.clone()) {
                return Err(format!("Cyclic source include '{}'", path.display()));
            }
            self.source_stack.push(path.clone());
        }

        let result = (|| {
            let root_source = self.source_stack.len() == 1;
            if root_source {
                for stmt in &source.statements {
                    match stmt {
                        Statement::ModuleDef { name, params, body, .. } =>
                            self.exec_module_def(name, params, body, &ModuleModifiers::default())?,
                        Statement::FunctionDef { name, params, body, .. } =>
                            self.exec_function_def(name, params, body)?,
                        _ => {}
                    }
                }
                for stmt in &source.statements {
                    if matches!(stmt, Statement::Include(_, _) | Statement::Use(_, _)) {
                        self.execute(stmt)?;
                    }
                }
                for stmt in &source.statements {
                    if let Statement::Assignment(assign) = stmt {
                        self.exec_assignment(assign)?;
                    }
                }
                source.statements.iter()
                    .filter(|stmt| !matches!(
                        stmt,
                        Statement::Assignment(_)
                            | Statement::ModuleDef { .. }
                            | Statement::FunctionDef { .. }
                            | Statement::Include(_, _)
                            | Statement::Use(_, _)
                    ))
                    .try_for_each(|stmt| self.execute(stmt).map(|_| ()))
            } else {
                source.statements.iter().try_for_each(|stmt| self.execute(stmt).map(|_| ()))
            }
        })();

        if let Some(path) = path {
            self.source_stack.pop();
            self.loading.remove(&path);
        }
        result
    }

    pub fn build_mesh(&self) -> Option<crate::csg::Mesh> {
        self.csg_stack.first()
            .and_then(|builder| builder.clone().build())
            .map(|node| node.to_mesh())
    }
    
    fn current_builder(&mut self) -> &mut crate::csg::CsgBuilder {
        self.csg_stack.last_mut().unwrap()
    }

    fn child_nodes(&mut self, children: &[Statement]) -> Result<Vec<crate::csg::CsgNode>, String> {
        let child_env = Environment::with_parent(self.env.clone());
        let mut child_exec = Executor::new(Rc::new(child_env));
        let mut nodes = Vec::new();
        for child in children {
            child_exec.csg_stack.push(CsgBuilder::new());
            child_exec.execute(child)?;
            if let Some(node) = child_exec.csg_stack.pop().unwrap().build() {
                nodes.push(node);
            }
        }
        Ok(nodes)
    }

    fn execute_with_forwarded_children(
        &mut self,
        stmt: &Statement,
        forwarded: &[Statement],
    ) -> Result<Option<Value>, String> {
        if let Statement::Block { statements, .. } = stmt {
            for statement in statements {
                self.execute_with_forwarded_children(statement, forwarded)?;
            }
            return Ok(None);
        }
        if let Statement::ModuleInstantiation { name, args, children, .. } = stmt {
            if name == "children" {
                for child in forwarded {
                    self.execute(child)?;
                }
                return Ok(None);
            }
            if name == "for" {
                let binding = args.first().ok_or_else(|| "for requires a loop variable".to_string())?;
                let variable = binding.name.clone();
                let values = binding.value.as_ref()
                    .ok_or_else(|| "for requires an iterable expression".to_string())
                    .and_then(|expr| self.eval_expr(expr))?;
                let values = match values {
                    Value::Vector(values) => values,
                    value => vec![value],
                };
                for value in values {
                    let loop_env = Environment::with_parent(self.env.clone());
                    loop_env.define(variable.clone(), value);
                    let mut loop_exec = Executor::new(Rc::new(loop_env));
                    for child in children {
                        loop_exec.execute_with_forwarded_children(child, forwarded)?;
                    }
                    if let Some(node) = loop_exec.csg_stack.into_iter().next().and_then(|builder| builder.build()) {
                        self.current_builder().union(node);
                    }
                }
                return Ok(None);
            }
        }
        self.execute(stmt)
    }

    fn contains_children_call(statements: &[Statement]) -> bool {
        statements.iter().any(|stmt| match stmt {
            Statement::ModuleInstantiation { name, children, .. } =>
                name == "children" || Self::contains_children_call(children),
            Statement::Block { statements, .. } => Self::contains_children_call(statements),
            Statement::If { then_branch, else_branch, .. } =>
                Self::contains_children_call(std::slice::from_ref(then_branch))
                    || else_branch.as_deref().is_some_and(|branch| {
                        Self::contains_children_call(std::slice::from_ref(branch))
                    }),
            _ => false,
        })
    }
    
    pub fn execute(&mut self, stmt: &Statement) -> Result<Option<Value>, String> {
        match stmt {
            Statement::Assignment(assign) => self.exec_assignment(assign).map(|_| None),
            Statement::ModuleInstantiation { name, args, children, modifiers, .. } => {
                self.exec_module_inst(name, args, children, modifiers).map(|_| None)
            }
            Statement::ModuleDef { name, params, body, .. } => {
                self.exec_module_def(name, params, body, &ModuleModifiers::default()).map(|_| None)
            }
            Statement::FunctionDef { name, params, body, .. } => {
                self.exec_function_def(name, params, body).map(|_| None)
            }
            Statement::If { condition, then_branch, else_branch, .. } => {
                self.exec_if(condition, then_branch, else_branch.as_deref())
            }
            Statement::Block { statements, .. } => {
                for stmt in statements {
                    self.execute(stmt)?;
                }
                Ok(None)
            }
            Statement::Include(path, _) => self.exec_source_directive(path, true).map(|_| None),
            Statement::Use(path, _) => self.exec_source_directive(path, false).map(|_| None),
        }
    }

    fn exec_source_directive(&mut self, path: &str, include_body: bool) -> Result<(), String> {
        let resolved = self.resolve_source_path(path)?;
        let source = crate::parser::Parser::parse_file(&resolved)
            .map_err(|error| format!("Failed to load '{}': {}", resolved.display(), error))?;

        if include_body {
            self.execute_source(&source)
        } else {
            for statement in &source.statements {
                if matches!(statement, Statement::FunctionDef { .. } | Statement::ModuleDef { .. }) {
                    self.execute(statement)?;
                }
            }
            Ok(())
        }
    }

    fn resolve_source_path(&self, path: &str) -> Result<PathBuf, String> {
        let requested = Path::new(path);
        let candidate = if requested.is_absolute() {
            requested.to_path_buf()
        } else if let Some(parent) = self.source_stack.last().and_then(|path| path.parent()) {
            parent.join(requested)
        } else {
            requested.to_path_buf()
        };
        if candidate.is_file() {
            Ok(Self::canonical_path(&candidate)?)
        } else {
            Err(format!("Source file '{}' not found", candidate.display()))
        }
    }

    fn canonical_path(path: &Path) -> Result<PathBuf, String> {
        path.canonicalize()
            .map_err(|error| format!("Cannot resolve source file '{}': {}", path.display(), error))
    }
    
    fn exec_assignment(&mut self, assign: &Assignment) -> Result<(), String> {
        let value = if let Some(expr) = &assign.value {
            self.eval_expr(expr)?
        } else {
            Value::Undef
        };
        self.env.define(assign.name.clone(), value);
        Ok(())
    }
    
    fn get_arg(&mut self, arguments: &[Assignment], name: &str) -> Result<Option<f64>, String> {
        if let Some(argument) = arguments.iter().find(|a| {
            a.name == name || a.name.strip_prefix('$') == Some(name)
        }) {
            if let Some(expr) = &argument.value {
                    let v = self.eval_expr(expr)?;
                    let num = v.as_number()?;
                    return Ok(Some(num));
            } else {
                return Ok(None);
            }

        }
        let value = self.env.get(name)
            .or_else(|| self.env.get(&format!("${name}")));
        match value {
            Some(value) => Ok(Some(value.as_number()?)),
            None => Ok(None),
        }
    }

    fn get_positional_arg(&mut self, arguments: &[Assignment], index: usize) -> Result<Option<f64>, String> {
        let Some(argument) = arguments.iter().filter(|argument| argument.name.is_empty()).nth(index) else {
            return Ok(None);
        };
        argument.value.as_ref()
            .map(|expression| self.eval_expr(expression).and_then(|value| value.as_number()).map(Some))
            .transpose()
            .map(|value| value.flatten())
    }

    fn get_string(&mut self, arguments: &[Assignment], name: &str) -> Result<Option<String>, String> {
        let Some(argument) = arguments.iter().find(|argument| {
            argument.name == name || argument.name.strip_prefix('$') == Some(name)
        }).or_else(|| {
            if matches!(name, "file" | "text") {
                arguments.iter().find(|argument| argument.name.is_empty())
            } else {
                None
            }
        }) else {
            return Ok(None);
        };
        let Some(expression) = &argument.value else {
            return Ok(None);
        };
        match self.eval_expr(expression)? {
            Value::String(value) => Ok(Some(value)),
            Value::Vector(values) if values.len() == 1 => match &values[0] {
                Value::String(value) => Ok(Some(value.clone())),
                _ => Err(format!("{name} must be a string")),
            },
            _ => Err(format!("{name} must be a string")),
        }
    }
    
    fn get_bool(&mut self, arguments: &[Assignment], name: &str, default: bool) -> Result<bool, String> {
        match arguments.iter().find(|a| {
            a.name == name || a.name.strip_prefix('$') == Some(name)
        }) {
            Some(a) => {
                if let Some(expr) = &a.value {
                    let v = self.eval_expr(expr)?;
                    Ok(v.as_bool())
                } else {
                    Ok(default)
                }
            }
            None => Ok(default),
        }
    }

    fn get_vector3(&mut self, arguments: &[Assignment]) -> Result<Option<[f64; 3]>, String> {
        let Some(argument) = arguments.iter().find(|argument| argument.name.is_empty()) else {
            return Ok(None);
        };
        let Some(expression) = &argument.value else {
            return Ok(None);
        };
        let value = self.eval_expr(expression)?;
        let Value::Vector(values) = value else {
            return Err("transformation vector must be a vector".into());
        };
        if values.len() == 2 {
            return Ok(Some([
                values[0].as_number()?,
                values[1].as_number()?,
                0.0,
            ]));
        }
        if values.len() != 3 {
            return Err("transformation vector must contain two or three values".into());
        }

        Ok(Some([
            values[0].as_number()?,
            values[1].as_number()?,
            values[2].as_number()?,
        ]))
    }

    fn get_vector(&mut self, arguments: &[Assignment], name: &str) -> Result<Option<Value>, String> {
        let argument = arguments.iter().find(|argument| {
            argument.name == name || argument.name.strip_prefix('$') == Some(name)
        }).or_else(|| {
            if matches!(name, "size" | "points" | "faces" | "color" | "matrix") {
                arguments.iter().find(|argument| argument.name.is_empty())
            } else {
                None
            }
        });
        let Some(argument) = argument else {
            return Ok(None);
        };
        argument.value.as_ref()
            .map(|expression| self.eval_expr(expression))
            .transpose()
    }

    fn get_color(&mut self, arguments: &[Assignment]) -> Result<[f64; 4], String> {
        if let Some(value) = self.get_vector(arguments, "color")? {
            if let Value::String(name) = value {
                let color = match name.to_ascii_lowercase().as_str() {
                    "black" => [0.0, 0.0, 0.0, 1.0],
                    "blue" => [0.0, 0.0, 1.0, 1.0],
                    "cyan" => [0.0, 1.0, 1.0, 1.0],
                    "gray" | "grey" => [0.5, 0.5, 0.5, 1.0],
                    "green" => [0.0, 1.0, 0.0, 1.0],
                    "lime" => [0.0, 1.0, 0.0, 1.0],
                    "magenta" => [1.0, 0.0, 1.0, 1.0],
                    "orange" => [1.0, 0.5, 0.0, 1.0],
                    "pink" => [1.0, 0.75, 0.8, 1.0],
                    "purple" => [0.5, 0.0, 0.5, 1.0],
                    "red" => [1.0, 0.0, 0.0, 1.0],
                    "white" => [1.0, 1.0, 1.0, 1.0],
                    "yellow" => [1.0, 1.0, 0.0, 1.0],
                    _ => return Err(format!("unknown color name '{name}'")),
                };
                return Ok(color);
            }
            let Value::Vector(values) = value else {
                return Err("color must be a three- or four-element vector".into());
            };
            if values.len() != 3 && values.len() != 4 {
                return Err("color must be a three- or four-element vector".into());
            }
            let mut color = [1.0; 4];
            for (index, value) in values.into_iter().enumerate() {
                color[index] = value.as_number()?;
            }
            return Ok(color);
        }
        Ok([
            self.get_arg(arguments, "r")?.unwrap_or(1.0),
            self.get_arg(arguments, "g")?.unwrap_or(1.0),
            self.get_arg(arguments, "b")?.unwrap_or(1.0),
            self.get_arg(arguments, "a")?.unwrap_or(1.0),
        ])
    }

    fn validate_tessellation(
        fn_: Option<f64>,
        fa: Option<f64>,
        fs: Option<f64>,
    ) -> Result<(Option<usize>, Option<f64>, Option<f64>), String> {
        let fn_ = fn_.filter(|&v| v > 0.0).map(|value| {
            if value < 3.0 || value.fract() != 0.0 {
                Err("$fn must be an integer greater than or equal to 3".to_string())
            } else {
                Ok(value as usize)
            }
        }).transpose()?;
        for (name, value) in [("$fa", fa), ("$fs", fs)] {
            if let Some(value) = value {
                if value <= 0.0 || !value.is_finite() {
                    return Err(format!("{name} must be positive"));
                }
            }
        }
        Ok((fn_, fa, fs))
    }

    fn get_matrix4(&mut self, arguments: &[Assignment]) -> Result<nalgebra::Matrix4<f64>, String> {
        let value = self.get_vector(arguments, "matrix")?
            .ok_or_else(|| "multmatrix requires a 4x4 matrix".to_string())?;
        let Value::Vector(rows) = value else {
            return Err("multmatrix requires a 4x4 matrix".into());
        };
        if rows.len() != 4 {
            return Err("multmatrix requires four matrix rows".into());
        }
        let mut values = Vec::with_capacity(16);
        for row in rows {
            let Value::Vector(row) = row else {
                return Err("multmatrix requires four matrix rows".into());
            };
            if row.len() != 4 {
                return Err("multmatrix requires four columns per row".into());
            }
            for value in row {
                values.push(value.as_number()?);
            }
        }
        if values.iter().any(|value| !value.is_finite()) {
            return Err("multmatrix values must be finite".into());
        }
        Ok(nalgebra::Matrix4::from_row_slice(&values))
    }

    fn value_points(value: Value) -> Result<Vec<[f64; 3]>, String> {
        let Value::Vector(points) = value else {
            return Err("polyhedron points must be a vector".into());
        };
        points.into_iter().map(|point| {
            let Value::Vector(values) = point else {
                return Err("each polyhedron point must be a vector".into());
            };
            if values.len() != 3 {
                return Err("each polyhedron point must contain three values".into());
            }
            Ok([values[0].as_number()?, values[1].as_number()?, values[2].as_number()?])
        }).collect()
    }

    fn value_faces(value: Value, point_count: usize) -> Result<Vec<Vec<usize>>, String> {
        let Value::Vector(faces) = value else {
            return Err("polyhedron faces must be a vector".into());
        };
        faces.into_iter().map(|face| {
            let Value::Vector(indices) = face else {
                return Err("each polyhedron face must be a vector".into());
            };
            let indices = indices.into_iter().map(|index| {
                let index = index.as_number()?;
                if index < 0.0 || index.fract() != 0.0 || index as usize >= point_count {
                    return Err("polyhedron face index is out of bounds".into());
                }
                Ok(index as usize)
            }).collect::<Result<Vec<_>, String>>()?;
            if indices.len() < 3 {
                return Err("polyhedron faces need at least three points".into());
            }
            Ok(indices)
        }).collect()
    }

    fn value_points2(value: Value) -> Result<Vec<[f64; 2]>, String> {
        let Value::Vector(points) = value else {
            return Err("polygon points must be a vector".into());
        };
        points.into_iter().map(|point| {
            let Value::Vector(values) = point else {
                return Err("each polygon point must be a vector".into());
            };
            if values.len() != 2 {
                return Err("each polygon point must contain two values".into());
            }
            Ok([values[0].as_number()?, values[1].as_number()?])
        }).collect()
    }

    fn value_paths(value: Value, point_count: usize) -> Result<Vec<Vec<usize>>, String> {
        let Value::Vector(paths) = value else {
            return Err("polygon paths must be a vector".into());
        };
        paths.into_iter().map(|path| {
            let Value::Vector(indices) = path else {
                return Err("each polygon path must be a vector".into());
            };
            let indices = indices.into_iter().map(|index| {
                let index = index.as_number()?;
                if index < 0.0 || index.fract() != 0.0 || index as usize >= point_count {
                    return Err("polygon path index is out of bounds".into());
                }
                Ok(index as usize)
            }).collect::<Result<Vec<_>, String>>()?;
            if indices.len() < 3 {
                return Err("polygon paths need at least three points".into());
            }
            if indices.windows(2).any(|window| window[0] == window[1])
                || indices.first() == indices.last()
            {
                return Err("polygon paths cannot contain duplicate adjacent points".into());
            }
            Ok(indices)
        }).collect()
    }

    fn validate_polygon_paths(points: &[[f64; 2]], paths: &[Vec<usize>]) -> Result<(), String> {
        for path in paths {
            let area: f64 = path.iter().enumerate().map(|(index, &current)| {
                let next = path[(index + 1) % path.len()];
                points[current][0] * points[next][1] - points[next][0] * points[current][1]
            }).sum::<f64>() / 2.0;
            if area.abs() <= f64::EPSILON {
                return Err("polygon paths must enclose a non-zero area".into());
            }
            for first in 0..path.len() {
                let first_next = (first + 1) % path.len();
                for second in (first + 1)..path.len() {
                    let second_next = (second + 1) % path.len();
                    if first == second
                        || first_next == second
                        || second_next == first
                    {
                        continue;
                    }
                    if Self::segments_intersect(
                        points[path[first]],
                        points[path[first_next]],
                        points[path[second]],
                        points[path[second_next]],
                    ) {
                        return Err("polygon paths cannot self-intersect".into());
                    }
                }
            }
        }
        if let Some(outer) = paths.first() {
            for hole in paths.iter().skip(1) {
                if !Self::point_in_polygon(points[hole[0]], points, outer) {
                    return Err("polygon holes must be inside the outer path".into());
                }
            }
        }
        Ok(())
    }

    fn point_in_polygon(point: [f64; 2], points: &[[f64; 2]], path: &[usize]) -> bool {
        let mut inside = false;
        for index in 0..path.len() {
            let a = points[path[index]];
            let b = points[path[(index + 1) % path.len()]];
            if (a[1] > point[1]) != (b[1] > point[1])
                && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
            {
                inside = !inside;
            }
        }
        inside
    }

    fn segments_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
        fn orientation(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
            (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
        }
        let ab_c = orientation(a, b, c);
        let ab_d = orientation(a, b, d);
        let cd_a = orientation(c, d, a);
        let cd_b = orientation(c, d, b);
        (ab_c > 0.0 && ab_d < 0.0 || ab_c < 0.0 && ab_d > 0.0)
            && (cd_a > 0.0 && cd_b < 0.0 || cd_a < 0.0 && cd_b > 0.0)
    }
    
    fn exec_module_inst(
        &mut self,
        name: &str,
        arguments: &[Assignment],
        children: &[Statement],
        _modifiers: &ModuleModifiers,
    ) -> Result<(), String> {
        if name == "for" {
            let binding = arguments.first()
                .ok_or_else(|| "for requires a loop variable".to_string())?;
            let variable = if binding.name.is_empty() {
                return Err("for requires a named loop variable".into());
            } else {
                binding.name.clone()
            };
            let values = binding.value.as_ref()
                .ok_or_else(|| "for requires an iterable expression".to_string())
                .and_then(|expr| self.eval_expr(expr))?;
            let values = match values {
                Value::Vector(values) => values,
                value => vec![value],
            };
            for value in values {
                let loop_env = Environment::with_parent(self.env.clone());
                loop_env.define(variable.clone(), value);
                let mut loop_exec = Executor::new(Rc::new(loop_env));
                for child in children {
                    loop_exec.execute(child)?;
                }
                if let Some(node) = loop_exec.csg_stack.into_iter().next().and_then(|builder| builder.build()) {
                    self.current_builder().union(node);
                }
            }
            return Ok(());
        }
        if name == "children" {
            for child in children {
                self.execute(child)?;
            }
            return Ok(());
        }
        // Handle built-in modules
        if let Some(()) = self.exec_builtin_module(name, arguments, children)? {
            return Ok(());
        }
        
        // Handle user-defined modules
        let module_val = self.env.get(name)
            .ok_or_else(|| format!("Undefined module '{}'", name))?;
        
        match module_val {
            Value::Module(module) => {
                let new_env = Environment::with_parent(self.env.clone());
                let mut positional = arguments.iter().filter(|argument| argument.name.is_empty());
                
                for (param_name, default) in &module.params {
                    let argument = arguments.iter()
                        .find(|argument| argument.name == *param_name)
                        .or_else(|| positional.next());
                    let value = if let Some(arg) = argument {
                        if let Some(expr) = &arg.value {
                            self.eval_expr(expr)?
                        } else {
                            Value::Undef
                        }
                    } else if let Some(default_expr) = default {
                        self.eval_expr(default_expr)?
                    } else {
                        return Err(format!("Missing argument for parameter '{}'", param_name));
                    };
                    new_env.define(param_name.clone(), value);
                }
                
                let mut exec = Executor::new(Rc::new(new_env));
                for stmt in &module.body {
                    exec.execute_with_forwarded_children(stmt, children)?;
                }
                
                if !Self::contains_children_call(&module.body) {
                    for child in children {
                        exec.execute(child)?;
                    }
                }
                if let Some(node) = exec.csg_stack.into_iter().next().and_then(|builder| builder.build()) {
                    self.current_builder().union(node);
                }
                Ok(())
            }
            Value::Function(_) => Err(format!("'{}' is a function, not a module", name)),
            _ => Err(format!("'{}' is not a module", name)),
        }
    }
    
    fn exec_builtin_module(
        &mut self,
        name: &str,
        arguments: &[Assignment],
        children: &[Statement],
    ) -> Result<Option<()>, String> {
        // Collect all arguments first to avoid borrow conflicts
        // let size_opt = self.get_arg(arguments, "size")?;
        // let x_opt = self.get_arg(arguments, "x")?;
        // let y_opt = self.get_arg(arguments, "y")?;
        // let z_opt = self.get_arg(arguments, "z")?;
        // let center_opt = self.get_arg(arguments, "center")?;
        // let r_opt = self.get_arg(arguments, "r")?;
        // let d_opt = self.get_arg(arguments, "d")?;
        // let h_opt = self.get_arg(arguments, "h")?;
        // let r1_opt = self.get_arg(arguments, "r1")?;
        // let r2_opt = self.get_arg(arguments, "r2")?;
        // let d1_opt = self.get_arg(arguments, "d1")?;
        // let d2_opt = self.get_arg(arguments, "d2")?;
        // let fn_opt = self.get_arg(arguments, "fn")?;
        // let fa_opt = self.get_arg(arguments, "fa")?;
        // let fs_opt = self.get_arg(arguments, "fs")?;
        // let center_bool = self.get_bool(arguments, "center", false)?;
        
        // Get translation/rotation/scale/mirror args
        // let trans_v = [
        //     self.get_arg(arguments, "x")?.unwrap_or(0.0),
        //     self.get_arg(arguments, "y")?.unwrap_or(0.0),
        //     self.get_arg(arguments, "z")?.unwrap_or(0.0),
        // ];
        
        // let center_bool = self.get_bool(arguments, "center", false)?;
        
        match name {
            "assert" => {
                let condition = arguments.first()
                    .and_then(|argument| argument.value.as_ref())
                    .map(|expression| self.eval_expr(expression))
                    .transpose()?
                    .map(|value| value.as_bool())
                    .unwrap_or(false);
                if !condition {
                    return Err("assertion failed".into());
                }
                Ok(Some(()))
            }
            "echo" => {
                let mut output = Vec::new();
                for argument in arguments {
                    if let Some(expression) = &argument.value {
                        let value = self.eval_expr(expression)?;
                        if argument.name.is_empty() {
                            output.push(value.to_string());
                        } else {
                            output.push(format!("{} = {}", argument.name, value));
                        }
                    }
                }
                println!("ECHO: {}", output.join(", "));
                Ok(Some(()))
            }
            "text" => {
                let text = self.get_string(arguments, "text")?
                    .ok_or_else(|| "text requires a string".to_string())?;
                let size = self.get_arg(arguments, "size")?.unwrap_or(10.0);
                if size <= 0.0 {
                    return Err("text size must be positive".into());
                }
                let halign = self.get_string(arguments, "halign")?
                    .unwrap_or_else(|| "left".to_string());
                let valign = self.get_string(arguments, "valign")?
                    .unwrap_or_else(|| "baseline".to_string());
                if !matches!(halign.as_str(), "left" | "center" | "right") {
                    return Err("text halign must be left, center, or right".into());
                }
                if !matches!(valign.as_str(), "baseline" | "bottom" | "center" | "top") {
                    return Err("text valign must be baseline, bottom, center, or top".into());
                }
                self.current_builder().add_primitive(Primitive::Text {
                    text,
                    size,
                    font: String::new(),
                    halign,
                    valign,
                });
                Ok(Some(()))
            }
            // Primitive shapes - add to current builder
            "cube" => {
                let size = if let Some(value) = self.get_vector(arguments, "size")? {
                    match value {
                        Value::Number(s) => [s; 3],
                        Value::Vector(values) if values.len() == 3 => [
                            values[0].as_number()?,
                            values[1].as_number()?,
                            values[2].as_number()?,
                        ],
                        _ => return Err("cube size must be a number or a three-element vector".into()),
                    }
                } else {
                    [
                        self.get_arg(arguments, "x")?.unwrap_or(1.0),
                        self.get_arg(arguments, "y")?.unwrap_or(1.0),
                        self.get_arg(arguments, "z")?.unwrap_or(1.0),
                    ]
                };
                if size.iter().any(|value| *value <= 0.0 || !value.is_finite()) {
                    return Err("cube dimensions must be positive and finite".into());
                }
                let center = self.get_bool(arguments, "center", false)?;
                self.current_builder().add_primitive(Primitive::Cube { size, center });
                Ok(Some(()))
            }
            "sphere" => {
                let r = {
                    let r_val = self.get_arg(arguments, "r")?.or(self.get_positional_arg(arguments, 0)?);
                    let d_val = self.get_arg(arguments, "d")?;
                    r_val.or_else(|| d_val.map(|d| d / 2.0)).unwrap_or(1.0)
                };
                if r <= 0.0 || !r.is_finite() {
                    return Err("sphere radius must be positive and finite".into());
                }
                let (fn_, fa, fs) = Self::validate_tessellation(
                    self.get_arg(arguments, "fn")?,
                    self.get_arg(arguments, "fa")?,
                    self.get_arg(arguments, "fs")?,
                )?;
                self.current_builder().add_primitive(Primitive::Sphere { r, d: None, fn_, fa, fs });
                Ok(Some(()))
            }
            "cylinder" => {
                let h = self.get_arg(arguments, "h")?.or(self.get_positional_arg(arguments, 0)?).unwrap_or(1.0);
                let r = self.get_arg(arguments, "r")?.or(self.get_positional_arg(arguments, 1)?);
                let d = self.get_arg(arguments, "d")?.map(|value| value / 2.0);
                let r1 = self.get_arg(arguments, "r1")?.or(r).or(d).unwrap_or(1.0);
                let r2 = self.get_arg(arguments, "r2")?.or(r).or(d).unwrap_or(r1);
                if h <= 0.0 || r1 < 0.0 || r2 < 0.0 || (r1 == 0.0 && r2 == 0.0) {
                    return Err("cylinder requires positive height and radius".into());
                }
                let (fn_, fa, fs) = Self::validate_tessellation(
                    self.get_arg(arguments, "fn")?,
                    self.get_arg(arguments, "fa")?,
                    self.get_arg(arguments, "fs")?,
                )?;
                let center = self.get_bool(arguments, "center", false)?;
                self.current_builder().add_primitive(Primitive::Cylinder {
                    h, r1, r2, d1: None, d2: None, center, fn_, fa, fs,
                });
                Ok(Some(()))
            }
            "square" => {
                let size = if let Some(s) = self.get_arg(arguments, "size")? {
                    [s, s]
                } else {
                    [
                        self.get_arg(arguments, "x")?.unwrap_or(1.0),
                        self.get_arg(arguments, "y")?.unwrap_or(1.0),
                    ]
                };
                if size.iter().any(|value| *value <= 0.0 || !value.is_finite()) {
                    return Err("square dimensions must be positive and finite".into());
                }
                let center = self.get_bool(arguments, "center", false)?;
                self.current_builder().add_primitive(Primitive::Square { size, center });
                Ok(Some(()))
            }
            "circle" => {
                let r = {
                    let r_val = self.get_arg(arguments, "r")?.or(self.get_positional_arg(arguments, 0)?);
                    let d_val = self.get_arg(arguments, "d")?;
                    r_val.or_else(|| d_val.map(|d| d / 2.0)).unwrap_or(1.0)
                };
                if r <= 0.0 || !r.is_finite() {
                    return Err("circle radius must be positive and finite".into());
                }
                let (fn_, fa, fs) = Self::validate_tessellation(
                    self.get_arg(arguments, "fn")?,
                    self.get_arg(arguments, "fa")?,
                    self.get_arg(arguments, "fs")?,
                )?;
                self.current_builder().add_primitive(Primitive::Circle { r, d: None, fn_, fa, fs });
                Ok(Some(()))
            }
            "polygon" => {
                let points = self.get_vector(arguments, "points")?
                    .ok_or_else(|| "polygon requires points".to_string())
                    .and_then(Self::value_points2)?;
                let paths = if let Some(value) = self.get_vector(arguments, "paths")? {
                    Self::value_paths(value, points.len())?
                } else {
                    vec![(0..points.len()).collect()]
                };
                Self::validate_polygon_paths(&points, &paths)?;
                self.current_builder().add_primitive(Primitive::Polygon { points, paths });
                Ok(Some(()))
            }
            "linear_extrude" => {
                let height = self.get_arg(arguments, "height")?.or(self.get_positional_arg(arguments, 0)?).unwrap_or(1.0);
                if height <= 0.0 {
                    return Err("linear_extrude height must be positive".into());
                }
                let center = self.get_bool(arguments, "center", false)?;
                let twist = self.get_arg(arguments, "twist")?.unwrap_or(0.0);
                let scale = self.get_arg(arguments, "scale")?.unwrap_or(1.0);
                if !twist.is_finite() || !scale.is_finite() || scale < 0.0 {
                    return Err("linear_extrude twist must be finite and scale must be non-negative".into());
                }
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    self.current_builder().union(crate::csg::CsgNode::LinearExtrude {
                        child: Box::new(child_csg),
                        height,
                        center,
                        twist,
                        scale,
                    });
                }
                Ok(Some(()))
            }
            "rotate_extrude" => {
                let angle = self.get_arg(arguments, "angle")?
                    .or(self.get_positional_arg(arguments, 0)?)
                    .unwrap_or(360.0);
                if angle <= 0.0 || angle > 360.0 {
                    return Err("rotate_extrude angle must be greater than zero and at most 360".into());
                }
                let segments = self.get_arg(arguments, "fn")?
                    .map(|value| value as usize)
                    .unwrap_or(32)
                    .max(3);
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    self.current_builder().union(crate::csg::CsgNode::RotateExtrude {
                        child: Box::new(child_csg),
                        angle,
                        segments,
                    });
                }
                Ok(Some(()))
            }
            "polyhedron" => {
                let points = self.get_vector(arguments, "points")?
                    .ok_or_else(|| "polyhedron requires points".to_string())
                    .and_then(Self::value_points)?;
                let faces = self.get_vector(arguments, "faces")?
                    .ok_or_else(|| "polyhedron requires faces".to_string())
                    .and_then(|value| Self::value_faces(value, points.len()))?;
                self.current_builder().add_primitive(Primitive::Polyhedron { points, faces });
                Ok(Some(()))
            }
            "import" => {
                let file = self.get_string(arguments, "file")?
                    .ok_or_else(|| "import requires a file path".to_string())?;
                let resolved = self.resolve_source_path(&file)?;
                let data = std::fs::read(&resolved)
                    .map_err(|error| format!("Failed to read '{}': {}", resolved.display(), error))?;
                let imported = if resolved.extension().and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("obj"))
                {
                    crate::csg::Mesh::from_obj(&data)
                } else {
                    crate::csg::Mesh::from_stl(&data)
                }
                    .map_err(|error| format!("Failed to import '{}': {}", resolved.display(), error))?;
                let faces = imported.faces.iter().map(|face| face.to_vec()).collect();
                self.current_builder().add_primitive(Primitive::Polyhedron {
                    points: imported.vertices,
                    faces,
                });
                Ok(Some(()))
            }
            // Transformations - wrap children in new CSG context
            "translate" => {
                let v = self.get_vector3(arguments)?.unwrap_or([
                    self.get_arg(arguments, "x")?.unwrap_or(0.0),
                    self.get_arg(arguments, "y")?.unwrap_or(0.0),
                    self.get_arg(arguments, "z")?.unwrap_or(0.0),
                ]);
                // Execute children in new CSG context
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    let mut b = CsgBuilder::from_node(child_csg);
                    b.translate(v);
                    if let Some(node) = b.build() {
                        self.current_builder().union(node);
                    }
                }
                Ok(Some(()))
            }
            "rotate" => {
                let v = if let Some(argument) = arguments.iter().find(|argument| argument.name.is_empty()) {
                    match argument.value.as_ref().map(|expr| self.eval_expr(expr)).transpose()? {
                        Some(Value::Number(angle)) => [0.0, 0.0, angle],
                        Some(Value::Vector(_)) => self.get_vector3(arguments)?.unwrap(),
                        Some(_) => return Err("rotation must be a number or vector".into()),
                        None => [0.0, 0.0, 0.0],
                    }
                } else {
                    [
                        self.get_arg(arguments, "x")?.unwrap_or(0.0),
                        self.get_arg(arguments, "y")?.unwrap_or(0.0),
                        self.get_arg(arguments, "z")?.unwrap_or(0.0),
                    ]
                };
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    let mut b = CsgBuilder::from_node(child_csg);
                    b.rotate(v);
                    if let Some(node) = b.build() {
                        self.current_builder().union(node);
                    }
                }
                Ok(Some(()))
            }
            "scale" => {
                let positional = arguments.iter()
                    .filter(|argument| argument.name.is_empty())
                    .next()
                    .and_then(|argument| argument.value.as_ref());
                let v = if let Some(expression) = positional {
                    match self.eval_expr(expression)? {
                        Value::Number(uniform) => [uniform; 3],
                        Value::Vector(values) if values.len() == 3 => [
                            values[0].as_number()?,
                            values[1].as_number()?,
                            values[2].as_number()?,
                        ],
                        Value::Vector(values) if values.len() == 2 => [
                            values[0].as_number()?,
                            values[1].as_number()?,
                            1.0,
                        ],
                        _ => return Err("scale requires a scalar or vector".into()),
                    }
                } else {
                    [
                        self.get_arg(arguments, "x")?.unwrap_or(1.0),
                        self.get_arg(arguments, "y")?.unwrap_or(1.0),
                        self.get_arg(arguments, "z")?.unwrap_or(1.0),
                    ]
                };
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    let mut b = CsgBuilder::from_node(child_csg);
                    b.scale(v);
                    if let Some(node) = b.build() {
                        self.current_builder().union(node);
                    }
                }
                Ok(Some(()))
            }
            "mirror" => {
                let v = self.get_vector3(arguments)?.unwrap_or([
                    self.get_arg(arguments, "x")?.unwrap_or(0.0),
                    self.get_arg(arguments, "y")?.unwrap_or(0.0),
                    self.get_arg(arguments, "z")?.unwrap_or(0.0),
                ]);
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    let mut b = CsgBuilder::from_node(child_csg);
                    b.mirror(v);
                    if let Some(node) = b.build() {
                        self.current_builder().union(node);
                    }
                }
                Ok(Some(()))
            }
            "multmatrix" => {
                let matrix = self.get_matrix4(arguments)?;
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    let mut builder = CsgBuilder::from_node(child_csg);
                    builder.matrix(matrix);
                    if let Some(node) = builder.build() {
                        self.current_builder().union(node);
                    }
                }
                Ok(Some(()))
            }
            "color" => {
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    let color = self.get_color(arguments)?;
                    self.current_builder().union(crate::csg::CsgNode::Color {
                        child: Box::new(child_csg),
                        color,
                    });
                }
                Ok(Some(()))
            }
            "render" => {
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    self.current_builder().union(child_csg);
                }
                Ok(Some(()))
            }
            "offset" | "projection" => {
                // Execute children and pass through geometry
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    self.current_builder().union(child_csg);
                }
                Ok(Some(()))
            }
            // Boolean operations
            "union" => {
                let mut nodes = self.child_nodes(children)?.into_iter();
                if let Some(first) = nodes.next() {
                    self.current_builder().union(first);
                    for node in nodes {
                        self.current_builder().union(node);
                    }
                }
                Ok(Some(()))
            }
            "difference" => {
                let mut nodes = self.child_nodes(children)?.into_iter();
                if let Some(first) = nodes.next() {
                    let mut base = CsgBuilder::from_node(first);
                    for node in nodes {
                        base.difference(node);
                    }
                    if let Some(result) = base.build() {
                        self.current_builder().union(result);
                    }
                }
                Ok(Some(()))
            }
            "intersection" => {
                let mut nodes = self.child_nodes(children)?.into_iter();
                if let Some(first) = nodes.next() {
                    let mut base = CsgBuilder::from_node(first);
                    for node in nodes {
                        base.intersection(node);
                    }
                    if let Some(result) = base.build() {
                        self.current_builder().union(result);
                    }
                }
                Ok(Some(()))
            }
            "hull" => {
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    self.current_builder().hull(vec![child_csg]);
                }
                Ok(Some(()))
            }
            "minkowski" => {
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    self.current_builder().minkowski(vec![child_csg]);
                }
                Ok(Some(()))
            }
            _ => Ok(None),
        }
    }
    
    fn exec_module_def(
        &mut self,
        name: &str,
        params: &[Assignment],
        body: &[Statement],
        _modifiers: &ModuleModifiers,
    ) -> Result<(), String> {
        let params: Vec<(String, Option<Expression>)> = params.iter()
            .map(|a| (a.name.clone(), a.value.clone()))
            .collect();
        
        let module = ModuleValue {
            name: name.to_string(),
            params,
            body: body.to_vec(),
            captures: self.env.clone(),
            modifiers: ModuleModifiers::default(),
        };
        
        self.env.define(name.to_string(), Value::Module(module));
        Ok(())
    }
    
    fn exec_function_def(
        &mut self,
        name: &str,
        params: &[Assignment],
        body: &Expression,
    ) -> Result<(), String> {
        let params: Vec<(String, Option<Expression>)> = params.iter()
            .map(|a| (a.name.clone(), a.value.clone()))
            .collect();
        
        let func = crate::interpreter::value::FunctionValue {
            name: name.to_string(),
            params,
            body: body.clone(),
            captures: self.env.clone(),
        };
        
        self.env.define(name.to_string(), Value::Function(func));
        Ok(())
    }
    
    fn exec_if(
        &mut self,
        condition: &Expression,
        then_branch: &Statement,
        else_branch: Option<&Statement>,
    ) -> Result<Option<Value>, String> {
        let cond_val = self.eval_expr(condition)?;
        if cond_val.as_bool() {
            self.execute(then_branch)
        } else if let Some(else_stmt) = else_branch {
            self.execute(else_stmt)
        } else {
            Ok(None)
        }
    }
    
    fn eval_expr(&mut self, expr: &Expression) -> Result<Value, String> {
        let mut evaluator = Evaluator::new(self.env.clone());
        evaluator.eval(expr)
    }
}