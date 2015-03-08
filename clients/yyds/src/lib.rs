//! yyds - Aggregate of the yyds ecosystem
//!
//! This crate provides a unified interface to the various components of the yyds ecosystem.
//!
//! # Example
//!
//! ```rust
//! use yyds::types::YYValue;
//! use yyds::wal::{WalManager, OpType};
//! use yyds::types::Redundancy;
//! use uuid::Uuid;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let wal = WalManager::new(100);
//!     let tenant_id = Uuid::new_v4();
//!     
//!     wal.write(
//!         tenant_id,
//!         "users".into(),
//!         OpType::Insert,
//!         Redundancy::TRUTH,
//!         YYValue::Text("Alice".into())
//!     ).await?;
//!     
//!     Ok(())
//! }
//! ```

pub use we_trust as trust;
pub use we_trust_postgres as postgres;
pub use we_trust_s3 as s3;
pub use we_trust_otel as otel;
pub use we_trust_arrow as arrow;
pub use we_trust_mysql as mysql;
pub use we_trust_vector as vector;

pub use yyds_types as types;
pub use yyds_types::{YYValue, YYError, Result};
pub use yyds_consensus as consensus;
pub use yyds_compute as compute;
pub use yyds_ai as ai;
pub use yykv_wal as wal;
pub use yykv_index as index;
pub use yykv_schema as schema;
pub use yyds_task as task;
pub use yykv_operators as operators;
pub use yykv_executor as executor;
pub use yyds_gateway as gateway;
pub use yyds_observability as observability;
pub use yyds_tenant as tenant;
pub use yyds_billing as billing;
pub use yyds_quota as quota;
pub use yyds_pubsub as pubsub;
pub use yyds_cache as cache;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
