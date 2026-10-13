use crate::ast::{SourceFile, Statement, Assignment, Expression, ModuleModifiers, Span, LiteralValue, UnaryOp, BinaryOp};
use pest::Parser as PestParser;
use pest::iterators::Pair;
use pest::Span as PestSpan;
use pest_derive::Parser;
use text_size::TextSize;
use anyhow::Result;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[grammar = "parser/openscad.pest"]
struct OpenSCADParser;

pub struct Parser;

impl Parser {
    pub fn parse(input: &str) -> Result<SourceFile> {
        Self::parse_with_path(input, String::new())
    }

    pub fn parse_file(path: impl AsRef<Path>) -> Result<SourceFile> {
        let path = path.as_ref();
        let bytes = fs::read(path)?;
        // Some libraries ship latin-1 encoded comments or strings; degrade to
        // replacement characters instead of failing the whole load.
        let input = match String::from_utf8(bytes) {
            Ok(input) => input,
            Err(error) => String::from_utf8_lossy(error.as_bytes()).into_owned(),
        };
        Self::parse_with_path(&input, path.to_string_lossy().into_owned())
    }

    pub fn parse_with_path(input: &str, path: String) -> Result<SourceFile> {
        let pairs = OpenSCADParser::parse(Rule::module, input)?;
        let mut file = SourceFile::new(path);
        
        for pair in pairs {
            // eprintln!("DEBUG parse: pair rule = {:?}", pair.as_rule());
            for inner in pair.into_inner() {
                // eprintln!("DEBUG parse: inner rule = {:?}", inner.as_rule());
                Self::parse_top_level(inner, &mut file)?;
            }
        }
        
        Ok(file)
    }
    
    fn pest_span_to_span(span: PestSpan) -> Span {
        Span::new(
            TextSize::try_from(span.start()).unwrap_or_default(),
            TextSize::try_from(span.end()).unwrap_or_default(),
        )
    }
    
    fn parse_top_level(pair: Pair<Rule>, file: &mut SourceFile) -> Result<()> {
        match pair.as_rule() {
            Rule::top_level => {
                for inner in pair.into_inner() {
                    // eprintln!("DEBUG parse_top_level: inner rule = {:?}", inner.as_rule());
                    match inner.as_rule() {
                        Rule::statement => {
                            if let Some(stmt) = Self::parse_statement(inner)? {
                                file.statements.push(stmt);
                            }
                        }
                        Rule::include_stmt => {
                            if let Some(inner2) = inner.clone().into_inner()
                                .find(|pair| pair.as_rule() == Rule::include_path)
                            {
                                let path = inner2.as_str().trim().to_string();
                                file.statements.push(Statement::Include(path, Span::default()));
                            }
                        }
                        Rule::use_directive => {
                            if let Some(inner2) = inner.clone().into_inner()
                                .find(|pair| pair.as_rule() == Rule::include_path)
                            {
                                let path = inner2.as_str().trim().to_string();
                                file.statements.push(Statement::Use(path, Span::default()));
                            }
                        }
                        Rule::EOI | Rule::WHITESPACE | Rule::COMMENT => {}
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    fn parse_statement(pair: Pair<Rule>) -> Result<Option<Statement>> {
        match pair.as_rule() {
            Rule::statement => {
                let mut inner = pair.clone().into_inner();
                if let Some(first) = inner.next() {
                    // eprintln!("DEBUG parse_statement: first rule = {:?}", first.as_rule());
                    match first.as_rule() {
                        Rule::module_instantiation => Ok(Some(Self::parse_module_instantiation(first)?)),
                        Rule::assignment => Ok(Some(Self::parse_assignment(first)?)),
                        Rule::module_def => Ok(Some(Self::parse_module_def(first)?)),
                        Rule::function_def => Ok(Some(Self::parse_function_def(first)?)),
                        Rule::if_stmt => Ok(Some(Self::parse_if_stmt(first)?)),
                        Rule::block => Ok(Some(Self::parse_block(first)?)),
                        // `include`/`use` are statements too: libraries place
                        // them inside module bodies and blocks.
                        Rule::include_stmt => Ok(Some(Self::parse_source_directive(
                            first,
                            true,
                        )?)),
                        Rule::use_directive => Ok(Some(Self::parse_source_directive(
                            first,
                            false,
                        )?)),
                        _ => Ok(None),
                    }
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
    
    fn parse_source_directive(pair: Pair<Rule>, is_include: bool) -> Result<Statement> {
        let span = Self::pest_span_to_span(pair.as_span());
        let path = pair
            .into_inner()
            .find(|p| p.as_rule() == Rule::include_path)
            .map(|p| p.as_str().trim().to_string())
            .unwrap_or_default();
        Ok(if is_include {
            Statement::Include(path, span)
        } else {
            Statement::Use(path, span)
        })
    }

    fn parse_module_instantiation(pair: Pair<Rule>) -> Result<Statement> {
        let mut modifiers = ModuleModifiers::default();
        let mut name = String::new();
        let mut args = Vec::new();
        let mut children = Vec::new();
        let span = Self::pest_span_to_span(pair.as_span());
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::modifier => {
                    match inner.as_str() {
                        "!" => modifiers.root = true,
                        "#" => modifiers.highlight = true,
                        "%" => modifiers.background = true,
                        "*" => modifiers.disabled = true,
                        _ => {}
                    }
                }
                Rule::module_id => {
                    name = inner.as_str().trim().to_string();
                }
                Rule::arguments => {
                    for arg in inner.into_inner() {
                        if let Rule::argument = arg.as_rule() {
                            if let Some(argument) = Self::parse_argument(arg)? {
                                args.push(argument);
                            }
                        }
                    }
                }
                Rule::child_block => {
                    for stmt in inner.into_inner() {
                        match stmt.as_rule() {
                            Rule::statement => {
                                if let Some(s) = Self::parse_statement(stmt)? {
                                    children.push(s);
                                }
                            }
                            Rule::block => {
                                for nested in stmt.into_inner() {
                                    if let Rule::statement = nested.as_rule() {
                                        if let Some(s) = Self::parse_statement(nested)? {
                                            children.push(s);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Rule::block => {
                    for stmt in inner.into_inner() {
                        if let Rule::statement = stmt.as_rule() {
                            if let Some(s) = Self::parse_statement(stmt)? {
                                children.push(s);
                            }
                        }
                    }
                }
                Rule::statement => {
                    if let Some(s) = Self::parse_statement(inner)? {
                        children.push(s);
                    }
                }
                _ => {}
            }
        }
        
        Ok(Statement::ModuleInstantiation {
            name,
            args,
            children,
            modifiers,
            span,
        })
    }
    
    /// Parses one call argument. Empty slots from stray commas (`f(a, , b)`)
    /// yield `None` so they are dropped, matching OpenSCAD.
    fn parse_argument(pair: Pair<Rule>) -> Result<Option<Assignment>> {
        let span = Self::pest_span_to_span(pair.as_span());
        let mut name = String::new();
        let mut value = None;
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::expression => {
                    value = Some(Self::parse_expression(inner)?);
                }
                Rule::identifier => {
                    name = inner.as_str().trim().to_string();
                }
                Rule::EqSign => {
                    // Skip the equals sign
                }
                _ => {}
            }
        }
        
        if name.is_empty() && value.is_none() {
            Ok(None)
        } else {
            Ok(Some(Assignment::new(name, value, span)))
        }
    }
    
    fn parse_assignment(pair: Pair<Rule>) -> Result<Statement> {
        let mut name = String::new();
        let mut value = None;
        let span = Self::pest_span_to_span(pair.as_span());
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => {
                    name = inner.as_str().trim().to_string();
                }
                Rule::expression => {
                    value = Some(Self::parse_expression(inner)?);
                }
                _ => {}
            }
        }
        
        Ok(Statement::Assignment(Assignment::new(name, value, span)))
    }
    
    fn parse_module_def(pair: Pair<Rule>) -> Result<Statement> {
        let mut name = String::new();
        let mut params = Vec::new();
        let mut body = Vec::new();
        let span = Self::pest_span_to_span(pair.as_span());
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => {
                    name = inner.as_str().trim().to_string();
                }
                Rule::parameters => {
                    for param in inner.into_inner() {
                        if let Rule::parameter = param.as_rule() {
                            let parsed = Self::parse_parameter(param)?;
                            if !parsed.name.is_empty() {
                                params.push(parsed);
                            }
                        }
                    }
                }
                Rule::statement => {
                    if let Some(s) = Self::parse_statement(inner)? {
                        body.push(s);
                    }
                }
                Rule::block => {
                    for stmt in inner.into_inner() {
                        if let Rule::statement = stmt.as_rule() {
                            if let Some(s) = Self::parse_statement(stmt)? {
                                body.push(s);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        Ok(Statement::ModuleDef { name, params, body, span })
    }
    
    fn parse_if_stmt(pair: Pair<Rule>) -> Result<Statement> {
        let mut condition = None;
        let mut then_branch = None;
        let mut else_branch = None;
        let span = Self::pest_span_to_span(pair.as_span());
        // eprintln!("DEBUG parse_if_stmt: pair rule = {:?}", pair.as_rule());
        
        for inner in pair.into_inner() {
            // eprintln!("DEBUG parse_if_stmt: inner rule = {:?}, text = {:?}", inner.as_rule(), inner.as_str());
            match inner.as_rule() {
                Rule::expression => {
                    if condition.is_none() {
                        condition = Some(Self::parse_expression(inner)?);
                    }
                }
                Rule::statement => {
                    // A bare `;` is a valid null branch: NopSCADlib writes
                    // `if (cutout) ; else ...`.
                    let stmt_span = Self::pest_span_to_span(inner.as_span());
                    let branch = Self::parse_statement(inner)?
                        .unwrap_or_else(|| Statement::Block {
                            statements: Vec::new(),
                            span: stmt_span,
                        });
                    if then_branch.is_none() {
                        then_branch = Some(Box::new(branch));
                    } else {
                        else_branch = Some(Box::new(branch));
                    }
                }
                _ => {}
            }
        }
        
        Ok(Statement::If {
            condition: condition.ok_or_else(|| anyhow::anyhow!("Missing if condition"))?,
            then_branch: then_branch.ok_or_else(|| anyhow::anyhow!("Missing then branch"))?,
            else_branch,
            span,
        })
    }
    
    fn parse_block(pair: Pair<Rule>) -> Result<Statement> {
        let mut statements = Vec::new();
        let span = Self::pest_span_to_span(pair.as_span());
        
        for inner in pair.into_inner() {
            if let Rule::statement = inner.as_rule() {
                if let Some(stmt) = Self::parse_statement(inner)? {
                    statements.push(stmt);
                }
            }
        }
        
        Ok(Statement::Block { statements, span })
    }
    
    fn parse_parameter(pair: Pair<Rule>) -> Result<Assignment> {
        let mut name = String::new();
        let mut value = None;
        let span = Self::pest_span_to_span(pair.as_span());
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => {
                    name = inner.as_str().trim().to_string();
                }
                Rule::expression => {
                    value = Some(Self::parse_expression(inner)?);
                }
                _ => {}
            }
        }
        
        Ok(Assignment::new(name, value, span))
    }
    
    fn parse_function_def(pair: Pair<Rule>) -> Result<Statement> {
        let mut name = String::new();
        let mut params = Vec::new();
        let mut body = None;
        let span = Self::pest_span_to_span(pair.as_span());
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => {
                    name = inner.as_str().trim().to_string();
                }
                Rule::parameters => {
                    for param in inner.into_inner() {
                        if let Rule::parameter = param.as_rule() {
                            let parsed = Self::parse_parameter(param)?;
                            if !parsed.name.is_empty() {
                                params.push(parsed);
                            }
                        }
                    }
                }
                Rule::expression => {
                    body = Some(Self::parse_expression(inner)?);
                }
                _ => {}
            }
        }
        
        Ok(Statement::FunctionDef {
            name,
            params,
            body: body.unwrap_or(Expression::Literal(LiteralValue::Undefined, Span::default())),
            span,
        })
    }
    
    fn parse_expression(pair: Pair<Rule>) -> Result<Expression> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();
        
        match rule {
            Rule::expression => {
                if let Some(first) = inner.next() {
                    return Self::parse_expression(first);
                }
                Ok(Expression::Literal(LiteralValue::Undefined, Span::default()))
            }
            Rule::ternary => {
                let exprs: Vec<_> = inner
                    .filter(|p| !matches!(p.as_rule(), Rule::Question | Rule::Colon))
                    .collect();
                if exprs.len() >= 3 {
                    let condition = Self::parse_expression(exprs[0].clone())?;
                    let then_expr = Self::parse_expression(exprs[1].clone())?;
                    let else_expr = Self::parse_expression(exprs[2].clone())?;
                    let span = Self::pest_span_to_span(pair.as_span());
                    Ok(Expression::TernaryOp(
                        Box::new(condition),
                        Box::new(then_expr),
                        Box::new(else_expr),
                        span,
                    ))
                } else if !exprs.is_empty() {
                    Self::parse_expression(exprs[0].clone())
                } else {
                    Ok(Expression::Literal(LiteralValue::Undefined, Span::default()))
                }
            }

            Rule::logic_or => Self::parse_binary_chain(pair, BinaryOp::LogicalOr),
            Rule::logic_and => Self::parse_binary_chain(pair, BinaryOp::LogicalAnd),
            Rule::equality => Self::parse_binary_chain_with_ops(pair, &[
                ("==", BinaryOp::Equal),
                ("!=", BinaryOp::NotEqual),
            ]),
            Rule::comparison => Self::parse_binary_chain_with_ops(pair, &[
                ("<", BinaryOp::Less),
                (">", BinaryOp::Greater),
                ("<=", BinaryOp::LessEqual),
                (">=", BinaryOp::GreaterEqual),
            ]),
            Rule::binary_or => Self::parse_binary_chain(pair, BinaryOp::BinaryOr),
            Rule::binary_and => Self::parse_binary_chain(pair, BinaryOp::BinaryAnd),
            Rule::shift => Self::parse_binary_chain_with_ops(pair, &[
                ("<<", BinaryOp::ShiftLeft),
                (">>", BinaryOp::ShiftRight),
            ]),
            Rule::addition => Self::parse_binary_chain_with_ops(pair, &[
                ("+", BinaryOp::Plus),
                ("-", BinaryOp::Minus),
            ]),
            Rule::multiplication => Self::parse_binary_chain_with_ops(pair, &[
                ("*", BinaryOp::Multiply),
                ("/", BinaryOp::Divide),
                ("%", BinaryOp::Modulo),
            ]),
            Rule::power => Self::parse_power(pair),
            Rule::unary => Self::parse_unary(pair),
            Rule::postfix => Self::parse_postfix(pair),
            Rule::literal => Self::parse_literal(pair),
            Rule::identifier => Ok(Expression::Lookup(
                pair.as_str().trim().to_string(),
                Self::pest_span_to_span(pair.as_span()),
            )),
            Rule::range => Self::parse_range(pair),
            Rule::vector => Self::parse_vector(pair),
            _ => Ok(Expression::Literal(LiteralValue::Undefined, Span::default())),
        }
    }
    
    fn parse_binary_chain(pair: Pair<Rule>, op: BinaryOp) -> Result<Expression> {
        let mut inner = pair.clone().into_inner();
        let mut left = Self::parse_expression(inner.next().unwrap())?;
        
        while let Some(_op_token) = inner.next() {
            if let Some(right_pair) = inner.next() {
                let right = Self::parse_expression(right_pair)?;
                left = Expression::BinaryOp(
                    op,
                    Box::new(left),
                    Box::new(right),
                    Self::pest_span_to_span(pair.as_span()),
                );
            }
        }
        
        Ok(left)
    }
    
    fn parse_binary_chain_with_ops(pair: Pair<Rule>, ops: &[(&str, BinaryOp)]) -> Result<Expression> {
        let mut inner = pair.clone().into_inner();
        let mut left = Self::parse_expression(inner.next().unwrap())?;
        
        while let Some(right_pair) = inner.next() {
            let op_str = right_pair.as_str();
            let op = ops.iter().find(|(s, _)| *s == op_str).map(|(_, o)| *o);
            if let Some(op) = op {
                let right = Self::parse_expression(inner.next().unwrap())?;
                left = Expression::BinaryOp(
                    op,
                    Box::new(left),
                    Box::new(right),
                    Self::pest_span_to_span(pair.as_span()),
                );
            }
        }
        
        Ok(left)
    }
    
    fn parse_unary(pair: Pair<Rule>) -> Result<Expression> {
        let span = Self::pest_span_to_span(pair.as_span());
        let mut operators = Vec::new();
        let mut expr = None;
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::postfix | Rule::unary => {
                    expr = Some(Self::parse_expression(inner)?);
                }
                Rule::Plus | Rule::Minus | Rule::Bang | Rule::Tilde => {
                    operators.push(match inner.as_str() {
                        "+" => UnaryOp::Positive,
                        "-" => UnaryOp::Negate,
                        "!" => UnaryOp::Not,
                        "~" => UnaryOp::BinaryNot,
                        _ => unreachable!(),
                    });
                }
                _ => {}
            }
        }
        
        let mut expr = expr.ok_or_else(|| anyhow::anyhow!("Empty unary expression"))?;
        for op in operators.into_iter().rev() {
            expr = Expression::UnaryOp(op, Box::new(expr), span);
        }
        Ok(expr)
    }

    fn parse_power(pair: Pair<Rule>) -> Result<Expression> {
        let mut inner = pair.clone().into_inner();
        let left = Self::parse_expression(inner.next().unwrap())?;
        if inner.next().is_some() {
            let right = Self::parse_expression(inner.next().unwrap())?;
            Ok(Expression::BinaryOp(
                BinaryOp::Exponent,
                Box::new(left),
                Box::new(right),
                Self::pest_span_to_span(pair.as_span()),
            ))
        } else {
            Ok(left)
        }
    }
    
    fn parse_postfix(pair: Pair<Rule>) -> Result<Expression> {
        let mut base_expr = None;
        let span = Self::pest_span_to_span(pair.as_span());
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::atom => {
                    base_expr = Some(Self::parse_atom(inner)?);
                }
                Rule::function_call_suffix => {
                    let suffixes: Vec<_> = inner.into_inner().collect();
                    if suffixes.is_empty() {
                        if let Some(e) = base_expr.take() {
                            let name = match &e {
                                Expression::Lookup(n, _) => n.clone(),
                                _ => String::new(),
                            };
                            base_expr = Some(Expression::FunctionCall {
                                name,
                                args: Vec::new(),
                                expr: Some(Box::new(e)),
                                span,
                            });
                        }
                    }
                    for suffix in suffixes {
                        match suffix.as_rule() {
                            Rule::arguments => {
                                let mut args = Vec::new();
                                for arg in suffix.into_inner() {
                                    if let Rule::argument = arg.as_rule() {
                                        if let Some(argument) = Self::parse_argument(arg)? {
                                            args.push(argument);
                                        }
                                    }
                                }
                                if let Some(e) = base_expr.take() {
                                    let name = match &e {
                                        Expression::Lookup(n, _) => n.clone(),
                                        _ => "".to_string(),
                                    };
                                    base_expr = Some(Expression::FunctionCall {
                                        name,
                                        args,
                                        expr: Some(Box::new(e)),
                                        span,
                                    });
                                }
                            }
                            Rule::expression => {
                                if let Some(e) = base_expr.take() {
                                    let idx = Self::parse_expression(suffix)?;
                                    base_expr = Some(Expression::ArrayLookup(
                                        Box::new(e),
                                        Box::new(idx),
                                        span,
                                    ));
                                }
                            }
                            Rule::identifier => {
                                if let Some(e) = base_expr.take() {
                                    base_expr = Some(Expression::MemberLookup(
                                        Box::new(e),
                                        suffix.as_str().trim().to_string(),
                                        span,
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        
        base_expr.ok_or_else(|| anyhow::anyhow!("Empty postfix expression"))
    }
    
    fn parse_atom(pair: Pair<Rule>) -> Result<Expression> {
        for inner in pair.into_inner() {
            // eprintln!("DEBUG parse_atom: inner rule = {:?}, text = {:?}", inner.as_rule(), inner.as_str());
            match inner.as_rule() {
                Rule::let_expr => return Self::parse_let_expr(inner),
                Rule::anonymous_function => return Self::parse_anonymous_function(inner),
                Rule::if_expr => return Self::parse_if_expression(inner),
                Rule::assert_expr => return Self::parse_assert_expr(inner),
                Rule::for_c_expr => return Self::parse_for_c_expr(inner),
                Rule::for_expr => return Self::parse_for_expr(inner),
                Rule::each_expr => return Self::parse_each_expr(inner),
                Rule::echo_expr => return Self::parse_echo_expr(inner),
                _ => return Self::parse_expression(inner),
            }
        }
        Ok(Expression::Literal(LiteralValue::Undefined, Span::default()))
    }

    fn parse_if_expression(pair: Pair<Rule>) -> Result<Expression> {
        let span = Self::pest_span_to_span(pair.as_span());
        let expressions = pair.into_inner()
            .filter(|inner| matches!(inner.as_rule(), Rule::expression | Rule::if_expr))
            .map(|inner| {
                if inner.as_rule() == Rule::if_expr {
                    Self::parse_if_expression(inner)
                } else {
                    Self::parse_expression(inner)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        if let [condition, then_expr, else_expr] = expressions.as_slice() {
            Ok(Expression::If {
                condition: Box::new(condition.clone()),
                then_branch: Box::new(then_expr.clone()),
                else_branch: Some(Box::new(else_expr.clone())),
                span,
            })
        } else if let [condition, then_expr] = expressions.as_slice() {
            Ok(Expression::If {
                condition: Box::new(condition.clone()),
                then_branch: Box::new(then_expr.clone()),
                else_branch: None,
                span,
            })
        } else {
            Err(anyhow::anyhow!("if expression requires a condition and value"))
        }
    }

    fn parse_anonymous_function(pair: Pair<Rule>) -> Result<Expression> {
        let span = Self::pest_span_to_span(pair.as_span());
        let mut params = Vec::new();
        let mut body = None;
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::parameters => {
                    for parameter in inner.into_inner() {
                        if let Rule::parameter = parameter.as_rule() {
                            let parsed = Self::parse_parameter(parameter)?;
                            if !parsed.name.is_empty() {
                                params.push(parsed);
                            }
                        }
                    }
                }
                Rule::expression => body = Some(Self::parse_expression(inner)?),
                _ => {}
            }
        }
        Ok(Expression::FunctionDef {
            params,
            body: Box::new(body.ok_or_else(|| anyhow::anyhow!("Missing anonymous function body"))?),
            span,
        })
    }
    
    fn parse_let_expr(pair: Pair<Rule>) -> Result<Expression> {
        let mut args = Vec::new();
        let mut body = None;
        let span = Self::pest_span_to_span(pair.as_span());
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::let_bindings => {
                    for binding in inner.into_inner() {
                        if let Rule::let_binding = binding.as_rule() {
                            let mut name = String::new();
                            let mut value = None;
                            let bind_span = Self::pest_span_to_span(binding.as_span());
                            for b in binding.into_inner() {
                                match b.as_rule() {
                                    Rule::identifier => name = b.as_str().trim().to_string(),
                                    Rule::expression => value = Some(Self::parse_expression(b)?),
                                    _ => {}
                                }
                            }
                            args.push(Assignment::new(name, value, bind_span));
                        }
                    }
                }
                Rule::expression => {
                    body = Some(Self::parse_expression(inner)?);
                }
                _ => {}
            }
        }
        
        Ok(Expression::Let {
            args,
            body: Box::new(body.ok_or_else(|| anyhow::anyhow!("Missing let body"))?),
            span,
        })
    }
    
    fn parse_literal(pair: Pair<Rule>) -> Result<Expression> {
        let span = Self::pest_span_to_span(pair.as_span());
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::boolean => {
                    let value = inner.as_str() == "true";
                    return Ok(Expression::Literal(LiteralValue::Bool(value), span));
                }
                Rule::number => {
                    return Self::parse_number(inner);
                }
                Rule::string_literal => {
                    let value = Self::parse_string(inner)?;
                    return Ok(Expression::Literal(LiteralValue::String(value), span));
                }
                Rule::Undef => {
                    return Ok(Expression::Literal(LiteralValue::Undefined, span));
                }
                _ => {}
            }
        }
        
        Ok(Expression::Literal(LiteralValue::Undefined, span))
    }
    
    fn parse_number(pair: Pair<Rule>) -> Result<Expression> {
        let span = Self::pest_span_to_span(pair.as_span());
        let text = pair.as_str().trim();
        // eprintln!("DEBUG parse_number: text = {:?}", text);
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::hex_number => {
                    let value = u64::from_str_radix(&text[2..], 16).unwrap_or(0) as f64;
                    return Ok(Expression::Literal(LiteralValue::Number(value), span));
                }
                Rule::float_number => {
                    let value = text.parse().unwrap_or(0.0);
                    return Ok(Expression::Literal(LiteralValue::Number(value), span));
                }
                Rule::int_number => {
                    let value = text.parse().unwrap_or(0) as f64;
                    return Ok(Expression::Literal(LiteralValue::Number(value), span));
                }
                _ => {}
            }
        }
        
        Ok(Expression::Literal(LiteralValue::Number(0.0), span))
    }
    
    fn parse_string(pair: Pair<Rule>) -> Result<String> {
        let text = pair.as_str();
        let content = &text[1..text.len()-1];
        let mut value = String::new();
        let mut chars = content.chars();
        while let Some(ch) = chars.next() {
            if ch != '\\' {
                value.push(ch);
                continue;
            }

            let escaped = chars.next()
                .ok_or_else(|| anyhow::anyhow!("Incomplete string escape"))?;
            match escaped {
                'n' => value.push('\n'),
                't' => value.push('\t'),
                'r' => value.push('\r'),
                '\\' => value.push('\\'),
                '"' => value.push('"'),
                'x' | 'u' | 'U' => {
                    // OpenSCAD decodes `\xHH` and `\uXXXX`; every other
                    // sequence just drops the backslash (`"\0"` -> `"0"`).
                    let width = if escaped == 'x' { 2 } else { 4 };
                    let hex: String = chars.as_str().chars().take(width).collect();
                    let codepoint = if hex.chars().count() == width && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                        u32::from_str_radix(&hex, 16).ok()
                    } else {
                        None
                    };
                    match codepoint.and_then(char::from_u32) {
                        Some(decoded) => {
                            for _ in 0..width {
                                chars.next();
                            }
                            // `\xHH` values map directly onto Latin-1.
                            value.push(decoded);
                        }
                        None => value.push(escaped),
                    }
                }
                other => value.push(other),
            }
        }
        Ok(value)
    }
    
    fn parse_range(pair: Pair<Rule>) -> Result<Expression> {
        let span = Self::pest_span_to_span(pair.as_span());
        let expressions: Vec<_> = pair.into_inner()
            .map(Self::parse_expression)
            .collect::<Result<Vec<_>>>()?;
        let (start, step, end) = match expressions.as_slice() {
            [start, end] => (start.clone(), None, end.clone()),
            [start, step, end] => (start.clone(), Some(Box::new(step.clone())), end.clone()),
            _ => return Err(anyhow::anyhow!("Range requires two or three expressions")),
        };
        
        Ok(Expression::Range {
            start: Box::new(start),
            step,
            end: Box::new(end),
            span,
        })
    }
    
    fn parse_vector(pair: Pair<Rule>) -> Result<Expression> {
        let span = Self::pest_span_to_span(pair.as_span());
        let mut elements = Vec::new();
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::vector_element => {
                    // OpenSCAD drops empty elements: `[1,,3]` has length 2.
                    if let Some(expr_pair) = inner.into_inner().next() {
                        elements.push(Self::parse_expression(expr_pair)?);
                    }
                }
                Rule::expression => {
                    elements.push(Self::parse_expression(inner)?);
                }
                _ => {}
            }
        }
        
        Ok(Expression::Vector(elements, span))
    }

    fn parse_for_expr(pair: Pair<Rule>) -> Result<Expression> {
        let mut args = Vec::new();
        let mut expressions = Vec::new();
        let mut let_args = Vec::new();
        let span = Self::pest_span_to_span(pair.as_span());
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::argument => {
                    if let Some(argument) = Self::parse_argument(inner)? {
                        args.push(argument);
                    }
                }
                Rule::for_let => {
                    for binding in inner.into_inner() {
                        if let Rule::let_bindings = binding.as_rule() {
                            for binding in binding.into_inner() {
                                if let Rule::let_binding = binding.as_rule() {
                                    let mut name = String::new();
                                    let mut value = None;
                                    for item in binding.into_inner() {
                                        match item.as_rule() {
                                            Rule::identifier => name = item.as_str().to_string(),
                                            Rule::expression => value = Some(Self::parse_expression(item)?),
                                            _ => {}
                                        }
                                    }
                                    let_args.push(Assignment::new(name, value, Span::default()));
                                }
                            }
                        }
                    }
                }
                Rule::expression => {
                    expressions.push(Self::parse_expression(inner)?);
                }
                _ => {}
            }
        }
        let mut body = expressions.into_iter().next().ok_or_else(|| anyhow::anyhow!("Missing for body"))?;
        if !let_args.is_empty() {
            body = Expression::Let {
                args: let_args,
                body: Box::new(body),
                span,
            };
        }
        Ok(Expression::For {
            args,
            condition: None,
            body: Box::new(body),
            span,
        })
    }

    fn parse_for_c_expr(pair: Pair<Rule>) -> Result<Expression> {
        let span = Self::pest_span_to_span(pair.as_span());
        let mut init = Vec::new();
        let mut condition = None;
        let mut update = Vec::new();
        let mut let_args = Vec::new();
        let mut expressions = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::for_c_init => {
                    for arg in inner.into_inner() {
                        if arg.as_rule() == Rule::argument {
                            if let Some(argument) = Self::parse_argument(arg)? {
                                init.push(argument);
                            }
                        }
                    }
                }
                Rule::for_c_cond => {
                    if let Some(expr_pair) = inner.into_inner().next() {
                        condition = Some(Box::new(Self::parse_expression(expr_pair)?));
                    }
                }
                Rule::for_c_update => {
                    for arg in inner.into_inner() {
                        if arg.as_rule() == Rule::argument {
                            if let Some(argument) = Self::parse_argument(arg)? {
                                update.push(argument);
                            }
                        }
                    }
                }
                Rule::for_let => {
                    for binding in inner.into_inner() {
                        if let Rule::let_bindings = binding.as_rule() {
                            for binding in binding.into_inner() {
                                if let Rule::let_binding = binding.as_rule() {
                                    let mut name = String::new();
                                    let mut value = None;
                                    for item in binding.into_inner() {
                                        match item.as_rule() {
                                            Rule::identifier => name = item.as_str().to_string(),
                                            Rule::expression => value = Some(Self::parse_expression(item)?),
                                            _ => {}
                                        }
                                    }
                                    let_args.push(Assignment::new(name, value, Span::default()));
                                }
                            }
                        }
                    }
                }
                Rule::expression => {
                    expressions.push(Self::parse_expression(inner)?);
                }
                _ => {}
            }
        }

        let mut body = expressions.into_iter().next().ok_or_else(|| anyhow::anyhow!("Missing for_c body"))?;

        if !let_args.is_empty() {
            body = Expression::Let {
                args: let_args,
                body: Box::new(body),
                span,
            };
        }

        Ok(Expression::ForC {
            init,
            condition,
            update,
            body: Box::new(body),
            span,
        })
    }
    fn parse_each_expr(pair: Pair<Rule>) -> Result<Expression> {
        let span = Self::pest_span_to_span(pair.as_span());
        let mut body = None;
        for inner in pair.into_inner() {
            if let Rule::expression = inner.as_rule() {
                body = Some(Self::parse_expression(inner)?);
            }
        }
        Ok(Expression::Each {
            args: vec![],
            body: Box::new(body.ok_or_else(|| anyhow::anyhow!("Missing each body"))?),
            span,
        })
    }


    fn parse_assert_expr(pair: Pair<Rule>) -> Result<Expression> {
        let mut args = Vec::new();
        let mut body = None;
        let span = Self::pest_span_to_span(pair.as_span());
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::arguments => {
                    for arg in inner.into_inner() {
                        if let Rule::argument = arg.as_rule() {
                            if let Some(argument) = Self::parse_argument(arg)? {
                                args.push(argument);
                            }
                        }
                    }
                }
                Rule::expression => {
                    body = Some(Self::parse_expression(inner)?);
                }
                _ => {}
            }
        }
        
        Ok(Expression::Assert {
            args,
            body: body.map(Box::new),
            span,
        })
    }
    
    fn parse_echo_expr(pair: Pair<Rule>) -> Result<Expression> {
        let mut args = Vec::new();
        let mut body = None;
        let span = Self::pest_span_to_span(pair.as_span());
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::arguments => {
                    for arg in inner.into_inner() {
                        if let Rule::argument = arg.as_rule() {
                            if let Some(argument) = Self::parse_argument(arg)? {
                                args.push(argument);
                            }
                        }
                    }
                }
                Rule::expression => {
                    body = Some(Self::parse_expression(inner)?);
                }
                _ => {}
            }
        }
        
        Ok(Expression::Echo {
            args,
            body: body.map(Box::new),
            span,
        })
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple() {
        let input = "cube(10);";
        let result = Parser::parse(input);
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_parse_module() {
        let input = "module foo() { cube(10); }";
        let result = Parser::parse(input);
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_parse_function() {
        let input = "function foo(x) = x + 1;";
        let result = Parser::parse(input);
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_parse_assignment() {
        let input = "x = 10;";
        let result = Parser::parse(input);
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_parse_expressions() {
        let input = "x = 1 + 2 * 3;";
        let result = Parser::parse(input);
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_parse_vector() {
        let input = "x = [1, 2, 3];";
        let result = Parser::parse(input);
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_parse_if_else_if() {
        let input = "x = if (fa == 0) a else if (fa * fb < 0) root_find(f, a, b);";
        let result = Parser::parse(input);
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_ident_with_digits() {
        let input = "123_block = 10;";
        let result = Parser::parse(input);
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_ternary() {
        let input = "x = false ? 1 : [1, 2];";
        let result = Parser::parse(input);
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_logical_and_or() {
        let input = "x = true && true;";
        let parsed = Parser::parse(input).unwrap();
        if let crate::ast::Statement::Assignment(assign) = &parsed.statements[0] {
            if let Some(crate::ast::Expression::BinaryOp(op, left, right, _)) = &assign.value {
                assert_eq!(*op, crate::ast::BinaryOp::LogicalAnd);
                assert!(matches!(**left, crate::ast::Expression::Literal(crate::ast::LiteralValue::Bool(true), _)));
                assert!(matches!(**right, crate::ast::Expression::Literal(crate::ast::LiteralValue::Bool(true), _)));
            } else {
                panic!("Expected BinaryOp");
            }
        }
    }

    #[test]
    fn test_parse_vector_empty_elements() {
        let input = "x = [1, , 3];";
        let parsed = Parser::parse(input).unwrap();
        if let crate::ast::Statement::Assignment(assign) = &parsed.statements[0] {
            if let Some(crate::ast::Expression::Vector(elems, _)) = &assign.value {
                // OpenSCAD drops empty elements: `echo([1,,3])` prints `[1, 3]`.
                assert_eq!(elems.len(), 2);
                assert!(matches!(elems[0], crate::ast::Expression::Literal(crate::ast::LiteralValue::Number(_), _)));
                assert!(matches!(elems[1], crate::ast::Expression::Literal(crate::ast::LiteralValue::Number(_), _)));
            } else {
                panic!("Expected Vector");
            }
        }
    }
}

