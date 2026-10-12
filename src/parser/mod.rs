use crate::ast::{SourceFile, Statement, Assignment, Expression, ModuleModifiers, Span, LiteralValue, UnaryOp, BinaryOp};
use pest::Parser as PestParser;
use pest::iterators::Pair;
use pest::Span as PestSpan;
use pest_derive::Parser;
use text_size::TextSize;
use anyhow::Result;

#[derive(Parser)]
#[grammar = "parser/openscad.pest"]
struct OpenSCADParser;

pub struct Parser;

impl Parser {
    pub fn parse(input: &str) -> Result<SourceFile> {
        let pairs = OpenSCADParser::parse(Rule::module, input)?;
        // eprintln!("DEBUG parse: pairs count = {}", pairs.len());
        let mut file = SourceFile::new(String::new());
        
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
                            if let Some(inner2) = inner.clone().into_inner().next() {
                                let path = inner2.as_str().to_string();
                                file.statements.push(Statement::Include(path, Span::default()));
                            }
                        }
                        Rule::use_directive => {
                            if let Some(inner2) = inner.clone().into_inner().next() {
                                let path = inner2.as_str().to_string();
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
                        _ => Ok(None),
                    }
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
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
                    name = inner.as_str().to_string();
                }
                Rule::arguments => {
                    for arg in inner.into_inner() {
                        if let Rule::argument = arg.as_rule() {
                            args.push(Self::parse_argument(arg)?);
                        }
                    }
                }
                Rule::child_block => {
                    for stmt in inner.into_inner() {
                        if let Rule::statement = stmt.as_rule() {
                            if let Some(s) = Self::parse_statement(stmt)? {
                                children.push(s);
                            }
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
    
    fn parse_argument(pair: Pair<Rule>) -> Result<Assignment> {
        let span = Self::pest_span_to_span(pair.as_span());
        let mut name = String::new();
        let mut value = None;
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::expression => {
                    value = Some(Self::parse_expression(inner)?);
                }
                Rule::identifier => {
                    name = inner.as_str().to_string();
                }
                Rule::EqSign => {
                    // Skip the equals sign
                }
                _ => {}
            }
        }
        
        if !name.is_empty() {
            Ok(Assignment::new(name, value, span))
        } else {
            Ok(Assignment::new(String::new(), value, span))
        }
    }
    
    fn parse_assignment(pair: Pair<Rule>) -> Result<Statement> {
        let mut name = String::new();
        let mut value = None;
        let span = Self::pest_span_to_span(pair.as_span());
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => {
                    name = inner.as_str().to_string();
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
                    name = inner.as_str().to_string();
                }
                Rule::parameters => {
                    for param in inner.into_inner() {
                        if let Rule::parameter = param.as_rule() {
                            params.push(Self::parse_parameter(param)?);
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
                    if then_branch.is_none() {
                        then_branch = Some(Box::new(Self::parse_statement(inner)?.ok_or_else(|| anyhow::anyhow!("Empty then branch"))?));
                    } else {
                        else_branch = Some(Box::new(Self::parse_statement(inner)?.ok_or_else(|| anyhow::anyhow!("Empty else branch"))?));
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
                    name = inner.as_str().to_string();
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
                    name = inner.as_str().to_string();
                }
                Rule::parameters => {
                    for param in inner.into_inner() {
                        if let Rule::parameter = param.as_rule() {
                            params.push(Self::parse_parameter(param)?);
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
                let condition = Self::parse_expression(inner.next().unwrap())?;
                if let Some(then_expr) = inner.next() {
                    let then_expr = Self::parse_expression(then_expr)?;
                    let else_expr = Self::parse_expression(inner.next().unwrap())?;
                    let span = Self::pest_span_to_span(pair.as_span());
                    Ok(Expression::TernaryOp(
                        Box::new(condition),
                        Box::new(then_expr),
                        Box::new(else_expr),
                        span,
                    ))
                } else {
                    Ok(condition)
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
        
        for right_pair in inner {
            let right = Self::parse_expression(right_pair)?;
            left = Expression::BinaryOp(
                op,
                Box::new(left),
                Box::new(right),
                Self::pest_span_to_span(pair.as_span()),
            );
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
        let mut expr = None;
        
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::postfix | Rule::unary => {
                    expr = Some(Self::parse_expression(inner)?);
                }
                Rule::Minus | Rule::Bang | Rule::Tilde => {
                    let op_str = inner.as_str();
                    if let Some(e) = expr.take() {
                        let op = match op_str {
                            "-" => UnaryOp::Negate,
                            "!" => UnaryOp::Not,
                            "~" => UnaryOp::BinaryNot,
                            _ => UnaryOp::Negate,
                        };
                        expr = Some(Expression::UnaryOp(op, Box::new(e), span));
                    }
                }
                _ => {}
            }
        }
        
        expr.ok_or_else(|| anyhow::anyhow!("Empty unary expression"))
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
                    for suffix in inner.into_inner() {
                        match suffix.as_rule() {
                            Rule::arguments => {
                                let mut args = Vec::new();
                                for arg in suffix.into_inner() {
                                    if let Rule::argument = arg.as_rule() {
                                        args.push(Self::parse_argument(arg)?);
                                    }
                                }
                                if let Some(e) = base_expr.take() {
                                    base_expr = Some(Expression::FunctionCall {
                                        name: match &e {
                                            Expression::Lookup(n, _) => n.clone(),
                                            _ => "".to_string(),
                                        },
                                        args,
                                        expr: None,
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
                                        suffix.as_str().to_string(),
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
                Rule::assert_expr => return Self::parse_assert_expr(inner),
                Rule::for_expr => return Self::parse_for_expr(inner),
                Rule::each_expr => return Self::parse_each_expr(inner),
                Rule::echo_expr => return Self::parse_echo_expr(inner),
                _ => return Self::parse_expression(inner),
            }
        }
        Ok(Expression::Literal(LiteralValue::Undefined, Span::default()))
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
        Ok(content.to_string())
    }
    
    fn parse_range(pair: Pair<Rule>) -> Result<Expression> {
        let span = Self::pest_span_to_span(pair.as_span());
        let mut inner = pair.into_inner();
        
        let start = Self::parse_expression(inner.next().unwrap())?;
        let end = Self::parse_expression(inner.next().unwrap())?;
        let step = inner.next().map(Self::parse_expression).transpose()?;
        
        Ok(Expression::Range {
            start: Box::new(start),
            step: step.map(Box::new),
            end: Box::new(end),
            span,
        })
    }
    
    fn parse_vector(pair: Pair<Rule>) -> Result<Expression> {
        let span = Self::pest_span_to_span(pair.as_span());
        let mut elements = Vec::new();
        
        for inner in pair.into_inner() {
            if let Rule::expression = inner.as_rule() {
                elements.push(Self::parse_expression(inner)?);
            }
        }
        
        Ok(Expression::Vector(elements, span))
    }

    fn parse_for_expr(pair: Pair<Rule>) -> Result<Expression> {
        let mut args = Vec::new();
        let mut body = None;
        let span = Self::pest_span_to_span(pair.as_span());
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::argument => {
                    args.push(Self::parse_argument(inner)?);
                }
                Rule::expression => {
                    body = Some(Self::parse_expression(inner)?);
                }
                _ => {}
            }
        }
        Ok(Expression::For {
            args,
            body: Box::new(body.ok_or_else(|| anyhow::anyhow!("Missing for body"))?),
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
                            args.push(Self::parse_argument(arg)?);
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
                            args.push(Self::parse_argument(arg)?);
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
    fn test_parse_range() {
        let input = "x = [0:10];";
        let result = Parser::parse(input);
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }
}
