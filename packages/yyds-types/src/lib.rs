#![warn(missing_docs)]

use bytes::{Bytes, BytesMut};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
pub use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
pub use uuid::Uuid;

mod errors;
pub mod layout;

pub use errors::{DsError, DsErrorKind, DsResult};

/// Page ID type.
pub type PageId = u64;
/// Log Sequence Number type.
pub type Lsn = u64;

/// Redundancy level for data storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Redundancy(pub u8);

impl Redundancy {
    /// Temporary data, no replication.
    pub const TEMP: Redundancy = Redundancy(0);
    /// Single replica.
    pub const SINGLE: Redundancy = Redundancy(1);
    /// Edge computing level redundancy.
    pub const EDGE: Redundancy = Redundancy(2);
    /// Safe level redundancy.
    pub const SAFE: Redundancy = Redundancy(3);
    /// Truth level (high availability).
    pub const TRUTH: Redundancy = Redundancy(5);

    /// Create from u8.
    pub fn from_u8(v: u8) -> Self {
        Self(v)
    }

    /// Number of replicas.
    pub fn replicas(&self) -> u8 {
        if self.0 == 0 { 1 } else { self.0 }
    }
}

/// Physical page in the storage engine.
#[derive(Debug, Clone)]
pub struct Page {
    /// Globally unique page ID.
    pub id: PageId,
    /// Raw page data.
    pub data: BytesMut,
}

/// Storage configuration for the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// List of storage devices.
    pub devices: Vec<DeviceConfig>,
    /// System-wide page size.
    pub page_size: usize,
    /// Path to WAL file.
    pub wal_path: Option<String>,
}

/// Configuration for a single storage device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Device ID.
    pub id: u32,
    /// Physical path or URI.
    pub path: String,
    /// Medium type.
    pub medium: MediumType,
}

/// Types of storage media.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MediumType {
    /// Non-Volatile Memory (Optane etc.)
    NVM,
    /// NVMe SSD
    NvmeSSD,
    /// SATA SSD
    SataSSD,
    /// Hard Disk Drive
    HDD,
}

impl fmt::Display for MediumType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MediumType::NVM => write!(f, "NVM"),
            MediumType::NvmeSSD => write!(f, "NVMe SSD"),
            MediumType::SataSSD => write!(f, "SATA SSD"),
            MediumType::HDD => write!(f, "HDD"),
        }
    }
}

/// Information about a page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    /// Page ID.
    pub id: PageId,
    /// Medium type.
    pub medium: MediumType,
    /// Size in bytes.
    pub size: u32,
}

/// Bit-packed Internal ID for fast routing and sharding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct InternalId(pub u64);

impl InternalId {
    /// Create a new InternalId.
    pub fn new(region_id: u8, shard_id: u16, sequence: u64) -> Self {
        let id = ((region_id as u64) << 56) | ((shard_id as u64) << 40) | (sequence & 0xFF_FFFF_FFFF);
        Self(id)
    }

    /// Get region ID.
    pub fn region_id(&self) -> u8 { (self.0 >> 56) as u8 }
    /// Get shard ID.
    pub fn shard_id(&self) -> u16 { ((self.0 >> 40) & 0xFFFF) as u16 }
    /// Get tenant sequence.
    pub fn sequence(&self) -> u64 { self.0 & 0xFF_FFFF_FFFF }
}

/// File reference in the storage engine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct DsFile {
    /// File ID.
    pub file_id: Uuid,
    /// File name.
    pub name: String,
    /// File size in bytes.
    pub size: u64,
    /// MIME type.
    pub mime_type: String,
}

/// Vector data type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Vector(pub Vec<f32>);

impl Vector {
    /// Calculate cosine similarity with another vector.
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        let a = &self.0;
        let b = &other.0;
        if a.len() != b.len() || a.is_empty() { return 0.0; }
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 { return 0.0; }
        dot_product / (norm_a * norm_b)
    }
}

/// Types of fields in a schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum FieldType {
    /// Null.
    Null,
    /// Boolean.
    Bool,
    /// Integer.
    Int,
    /// Float.
    Float,
    /// Text.
    Text,
    /// Bytes.
    Bytes,
    /// UUID.
    Uuid,
    /// Decimal.
    Decimal,
    /// Date.
    Date,
    /// Time.
    Time,
    /// DateTime.
    DateTime,
    /// Vector.
    Vector,
    /// File.
    File,
    /// List.
    List,
    /// Dictionary.
    Dict,
    /// Structured Object with schema ID.
    Object(u32),
    /// Extended type with type name.
    Ext(String),
}

/// Constraint for a field in a schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct FieldConstraint {
    /// Field name.
    pub name: String,
    /// Field type.
    pub field_type: FieldType,
    /// Whether the field can be null.
    pub nullable: bool,
    /// Default value for the field.
    pub default_value: Option<DsValue>,
}

/// Schema definition for structured data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct DsSchema {
    /// Schema ID.
    pub id: u32,
    /// Schema name.
    pub name: String,
    /// Fields in the schema.
    pub fields: BTreeMap<u32, FieldConstraint>,
}

impl DsSchema {
    /// Create a new DsSchema.
    pub fn new(id: u32, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            fields: BTreeMap::new(),
        }
    }

    /// Add a field to the schema.
    pub fn add_field(&mut self, tag: u32, name: &str, field_type: FieldType, nullable: bool) {
        self.fields.insert(tag, FieldConstraint {
            name: name.to_string(),
            field_type,
            nullable,
            default_value: None,
        });
    }

    /// Validate a value against the schema.
    pub fn validate(&self, value: &DsValue) -> DsResult<()> {
        match value {
            DsValue::Object { schema_id, fields } => {
                if *schema_id != self.id {
                    return Err(DsError::schema(Some(self.name.clone()), None, format!("Schema ID mismatch: expected {}, got {}", self.id, schema_id)));
                }
                for (tag, constraint) in &self.fields {
                    if fields.get(tag).is_none() && !constraint.nullable {
                        return Err(DsError::schema(Some(self.name.clone()), Some(constraint.name.clone()), format!("Missing non-nullable field: {}", constraint.name)));
                    }
                }
                Ok(())
            }
            _ => Err(DsError::schema(Some(self.name.clone()), None, "Value is not an object")),
        }
    }
}

/// Table metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    /// Table name.
    pub name: String,
    /// Columns in the table.
    pub columns: Vec<ColumnInfo>,
    /// Description.
    pub description: Option<String>,
}

/// Column metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    /// Column name.
    pub name: String,
    /// Data type.
    pub data_type: String,
    /// Nullability.
    pub is_nullable: bool,
    /// Primary key.
    pub is_primary_key: bool,
    /// Enum.
    pub is_enum: bool,
    /// Foreign key.
    pub foreign_key: Option<ForeignKeyInfo>,
    /// Default value.
    pub default: Option<String>,
    /// Description.
    pub description: Option<String>,
}

/// Foreign key metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyInfo {
    /// Target table name.
    pub foreign_table: String,
}

/// Enum metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumInfo {
    /// Enum name.
    pub name: String,
    /// Variants.
    pub variants: Vec<String>,
}

/// Trait for inspecting database schema.
#[async_trait::async_trait]
pub trait SchemaInspector: Send + Sync {
    /// Retrieve tables and enums.
    async fn introspect(&self, schema: Option<&str>) -> DsResult<(Vec<TableInfo>, Vec<EnumInfo>)>;
}

/// Core data unit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DsValue {
    /// Null.
    Null,
    /// Boolean.
    Bool(bool),
    /// Integer.
    Int(i64),
    /// Float.
    Float(f64),
    /// Text.
    Text(String),
    /// Bytes.
    Bytes(Bytes),
    /// Binary data.
    Binary(Bytes),
    /// UUID.
    Uuid(Uuid),
    /// Decimal.
    Decimal(Decimal),
    /// Date.
    Date(NaiveDate),
    /// Time.
    Time(NaiveTime),
    /// DateTime.
    DateTime(DateTime<Utc>),
    /// Vector.
    Vector(Vector),
    /// File.
    File(DsFile),
    /// List.
    List(Vec<DsValue>),
    /// Dictionary.
    Dict(BTreeMap<String, DsValue>),
    /// Structured Object.
    Object {
        /// Schema ID.
        schema_id: u32,
        /// Fields.
        fields: BTreeMap<u32, DsValue>,
    },
    /// Extended type.
    Ext(String, Vec<u8>),
}

impl PartialOrd for DsValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (DsValue::Null, DsValue::Null) => Some(std::cmp::Ordering::Equal),
            (DsValue::Bool(a), DsValue::Bool(b)) => a.partial_cmp(b),
            (DsValue::Int(a), DsValue::Int(b)) => a.partial_cmp(b),
            (DsValue::Float(a), DsValue::Float(b)) => a.partial_cmp(b),
            (DsValue::Text(a), DsValue::Text(b)) => a.partial_cmp(b),
            (DsValue::Bytes(a), DsValue::Bytes(b)) => a.partial_cmp(b),
            (DsValue::Binary(a), DsValue::Binary(b)) => a.partial_cmp(b),
            (DsValue::Uuid(a), DsValue::Uuid(b)) => a.partial_cmp(b),
            (DsValue::Decimal(a), DsValue::Decimal(b)) => a.partial_cmp(b),
            (DsValue::Date(a), DsValue::Date(b)) => a.partial_cmp(b),
            (DsValue::Time(a), DsValue::Time(b)) => a.partial_cmp(b),
            (DsValue::DateTime(a), DsValue::DateTime(b)) => a.partial_cmp(b),
            (DsValue::Vector(a), DsValue::Vector(b)) => a.partial_cmp(b),
            (DsValue::File(a), DsValue::File(b)) => a.partial_cmp(b),
            (DsValue::List(a), DsValue::List(b)) => a.partial_cmp(b),
            (DsValue::Dict(a), DsValue::Dict(b)) => a.partial_cmp(b),
            (DsValue::Object { schema_id: s1, fields: f1 }, DsValue::Object { schema_id: s2, fields: f2 }) => {
                match s1.partial_cmp(s2) {
                    Some(std::cmp::Ordering::Equal) => f1.partial_cmp(f2),
                    ord => ord,
                }
            }
            (DsValue::Ext(s1, v1), DsValue::Ext(s2, v2)) => {
                match s1.partial_cmp(s2) {
                    Some(std::cmp::Ordering::Equal) => v1.partial_cmp(v2),
                    ord => ord,
                }
            }
            _ => None,
        }
    }
}

impl fmt::Display for DsValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DsValue::Null => write!(f, "NULL"),
            DsValue::Bool(b) => write!(f, "{}", b),
            DsValue::Int(i) => write!(f, "{}", i),
            DsValue::Float(fl) => write!(f, "{}", fl),
            DsValue::Text(s) => write!(f, "{}", s),
            DsValue::Bytes(b) | DsValue::Binary(b) => write!(f, "0x{}", hex::encode(b)),
            DsValue::Uuid(u) => write!(f, "{}", u),
            DsValue::Decimal(d) => write!(f, "{}", d),
            DsValue::Date(d) => write!(f, "{}", d),
            DsValue::Time(t) => write!(f, "{}", t),
            DsValue::DateTime(dt) => write!(f, "{}", dt),
            DsValue::Vector(v) => write!(f, "Vector({:?})", v.0),
            DsValue::Dict(d) => write!(f, "Dict({:?})", d),
            DsValue::List(l) => write!(f, "List({:?})", l),
            DsValue::Object { schema_id, fields } => write!(f, "Object(schema={}, fields={:?})", schema_id, fields),
            DsValue::Ext(t, b) => write!(f, "Ext({}, 0x{})", t, hex::encode(b)),
            DsValue::File(file) => write!(f, "File({}, name={}, size={})", file.file_id, file.name, file.size),
        }
    }
}

/// Trait for converting to DsValue.
pub trait ToValue {
    /// Convert to DsValue.
    fn to_value(&self) -> DsValue;
}

impl ToValue for DsValue { fn to_value(&self) -> DsValue { self.clone() } }
impl ToValue for i64 { fn to_value(&self) -> DsValue { DsValue::Int(*self) } }
impl ToValue for f32 { fn to_value(&self) -> DsValue { DsValue::Float(*self as f64) } }
impl ToValue for String { fn to_value(&self) -> DsValue { DsValue::Text(self.clone()) } }
impl ToValue for &str { fn to_value(&self) -> DsValue { DsValue::Text(self.to_string()) } }
impl ToValue for bool { fn to_value(&self) -> DsValue { DsValue::Bool(*self) } }
impl ToValue for Uuid { fn to_value(&self) -> DsValue { DsValue::Uuid(*self) } }

/// Trait for converting from DsValue.
pub trait FromValue: Sized {
    /// Convert from DsValue.
    fn from_value(v: DsValue) -> DsResult<Self>;
}

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
/// YYDS file type.
pub type YyFile = DsFile;
