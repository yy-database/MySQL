//! yyds-types: Common type system for the yyds ecosystem.
//! 
//! This crate re-exports types from yykv-types to ensure compatibility
//! across the entire ecosystem while providing a consistent naming convention.

pub use yykv_types::{
    DsError, DsErrorKind, DsResult, DsValue, Redundancy, PageId, Lsn, Page,
    StorageConfig, DeviceConfig, MediumType, FieldType, FieldConstraint,
    InternalId, Vector, DsFile, DsSchema, TableInfo, IndexInfo, ColumnInfo,
    ConstraintInfo, ForeignKeyInfo, EnumInfo, SchemaInspector, ToValue,
};

/// Specialized result type for YYDS.
pub type Result<T> = std::result::Result<T, YYError>;
/// YYDS error type.
pub type YYError = DsError;
/// YYDS error kind.
pub type YYErrorKind = DsErrorKind;
/// YYDS value type.
pub type YYValue = DsValue;
/// YYDS schema type.
pub type YYSchema = DsSchema;

pub mod layout {
    //! Data layout and serialization.
    pub use yykv_types::layout::*;
}

pub mod errors {
    //! Error types and utilities.
    pub use yykv_types::DsError as Error;
    pub use yykv_types::DsErrorKind as ErrorKind;
    pub use yykv_types::DsResult as Result;
}
