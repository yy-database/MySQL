#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use yyds_types::{DsError, DsResult, InternalId, Redundancy};

/// Tenant metadata in the YYDS SaaS ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tenant {
    /// Public globally unique identifier.
    pub uuid: Uuid,
    /// Internal efficient identifier with sharding info.
    pub internal_id: InternalId,
    /// Display name of the tenant.
    pub name: String,
    /// Service tier/SDR level for storage.
    pub tier: Redundancy,
    /// Whether the tenant is active.
    pub active: bool,
    /// Secret key for Trust protocol signature.
    pub secret_key: Vec<u8>,
}

/// TenantManager handles the lifecycle and lookup of tenants.
pub struct TenantManager {
    tenants_by_uuid: HashMap<Uuid, Tenant>,
    tenants_by_id: HashMap<u64, Uuid>,
}

impl TenantManager {
    pub fn new() -> Self {
        Self {
            tenants_by_uuid: HashMap::new(),
            tenants_by_id: HashMap::new(),
        }
    }

    /// Register a new tenant with sharding info.
    pub fn register_tenant(
        &mut self,
        uuid: Uuid,
        region_id: u8,
        shard_id: u16,
        sequence: u64,
        name: String,
        tier: Redundancy,
        secret_key: Vec<u8>,
    ) -> DsResult<()> {
        if self.tenants_by_uuid.contains_key(&uuid) {
            return Err(DsError::internal(format!(
                "Tenant with UUID {} already exists",
                uuid
            )));
        }

        let internal_id = InternalId::new(region_id, shard_id, sequence);
        if self.tenants_by_id.contains_key(&internal_id.0) {
            return Err(DsError::internal(format!(
                "Tenant with internal ID {} already exists",
                internal_id.0
            )));
        }

        let tenant = Tenant {
            uuid,
            internal_id,
            name,
            tier,
            active: true,
            secret_key,
        };

        self.tenants_by_uuid.insert(uuid, tenant);
        self.tenants_by_id.insert(internal_id.0, uuid);
        Ok(())
    }

    /// Lookup tenant by internal ID.
    pub fn get_by_id(&self, internal_id: u64) -> Option<&Tenant> {
        let uuid = self.tenants_by_id.get(&internal_id)?;
        self.tenants_by_uuid.get(uuid)
    }

    /// Lookup tenant by UUID.
    pub fn get_by_uuid(&self, uuid: &Uuid) -> Option<&Tenant> {
        self.tenants_by_uuid.get(uuid)
    }

    /// Lookup internal ID by UUID.
    pub fn get_internal_id(&self, uuid: Uuid) -> Option<u64> {
        self.tenants_by_uuid.get(&uuid).map(|t| t.internal_id.0)
    }
}
