use std::fmt;
use text_size::{TextRange, TextSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: TextSize,
    pub end: TextSize,
}

impl Span {
    pub fn new(start: TextSize, end: TextSize) -> Self {
        Self { start, end }
    }

    pub fn range(&self) -> TextRange {
        TextRange::new(self.start, self.end)
    }
}

impl Default for Span {
    fn default() -> Self {
        Self::new(TextSize::default(), TextSize::default())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Bool(bool),
    Number(f64),
    String(String),
    Undefined,
}

impl fmt::Display for LiteralValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LiteralValue::Bool(b) => write!(f, "{}", b),
            LiteralValue::Number(n) => write!(f, "{}", n),
            LiteralValue::String(s) => write!(f, "\"{}\"", s),
            LiteralValue::Undefined => write!(f, "undef"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    BinaryNot,
    Negate,
    Positive,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol = match self {
            UnaryOp::Not => "!",
            UnaryOp::BinaryNot => "~",
            UnaryOp::Negate => "-",
            UnaryOp::Positive => "+",
        };
        f.write_str(symbol)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    LogicalAnd,
    LogicalOr,
    Exponent,
    Multiply,
    Divide,
    Modulo,
    Plus,
    Minus,
    ShiftLeft,
    ShiftRight,
    BinaryAnd,
    BinaryOr,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol = match self {
            BinaryOp::LogicalAnd => "&&",
            BinaryOp::LogicalOr => "||",
            BinaryOp::Exponent => "^",
            BinaryOp::Multiply => "*",
            BinaryOp::Divide => "/",
            BinaryOp::Modulo => "%",
            BinaryOp::Plus => "+",
            BinaryOp::Minus => "-",
            BinaryOp::ShiftLeft => "<<",
            BinaryOp::ShiftRight => ">>",
            BinaryOp::BinaryAnd => "&",
            BinaryOp::BinaryOr => "|",
            BinaryOp::Less => "<",
            BinaryOp::LessEqual => "<=",
            BinaryOp::Greater => ">",
            BinaryOp::GreaterEqual => ">=",
            BinaryOp::Equal => "==",
            BinaryOp::NotEqual => "!=",
        };
        f.write_str(symbol)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(LiteralValue, Span),
    UnaryOp(UnaryOp, Box<Expression>, Span),
    BinaryOp(BinaryOp, Box<Expression>, Box<Expression>, Span),
    TernaryOp(Box<Expression>, Box<Expression>, Box<Expression>, Span),
    If {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Option<Box<Expression>>,
        span: Span,
    },
    ArrayLookup(Box<Expression>, Box<Expression>, Span),
    Lookup(String, Span),
    MemberLookup(Box<Expression>, String, Span),
    FunctionCall {
        name: String,
        args: Vec<Assignment>,
        expr: Option<Box<Expression>>,
        span: Span,
    },
    FunctionDef {
        params: Vec<Assignment>,
        body: Box<Expression>,
        span: Span,
    },
    Let {
        args: Vec<Assignment>,
        body: Box<Expression>,
        span: Span,
    },
    Assert {
        args: Vec<Assignment>,
        body: Option<Box<Expression>>,
        span: Span,
    },
    Echo {
        args: Vec<Assignment>,
        body: Option<Box<Expression>>,
        span: Span,
    },
    For {
        args: Vec<Assignment>,
        condition: Option<Box<Expression>>,
        body: Box<Expression>,
        span: Span,
    },
    ForC {
        init: Vec<Assignment>,
        condition: Option<Box<Expression>>,
        update: Vec<Assignment>,
        body: Box<Expression>,
        span: Span,
    },
    Each {
        args: Vec<Assignment>,
        body: Box<Expression>,
        span: Span,
    },
    Range {
        start: Box<Expression>,
        step: Option<Box<Expression>>,
        end: Box<Expression>,
        span: Span,
    },
    Vector(Vec<Expression>, Span),
}

impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Expression::Literal(_, s) => *s,
            Expression::UnaryOp(_, _, s) => *s,
            Expression::BinaryOp(_, _, _, s) => *s,
            Expression::TernaryOp(_, _, _, s) => *s,
            Expression::If { span, .. } => *span,
            Expression::ArrayLookup(_, _, s) => *s,
            Expression::Lookup(_, s) => *s,
            Expression::MemberLookup(_, _, s) => *s,
            Expression::FunctionCall { span, .. } => *span,
            Expression::FunctionDef { span, .. } => *span,
            Expression::Let { span, .. } => *span,
            Expression::Assert { span, .. } => *span,
            Expression::Echo { span, .. } => *span,
            Expression::Range { span, .. } => *span,
            Expression::For { span, .. } => *span,
            Expression::ForC { span, .. } => *span,
            Expression::Each { span, .. } => *span,
            Expression::Vector(_, s) => *s,
        }
    }

    /// Render the expression back to OpenSCAD source syntax.
    ///
    /// Used when `echo()` prints a function value, mirroring the upstream
    /// OpenSCAD format `function(a, b = 2) (a + b)`.
    pub fn to_source(&self) -> String {
        fn arg(arg: &Assignment) -> String {
            match (&arg.name, &arg.value) {
                (name, Some(expr)) if !name.is_empty() => format!("{name} = {}", expr.to_source()),
                (_, Some(expr)) => expr.to_source(),
                (name, None) => name.clone(),
            }
        }
        fn args(list: &[Assignment]) -> String {
            list.iter().map(arg).collect::<Vec<_>>().join(", ")
        }

        match self {
            Expression::Literal(lit, _) => lit.to_string(),
            Expression::UnaryOp(op, expr, _) => format!("{op}{}", expr.to_source()),
            Expression::BinaryOp(op, left, right, _) => {
                format!("({} {} {})", left.to_source(), op, right.to_source())
            }
            Expression::TernaryOp(cond, then_branch, else_branch, _) => format!(
                "({} ? {} : {})",
                cond.to_source(),
                then_branch.to_source(),
                else_branch.to_source()
            ),
            Expression::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => match else_branch {
                Some(else_branch) => format!(
                    "if ({}) {} else {}",
                    condition.to_source(),
                    then_branch.to_source(),
                    else_branch.to_source()
                ),
                None => format!("if ({}) {}", condition.to_source(), then_branch.to_source()),
            },
            Expression::ArrayLookup(expr, index, _) => {
                format!("{}[{}]", expr.to_source(), index.to_source())
            }
            Expression::Lookup(name, _) => name.clone(),
            Expression::MemberLookup(expr, field, _) => {
                format!("{}.{}", expr.to_source(), field)
            }
            Expression::FunctionCall {
                name,
                args: call_args,
                expr,
                ..
            } => {
                let callee = match expr {
                    Some(expr) => expr.to_source(),
                    None => name.clone(),
                };
                format!("{callee}({})", args(call_args))
            }
            Expression::FunctionDef { params, body, .. } => {
                format!("function({}) {}", args(params), body.to_source())
            }
            Expression::Let {
                args: bindings,
                body,
                ..
            } => format!("let({}) {}", args(bindings), body.to_source()),
            Expression::Assert {
                args: check, body, ..
            } => {
                let call = format!("assert({})", args(check));
                match body {
                    Some(body) => format!("{call} {}", body.to_source()),
                    None => call,
                }
            }
            Expression::Echo {
                args: echo_args,
                body,
                ..
            } => {
                let call = format!("echo({})", args(echo_args));
                match body {
                    Some(body) => format!("{call} {}", body.to_source()),
                    None => call,
                }
            }
            Expression::Range {
                start, step, end, ..
            } => match step {
                Some(step) => format!(
                    "[{}:{}:{}]",
                    start.to_source(),
                    step.to_source(),
                    end.to_source()
                ),
                None => format!("[{}:{}]", start.to_source(), end.to_source()),
            },
            Expression::For {
                args: loop_args,
                condition,
                body,
                ..
            } => {
                let head = match condition {
                    Some(cond) => format!("for ({} : {})", args(loop_args), cond.to_source()),
                    None => format!("for ({})", args(loop_args)),
                };
                format!("{head} {}", body.to_source())
            }
            Expression::ForC {
                init,
                condition,
                update,
                body,
                ..
            } => {
                let cond = condition
                    .as_ref()
                    .map(|expr| expr.to_source())
                    .unwrap_or_default();
                format!(
                    "for ({}; {}; {}) {}",
                    args(init),
                    cond,
                    args(update),
                    body.to_source()
                )
            }
            Expression::Each { body, .. } => format!("each {}", body.to_source()),
            Expression::Vector(exprs, _) => format!(
                "[{}]",
                exprs
                    .iter()
                    .map(|expr| expr.to_source())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    pub name: String,
    pub value: Option<Expression>,
    pub span: Span,
}

impl Assignment {
    pub fn new(name: String, value: Option<Expression>, span: Span) -> Self {
        Self { name, value, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    ModuleInstantiation {
        name: String,
        args: Vec<Assignment>,
        children: Vec<Statement>,
        modifiers: ModuleModifiers,
        span: Span,
    },
    Assignment(Assignment),
    ModuleDef {
        name: String,
        params: Vec<Assignment>,
        body: Vec<Statement>,
        span: Span,
    },
    FunctionDef {
        name: String,
        params: Vec<Assignment>,
        body: Expression,
        span: Span,
    },
    If {
        condition: Expression,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
        span: Span,
    },
    Block {
        statements: Vec<Statement>,
        span: Span,
    },
    Include(String, Span),
    Use(String, Span),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModuleModifiers {
    pub root: bool,
    pub highlight: bool,
    pub background: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    pub path: String,
    pub statements: Vec<Statement>,
    pub includes: Vec<IncludeInfo>,
    pub uses: Vec<UseInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IncludeInfo {
    pub local_path: String,
    pub resolved_path: String,
    pub span: Span,
    pub is_main: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseInfo {
    pub path: String,
    pub resolved_path: Option<String>,
    pub span: Span,
    pub is_main: bool,
}

impl SourceFile {
    pub fn new(path: String) -> Self {
        Self {
            path,
            statements: Vec::new(),
            includes: Vec::new(),
            uses: Vec::new(),
        }
    }
}
