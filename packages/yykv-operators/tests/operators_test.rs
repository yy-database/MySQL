use yykv_operators::*;
use yykv_types::DsValue;

#[test]
fn test_expr_eval() {
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("age".into(), DsValue::Int(25));
    let row = DsValue::Dict(fields);

    let expr = Expr::BinaryOp(
        Box::new(Expr::Column("age".into())),
        BinaryOperator::Eq,
        Box::new(Expr::Literal(DsValue::Int(25))),
    );

    assert_eq!(expr.evaluate(&row), Ok(DsValue::Bool(true)));
}
