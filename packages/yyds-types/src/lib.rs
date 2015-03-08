pub use yykv_types::layout;
pub use yykv_types::{
    FieldConstraint, FieldType, InternalId, DsError, DsErrorKind, DsValue, MediumType, PageId,
    Redundancy, ToValue, Uuid, Vector, DsSchema, DsFile,
};

/// A specialized Result type for YYDS operations.
pub type Result<T> = std::result::Result<T, YYError>;
/// YYDS Error type.
pub type YYError = DsError;
/// YYDS Error kind.
pub type YYErrorKind = DsErrorKind;
/// YYDS Value type, the core data unit.
pub type YYValue = DsValue;
/// YYDS File type.
pub type YyFile = DsFile;
/// YYDS Schema type.
pub type YYSchema = DsSchema;
