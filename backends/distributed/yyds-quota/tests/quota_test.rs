use yyds_quota::*;

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
