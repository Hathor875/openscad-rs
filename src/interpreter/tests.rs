#[cfg(test)]
mod tests {
    use crate::parser::Parser;
    use crate::interpreter::{Executor, Environment};
    use std::rc::Rc;

    fn eval_source(input: &str) -> Result<Rc<Environment>, String> {
        let parsed = Parser::parse(input).map_err(|e| format!("Parse error: {:?}", e))?;
        let env = Rc::new(Environment::new());
        let mut exec = Executor::new(env.clone());
        for stmt in &parsed.statements {
            exec.execute(stmt)?;
        }
        Ok(env)
    }

    #[test]
    fn test_eval_assignment() {
        let input = "x = 10;";
        let env = eval_source(input).unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(10.0)));
    }

    #[test]
    fn test_eval_expression() {
        let input = "x = 1 + 2 * 3;";
        let env = eval_source(input).unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(7.0)));
    }

    #[test]
    fn test_eval_vector() {
        let input = "x = [1, 2, 3];";
        let env = eval_source(input).unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Vector(vec![
            crate::interpreter::Value::Number(1.0),
            crate::interpreter::Value::Number(2.0),
            crate::interpreter::Value::Number(3.0),
        ])));
    }

    #[test]
    fn test_eval_range() {
        let input = "x = [0:2:10];";
        let env = eval_source(input).unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Vector(vec![
            crate::interpreter::Value::Number(0.0),
            crate::interpreter::Value::Number(2.0),
            crate::interpreter::Value::Number(4.0),
            crate::interpreter::Value::Number(6.0),
            crate::interpreter::Value::Number(8.0),
            crate::interpreter::Value::Number(10.0),
        ])));
    }

    #[test]
    fn test_eval_function() {
        let env = eval_source("function foo(x) = x + 1; y = foo(4);").unwrap();
        assert_eq!(env.get("y"), Some(crate::interpreter::Value::Number(5.0)));
    }

    #[test]
    fn test_eval_module() {
        let input = "module foo() { cube(10); }";
        assert!(eval_source(input).is_ok());
    }

    #[test]
    fn test_eval_builtin_sin() {
        let input = "x = sin(90);";
        let parsed = Parser::parse(input);
        println!("Parse result: {:?}", parsed);
        let env = eval_source(input).unwrap();
        assert!(matches!(env.get("x"), Some(crate::interpreter::Value::Number(_))));
    }

    #[test]
    fn test_eval_if() {
        let input = "if (true) { x = 1; }";
        let parsed = Parser::parse(input);
        println!("Parse result: {:?}", parsed);
        let env = eval_source(input).unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(1.0)));
    }

    #[test]
    fn test_eval_let() {
        let input = "x = let(a = 1, b = 2) a + b;";
        let parsed = Parser::parse(input);
        println!("Parse result: {:?}", parsed);
        let env = eval_source(input).unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(3.0)));
    }

    #[test]
    fn test_unary_and_exponent() {
        let env = eval_source("x = -2 ^ 3; y = !false;").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(-8.0)));
        assert_eq!(env.get("y"), Some(crate::interpreter::Value::Bool(true)));
    }

    #[test]
    fn test_named_function_argument() {
        let env = eval_source("function add(a, b) = a + b; x = add(b = 4, a = 3);").unwrap();
        assert_eq!(env.get("x"), Some(crate::interpreter::Value::Number(7.0)));
    }
}