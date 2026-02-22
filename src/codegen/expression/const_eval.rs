use super::*;

pub fn evaluate_const_expr(_codegen: &CodeGen, expr: &Expression) -> Result<DefineValue> {
    match expr {
        Expression::Literal(lit) => match lit {
            Literal::Integer(v, _) => Ok(DefineValue::Int(*v)),
            Literal::Float(v, _) => Ok(DefineValue::Float(*v)),
            Literal::Bool(v, _) => Ok(DefineValue::Bool(*v)),
            Literal::String(v, _) => Ok(DefineValue::String(v.clone())),
            Literal::MultilineString(v, _) => Ok(DefineValue::String(v.clone())),
            Literal::Char(c, _) => Ok(DefineValue::Int(*c as i64)),
            _ => Err(anyhow!("Unsupported literal in const expr: {:?}", lit)),
        },
        Expression::New(new_expr) => {
            if new_expr.type_name != "Error" || new_expr.args.len() != 1 {
                return Err(anyhow!("Unsupported const expr"));
            }
            match &new_expr.args[0] {
                Expression::Literal(Literal::String(msg, _))
                | Expression::Literal(Literal::MultilineString(msg, _)) => {
                    Ok(DefineValue::Error(msg.clone()))
                }
                _ => Err(anyhow!("Unsupported const expr")),
            }
        }
        _ => Err(anyhow!("Unsupported const expr")),
    }
}
