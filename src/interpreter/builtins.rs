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
        functions.insert("ident".to_string(), builtin_ident as BuiltinFn);
        
        functions.insert("undef".to_string(), builtin_undef as BuiltinFn);
        functions.insert("is_undef".to_string(), builtin_is_undef as BuiltinFn);
        functions.insert("is_bool".to_string(), builtin_is_bool as BuiltinFn);
        functions.insert("is_num".to_string(), builtin_is_num as BuiltinFn);
        functions.insert("is_string".to_string(), builtin_is_string as BuiltinFn);
        functions.insert("is_list".to_string(), builtin_is_list as BuiltinFn);
        functions.insert("is_vector".to_string(), builtin_is_list as BuiltinFn);
        functions.insert("is_num".to_string(), builtin_is_num as BuiltinFn);
        functions.insert("is_int".to_string(), builtin_is_int as BuiltinFn);
        functions.insert("is_finite".to_string(), builtin_is_finite as BuiltinFn);
        functions.insert("is_nan".to_string(), builtin_is_nan as BuiltinFn);
        functions.insert("is_def".to_string(), builtin_is_def as BuiltinFn);
        functions.insert("last".to_string(), builtin_last as BuiltinFn);
        functions.insert("default".to_string(), builtin_default as BuiltinFn);
        functions.insert("concat".to_string(), builtin_concat as BuiltinFn);
        functions.insert("select".to_string(), builtin_select as BuiltinFn);
        functions.insert("posmod".to_string(), builtin_posmod as BuiltinFn);
        functions.insert("ord".to_string(), builtin_ord as BuiltinFn);
        functions.insert("rands".to_string(), builtin_rands as BuiltinFn);
        
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

fn builtin_ord(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("ord expects 1 argument".into());
    }
    let s = args[0].as_string()?;
    if s.is_empty() {
        return Err("ord expects a non-empty string".into());
    }
    let code = s.chars().next().unwrap() as u32;
    Ok(Value::Number(code as f64))
}

fn builtin_rands(args: Vec<Value>) -> Result<Value, String> {
    if args.len() < 3 || args.len() > 4 {
        return Err("rands expects 3 or 4 arguments".into());
    }
    let min = args[0].as_number()?;
    let max = args[1].as_number()?;
    let count = args[2].as_number()?;

    let mut state: u64 = if args.len() == 4 {
        args[3].as_number()? as u64
    } else {
        123456789u64
    };

    fn xorshift64(state: &mut u64) -> u64 {
        *state ^= *state >> 12;
        *state ^= *state << 25;
        *state ^= *state >> 27;
        (*state).wrapping_mul(0x2545F4914f6cdd1d)
    }

    let count = count.trunc().max(0.0) as usize;
    let mut result = Vec::new();
    for _ in 0..count {
        let r = xorshift64(&mut state) as f64 / (u64::MAX as f64);
        let value = min + r * (max - min);
        result.push(Value::Number(value));
    }
    Ok(Value::Vector(result))
}

fn builtin_search(args: Vec<Value>) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 { return Err("search expects 2 or 3 arguments".into()); }
    let pattern = args[0].as_string()?;
    let text = &args[1];
    
    match text {
        Value::String(s) => {
            let _case_insensitive = args.get(2).map(|v| v.as_bool()).unwrap_or(false);
            
            if let Some(pos) = s.find(&pattern) {
                Ok(Value::Number(pos as f64))
            } else {
                Ok(Value::Undef)
            }
        }
        Value::Vector(vec) => {
            let case_insensitive = args.get(2).map(|v| v.as_bool()).unwrap_or(false);
            let mut indices: Vec<Value> = Vec::new();
            for (i, element) in vec.iter().enumerate() {
                let element_str = element.as_string();
                if let Ok(es) = element_str {
                    if case_insensitive {
                        if es.to_lowercase().contains(&pattern.to_lowercase()) {
                            indices.push(Value::Number(i as f64));
                        }
                    } else if es.contains(&pattern) {
                        indices.push(Value::Number(i as f64));
                    }
                }
            }
            if indices.is_empty() {
                Ok(Value::Undef)
            } else if indices.len() == 1 {
                Ok(indices[0].clone())
            } else {
                Ok(Value::Vector(indices))
            }
        }
        _ => Err("search expects string or vector as second argument".into()),
    }
}

fn builtin_version(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() { return Err("version expects no arguments".into()); }
    Ok(Value::Vector(vec![
        Value::Number(2021.0),
        Value::Number(1.0),
        Value::Number(0.0),
    ]))
}

fn builtin_version_num(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("version_num expects no arguments".into());
    }

    Ok(Value::Number(20210100.0))
}

fn builtin_ident(args: Vec<Value>) -> Result<Value, String> {
    let size = expect_num(&args, 1)?[0];
    if size < 1.0 || size.fract() != 0.0 || size > 32.0 {
        return Err("ident size must be an integer between 1 and 32".into());
    }
    let size = size as usize;
    Ok(Value::Vector((0..size).map(|row| {
        Value::Vector((0..size).map(|column| {
            Value::Number(if row == column { 1.0 } else { 0.0 })
        }).collect())
    }).collect()))
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

fn builtin_is_int(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("is_int expects 1 argument".into()); }
    Ok(Value::Bool(matches!(args[0], Value::Number(n) if n.is_finite() && n.fract() == 0.0)))
}

fn builtin_is_finite(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("is_finite expects 1 argument".into()); }
    Ok(Value::Bool(matches!(args[0], Value::Number(n) if n.is_finite())))
}

fn builtin_is_nan(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("is_nan expects 1 argument".into()); }
    Ok(Value::Bool(matches!(args[0], Value::Number(n) if n.is_nan())))
}

fn builtin_is_def(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("is_def expects 1 argument".into()); }
    Ok(Value::Bool(!args[0].is_undef()))
}

fn builtin_last(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("last expects 1 argument".into()); }
    match &args[0] {
        Value::Vector(values) => values.last().cloned().ok_or_else(|| "last expects a non-empty list".into()),
        _ => Err("last expects a list".into()),
    }
}

fn builtin_default(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 { return Err("default expects 2 arguments".into()); }
    Ok(if args[0].is_undef() { args[1].clone() } else { args[0].clone() })
}

fn builtin_concat(args: Vec<Value>) -> Result<Value, String> {
    let mut result = Vec::new();
    for value in args {
        match value {
            Value::Vector(values) => result.extend(values),
            value => result.push(value),
        }
    }
    Ok(Value::Vector(result))
}

fn builtin_select(args: Vec<Value>) -> Result<Value, String> {
    if args.len() < 2 { return Err("select expects a list and index".into()); }
    let values = match &args[0] {
        Value::Vector(values) => values,
        _ => return Err("select expects a list as its first argument".into()),
    };
    let index = |value: &Value| -> Result<isize, String> {
        Ok(value.as_number()?.trunc() as isize)
    };
    let get = |position: isize| -> Result<Value, String> {
        let position = if position < 0 {
            values.len() as isize + position
        } else {
            position
        };
        values.get(position as usize)
            .cloned()
            .ok_or_else(|| "select index out of bounds".to_string())
    };
    if args.len() == 2 {
        return match &args[1] {
            Value::Vector(indices) => Ok(Value::Vector(
                indices
                    .iter()
                    .map(|value| get(index(value)?))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            value => get(index(value)?),
        };
    }
    if args.len() == 3 {
        let start = index(&args[1])?;
        let end = index(&args[2])?;
        let step = if start <= end { 1 } else { -1 };
        let mut result = Vec::new();
        let mut current = start;
        loop {
            result.push(get(current)?);
            if current == end { break; }
            current += step;
        }
        return Ok(Value::Vector(result));
    }
    Err("select expects 2 or 3 arguments".into())
}

fn builtin_posmod(args: Vec<Value>) -> Result<Value, String> {
    let values = expect_num(&args, 2)?;
    if values[1] == 0.0 { return Err("posmod divisor cannot be zero".into()); }
    Ok(Value::Number(values[0].rem_euclid(values[1].abs())))
}