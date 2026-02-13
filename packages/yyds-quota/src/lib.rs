use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Resource limits for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quota {
    pub tenant_id: u64,
    /// Maximum storage in bytes.
    pub max_storage: u64,
    /// Maximum CPU time per second in milliseconds.
    pub cpu_burst_limit: u32,
    /// Maximum memory in bytes.
    pub max_memory: u64,
}

/// QuotaManager handles the enforcement and tracking of tenant quotas.
pub struct QuotaManager {
    quotas: HashMap<u64, Quota>,
}

impl QuotaManager {
    pub fn new() -> Self {
        Self {
            quotas: HashMap::new(),
        }
    }

    pub fn set_quota(&mut self, quota: Quota) {
        self.quotas.insert(quota.tenant_id, quota);
    }

    pub fn get_quota(&self, tenant_id: u64) -> Option<&Quota> {
        self.quotas.get(&tenant_id)
    }

    /// Check if a tenant has exceeded their storage quota.
    pub fn check_storage(&self, tenant_id: u64, current_usage: u64) -> bool {
        if let Some(quota) = self.quotas.get(&tenant_id) {
            current_usage <= quota.max_storage
        } else {
            true // No quota set
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_management() {
        let mut manager = QuotaManager::new();
        let tenant_id = 1001;
        
        manager.set_quota(Quota {
            tenant_id,
            max_storage: 1024 * 1024 * 1024, // 1GB
            cpu_burst_limit: 100,
            max_memory: 512 * 1024 * 1024,
        });

        assert!(manager.check_storage(tenant_id, 512 * 1024 * 1024));
        assert!(!manager.check_storage(tenant_id, 2 * 1024 * 1024 * 1024));
    }
}
