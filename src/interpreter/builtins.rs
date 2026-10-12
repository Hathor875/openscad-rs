use std::collections::HashMap;
use crate::interpreter::value::Value;

type BuiltinFn = fn(Vec<Value>) -> Result<Value, String>;

pub struct Builtins {
    functions: HashMap<String, BuiltinFn>,
}

impl Builtins {
    pub fn new() -> Self {
        let mut functions = HashMap::new();
        
        functions.insert("sin".to_string(), builtin_sin as BuiltinFn);
        functions.insert("cos".to_string(), builtin_cos as BuiltinFn);
        functions.insert("tan".to_string(), builtin_tan as BuiltinFn);
        functions.insert("asin".to_string(), builtin_asin as BuiltinFn);
        functions.insert("acos".to_string(), builtin_acos as BuiltinFn);
        functions.insert("atan".to_string(), builtin_atan as BuiltinFn);
        functions.insert("atan2".to_string(), builtin_atan2 as BuiltinFn);
        
        functions.insert("sqrt".to_string(), builtin_sqrt as BuiltinFn);
        functions.insert("pow".to_string(), builtin_pow as BuiltinFn);
        functions.insert("exp".to_string(), builtin_exp as BuiltinFn);
        functions.insert("ln".to_string(), builtin_ln as BuiltinFn);
        functions.insert("log".to_string(), builtin_log as BuiltinFn);
        functions.insert("log10".to_string(), builtin_log10 as BuiltinFn);
        
        functions.insert("abs".to_string(), builtin_abs as BuiltinFn);
        functions.insert("floor".to_string(), builtin_floor as BuiltinFn);
        functions.insert("ceil".to_string(), builtin_ceil as BuiltinFn);
        functions.insert("round".to_string(), builtin_round as BuiltinFn);
        functions.insert("sign".to_string(), builtin_sign as BuiltinFn);
        
        functions.insert("min".to_string(), builtin_min as BuiltinFn);
        functions.insert("max".to_string(), builtin_max as BuiltinFn);
        
        functions.insert("len".to_string(), builtin_len as BuiltinFn);
        functions.insert("norm".to_string(), builtin_norm as BuiltinFn);
        functions.insert("cross".to_string(), builtin_cross as BuiltinFn);
        
        functions.insert("str".to_string(), builtin_str as BuiltinFn);
        functions.insert("chr".to_string(), builtin_chr as BuiltinFn);
        functions.insert("search".to_string(), builtin_search as BuiltinFn);
        functions.insert("version".to_string(), builtin_version as BuiltinFn);
        functions.insert("version_num".to_string(), builtin_version_num as BuiltinFn);
        
        functions.insert("undef".to_string(), builtin_undef as BuiltinFn);
        functions.insert("is_undef".to_string(), builtin_is_undef as BuiltinFn);
        functions.insert("is_bool".to_string(), builtin_is_bool as BuiltinFn);
        functions.insert("is_num".to_string(), builtin_is_num as BuiltinFn);
        functions.insert("is_string".to_string(), builtin_is_string as BuiltinFn);
        functions.insert("is_list".to_string(), builtin_is_list as BuiltinFn);
        
        Self { functions }
    }
    
    pub fn get_function(&self, name: &str) -> Option<BuiltinFn> {
        self.functions.get(name).copied()
    }
}

fn expect_num(args: &[Value], n: usize) -> Result<Vec<f64>, String> {
    if args.len() != n {
        return Err(format!("Expected {} arguments, got {}", n, args.len()));
    }
    args.iter().map(|v| v.as_number()).collect()
}

fn expect_vec(args: &[Value], n: usize) -> Result<Vec<Vec<Value>>, String> {
    if args.len() != n {
        return Err(format!("Expected {} arguments, got {}", n, args.len()));
    }
    args.iter().map(|v| match v {
        Value::Vector(v) => Ok(v.clone()),
        _ => Err("Expected vector".into()),
    }).collect()
}

fn builtin_sin(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    Ok(Value::Number(nums[0].to_radians().sin()))
}

fn builtin_cos(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    Ok(Value::Number(nums[0].to_radians().cos()))
}

fn builtin_tan(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    Ok(Value::Number(nums[0].to_radians().tan()))
}

fn builtin_asin(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    Ok(Value::Number(nums[0].asin().to_degrees()))
}

fn builtin_acos(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    Ok(Value::Number(nums[0].acos().to_degrees()))
}

fn builtin_atan(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    Ok(Value::Number(nums[0].atan().to_degrees()))
}

fn builtin_atan2(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 2)?;
    Ok(Value::Number(nums[1].atan2(nums[0]).to_degrees()))
}

fn builtin_sqrt(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    if nums[0] < 0.0 { return Err("sqrt of negative number".into()); }
    Ok(Value::Number(nums[0].sqrt()))
}

fn builtin_pow(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 2)?;
    Ok(Value::Number(nums[0].powf(nums[1])))
}

fn builtin_exp(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    Ok(Value::Number(nums[0].exp()))
}

fn builtin_ln(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    if nums[0] <= 0.0 { return Err("ln of non-positive number".into()); }
    Ok(Value::Number(nums[0].ln()))
}

fn builtin_log(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, if args.len() == 1 { 1 } else { 2 })?;
    if args.len() == 1 {
        if nums[0] <= 0.0 { return Err("log of non-positive number".into()); }
        Ok(Value::Number(nums[0].ln()))
    } else {
        if nums[0] <= 0.0 || nums[1] <= 0.0 { return Err("log of non-positive number".into()); }
        Ok(Value::Number(nums[0].ln() / nums[1].ln()))
    }
}

fn builtin_log10(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    if nums[0] <= 0.0 { return Err("log10 of non-positive number".into()); }
    Ok(Value::Number(nums[0].log10()))
}

fn builtin_abs(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    Ok(Value::Number(nums[0].abs()))
}

fn builtin_floor(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    Ok(Value::Number(nums[0].floor()))
}

fn builtin_ceil(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    Ok(Value::Number(nums[0].ceil()))
}

fn builtin_round(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    Ok(Value::Number(nums[0].round()))
}

fn builtin_sign(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    Ok(Value::Number(if nums[0] > 0.0 { 1.0 } else if nums[0] < 0.0 { -1.0 } else { 0.0 }))
}

fn builtin_min(args: Vec<Value>) -> Result<Value, String> {
    if args.is_empty() { return Err("min expects at least 1 argument".into()); }
    let nums = expect_num(&args, args.len())?;
    Ok(Value::Number(nums.into_iter().fold(f64::INFINITY, f64::min)))
}

fn builtin_max(args: Vec<Value>) -> Result<Value, String> {
    if args.is_empty() { return Err("max expects at least 1 argument".into()); }
    let nums = expect_num(&args, args.len())?;
    Ok(Value::Number(nums.into_iter().fold(f64::NEG_INFINITY, f64::max)))
}

fn builtin_len(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("len expects 1 argument".into()); }
    match &args[0] {
        Value::Vector(v) => Ok(Value::Number(v.len() as f64)),
        Value::String(s) => Ok(Value::Number(s.len() as f64)),
        _ => Err("len expects vector or string".into()),
    }
}

fn builtin_norm(args: Vec<Value>) -> Result<Value, String> {
    let vecs = expect_vec(&args, 1)?;
    let sum: f64 = vecs[0].iter().filter_map(|v| v.as_number().ok()).map(|x| x * x).sum();
    Ok(Value::Number(sum.sqrt()))
}

fn builtin_cross(args: Vec<Value>) -> Result<Value, String> {
    let vecs = expect_vec(&args, 2)?;
    let a = &vecs[0]; let b = &vecs[1];
    if a.len() != 3 || b.len() != 3 { return Err("cross product requires 3D vectors".into()); }
    let ax = a[0].as_number()?; let ay = a[1].as_number()?; let az = a[2].as_number()?;
    let bx = b[0].as_number()?; let by = b[1].as_number()?; let bz = b[2].as_number()?;
    Ok(Value::Vector(vec![
        Value::Number(ay * bz - az * by),
        Value::Number(az * bx - ax * bz),
        Value::Number(ax * by - ay * bx),
    ]))
}

fn builtin_str(args: Vec<Value>) -> Result<Value, String> {
    let mut result = String::new();
    for arg in args {
        result.push_str(&arg.as_string()?);
    }
    Ok(Value::String(result))
}

fn builtin_chr(args: Vec<Value>) -> Result<Value, String> {
    let nums = expect_num(&args, 1)?;
    let c = char::from_u32(nums[0] as u32).ok_or("Invalid Unicode code point")?;
    Ok(Value::String(c.to_string()))
}

fn builtin_search(args: Vec<Value>) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 { return Err("search expects 2 or 3 arguments".into()); }
    let pattern = args[0].as_string()?;
    let text = args[1].as_string()?;
    let _case_insensitive = args.get(2).map(|v| v.as_bool()).unwrap_or(false);
    
    if let Some(pos) = text.find(&pattern) {
        Ok(Value::Number(pos as f64))
    } else {
        Ok(Value::Undef)
    }
}

fn builtin_version(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() { return Err("version expects no arguments".into()); }
    Ok(Value::Vector(vec![
        Value::Number(2024.01),
        Value::Number(0.0),
        Value::Number(0.0),
    ]))
}

fn builtin_version_num(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("version_num expects no arguments".into());
    }
    Ok(Value::Number(20240100.0))
}

fn builtin_undef(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() { return Err("undef expects no arguments".into()); }
    Ok(Value::Undef)
}

fn builtin_is_undef(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("is_undef expects 1 argument".into()); }
    Ok(Value::Bool(args[0].is_undef()))
}

fn builtin_is_bool(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("is_bool expects 1 argument".into()); }
    Ok(Value::Bool(matches!(args[0], Value::Bool(_))))
}

fn builtin_is_num(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("is_num expects 1 argument".into()); }
    Ok(Value::Bool(matches!(args[0], Value::Number(_))))
}

fn builtin_is_string(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("is_string expects 1 argument".into()); }
    Ok(Value::Bool(matches!(args[0], Value::String(_))))
}

fn builtin_is_list(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("is_list expects 1 argument".into()); }
    Ok(Value::Bool(matches!(args[0], Value::Vector(_))))
}