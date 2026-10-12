use std::rc::Rc;
use crate::ast::{Statement, Assignment, Expression, ModuleModifiers};
use crate::interpreter::value::{Value, Environment, ModuleValue};
use crate::interpreter::eval::Evaluator;
use crate::csg::{Primitive, CsgBuilder};
use anyhow::Result;

pub struct Executor {
    pub env: Rc<Environment>,
    csg_stack: Vec<crate::csg::CsgBuilder>,
}

impl Executor {
    pub fn new(env: Rc<Environment>) -> Self {
        Self {
            env,
            csg_stack: vec![crate::csg::CsgBuilder::new()],
        }
    }
    
    fn current_builder(&mut self) -> &mut crate::csg::CsgBuilder {
        self.csg_stack.last_mut().unwrap()
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
            Statement::Include(_, _) => Ok(None),
            Statement::Use(_, _) => Ok(None),
        }
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
        match arguments.iter().find(|a| a.name == name) {
            Some(a) => {
                if let Some(expr) = &a.value {
                    let v = self.eval_expr(expr)?;
                    let num = v.as_number()?;
                    Ok(Some(num))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }
    
    fn get_bool(&mut self, arguments: &[Assignment], name: &str, default: bool) -> Result<bool, String> {
        match arguments.iter().find(|a| a.name == name) {
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
    
    fn exec_module_inst(
        &mut self,
        name: &str,
        arguments: &[Assignment],
        children: &[Statement],
        _modifiers: &ModuleModifiers,
    ) -> Result<(), String> {
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
                
                for (param_name, default) in &module.params {
                    let value = if let Some(arg) = arguments.iter().find(|a| a.name == *param_name) {
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
                    exec.execute(stmt)?;
                }
                
                for child in children {
                    exec.execute(child)?;
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
            // Primitive shapes - add to current builder
            "cube" => {
                let size = if let Some(s) = self.get_arg(arguments, "size")? {
                    [s; 3]
                } else {
                    [
                        self.get_arg(arguments, "x")?.unwrap_or(1.0),
                        self.get_arg(arguments, "y")?.unwrap_or(1.0),
                        self.get_arg(arguments, "z")?.unwrap_or(1.0),
                    ]
                };
                let center = self.get_bool(arguments, "center", false)?;
                self.current_builder().add_primitive(Primitive::Cube { size, center });
                Ok(Some(()))
            }
            "sphere" => {
                let r = {
                    let r_val = self.get_arg(arguments, "r")?;
                    let d_val = self.get_arg(arguments, "d")?;
                    r_val.or_else(|| d_val.map(|d| d / 2.0)).unwrap_or(1.0)
                };
                let fn_ = self.get_arg(arguments, "fn")?.map(|f| f as usize);
                let fa = self.get_arg(arguments, "fa")?;
                let fs = self.get_arg(arguments, "fs")?;
                self.current_builder().add_primitive(Primitive::Sphere { r, d: None, fn_, fa, fs });
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
                let center = self.get_bool(arguments, "center", false)?;
                self.current_builder().add_primitive(Primitive::Square { size, center });
                Ok(Some(()))
            }
            "circle" => {
                let r = {
                    let r_val = self.get_arg(arguments, "r")?;
                    let d_val = self.get_arg(arguments, "d")?;
                    r_val.or_else(|| d_val.map(|d| d / 2.0)).unwrap_or(1.0)
                };
                let fn_ = self.get_arg(arguments, "fn")?.map(|f| f as usize);
                let fa = self.get_arg(arguments, "fa")?;
                let fs = self.get_arg(arguments, "fs")?;
                self.current_builder().add_primitive(Primitive::Circle { r, d: None, fn_, fa, fs });
                Ok(Some(()))
            }
            "polyhedron" => {
                // For now, just add a placeholder mesh
                self.current_builder().add_primitive(Primitive::Polyhedron { points: vec![], faces: vec![] });
                Ok(Some(()))
            }
            // Transformations - wrap children in new CSG context
            "translate" => {
                let v = [
                    self.get_arg(arguments, "x")?.unwrap_or(0.0),
                    self.get_arg(arguments, "y")?.unwrap_or(0.0),
                    self.get_arg(arguments, "z")?.unwrap_or(0.0),
                ];
                // Execute children in new CSG context
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    let mut b = CsgBuilder::new();
                    b.add_primitive(Primitive::Import { file: "".to_string(), convexity: 10 });
                    b.translate(v);
                    b.union(child_csg);
                    *self.current_builder() = b;
                }
                Ok(Some(()))
            }
            "rotate" => {
                let v = [
                    self.get_arg(arguments, "x")?.unwrap_or(0.0),
                    self.get_arg(arguments, "y")?.unwrap_or(0.0),
                    self.get_arg(arguments, "z")?.unwrap_or(0.0),
                ];
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    let mut b = CsgBuilder::new();
                    b.add_primitive(Primitive::Import { file: "".to_string(), convexity: 10 });
                    b.rotate(v);
                    b.union(child_csg);
                    *self.current_builder() = b;
                }
                Ok(Some(()))
            }
            "scale" => {
                let v = [
                    self.get_arg(arguments, "x")?.unwrap_or(1.0),
                    self.get_arg(arguments, "y")?.unwrap_or(1.0),
                    self.get_arg(arguments, "z")?.unwrap_or(1.0),
                ];
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    let mut b = CsgBuilder::new();
                    b.add_primitive(Primitive::Import { file: "".to_string(), convexity: 10 });
                    b.scale(v);
                    b.union(child_csg);
                    *self.current_builder() = b;
                }
                Ok(Some(()))
            }
            "mirror" => {
                let v = [
                    self.get_arg(arguments, "x")?.unwrap_or(0.0),
                    self.get_arg(arguments, "y")?.unwrap_or(0.0),
                    self.get_arg(arguments, "z")?.unwrap_or(0.0),
                ];
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    let mut b = CsgBuilder::new();
                    b.add_primitive(Primitive::Import { file: "".to_string(), convexity: 10 });
                    b.mirror(v);
                    b.union(child_csg);
                    *self.current_builder() = b;
                }
                Ok(Some(()))
            }
            "multmatrix" => {
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                Ok(Some(()))
            }
            "color" => {
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                Ok(Some(()))
            }
            // Boolean operations
            "union" => {
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    self.current_builder().union(child_csg);
                }
                Ok(Some(()))
            }
            "difference" => {
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    self.current_builder().difference(child_csg);
                }
                Ok(Some(()))
            }
            "intersection" => {
                self.csg_stack.push(CsgBuilder::new());
                for child in children {
                    self.execute(child)?;
                }
                if let Some(child_csg) = self.csg_stack.pop().unwrap().build() {
                    self.current_builder().intersection(child_csg);
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
            captures: (*self.env).clone(),
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
            captures: (*self.env).clone(),
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