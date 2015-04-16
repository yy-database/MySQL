use crate::btree_page::BTreePage;
use crate::page::PageId;
use crate::pager::Pager;
use std::sync::Arc;
use yykv_types::{DsResult, DsValue};

pub struct BTree {
    pager: Arc<Pager>,
    root_page_id: PageId,
}

impl BTree {
    pub async fn open(pager: Arc<Pager>) -> DsResult<Self> {
        let root_page_id = pager.get_root_page_id().await?;
        if root_page_id == 0 {
            let root_page_arc = pager.allocate_page().await?;
            let root_id = root_page_arc.read().await.id;
            let root_btree = BTreePage::new(root_page_arc);
            root_btree.init(true).await;
            pager.set_root_page_id(root_id).await?;
            Ok(Self {
                pager,
                root_page_id: root_id,
            })
        } else {
            Ok(Self {
                pager,
                root_page_id,
            })
        }
    }

    pub async fn get(&self, _tenant_id: u64, key: &str) -> DsResult<Option<DsValue>> {
        let key_value = DsValue::Text(key.to_string());
        let mut current_id = self.root_page_id;
        while current_id != 0 {
            let page = self.pager.get_page(current_id).await?;
            let btree_page = BTreePage::new(page);
            let index = btree_page.find_cell(&key_value).await?;
            let count = btree_page.cell_count().await;

            if btree_page.is_leaf().await {
                if index < count {
                    let found_key = btree_page.get_cell_key(index).await?;
                    if found_key == key_value {
                        return Ok(Some(btree_page.get_leaf_value(index).await?));
                    }
                }
                return Ok(None);
            } else {
                current_id = if index < count {
                    btree_page.get_internal_child(index).await?
                } else {
                    btree_page.right_child().await
                };
            }
        }
        Ok(None)
    }

    pub async fn insert(&self, key: &DsValue, value: &DsValue) -> DsResult<()> {
        // Implementation similar to yydb-storage/btree.rs but async
        Ok(())
    }

    /// Put a value with tenant ID and redundancy (ignored for now).
    pub async fn put(&self, _tenant_id: u64, key: String, value: DsValue, _redundancy: yykv_types::Redundancy) -> DsResult<()> {
        self.insert(&DsValue::Text(key), &value).await
    }

    /// Delete a value by key.
    pub async fn delete(&self, _tenant_id: u64, key: String) -> DsResult<()> {
        // Implementation for delete
        Ok(())
    }
}
