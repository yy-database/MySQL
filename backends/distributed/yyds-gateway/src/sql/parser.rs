use crate::sql::ast::{self, SqlStatement};
use oak_sql::ast::{self as oak_ast, SqlRoot};
use yykv_operators::{BinaryOperator, Expr, JoinType, OpsGraph, OperatorNode, SourceOp, ProjectOp, FilterOp};
use yykv_types::{DsError, DsResult, DsValue, Vector};
use uuid::Uuid;

pub struct SqlParser;

impl SqlParser {
    pub fn parse_sql(sql: &str) -> DsResult<SqlStatement> {
        let root = oak_sql::parse(sql).map_err(|e| DsError::parse(e))?;
        
        if let Some(stmt) = root.statements.first() {
            match stmt {
                oak_ast::SqlStatement::Select(s) => Self::process_select(s),
                oak_ast::SqlStatement::Insert(s) => Self::process_insert(s),
                oak_ast::SqlStatement::Update(s) => Self::process_update(s),
                oak_ast::SqlStatement::Delete(s) => Self::process_delete(s),
                oak_ast::SqlStatement::Create(s) => Self::process_create(s),
                oak_ast::SqlStatement::Drop(s) => Self::process_drop(s),
                oak_ast::SqlStatement::Unknown { .. } => Err(DsError::parse("Unknown SQL statement")),
                _ => Err(DsError::parse("Unsupported statement type")),
            }
        } else {
            Err(DsError::parse("Empty SQL"))
        }
    }

    fn process_select(stmt: &oak_ast::SelectStatement) -> DsResult<SqlStatement> {
        // Simplified implementation for now
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        
        // FROM
        let table_name = if let Some(from) = &stmt.from {
             from.name.name.to_string()
        } else {
             "__dual__".to_string()
        };
        
        let source_node_idx = nodes.len();
        nodes.push(OperatorNode::Source(SourceOp { table: table_name.clone(), columns: vec![] }));
        
        // WHERE
        if let Some(selection) = &stmt.expr {
            let expr = Self::process_expression(selection)?;
            nodes.push(OperatorNode::Filter(FilterOp { expression: expr }));
            let prev_idx = nodes.len() - 2;
            let curr_idx = nodes.len() - 1;
            edges.push(yykv_operators::Edge { from: prev_idx, to: curr_idx });
        }
        
        // SELECT list
        let mut projections = Vec::new();
        for item in &stmt.items {
            match item {
                oak_ast::SelectItem::Star { .. } => {
                    projections.push(("*".to_string(), Expr::Column("*".to_string())));
                }
                oak_ast::SelectItem::Expression { expr, alias, .. } => {
                    let processed_expr = Self::process_expression(expr)?;
                    let name = if let Some(alias) = alias {
                        alias.name.to_string()
                    } else {
                        match &processed_expr {
                            Expr::Column(n) => n.clone(),
                            _ => format!("col_{}", projections.len()),
                        }
                    };
                    projections.push((name, processed_expr));
                }
            }
        }

        if !projections.is_empty() {
            nodes.push(OperatorNode::Project(ProjectOp { projections }));
            let prev_idx = nodes.len() - 2;
            let curr_idx = nodes.len() - 1;
            edges.push(yykv_operators::Edge { from: prev_idx, to: curr_idx });
        }
        
        Ok(SqlStatement::Query(OpsGraph { id: Uuid::new_v4(), nodes, edges }))
    }

    fn process_insert(stmt: &oak_ast::InsertStatement) -> DsResult<SqlStatement> {
        let table = stmt.table_name.name.name.to_string();
        let mut values = Vec::new();
        
        for val in &stmt.values {
            let expr = Self::process_expression(val)?;
            if let Expr::Literal(v) = expr {
                values.push(v);
            } else {
                 return Err(DsError::parse("INSERT values must be literals"));
            }
        }
        
        if values.is_empty() {
            return Err(DsError::parse("INSERT must have at least one value"));
        }
        
        let key = values[0].clone();
        let value = if values.len() == 1 { key.clone() } else { DsValue::List(values) };
        
        Ok(SqlStatement::Insert { table, key, value })
    }

    fn process_update(stmt: &oak_ast::UpdateStatement) -> DsResult<SqlStatement> {
        let table = stmt.table_name.name.name.to_string();
        let mut updates = Vec::new();
        
        for assignment in &stmt.assignments {
            let col = assignment.column.name.to_string();
            let expr = Self::process_expression(&assignment.value)?;
            updates.push((col, expr));
        }
        
        let selection = if let Some(expr) = &stmt.selection {
            Some(Self::process_expression(expr)?)
        } else {
            None
        };
        
        Ok(SqlStatement::Update { table, updates, selection })
    }

    fn process_delete(stmt: &oak_ast::DeleteStatement) -> DsResult<SqlStatement> {
        let table = stmt.table_name.name.name.to_string();
        let selection = if let Some(expr) = &stmt.selection {
            Some(Self::process_expression(expr)?)
        } else {
            None
        };
        
        Ok(SqlStatement::Delete { table, selection })
    }

    fn process_create(stmt: &oak_ast::CreateStatement) -> DsResult<SqlStatement> {
         let table = stmt.name.name.to_string();
         Ok(SqlStatement::CreateTable { name: table, schema: None })
    }

    fn process_drop(stmt: &oak_ast::DropStatement) -> DsResult<SqlStatement> {
        Ok(SqlStatement::Drop {
            object_type: "TABLE".to_string(), // Simplified
            name: stmt.name.name.to_string(),
            if_exists: false, // simplified
        })
    }

    fn process_expression(expr: &oak_ast::Expression) -> DsResult<Expr> {
        match expr {
            oak_ast::Expression::Literal(lit) => {
                match lit {
                    oak_ast::Literal::Number(n, _) => {
                         if n.contains('.') {
                             Ok(Expr::Literal(DsValue::Float(n.parse().unwrap_or(0.0))))
                         } else {
                             Ok(Expr::Literal(DsValue::Int(n.parse().unwrap_or(0))))
                         }
                    }
                    oak_ast::Literal::String(s, _) => Ok(Expr::Literal(DsValue::Text(s.to_string()))),
                    oak_ast::Literal::Boolean(b, _) => Ok(Expr::Literal(DsValue::Bool(*b))),
                    oak_ast::Literal::Null(_) => Ok(Expr::Literal(DsValue::Null)),
                }
            }
            oak_ast::Expression::Vector { elements, .. } => {
                let mut vec_vals = Vec::new();
                for elem in elements {
                    let val = Self::process_expression(elem)?;
                    if let Expr::Literal(v) = val {
                        if let DsValue::Float(f) = v {
                            vec_vals.push(f as f32);
                        } else if let DsValue::Int(i) = v {
                            vec_vals.push(i as f32);
                        } else {
                            return Err(DsError::parse("Vector elements must be numbers"));
                        }
                    } else {
                        return Err(DsError::parse("Vector elements must be literals"));
                    }
                }
                Ok(Expr::Literal(DsValue::Vector(Vector(vec_vals))))
            }
            oak_ast::Expression::Binary { left, op, right, .. } => {
                let left_expr = Self::process_expression(left)?;
                let right_expr = Self::process_expression(right)?;
                let bin_op = match op {
                    oak_ast::BinaryOperator::Equal => BinaryOperator::Eq,
                    oak_ast::BinaryOperator::NotEqual => BinaryOperator::Neq,
                    oak_ast::BinaryOperator::Less => BinaryOperator::Lt,
                    oak_ast::BinaryOperator::LessEqual => BinaryOperator::Lte,
                    oak_ast::BinaryOperator::Greater => BinaryOperator::Gt,
                    oak_ast::BinaryOperator::GreaterEqual => BinaryOperator::Gte,
                    _ => return Err(DsError::parse("Unsupported binary operator")),
                };
                Ok(Expr::BinaryOp(Box::new(left_expr), bin_op, Box::new(right_expr)))
            }
            oak_ast::Expression::Identifier(id) => {
                Ok(Expr::Column(id.name.to_string()))
            }
            _ => Err(DsError::parse("Unsupported expression type")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yykv_types::{DsValue, Vector};

    #[test]
    fn test_parse_vector_literal() {
        let sql = "INSERT INTO vectors (v) VALUES ([1.0, 2.0, 3.0])";
        let result = SqlParser::parse_sql(sql);
        
        // Debug output
        if let Err(e) = &result {
             println!("Parse error: {:?}", e);
        } else if let Ok(SqlStatement::Insert { value, .. }) = &result {
             println!("Parsed value: {:?}", value);
        }

        assert!(result.is_ok(), "Failed to parse vector literal: {:?}", result.err());
        
        if let Ok(SqlStatement::Insert { value, .. }) = result {
            if let DsValue::Vector(Vector(v)) = value {
                assert_eq!(v.len(), 3);
                assert_eq!(v[0], 1.0);
                assert_eq!(v[1], 2.0);
                assert_eq!(v[2], 3.0);
            } else {
                panic!("Expected vector literal, got {:?}", value);
            }
        } else {
            panic!("Expected Insert statement");
        }
    }
}
