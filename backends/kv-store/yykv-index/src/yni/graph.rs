use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;
use yykv_types::DsResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDirection {
    Outgoing,
    Incoming,
    Both,
}

pub struct MemoryAdjacencyIndex {
    // tenant_id -> (edge_type -> (from_id -> set of to_ids))
    forward: BTreeMap<Uuid, BTreeMap<String, BTreeMap<Uuid, BTreeSet<Uuid>>>>,
    backward: BTreeMap<Uuid, BTreeMap<String, BTreeMap<Uuid, BTreeSet<Uuid>>>>,
}

impl Default for MemoryAdjacencyIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryAdjacencyIndex {
    pub fn new() -> Self {
        Self {
            forward: BTreeMap::new(),
            backward: BTreeMap::new(),
        }
    }

    pub fn add_edge(
        &mut self,
        tenant_id: Uuid,
        from: Uuid,
        to: Uuid,
        edge_type: &str,
    ) -> DsResult<()> {
        self.forward
            .entry(tenant_id)
            .or_default()
            .entry(edge_type.to_string())
            .or_default()
            .entry(from)
            .or_default()
            .insert(to);
        self.backward
            .entry(tenant_id)
            .or_default()
            .entry(edge_type.to_string())
            .or_default()
            .entry(to)
            .or_default()
            .insert(from);
        Ok(())
    }

    pub fn neighbors(
        &self,
        tenant_id: Uuid,
        from: Uuid,
        edge_type: &str,
        direction: EdgeDirection,
    ) -> DsResult<Vec<Uuid>> {
        let mut results = BTreeSet::new();

        if matches!(direction, EdgeDirection::Outgoing | EdgeDirection::Both)
            && let Some(tos) = self
                .forward
                .get(&tenant_id)
                .and_then(|m| m.get(edge_type))
                .and_then(|m| m.get(&from))
        {
            results.extend(tos);
        }

        if matches!(direction, EdgeDirection::Incoming | EdgeDirection::Both)
            && let Some(froms) = self
                .backward
                .get(&tenant_id)
                .and_then(|m| m.get(edge_type))
                .and_then(|m| m.get(&from))
        {
            results.extend(froms);
        }

        Ok(results.into_iter().collect())
    }

    pub fn delete_node(&mut self, id: Uuid, tenant_id: Uuid) -> DsResult<()> {
        // Remove all edges from/to this node in all edge types for this tenant
        if let Some(forward_map) = self.forward.get_mut(&tenant_id) {
            for edge_map in forward_map.values_mut() {
                edge_map.remove(&id);
                for tos in edge_map.values_mut() {
                    tos.remove(&id);
                }
            }
        }
        if let Some(backward_map) = self.backward.get_mut(&tenant_id) {
            for edge_map in backward_map.values_mut() {
                edge_map.remove(&id);
                for froms in edge_map.values_mut() {
                    froms.remove(&id);
                }
            }
        }
        Ok(())
    }
}
