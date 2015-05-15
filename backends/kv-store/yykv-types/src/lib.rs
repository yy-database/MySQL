#![warn(missing_docs)]

use bytes::BytesMut;
pub use rust_decimal::Decimal;
pub use uuid::Uuid;

pub mod layout;

// Re-export core types from yyds-types
pub use yyds_types::{
    ColumnInfo, DeviceConfig, DsError, DsErrorKind, DsFile, DsResult, DsSchema, DsValue, EnumInfo,
    FieldConstraint, FieldType, ForeignKeyInfo, FromValue, InternalId, Lsn, MediumType, Page,
    PageId, PageInfo, Redundancy, SchemaInspector, StorageConfig, TableInfo, ToValue, Vector,
};

/// Alias for DsSchema.
pub type Schema = DsSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YYKVConfig {
    pub data_dir: String,
    pub wal_dir: String,
}

use serde::{Deserialize, Serialize};
use std::fmt;

// Note: InternalId and others are already re-exported from yyds_types
