use crate::ast::{Expression, ModuleModifiers, Statement};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Undef,
    Bool(bool),
    Number(f64),
    String(String),
    Vector(Vec<Value>),
    /// Object value, as returned by builtins like `textmetrics()`.
    /// Keys iterate in sorted order for deterministic output.
    Object(BTreeMap<String, Value>),
    Function(FunctionValue),
    Module(ModuleValue),
}

#[derive(Clone, PartialEq)]
pub struct FunctionValue {
    pub name: String,
    pub params: Vec<(String, Option<Expression>)>,
    pub body: Expression,
    pub captures: Rc<Environment>,
}

impl fmt::Debug for FunctionValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionValue")
            .field("name", &self.name)
            .field("params", &self.params)
            .field("body", &self.body)
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub struct ModuleValue {
    pub name: String,
    pub params: Vec<(String, Option<Expression>)>,
    pub body: Vec<Statement>,
    pub captures: Rc<Environment>,
    pub modifiers: ModuleModifiers,
}

impl fmt::Debug for ModuleValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModuleValue")
            .field("name", &self.name)
            .field("params", &self.params)
            .field("modifiers", &self.modifiers)
            .finish()
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct Environment {
    pub vars: RefCell<HashMap<String, Value>>,
    pub functions: RefCell<HashMap<String, FunctionValue>>,
    pub modules: RefCell<HashMap<String, ModuleValue>>,
    pub parent: Option<Rc<Environment>>,
}

impl fmt::Debug for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let var_keys: Vec<_> = self.vars.borrow().keys().cloned().collect();
        f.debug_struct("Environment")
            .field("var_keys", &var_keys)
            .field("has_parent", &self.parent.is_some())
            .finish()
    }
}

impl Environment {
    pub fn new() -> Self {
        let env = Self {
            vars: RefCell::new(HashMap::new()),
            functions: RefCell::new(HashMap::new()),
            modules: RefCell::new(HashMap::new()),
            parent: None,
        };
        env.init_defaults();
        env
    }

    fn init_defaults(&self) {
        let mut vars = self.vars.borrow_mut();
        vars.insert("$preview".to_string(), Value::Bool(false));
        vars.insert("$t".to_string(), Value::Number(0.0));
        vars.insert("$layer_height".to_string(), Value::Number(0.2));
        vars.insert(
            "$vpr".to_string(),
            Value::Vector(vec![
                Value::Number(0.0),
                Value::Number(0.0),
                Value::Number(0.0),
            ]),
        );
        vars.insert(
            "$vpt".to_string(),
            Value::Vector(vec![
                Value::Number(0.0),
                Value::Number(0.0),
                Value::Number(0.0),
            ]),
        );
        vars.insert("$vpd".to_string(), Value::Number(500.0));
        vars.insert("$vpf".to_string(), Value::Number(22.5));
        vars.insert("$children".to_string(), Value::Number(0.0));
    }

    pub fn with_parent(parent: Rc<Environment>) -> Self {
        Self {
            vars: RefCell::new(HashMap::new()),
            functions: RefCell::new(HashMap::new()),
            modules: RefCell::new(HashMap::new()),
            parent: Some(parent),
        }
    }

    pub fn define(&self, name: String, value: Value) {
        self.vars.borrow_mut().insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        let mut curr = Some(self);
        while let Some(env) = curr {
            if let Some(val) = env.vars.borrow().get(name) {
                return Some(val.clone());
            }
            curr = env.parent.as_deref();
        }
        None
    }

    pub fn define_function(&self, name: String, func: FunctionValue) {
        self.functions.borrow_mut().insert(name, func);
    }

    pub fn get_function(&self, name: &str) -> Option<FunctionValue> {
        let mut curr = Some(self);
        while let Some(env) = curr {
            if let Some(val) = env.functions.borrow().get(name) {
                return Some(val.clone());
            }
            curr = env.parent.as_deref();
        }
        None
    }

    pub fn define_module(&self, name: String, module: ModuleValue) {
        self.modules.borrow_mut().insert(name, module);
    }

    pub fn get_module(&self, name: &str) -> Option<ModuleValue> {
        let mut curr = Some(self);
        while let Some(env) = curr {
            if let Some(val) = env.modules.borrow().get(name) {
                return Some(val.clone());
            }
            curr = env.parent.as_deref();
        }
        None
    }

    pub fn assign(&self, name: &str, value: Value) -> Result<(), String> {
        let mut curr = Some(self);
        while let Some(env) = curr {
            if env.vars.borrow().contains_key(name) {
                env.vars.borrow_mut().insert(name.to_string(), value);
                return Ok(());
            }
            curr = env.parent.as_deref();
        }
        Err(format!("Variable '{}' not found", name))
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
            Value::Object(_) => true,
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
            Value::Vector(_) => Ok(format!("{}", self)),
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
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
            Value::Function(func) => {
                let params = func
                    .params
                    .iter()
                    .map(|(name, default)| match default {
                        Some(expr) => format!("{name} = {}", expr.to_source()),
                        None => name.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "function({params}) {}", func.body.to_source())
            }
            Value::Module(mod_) => write!(f, "module {}()", mod_.name),
            Value::Object(map) => {
                write!(f, "{{")?;
                for (i, (key, val)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{} = {}", key, val)?;
                }
                write!(f, "}}")
            }
        }
    }
}
