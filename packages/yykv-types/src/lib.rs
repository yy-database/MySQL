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

pub type PageId = u64;
pub type Lsn = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Redundancy(pub u8);

impl Redundancy {
    pub const TEMP: Redundancy = Redundancy(0);
    pub const SINGLE: Redundancy = Redundancy(1);
    pub const EDGE: Redundancy = Redundancy(2);
    pub const SAFE: Redundancy = Redundancy(3);
    pub const TRUTH: Redundancy = Redundancy(5);

    pub fn from_u8(v: u8) -> Self {
        Self(v)
    }

    pub fn replicas(&self) -> u8 {
        if self.0 == 0 { 1 } else { self.0 }
    }
}

#[derive(Debug, Clone)]
pub struct Page {
    pub id: PageId,
    pub data: BytesMut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub devices: Vec<DeviceConfig>,
    pub page_size: usize,
    pub wal_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub id: u32,
    pub path: String,
    pub medium: MediumType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MediumType {
    NVM,     // Non-Volatile Memory (Optane etc.)
    NvmeSSD, // NVMe SSD
    SataSSD, // SATA SSD
    HDD,     // Hard Disk Drive
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub id: PageId,
    pub medium: MediumType,
    pub size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YYKVConfig {
    pub data_dir: String,
    pub wal_dir: String,
}

/// Bit-packed Internal ID for fast routing and sharding.
///
/// Layout:
/// [63-56]: Region ID (8 bits)
/// [55-40]: Shard ID (16 bits)
/// [39-0] : Tenant Sequence (40 bits)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct InternalId(pub u64);

impl InternalId {
    pub fn new(region_id: u8, shard_id: u16, sequence: u64) -> Self {
        let id =
            ((region_id as u64) << 56) | ((shard_id as u64) << 40) | (sequence & 0xFF_FFFF_FFFF);
        Self(id)
    }

    pub fn region_id(&self) -> u8 {
        (self.0 >> 56) as u8
    }

    pub fn shard_id(&self) -> u16 {
        ((self.0 >> 40) & 0xFFFF) as u16
    }

    pub fn sequence(&self) -> u64 {
        self.0 & 0xFF_FFFF_FFFF
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct DsFile {
    pub file_id: Uuid,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Vector(pub Vec<f32>);

impl Vector {
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        let a = &self.0;
        let b = &other.0;
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot_product / (norm_a * norm_b)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum FieldType {
    Null,
    Bool,
    Int,
    Float,
    Text,
    Bytes,
    Uuid,
    Decimal,
    Date,
    Time,
    DateTime,
    Vector,
    File,
    List,
    Dict,
    Object(u32), // schema_id
    Ext(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct FieldConstraint {
    pub name: String,
    pub field_type: FieldType,
    pub nullable: bool,
    pub default_value: Option<DsValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct DsSchema {
    pub id: u32,
    pub name: String,
    pub fields: BTreeMap<u32, FieldConstraint>,
}

pub type YYSchema = DsSchema;

impl DsSchema {
    pub fn new(id: u32, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            fields: BTreeMap::new(),
        }
    }

    pub fn add_field(&mut self, tag: u32, name: &str, field_type: FieldType, nullable: bool) {
        self.fields.insert(
            tag,
            FieldConstraint {
                name: name.to_string(),
                field_type,
                nullable,
                default_value: None,
            },
        );
    }

    pub fn validate(&self, value: &DsValue) -> DsResult<()> {
        match value {
            DsValue::Object { schema_id, fields } => {
                if *schema_id != self.id {
                    return Err(DsError::schema(
                        Some(self.name.clone()),
                        None,
                        format!(
                            "Schema ID mismatch: expected {}, got {}",
                            self.id, schema_id
                        ),
                    ));
                }
                for (tag, constraint) in &self.fields {
                    if let Some(_field_value) = fields.get(tag) {
                        // TODO: deeper type check
                    } else if !constraint.nullable {
                        return Err(DsError::schema(
                            Some(self.name.clone()),
                            Some(constraint.name.clone()),
                            format!("Missing non-nullable field: {}", constraint.name),
                        ));
                    }
                }
                Ok(())
            }
            _ => Err(DsError::schema(
                Some(self.name.clone()),
                None,
                "Value is not an object",
            )),
        }
    }
}

/// Schema inspection structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub is_enum: bool,
    pub foreign_key: Option<ForeignKeyInfo>,
    pub default: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyInfo {
    pub foreign_table: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumInfo {
    pub name: String,
    pub variants: Vec<String>,
}

#[async_trait::async_trait]
pub trait SchemaInspector: Send + Sync {
    async fn introspect(&self, schema: Option<&str>) -> DsResult<(Vec<TableInfo>, Vec<EnumInfo>)>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DsValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Bytes(Bytes),
    Binary(Bytes),
    Uuid(Uuid),
    Decimal(Decimal),
    Date(NaiveDate),
    Time(NaiveTime),
    DateTime(DateTime<Utc>),
    Vector(Vector),
    File(DsFile),
    List(Vec<DsValue>),
    Dict(BTreeMap<String, DsValue>),
    Object {
        schema_id: u32,
        fields: BTreeMap<u32, DsValue>,
    },
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
            (
                DsValue::Object {
                    schema_id: s1,
                    fields: f1,
                },
                DsValue::Object {
                    schema_id: s2,
                    fields: f2,
                },
            ) => match s1.partial_cmp(s2) {
                Some(std::cmp::Ordering::Equal) => f1.partial_cmp(f2),
                ord => ord,
            },
            (DsValue::Ext(s1, v1), DsValue::Ext(s2, v2)) => match s1.partial_cmp(s2) {
                Some(std::cmp::Ordering::Equal) => v1.partial_cmp(v2),
                ord => ord,
            },
            _ => None,
        }
    }
}

impl DsValue {}

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
            DsValue::Object { schema_id, fields } => {
                write!(f, "Object(schema={}, fields={:?})", schema_id, fields)
            }
            DsValue::Ext(t, b) => write!(f, "Ext({}, 0x{})", t, hex::encode(b)),
            DsValue::File(file) => write!(
                f,
                "File({}, name={}, size={})",
                file.file_id, file.name, file.size
            ),
        }
    }
}

pub trait ToValue {
    fn to_value(&self) -> DsValue;
}

pub trait FromValue: Sized {
    fn from_value(v: DsValue) -> DsResult<Self>;
}

impl ToValue for DsValue {
    fn to_value(&self) -> DsValue {
        self.clone()
    }
}

impl ToValue for i64 {
    fn to_value(&self) -> DsValue {
        DsValue::Int(*self)
    }
}

impl ToValue for f32 {
    fn to_value(&self) -> DsValue {
        DsValue::Float(*self as f64)
    }
}

impl ToValue for String {
    fn to_value(&self) -> DsValue {
        DsValue::Text(self.clone())
    }
}

impl ToValue for &str {
    fn to_value(&self) -> DsValue {
        DsValue::Text(self.to_string())
    }
}

impl ToValue for bool {
    fn to_value(&self) -> DsValue {
        DsValue::Bool(*self)
    }
}

impl ToValue for Uuid {
    fn to_value(&self) -> DsValue {
        DsValue::Uuid(*self)
    }
}
