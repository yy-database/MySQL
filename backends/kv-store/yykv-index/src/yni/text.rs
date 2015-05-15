use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;
use yykv_types::DsResult;

pub struct WhitespaceTokenizer;
impl WhitespaceTokenizer {
    pub fn tokenize(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|s| {
                s.to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect()
            })
            .filter(|s: &String| !s.is_empty())
            .collect()
    }
}

pub struct MemoryTextStore {
    indices: BTreeMap<Uuid, BTreeMap<String, BTreeSet<Uuid>>>,
}

impl Default for MemoryTextStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryTextStore {
    pub fn new() -> Self {
        Self {
            indices: BTreeMap::new(),
        }
    }

    pub fn add_term(&mut self, term: &str, doc_id: Uuid, tenant_id: Uuid) -> DsResult<()> {
        self.indices
            .entry(tenant_id)
            .or_default()
            .entry(term.to_string())
            .or_default()
            .insert(doc_id);
        Ok(())
    }
    pub fn get_docs(&self, term: &str, tenant_id: Uuid) -> DsResult<BTreeSet<Uuid>> {
        Ok(self
            .indices
            .get(&tenant_id)
            .and_then(|terms| terms.get(term))
            .cloned()
            .unwrap_or_default())
    }

    pub fn delete_doc(&mut self, doc_id: Uuid, tenant_id: Uuid) -> DsResult<()> {
        if let Some(terms) = self.indices.get_mut(&tenant_id) {
            for docs in terms.values_mut() {
                docs.remove(&doc_id);
            }
        }
        Ok(())
    }
}
