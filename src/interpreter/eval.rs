use std::rc::Rc;
use crate::ast::{Expression, BinaryOp, UnaryOp, Assignment, LiteralValue};
use crate::interpreter::value::{Value, Environment};
use crate::interpreter::builtins::Builtins;

pub struct Evaluator {
    pub env: Rc<Environment>,
    builtins: Builtins,
    pub depth: usize,
}

impl Evaluator {
    pub fn new(env: Rc<Environment>) -> Self {
        Self {
            env,
            builtins: Builtins::new(),
            depth: 0,
        }
    }

    pub fn with_depth(env: Rc<Environment>, depth: usize) -> Self {
        Self {
            env,
            builtins: Builtins::new(),
            depth,
        }
    }

    pub fn eval(&mut self, expr: &Expression) -> Result<Value, String> {
        // eprintln!("DEBUG eval: expr = {:?}", expr);
        match expr {
            Expression::Literal(lit, _span) => self.eval_literal(lit),
            Expression::Lookup(name, _span) => self.eval_identifier(name),
            Expression::UnaryOp(op, expr, _span) => self.eval_unary(op, expr),
            Expression::BinaryOp(op, left, right, _span) => self.eval_binary(op, left, right),
            Expression::TernaryOp(cond, then_e, else_e, _span) => self.eval_ternary(cond, then_e, else_e),
            Expression::If { condition, then_branch, else_branch, .. } => {
                let cond_val = self.eval(condition)?;
                if cond_val.as_bool() {
                    self.eval(then_branch)
                } else if let Some(else_branch) = else_branch {
                    self.eval(else_branch)
                } else {
                    Ok(Value::Undef)
                }
            }
            Expression::Vector(exprs, _span) => self.eval_vector(exprs),
            Expression::Range { start, step, end, .. } => self.eval_range(start, step.as_deref(), end),
            Expression::FunctionCall { name, args, expr, .. } => self.eval_function_call(name, args, expr.as_deref()),
            Expression::ArrayLookup(expr, index, _span) => self.eval_index(expr, index),
            Expression::MemberLookup(expr, field, _span) => self.eval_dot(expr, field),
            Expression::Let { args, body, .. } => self.eval_let(args, body),
            Expression::FunctionDef { params, body, .. } => self.eval_function_def(params, body),
            Expression::Assert { args, body, .. } => self.eval_assert(args, body.as_deref()),
            Expression::Echo { args, .. } => self.eval_echo(args),
            Expression::For { args, condition, body, .. } => self.eval_for(args, condition.as_deref(), body),
            Expression::ForC { init, condition, update, body, .. } => self.eval_for_c(init, condition.as_deref(), update, body),
            Expression::Each { body, .. } => self.eval_each(body),
        }
    }

    fn eval_literal(&self, lit: &LiteralValue) -> Result<Value, String> {
        Ok(match lit {
            LiteralValue::Bool(b) => Value::Bool(*b),
            LiteralValue::Number(n) => Value::Number(*n),
            LiteralValue::String(s) => Value::String(s.clone()),
            LiteralValue::Undefined => Value::Undef,
        })
    }

    fn eval_identifier(&self, name: &str) -> Result<Value, String> {
        Ok(self.env.get(name).unwrap_or(Value::Undef))
    }

    fn eval_unary(&mut self, op: &UnaryOp, expr: &Expression) -> Result<Value, String> {
        let val = self.eval(expr)?;
        if val.is_undef() {
            return Ok(Value::Undef);
        }
        Ok(match op {
            UnaryOp::Negate => match &val {
                Value::Vector(v) => Value::Vector(
                    v.iter()
                        .map(|x| Ok(Value::Number(-x.as_number()?)))
                        .collect::<Result<Vec<_>, String>>()?,
                ),
                _ => Value::Number(-val.as_number()?),
            },
            UnaryOp::Positive => match &val {
                Value::Vector(v) => Value::Vector(v.clone()),
                _ => Value::Number(val.as_number()?),
            },
            UnaryOp::Not => Value::Bool(!val.as_bool()),
            UnaryOp::BinaryNot => Value::Number(!(val.as_number()? as i64) as f64),
        })
    }

    fn eval_binary(&mut self, op: &BinaryOp, left: &Expression, right: &Expression) -> Result<Value, String> {
        let short_circuit = matches!(op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr);
        
        let left_val = self.eval(left)?;
        
        if short_circuit {
            let left_bool = left_val.as_bool();
            if matches!(op, BinaryOp::LogicalAnd) && !left_bool {
                return Ok(Value::Bool(false));
            }
            if matches!(op, BinaryOp::LogicalOr) && left_bool {
                return Ok(Value::Bool(true));
            }
        }
        
        let right_val = self.eval(right)?;

        Ok(match op {
            BinaryOp::Plus => match (&left_val, &right_val) {
                (Value::Undef, _) | (_, Value::Undef) => Value::Undef,
                (Value::Vector(left), Value::Vector(right)) if left.len() == right.len() =>
                    Value::Vector(left.iter().zip(right).map(|(a, b)| {
                        Ok(Value::Number(a.as_number()? + b.as_number()?))
                    }).collect::<Result<Vec<_>, String>>()?),
                _ => Value::Number(left_val.as_number()? + right_val.as_number()?),
            },
            BinaryOp::Minus => match (&left_val, &right_val) {
                (Value::Undef, _) | (_, Value::Undef) => Value::Undef,
                (Value::Vector(left), Value::Vector(right)) if left.len() == right.len() =>
                    Value::Vector(left.iter().zip(right).map(|(a, b)| {
                        Ok(Value::Number(a.as_number()? - b.as_number()?))
                    }).collect::<Result<Vec<_>, String>>()?),
                _ => Value::Number(left_val.as_number()? - right_val.as_number()?),
            },
            BinaryOp::Multiply => match (&left_val, &right_val) {
                (Value::Undef, _) | (_, Value::Undef) => Value::Undef,
                (Value::Number(scalar), Value::Vector(vector))
                | (Value::Vector(vector), Value::Number(scalar)) =>
                    Value::Vector(vector.iter().map(|value| {
                        Ok(Value::Number(value.as_number()? * scalar))
                    }).collect::<Result<Vec<_>, String>>()?),
                // Matrix * Matrix (e.g. 4x4 * 4x4 or NxK * KxM)
                (Value::Vector(m1), Value::Vector(m2)) if !m1.is_empty() && matches!(m1.first(), Some(Value::Vector(_))) && !m2.is_empty() && matches!(m2.first(), Some(Value::Vector(_))) => {
                    let rows_a = m1.len();
                    let cols_a = match &m1[0] { Value::Vector(v) => v.len(), _ => 0 };
                    let rows_b = m2.len();
                    let cols_b = match &m2[0] { Value::Vector(v) => v.len(), _ => 0 };
                    if cols_a == rows_b {
                        let mut res_mat = Vec::with_capacity(rows_a);
                        for i in 0..rows_a {
                            let mut row = Vec::with_capacity(cols_b);
                            for j in 0..cols_b {
                                let mut sum = 0.0;
                                for k in 0..cols_a {
                                    let a_ik = match &m1[i] { Value::Vector(v) => v[k].as_number()?, _ => 0.0 };
                                    let b_kj = match &m2[k] { Value::Vector(v) => v[j].as_number()?, _ => 0.0 };
                                    sum += a_ik * b_kj;
                                }
                                row.push(Value::Number(sum));
                            }
                            res_mat.push(Value::Vector(row));
                        }
                        Value::Vector(res_mat)
                    } else {
                        return Err(format!("Matrix dimensions mismatch for multiplication: {}x{} vs {}x{}", rows_a, cols_a, rows_b, cols_b));
                    }
                },
                // Matrix * Vector (or Vector * Matrix)
                (Value::Vector(m), Value::Vector(v)) if !m.is_empty() && matches!(m.first(), Some(Value::Vector(_))) => {
                    let mut res = Vec::with_capacity(m.len());
                    for row in m {
                        if let Value::Vector(row_vec) = row {
                            let mut sum = 0.0;
                            for (a, b) in row_vec.iter().zip(v.iter()) {
                                sum += a.as_number()? * b.as_number()?;
                            }
                            res.push(Value::Number(sum));
                        }
                    }
                    Value::Vector(res)
                },
                _ => Value::Number(left_val.as_number()? * right_val.as_number()?),
            },
            BinaryOp::Divide => {
                if left_val.is_undef() || right_val.is_undef() {
                    Value::Undef
                } else if let (Value::Vector(vec), Value::Number(scalar)) = (&left_val, &right_val) {
                    Value::Vector(vec.iter().map(|item| {
                        Ok(Value::Number(item.as_number()? / scalar))
                    }).collect::<Result<Vec<_>, String>>()?)
                } else {
                    let r = right_val.as_number()?;
                    Value::Number(left_val.as_number()? / r)
                }
            }
            BinaryOp::Modulo => {
                if left_val.is_undef() || right_val.is_undef() {
                    Value::Undef
                } else {
                    Value::Number(left_val.as_number()? % right_val.as_number()?)
                }
            }
            BinaryOp::Exponent => {
                if left_val.is_undef() || right_val.is_undef() {
                    Value::Undef
                } else {
                    Value::Number(left_val.as_number()?.powf(right_val.as_number()?))
                }
            }
            
            BinaryOp::Equal => Value::Bool(values_equal(&left_val, &right_val)),
            BinaryOp::NotEqual => Value::Bool(!values_equal(&left_val, &right_val)),
            BinaryOp::Less => match values_cmp(&left_val, &right_val) {
                Some(ord) => Value::Bool(ord.is_lt()),
                None => Value::Undef,
            },
            BinaryOp::Greater => match values_cmp(&left_val, &right_val) {
                Some(ord) => Value::Bool(ord.is_gt()),
                None => Value::Undef,
            },
            BinaryOp::LessEqual => match values_cmp(&left_val, &right_val) {
                Some(ord) => Value::Bool(ord.is_le()),
                None => Value::Undef,
            },
            BinaryOp::GreaterEqual => match values_cmp(&left_val, &right_val) {
                Some(ord) => Value::Bool(ord.is_ge()),
                None => Value::Undef,
            },
            
            BinaryOp::LogicalAnd => Value::Bool(left_val.as_bool() && right_val.as_bool()),
            BinaryOp::LogicalOr => Value::Bool(left_val.as_bool() || right_val.as_bool()),
            
            BinaryOp::BinaryAnd => {
                if left_val.is_undef() || right_val.is_undef() {
                    Value::Undef
                } else {
                    Value::Number((left_val.as_number()? as i64 & right_val.as_number()? as i64) as f64)
                }
            }
            BinaryOp::BinaryOr => {
                if left_val.is_undef() || right_val.is_undef() {
                    Value::Undef
                } else {
                    Value::Number((left_val.as_number()? as i64 | right_val.as_number()? as i64) as f64)
                }
            }
            BinaryOp::ShiftLeft => {
                if left_val.is_undef() || right_val.is_undef() {
                    Value::Undef
                } else {
                    Value::Number(((left_val.as_number()? as i64) << (right_val.as_number()? as i64)) as f64)
                }
            }
            BinaryOp::ShiftRight => {
                if left_val.is_undef() || right_val.is_undef() {
                    Value::Undef
                } else {
                    Value::Number(((left_val.as_number()? as i64) >> (right_val.as_number()? as i64)) as f64)
                }
            }
        })
    }

    fn eval_ternary(&mut self, cond: &Expression, then_e: &Expression, else_e: &Expression) -> Result<Value, String> {
        let cond_val = self.eval(cond)?;
        if cond_val.as_bool() {
            self.eval(then_e)
        } else {
            self.eval(else_e)
        }
    }

    fn eval_vector(&mut self, exprs: &[Expression]) -> Result<Value, String> {
        let mut values = Vec::new();
        for e in exprs {
            let value = self.eval(e)?;
            if matches!(e, Expression::For { .. } | Expression::ForC { .. } | Expression::Each { .. }) {
                if let Value::Vector(items) = value {
                    values.extend(items.into_iter().filter(|item| !item.is_undef()));
                    continue;
                }
            }
            if let Expression::If { else_branch, .. } = e {
                if else_branch.is_none() && value.is_undef() {
                    continue;
                }
            }
            values.push(value);
        }
        Ok(Value::Vector(values))
    }

    fn eval_for(&mut self, bindings: &[Assignment], condition: Option<&Expression>, body: &Expression) -> Result<Value, String> {
        if bindings.is_empty() {
            return Err("for expression requires at least one binding".into());
        }

        let mut values = Vec::new();
        self.eval_for_bindings(bindings, 0, condition, body, &mut values)?;
        Ok(Value::Vector(values))
    }

    fn eval_for_bindings(
        &mut self,
        bindings: &[Assignment],
        index: usize,
        condition: Option<&Expression>,
        body: &Expression,
        values: &mut Vec<Value>,
    ) -> Result<(), String> {
        let binding = &bindings[index];
        if binding.name.is_empty() {
            return Err("for binding requires a variable name".into());
        }
        let iterable = binding.value.as_ref()
            .ok_or_else(|| format!("for binding '{}' requires an expression", binding.name))?;
        let iterable = self.eval(iterable)?;
        let items = match iterable {
            Value::Vector(items) => items,
            _ => return Err(format!("for binding '{}' must evaluate to a vector", binding.name)),
        };

        for item in items {
            let child = Environment::with_parent(self.env.clone());
            child.define(binding.name.clone(), item);
            let mut evaluator = Evaluator::new(Rc::new(child));
            if index + 1 == bindings.len() {
                let include = match condition {
                    Some(condition) => evaluator.eval(condition)?.as_bool(),
                    None => true,
                };
                if include {
                    values.push(evaluator.eval(body)?);
                }
            } else {
                evaluator.eval_for_bindings(bindings, index + 1, condition, body, values)?;
            }
        }
        Ok(())
    }

    fn eval_each(&mut self, body: &Expression) -> Result<Value, String> {
        match self.eval(body)? {
            Value::Vector(items) => {
                let mut flattened = Vec::new();
                for item in items {
                    match item {
                        Value::Vector(nested) => flattened.extend(nested),
                        value => flattened.push(value),
                    }
                }
                Ok(Value::Vector(flattened))
            }
            value => Ok(value),
        }
    }

    fn eval_for_c(
        &mut self,
        init: &[Assignment],
        condition: Option<&Expression>,
        update: &[Assignment],
        body: &Expression,
    ) -> Result<Value, String> {
        let mut current_env = self.env.clone();

        // 1. Sequentially evaluate init assignments
        if !init.is_empty() {
            let init_env = Environment::with_parent(current_env.clone());
            for assign in init {
                let val = if let Some(expr) = &assign.value {
                    let mut eval = Evaluator::with_depth(Rc::new(init_env.clone()), self.depth);
                    eval.eval(expr)?
                } else {
                    Value::Undef
                };
                init_env.define(assign.name.clone(), val);
            }
            current_env = Rc::new(init_env);
        }

        let mut values = Vec::new();
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 1_000_000;

        loop {
            if iterations >= MAX_ITERATIONS {
                return Err("Maximum iteration limit (1000000) exceeded in C-style for loop".into());
            }
            iterations += 1;

            // 2. Check condition
            if let Some(cond_expr) = condition {
                let mut eval = Evaluator::with_depth(current_env.clone(), self.depth);
                let cond_val = eval.eval(cond_expr)?;
                if !cond_val.as_bool() {
                    break;
                }
            }

            // 3. Evaluate body
            let mut eval = Evaluator::with_depth(current_env.clone(), self.depth);
            let val = eval.eval(body)?;

            if matches!(body, Expression::Each { .. }) {
                if let Value::Vector(items) = val {
                    values.extend(items);
                } else {
                    values.push(val);
                }
            } else if let Expression::If { else_branch, .. } = body {
                if else_branch.is_none() && val.is_undef() {
                    // Filtered out by comprehension if
                } else {
                    values.push(val);
                }
            } else {
                values.push(val);
            }

            // 4. Sequentially evaluate update assignments
            if !update.is_empty() {
                let next_env = Environment::with_parent(current_env.clone());
                for assign in update {
                    let val = if let Some(expr) = &assign.value {
                        let mut eval = Evaluator::with_depth(Rc::new(next_env.clone()), self.depth);
                        eval.eval(expr)?
                    } else {
                        Value::Undef
                    };
                    next_env.define(assign.name.clone(), val);
                }
                current_env = Rc::new(next_env);
            } else if condition.is_none() {
                break;
            }
        }

        Ok(Value::Vector(values))
    }

    fn eval_range(&mut self, start: &Expression, step: Option<&Expression>, end: &Expression) -> Result<Value, String> {
        let start = self.eval(start)?.as_number()?;
        let end = self.eval(end)?.as_number()?;
        let step = step.map(|s| self.eval(s)?.as_number()).transpose()?.unwrap_or(1.0);
        
        if step == 0.0 { return Err("Range step cannot be zero".into()); }
        
        let mut values = Vec::new();
        let mut current = start;
        
        if step > 0.0 {
            while current <= end + 1e-10 {
                values.push(Value::Number(current));
                current += step;
            }
        } else {
            while current >= end - 1e-10 {
                values.push(Value::Number(current));
                current += step;
            }
        }
        
        Ok(Value::Vector(values))
    }

    fn eval_function_call(&mut self, name: &str, args: &[Assignment], callee_expr: Option<&Expression>) -> Result<Value, String> {
        if !name.is_empty() {
            if let Some(builtin) = self.builtins.get_function(name) {
                let mut evaluated_args = Vec::new();
                for arg in args {
                    if let Some(expr) = &arg.value {
                        evaluated_args.push(self.eval(expr)?);
                    } else {
                        evaluated_args.push(Value::Undef);
                    }
                }
                return builtin(evaluated_args);
            }
        }

        let func_val = if !name.is_empty() {
            self.env.get_function(name)
                .map(Value::Function)
                .or_else(|| self.env.get(name))
        } else if let Some(callee) = callee_expr {
            Some(self.eval(callee)?)
        } else {
            None
        }.ok_or_else(|| format!("Undefined function '{}'", name))?;

        match func_val {
            Value::Function(func) => {
                if self.depth >= 100 {
                    return Err(format!("Recursion detected calling function '{}'", func.name));
                }
                let parent_env = if func.name == "anonymous" {
                    func.captures.clone()
                } else {
                    self.env.clone()
                };
                let new_env = Environment::with_parent(parent_env);
                let new_env_rc = Rc::new(new_env);
                let mut positional = args.iter().filter(|arg| arg.name.is_empty());
                
                for (param_name, default) in &func.params {
                    let argument = args.iter()
                        .find(|arg| arg.name == *param_name)
                        .or_else(|| positional.next());
                    let value = if let Some(arg) = argument {
                        if let Some(expr) = &arg.value {
                            self.eval(expr)?
                        } else {
                            Value::Undef
                        }
                    } else if let Some(default_expr) = default {
                        let mut eval = Evaluator::with_depth(new_env_rc.clone(), self.depth + 1);
                        eval.eval(default_expr)?
                    } else {
                        return Err(format!("Missing argument for parameter '{}'", param_name));
                    };
                    new_env_rc.define(param_name.clone(), value);
                }
                
                let mut eval = Evaluator::with_depth(new_env_rc, self.depth + 1);
                eval.eval(&func.body)
            }
            _ => Err(format!("'{}' is not a function", name)),
        }
    }

    fn eval_index(&mut self, expr: &Expression, index: &Expression) -> Result<Value, String> {
        let vec_val = self.eval(expr)?;
        let idx_val = self.eval(index)?;
        let Ok(idx_num) = idx_val.as_number() else {
            return Ok(Value::Undef);
        };
        if idx_num < 0.0 {
            return Ok(Value::Undef);
        }
        let idx = idx_num as usize;
        
        match vec_val {
            Value::Vector(v) => Ok(v.get(idx).cloned().unwrap_or(Value::Undef)),
            Value::String(s) => {
                let chars: Vec<char> = s.chars().collect();
                Ok(chars.get(idx).map(|c| Value::String(c.to_string())).unwrap_or(Value::Undef))
            }
            Value::Undef => Ok(Value::Undef),
            _ => Ok(Value::Undef),
        }
    }

    fn eval_dot(&mut self, expr: &Expression, field: &str) -> Result<Value, String> {
        let val = self.eval(expr)?;
        match val {
            Value::Vector(v) => match field {
                "x" | "0" => v.get(0).cloned().ok_or_else(|| "Vector index out of bounds".into()),
                "y" | "1" => v.get(1).cloned().ok_or_else(|| "Vector index out of bounds".into()),
                "z" | "2" => v.get(2).cloned().ok_or_else(|| "Vector index out of bounds".into()),
                "len" => Ok(Value::Number(v.len() as f64)),
                _ => Err(format!("Vector has no field '{}'", field)),
            }
            _ => Err("Dot operator only works on vectors".into()),
        }
    }

    fn eval_let(&mut self, bindings: &[Assignment], body: &Expression) -> Result<Value, String> {
        let new_env = Environment::with_parent(self.env.clone());
        let new_env_rc = Rc::new(new_env);
        
        for binding in bindings {
            if let Some(expr) = &binding.value {
                let mut eval = Evaluator::new(new_env_rc.clone());
                let value = eval.eval(expr)?;
                new_env_rc.define(binding.name.clone(), value);
            }
        }
        
        let mut eval = Evaluator::new(new_env_rc);
        eval.eval(body)
    }

    fn eval_function_def(&mut self, params: &[Assignment], body: &Expression) -> Result<Value, String> {
        let params: Vec<(String, Option<Expression>)> = params.iter()
            .map(|a| (a.name.clone(), a.value.clone()))
            .collect();
        
        let func = crate::interpreter::value::FunctionValue {
            name: "anonymous".to_string(),
            params,
            body: body.clone(),
            captures: self.env.clone(),
        };
        
        Ok(Value::Function(func))
    }

    fn eval_assert(&mut self, args: &[Assignment], body: Option<&Expression>) -> Result<Value, String> {
        if args.is_empty() {
            return Err("assert requires a condition".into());
        }
        let cond_val = if let Some(expr) = &args[0].value {
            self.eval(expr)?
        } else {
            return Err("assert requires a condition".into());
        };
        
        if !cond_val.as_bool() {
            let msg = if args.len() > 1 {
                if let Some(msg_expr) = &args[1].value {
                    self.eval(msg_expr)?.as_string().unwrap_or_else(|_| "Assertion failed".to_string())
                } else {
                    "Assertion failed".to_string()
                }
            } else {
                "Assertion failed".to_string()
            };
            return Err(msg);
        }
        if let Some(body_expr) = body {
            self.eval(body_expr)
        } else {
            Ok(Value::Undef)
        }
    }


    fn format_echo(args: &[Assignment], values: &[Value]) -> String {
        values.iter().enumerate().map(|(index, value)| {
            let name = args[index].name.as_str();
            if name.is_empty() {
                value.to_string()
            } else {
                format!("{} = {}", name, value)
            }
        }).collect::<Vec<_>>().join(", ")
    }

    fn eval_echo(&mut self, args: &[Assignment]) -> Result<Value, String> {
        let mut evaluated = Vec::new();
        for arg in args {
            if let Some(expr) = &arg.value {
                let val = self.eval(expr)?;
                evaluated.push(val);
            }
        }
        println!("ECHO: {}", Self::format_echo(args, &evaluated));
        Ok(Value::Undef)
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Undef, Value::Undef) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => (a - b).abs() < 1e-10,
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Vector(a), Value::Vector(b)) => a.len() == b.len() && a.iter().zip(b).all(|(x, y)| values_equal(x, y)),
        (Value::Function(a), Value::Function(b)) => {
            if a.name == "anonymous" && b.name == "anonymous" {
                Rc::ptr_eq(&a.captures, &b.captures) && a.body == b.body
            } else {
                a.name == b.name
            }
        }
        (Value::Module(a), Value::Module(b)) => a.name == b.name,
        _ => false,
    }
}

fn values_cmp(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => a.partial_cmp(b),
        (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
        (Value::Bool(a), Value::Bool(b)) => Some(a.cmp(b)),
        (Value::Vector(a), Value::Vector(b)) => {
            for (x, y) in a.iter().zip(b.iter()) {
                match values_cmp(x, y) {
                    Some(std::cmp::Ordering::Equal) => continue,
                    non_eq => return non_eq,
                }
            }
            Some(a.len().cmp(&b.len()))
        }
        _ => None,
    }
}