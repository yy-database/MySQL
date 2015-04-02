use yyds_tenant::*;
use yyds_types::{Uuid, Redundancy, InternalId};

#[test]
fn test_tenant_registration() {
    let mut manager = TenantManager::new();
    let uuid = Uuid::new_v4();
    let region_id = 1;
    let shard_id = 100;
    let sequence = 1001;
    let secret = b"secret123".to_vec();

    manager.register_tenant(uuid, region_id, shard_id, sequence, "Test Tenant".into(), Redundancy::TRUTH, secret.clone()).unwrap();

    let internal_id_raw = manager.get_internal_id(uuid).unwrap();
    let internal_id = InternalId(internal_id_raw);
    
    assert_eq!(internal_id.region_id(), region_id);
    assert_eq!(internal_id.shard_id(), shard_id);
    assert_eq!(internal_id.sequence(), sequence);

    let tenant = manager.get_by_id(internal_id_raw).unwrap();
    assert_eq!(tenant.name, "Test Tenant");
    assert_eq!(tenant.tier, Redundancy::TRUTH);
    assert_eq!(tenant.secret_key, secret);
}
