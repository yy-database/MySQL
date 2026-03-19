use yykv_operators::{OpsGraph, OperatorNode, Edge, Expr, BinaryOperator, FilterOp, ProjectOp, SourceOp};
use yykv_types::{DsError, DsResult};

/// Query optimizer for SQL operations
pub struct QueryOptimizer;

impl QueryOptimizer {
    /// Optimizes the operation graph generated from SQL
    pub fn optimize(graph: OpsGraph) -> DsResult<OpsGraph> {
        let mut optimized = graph;
        
        // Apply various optimization rules
        optimized = Self::push_down_predicates(optimized);
        optimized = Self::reorder_operations(optimized);
        optimized = Self::optimize_expressions(optimized);
        
        Ok(optimized)
    }
    
    /// Pushes down filter predicates as close to the data source as possible
    fn push_down_predicates(mut graph: OpsGraph) -> OpsGraph {
        // Create a new graph with optimized node order
        let mut new_nodes = Vec::new();
        let mut new_edges = Vec::new();
        
        // First, find the source node
        let source_node = graph.nodes.iter().find(|node| matches!(node, OperatorNode::Source(_)));
        if let Some(source) = source_node {
            // Add source node first
            new_nodes.push(source.clone());
            let source_idx = 0;
            
            // Find all filter nodes
            let filter_nodes: Vec<FilterOp> = graph.nodes
                .iter()
                .filter_map(|node| {
                    if let OperatorNode::Filter(op) = node {
                        Some(op.clone())
                    } else {
                        None
                    }
                })
                .collect();
            
            // Add filter nodes after source (these are WHERE clauses)
            let mut current_idx = source_idx;
            for filter_op in filter_nodes {
                new_nodes.push(OperatorNode::Filter(filter_op));
                let filter_idx = new_nodes.len() - 1;
                new_edges.push(Edge { from: current_idx, to: filter_idx });
                current_idx = filter_idx;
            }
            
            // Add remaining nodes (excluding source and filters)
            let other_nodes: Vec<OperatorNode> = graph.nodes
                .iter()
                .filter(|node| {
                    !matches!(node, OperatorNode::Source(_)) && !matches!(node, OperatorNode::Filter(_))
                })
                .cloned()
                .collect();
            
            for node in other_nodes {
                new_nodes.push(node);
                let node_idx = new_nodes.len() - 1;
                new_edges.push(Edge { from: current_idx, to: node_idx });
                current_idx = node_idx;
            }
            
            // Create new graph
            graph = OpsGraph {
                id: graph.id,
                nodes: new_nodes,
                edges: new_edges,
            };
        }
        
        graph
    }
    
    /// Reorders operations for better performance
    fn reorder_operations(graph: OpsGraph) -> OpsGraph {
        // For simplicity, we'll just return the graph as is
        // In a real optimizer, we'd implement more complex reordering logic
        graph
    }
    
    /// Optimizes expressions in the operation graph
    fn optimize_expressions(mut graph: OpsGraph) -> OpsGraph {
        // Optimize expressions in filter nodes
        for node in &mut graph.nodes {
            match node {
                OperatorNode::Filter(FilterOp { expression }) => {
                    *expression = Self::optimize_expression(expression);
                }
                OperatorNode::Project(ProjectOp { projections }) => {
                    for (_, expr) in projections {
                        *expr = Self::optimize_expression(expr);
                    }
                }
                _ => {}
            }
        }
        
        graph
    }
    
    /// Optimizes a single expression
    fn optimize_expression(expr: &Expr) -> Expr {
        match expr {
            Expr::BinaryOp(left, op, right) => {
                let optimized_left = Self::optimize_expression(left);
                let optimized_right = Self::optimize_expression(right);
                
                // Constant folding would go here
                // For simplicity, we'll just return the expression as is
                Expr::BinaryOp(Box::new(optimized_left), op.clone(), Box::new(optimized_right))
            }
            _ => expr.clone(),
        }
    }
}
