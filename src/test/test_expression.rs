use crate::expression::resolve_helper::parse_str_axis_to_vec3;
use crate::{CataContext, eval_str_to_f64};

#[test]
fn test_parse_expression() {
    let str = "MIN(1.0, 2.0)";
    let context = CataContext::default();
    let value = crate::eval_str_to_f64(str, &context, "").unwrap();
    dbg!(value);
}

#[test]
fn test_parse_complex_expression() {
    let context = CataContext::default();
    context.insert("DESP63".to_string(), "0.2".to_string());
    context.insert("DESP7".to_string(), "0.2".to_string());
    context.insert("DESP28".to_string(), "1.0".to_string());
    context.insert("DDANGLE".to_string(), "1.0".to_string());

    let dir_str = "Z DDANGLE X";
    let dir = parse_str_axis_to_vec3(dir_str, &context).unwrap_or_default();
    dbg!(dir);

    let str = "( MAX (0.1, DESP[63 ] ) )";
    let d = eval_str_to_f64(str, &context, "").unwrap_or_default();
    dbg!(d);
    assert_eq!(d, 0.2);

    let dir_str = "Z ( MAX (0.1, DESP[63 ] ) ) -X ( 90.0 ) Y";
    let dir = parse_str_axis_to_vec3(dir_str, &context).unwrap_or_default();
    dbg!(dir);
    assert_eq!(dir.y, 1.0);

    let test_if = "if(0.0==1.0, 1.0, 2.0+5.0)";
    use evalexpr::*;
    dbg!(eval(test_if));

    // min (500,3000)
    let mut test_if = "min (500,3000) ";
    let result = eval_str_to_f64(test_if, &context, "").unwrap();
    dbg!(result);

    let mut test_if = "( IFTRUE (DESP[7] LT 0, DESP[28] / 2, -1 * DESP[28] / 2 ))";
    let result = eval_str_to_f64(test_if, &context, "").unwrap();
    dbg!(result);
    assert_eq!(result, -0.5);
}
