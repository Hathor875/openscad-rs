use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;
use crate::ast::{Expression, Statement, ModuleModifiers};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Undef,
    Bool(bool),
    Number(f64),
    String(String),
    Vector(Vec<Value>),
    Function(FunctionValue),
    Module(ModuleValue),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionValue {
    pub name: String,
    pub params: Vec<(String, Option<Expression>)>,
    pub body: Expression,
    pub captures: Rc<Environment>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleValue {
    pub name: String,
    pub params: Vec<(String, Option<Expression>)>,
    pub body: Vec<Statement>,
    pub captures: Rc<Environment>,
    pub modifiers: ModuleModifiers,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Environment {
    pub vars: RefCell<HashMap<String, Value>>,
    pub parent: Option<Rc<Environment>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            vars: RefCell::new(HashMap::new()),
            parent: None,
        }
    }

    pub fn with_parent(parent: Rc<Environment>) -> Self {
        Self {
            vars: RefCell::new(HashMap::new()),
            parent: Some(parent),
        }
    }

    pub fn define(&self, name: String, value: Value) {
        self.vars.borrow_mut().insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        self.vars.borrow().get(name).cloned()
            .or_else(|| self.parent.as_ref().and_then(|p| p.get(name)))
    }

    pub fn assign(&self, name: &str, value: Value) -> Result<(), String> {
        if self.vars.borrow().contains_key(name) {
            self.vars.borrow_mut().insert(name.to_string(), value);
            Ok(())
        } else if let Some(parent) = &self.parent {
            parent.assign(name, value)
        } else {
            Err(format!("Variable '{}' not found", name))
        }
    }

    pub fn has_local(&self, name: &str) -> bool {
        self.vars.borrow().contains_key(name)
    }
}

impl Value {
    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Vector(v) => !v.is_empty(),
            Value::Undef => false,
            Value::Function(_) => true,
            Value::Module(_) => true,
        }
    }

    pub fn as_number(&self) -> Result<f64, String> {
        match self {
            Value::Number(n) => Ok(*n),
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            _ => Err(format!("Cannot convert {:?} to number", self)),
        }
    }

    pub fn as_string(&self) -> Result<String, String> {
        match self {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Undef => Ok("undef".to_string()),
            _ => Err(format!("Cannot convert {:?} to string", self)),
        }
    }

    pub fn is_undef(&self) -> bool {
        matches!(self, Value::Undef)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Undef => write!(f, "undef"),
            Value::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Vector(v) => {
                write!(f, "[")?;
                for (i, val) in v.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
            Value::Function(func) => write!(f, "function {}()", func.name),
            Value::Module(mod_) => write!(f, "module {}()", mod_.name),
        }
    }
}