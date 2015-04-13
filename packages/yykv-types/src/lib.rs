#![warn(missing_docs)]

use bytes::{BytesMut};
pub use rust_decimal::Decimal;
pub use uuid::Uuid;

pub mod layout;

// Re-export core types from yyds-types
pub use yyds_types::{
    DsError, DsErrorKind, DsResult,
    DsValue, DsFile, Vector, FieldType, FieldConstraint, DsSchema,
    ToValue, FromValue, PageId, Lsn, Redundancy, Page, StorageConfig,
    DeviceConfig, MediumType, PageInfo, InternalId, TableInfo, ColumnInfo,
    ForeignKeyInfo, EnumInfo, SchemaInspector,
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
