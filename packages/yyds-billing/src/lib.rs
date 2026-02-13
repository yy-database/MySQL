use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use oak_json::JsonValue;

/// Types of resources that can be metered.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceType {
    /// Storage in bytes.
    Storage,
    /// Number of I/O operations.
    IOPS,
    /// CPU compute time in milliseconds.
    ComputeTime,
    /// Network egress in bytes.
    NetworkEgress,
}

impl ResourceType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Storage" => Some(ResourceType::Storage),
            "IOPS" => Some(ResourceType::IOPS),
            "ComputeTime" => Some(ResourceType::ComputeTime),
            "NetworkEgress" => Some(ResourceType::NetworkEgress),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::Storage => "Storage",
            ResourceType::IOPS => "IOPS",
            ResourceType::ComputeTime => "ComputeTime",
            ResourceType::NetworkEgress => "NetworkEgress",
        }
    }
}

/// A single metering record for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteringRecord {
    pub tenant_id: u64,
    pub resource_type: ResourceType,
    pub amount: u64,
    pub timestamp: DateTime<Utc>,
}

impl MeteringRecord {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"tenant_id\":{},\"resource_type\":\"{}\",\"amount\":{},\"timestamp\":\"{}\"}}",
            self.tenant_id,
            self.resource_type.as_str(),
            self.amount,
            self.timestamp.to_rfc3339()
        )
    }

    pub fn from_json_value(val: &JsonValue) -> Option<Self> {
        let obj = val.as_object()?;
        let tenant_id = obj.get("tenant_id")?.as_f64()? as u64;
        let resource_type_str = obj.get("resource_type")?.as_str()?;
        let resource_type = ResourceType::from_str(resource_type_str)?;
        let amount = obj.get("amount")?.as_f64()? as u64;
        let timestamp_str = obj.get("timestamp")?.as_str()?;
        let timestamp = DateTime::parse_from_rfc3339(timestamp_str).ok()?.with_timezone(&Utc);
        
        Some(MeteringRecord {
            tenant_id,
            resource_type,
            amount,
            timestamp,
        })
    }
}

/// Persistent state for billing
#[derive(Debug, Serialize, Deserialize)]
struct BillingState {
    records: Vec<MeteringRecord>,
    daily_usage: Vec<((u64, ResourceType), u64)>,
}

impl BillingState {
    pub fn to_json(&self) -> String {
        let records_json = self.records.iter()
            .map(|r| r.to_json())
            .collect::<Vec<_>>()
            .join(",");
            
        let usage_json = self.daily_usage.iter()
            .map(|((id, ty), amt)| {
                format!("[[{},\"{}\"],{}]", id, ty.as_str(), amt)
            })
            .collect::<Vec<_>>()
            .join(",");
            
        format!(
            "{{\"records\":[{}],\"daily_usage\":[{}]}}",
            records_json,
            usage_json
        )
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let val = oak_json::parse(json)?;
        let obj = val.as_object().ok_or("Expected object")?;
        
        let records_val = obj.get("records")
            .and_then(|v| v.as_array())
            .ok_or("Expected 'records' field as array")?;
            
        let mut records = Vec::new();
        for r_val in &records_val.elements {
            records.push(MeteringRecord::from_json_value(r_val).ok_or("Invalid record")?);
        }
        
        let usage_val = obj.get("daily_usage")
            .and_then(|v| v.as_array())
            .ok_or("Expected 'daily_usage' field as array")?;
            
        let mut daily_usage = Vec::new();
        for u_val in &usage_val.elements {
            let pair = u_val.as_array().ok_or("Expected pair")?;
            if pair.elements.len() != 2 { return Err("Invalid usage pair".to_string()); }
            
            let key_val = pair.elements[0].as_array().ok_or("Expected key array")?;
            if key_val.elements.len() != 2 { return Err("Invalid key array".to_string()); }
            
            let id = key_val.elements[0].as_f64().ok_or("Invalid tenant_id")? as u64;
            let ty_str = key_val.elements[1].as_str().ok_or("Invalid resource_type")?;
            let ty = ResourceType::from_str(ty_str).ok_or("Unknown resource_type")?;
            
            let amt = pair.elements[1].as_f64().ok_or("Invalid amount")? as u64;
            
            daily_usage.push(((id, ty), amt));
        }
        
        Ok(BillingState { records, daily_usage })
    }
}

/// BillingManager handles the collection and aggregation of resource usage.
pub struct BillingManager {
    records: Vec<MeteringRecord>,
    daily_usage: HashMap<(u64, ResourceType), u64>,
}

impl BillingManager {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            daily_usage: HashMap::new(),
        }
    }

    /// Load billing data from a file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        if !path.as_ref().exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path)?;
        let state = BillingState::from_json(&content)?;

        Ok(Self {
            records: state.records,
            daily_usage: state.daily_usage.into_iter().collect(),
        })
    }

    /// Save billing data to a file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let state = BillingState {
            records: self.records.clone(),
            daily_usage: self.daily_usage.iter().map(|(&k, &v)| (k, v)).collect(),
        };

        let content = state.to_json();
        fs::write(path, content)?;
        Ok(())
    }

    /// Record resource usage for a tenant.
    pub fn record_usage(&mut self, tenant_id: u64, resource_type: ResourceType, amount: u64) {
        let record = MeteringRecord {
            tenant_id,
            resource_type,
            amount,
            timestamp: Utc::now(),
        };

        // Update daily aggregation
        let entry = self.daily_usage.entry((tenant_id, resource_type)).or_insert(0);
        *entry += amount;

        self.records.push(record);
    }

    /// Get total usage for a tenant by resource type.
    pub fn get_total_usage(&self, tenant_id: u64, resource_type: ResourceType) -> u64 {
        *self.daily_usage.get(&(tenant_id, resource_type)).unwrap_or(&0)
    }

    /// Clear records (e.g. after pushing to a persistent billing system).
    pub fn flush_records(&mut self) -> Vec<MeteringRecord> {
        std::mem::take(&mut self.records)
    }
}
