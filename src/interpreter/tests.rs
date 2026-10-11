#[cfg(test)]
mod tests {
    use crate::parser::Parser;
    use crate::interpreter::{Executor, Environment};
    use std::rc::Rc;

    fn eval_source(input: &str) -> Result<crate::interpreter::Value, String> {
        let parsed = Parser::parse(input).map_err(|e| format!("Parse error: {:?}", e))?;
        let env = Rc::new(Environment::new());
        let mut exec = Executor::new(env);
        for stmt in &parsed.statements {
            exec.execute(stmt)?;
        }
        Ok(crate::interpreter::Value::Undef)
    }

    fn eval_expr(input: &str) -> Result<crate::interpreter::Value, String> {
        let parsed = Parser::parse(input).map_err(|e| format!("Parse error: {:?}", e))?;
        let env = Rc::new(Environment::new());
        let mut exec = Executor::new(env);
        for stmt in &parsed.statements {
            exec.execute(stmt)?;
        }
        Ok(crate::interpreter::Value::Undef)
    }

    #[test]
    fn test_eval_assignment() {
        let input = "x = 10;";
        let result = eval_source(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_eval_expression() {
        let input = "x = 1 + 2 * 3;";
        let result = eval_source(input);
        if let Err(e) = &result {
            eprintln!("Error: {}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_eval_vector() {
        let input = "x = [1, 2, 3];";
        let result = eval_source(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_eval_range() {
        let input = "x = [0:2:10];";
        let result = eval_source(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_eval_function() {
        let input = "function foo(x) = x + 1;";
        let result = eval_source(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_eval_module() {
        let input = "module foo() { cube(10); }";
        let result = eval_source(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_eval_builtin_sin() {
        let input = "x = sin(90);";
        let parsed = Parser::parse(input);
        println!("Parse result: {:?}", parsed);
        let result = eval_source(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_eval_if() {
        let input = "if (true) { x = 1; }";
        let parsed = Parser::parse(input);
        println!("Parse result: {:?}", parsed);
        let result = eval_source(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_eval_let() {
        let input = "x = let(a = 1, b = 2) a + b;";
        let parsed = Parser::parse(input);
        println!("Parse result: {:?}", parsed);
        let result = eval_source(input);
        assert!(result.is_ok());
    }
}