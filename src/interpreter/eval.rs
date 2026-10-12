use std::rc::Rc;
use crate::ast::{Expression, BinaryOp, UnaryOp, Assignment, LiteralValue};
use crate::interpreter::value::{Value, Environment};
use crate::interpreter::builtins::Builtins;

pub struct Evaluator {
    pub env: Rc<Environment>,
    builtins: Builtins,
}

impl Evaluator {
    pub fn new(env: Rc<Environment>) -> Self {
        Self {
            env,
            builtins: Builtins::new(),
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
            Expression::Vector(exprs, _span) => self.eval_vector(exprs),
            Expression::Range { start, step, end, .. } => self.eval_range(start, step.as_deref(), end),
            Expression::FunctionCall { name, args, .. } => self.eval_function_call(name, args),
            Expression::ArrayLookup(expr, index, _span) => self.eval_index(expr, index),
            Expression::MemberLookup(expr, field, _span) => self.eval_dot(expr, field),
            Expression::Let { args, body, .. } => self.eval_let(args, body),
            Expression::FunctionDef { params, body, .. } => self.eval_function_def(params, body),
            Expression::Assert { args, body, .. } => self.eval_assert(args, body.as_deref()),
            Expression::Echo { args, .. } => self.eval_echo(args),
            Expression::For { args, condition, body, .. } => self.eval_for(args, condition.as_deref(), body),
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
        self.env.get(name)
            .ok_or_else(|| format!("Undefined variable '{}'", name))
    }

    fn eval_unary(&mut self, op: &UnaryOp, expr: &Expression) -> Result<Value, String> {
        let val = self.eval(expr)?;
        Ok(match op {
            UnaryOp::Negate => Value::Number(-val.as_number()?),
            UnaryOp::Not => Value::Bool(!val.as_bool()),
            UnaryOp::BinaryNot => Value::Number(!(val.as_number()? as i64) as f64),
        })
    }

    fn eval_binary(&mut self, op: &BinaryOp, left: &Expression, right: &Expression) -> Result<Value, String> {
        let short_circuit = matches!(op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr);
        
        let left_val = self.eval(left)?;
        // eprintln!("DEBUG eval_binary: op={:?}, left_val={:?}", op, left_val);
        
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
        // eprintln!("DEBUG eval_binary: op={:?}, right_val={:?}", op, right_val);

        Ok(match op {
            BinaryOp::Plus => match (left_val, right_val) {
                (Value::Vector(left), Value::Vector(right)) if left.len() == right.len() =>
                    Value::Vector(left.into_iter().zip(right).map(|(a, b)| {
                        Ok(Value::Number(a.as_number()? + b.as_number()?))
                    }).collect::<Result<Vec<_>, String>>()?),
                (left, right) => Value::Number(left.as_number()? + right.as_number()?),
            },
            BinaryOp::Minus => match (left_val, right_val) {
                (Value::Vector(left), Value::Vector(right)) if left.len() == right.len() =>
                    Value::Vector(left.into_iter().zip(right).map(|(a, b)| {
                        Ok(Value::Number(a.as_number()? - b.as_number()?))
                    }).collect::<Result<Vec<_>, String>>()?),
                (left, right) => Value::Number(left.as_number()? - right.as_number()?),
            },
            BinaryOp::Multiply => match (left_val, right_val) {
                (Value::Number(scalar), Value::Vector(vector))
                | (Value::Vector(vector), Value::Number(scalar)) =>
                    Value::Vector(vector.into_iter().map(|value| {
                        Ok(Value::Number(value.as_number()? * scalar))
                    }).collect::<Result<Vec<_>, String>>()?),
                (left, right) => Value::Number(left.as_number()? * right.as_number()?),
            },
            BinaryOp::Divide => {
                let r = right_val.as_number()?;
                if r == 0.0 { return Err("Division by zero".into()); }
                Value::Number(left_val.as_number()? / r)
            }
            BinaryOp::Modulo => Value::Number(left_val.as_number()? % right_val.as_number()?),
            BinaryOp::Exponent => Value::Number(left_val.as_number()?.powf(right_val.as_number()?)),
            
            BinaryOp::Equal => Value::Bool(values_equal(&left_val, &right_val)),
            BinaryOp::NotEqual => Value::Bool(!values_equal(&left_val, &right_val)),
            BinaryOp::Less => Value::Bool(left_val.as_number()? < right_val.as_number()?),
            BinaryOp::Greater => Value::Bool(left_val.as_number()? > right_val.as_number()?),
            BinaryOp::LessEqual => Value::Bool(left_val.as_number()? <= right_val.as_number()?),
            BinaryOp::GreaterEqual => Value::Bool(left_val.as_number()? >= right_val.as_number()?),
            
            BinaryOp::LogicalAnd => Value::Bool(left_val.as_bool() && right_val.as_bool()),
            BinaryOp::LogicalOr => Value::Bool(left_val.as_bool() || right_val.as_bool()),
            
            BinaryOp::BinaryAnd => Value::Number((left_val.as_number()? as i64 & right_val.as_number()? as i64) as f64),
            BinaryOp::BinaryOr => Value::Number((left_val.as_number()? as i64 | right_val.as_number()? as i64) as f64),
            BinaryOp::ShiftLeft => Value::Number(((left_val.as_number()? as i64) << (right_val.as_number()? as i64)) as f64),
            BinaryOp::ShiftRight => Value::Number(((left_val.as_number()? as i64) >> (right_val.as_number()? as i64)) as f64),
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
            if matches!(e, Expression::For { .. } | Expression::Each { .. }) {
                if let Value::Vector(items) = value {
                    values.extend(items);
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

    fn eval_function_call(&mut self, name: &str, args: &[Assignment]) -> Result<Value, String> {
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

        let func_val = self.env.get(name)
            .ok_or_else(|| format!("Undefined function '{}'", name))?;

        match func_val {
            Value::Function(func) => {
                let new_env = Environment::with_parent(self.env.clone());
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
                        let mut eval = Evaluator::new(Rc::new(new_env.clone()));
                        eval.eval(default_expr)?
                    } else {
                        return Err(format!("Missing argument for parameter '{}'", param_name));
                    };
                    new_env.define(param_name.clone(), value);
                }
                
                let mut eval = Evaluator::new(Rc::new(new_env));
                eval.eval(&func.body)
            }
            _ => Err(format!("'{}' is not a function", name)),
        }
    }

    fn eval_index(&mut self, expr: &Expression, index: &Expression) -> Result<Value, String> {
        let vec_val = self.eval(expr)?;
        let idx_val = self.eval(index)?;
        let idx = idx_val.as_number()? as usize;
        
        match vec_val {
            Value::Vector(v) => v.get(idx).cloned().ok_or_else(|| "Index out of bounds".into()),
            Value::String(s) => {
                let chars: Vec<char> = s.chars().collect();
                chars.get(idx).map(|c| Value::String(c.to_string())).ok_or_else(|| "Index out of bounds".into())
            }
            _ => Err("Index operator only works on vectors and strings".into()),
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
        
        for binding in bindings {
            if let Some(expr) = &binding.value {
                let value = self.eval(expr)?;
                new_env.define(binding.name.clone(), value);
            }
        }
        
        let mut eval = Evaluator::new(Rc::new(new_env));
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
            captures: (*self.env).clone(),
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
            let msg = if let Some(body_expr) = body {
                self.eval(body_expr)?.as_string().unwrap_or_else(|_| "Assertion failed".to_string())
            } else {
                "Assertion failed".to_string()
            };
            return Err(msg);
        }
        Ok(Value::Undef)
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
        (Value::Function(a), Value::Function(b)) => a.name == b.name,
        (Value::Module(a), Value::Module(b)) => a.name == b.name,
        _ => false,
    }
}