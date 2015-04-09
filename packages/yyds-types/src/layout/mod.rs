use crate::{DsError, DsValue};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use chrono::{Datelike, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};
use crc32fast::Hasher;
use uuid::Uuid;

/// DsValue Binary Header (16 bytes minimum)
/// | Magic (2B) | Type (1B) | Flags (1B) | Length (4B) | Checksum (4B) | Payload_Offset (4B) |
#[derive(Debug, Clone, PartialEq)]
pub struct ValueHeader {
    pub magic: [u8; 2],
    pub value_type: u8,
    pub flags: u8,
    pub length: u32,
    pub checksum: u32,
    pub payload_offset: u32,
}

pub const VALUE_MAGIC: [u8; 2] = *b"YY";

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueTypeId {
    Null = 0,
    Bool = 1,
    Int = 2,
    Float = 3,
    Text = 4,
    Bytes = 5,
    Uuid = 6,
    Decimal = 7,
    Date = 8,
    Time = 9,
    DateTime = 10,
    Vector = 12,
    List = 13,
    Dict = 14,
    Object = 15,
    Ext = 16,
    Binary = 17,
    File = 18,
}

impl ValueHeader {
    pub const SIZE: usize = 16;

    pub fn encode(&self, dst: &mut BytesMut) {
        dst.put_slice(&self.magic);
        dst.put_u8(self.value_type);
        dst.put_u8(self.flags);
        dst.put_u32(self.length);
        dst.put_u32(self.checksum);
        dst.put_u32(self.payload_offset);
    }

    pub fn decode(src: &[u8]) -> Result<Self, DsError> {
        if src.len() < Self::SIZE {
            return Err(DsError::serialization_with_format(
                "binary",
                "Insufficient header data",
            ));
        }

        let magic = [src[0], src[1]];
        if magic != VALUE_MAGIC {
            return Err(DsError::serialization_with_format(
                "binary",
                "Invalid value magic",
            ));
        }

        let value_type = src[2];
        let flags = src[3];
        let length = u32::from_be_bytes([src[4], src[5], src[6], src[7]]);
        let checksum = u32::from_be_bytes([src[8], src[9], src[10], src[11]]);
        let payload_offset = u32::from_be_bytes([src[12], src[13], src[14], src[15]]);

        Ok(ValueHeader {
            magic,
            value_type,
            flags,
            length,
            checksum,
            payload_offset,
        })
    }
}

pub struct DsValueEncoder;

impl DsValueEncoder {
    pub fn encode(value: &DsValue) -> Result<Bytes, DsError> {
        let mut result = BytesMut::new();
        Self::encode_to_buf(value, &mut result)?;
        Ok(result.freeze())
    }

    pub fn encode_to_buf(value: &DsValue, dst: &mut BytesMut) -> Result<(), DsError> {
        let mut payload = BytesMut::new();
        let type_id = Self::encode_payload(value, &mut payload)?;

        let mut hasher = Hasher::new();
        hasher.update(&payload);
        let checksum = hasher.finalize();

        let header = ValueHeader {
            magic: VALUE_MAGIC,
            value_type: type_id as u8,
            flags: 0,
            length: (ValueHeader::SIZE + payload.len()) as u32,
            checksum,
            payload_offset: ValueHeader::SIZE as u32,
        };

        header.encode(dst);
        dst.put(payload);
        Ok(())
    }

    pub fn encode_value(value: &DsValue, dst: &mut BytesMut) -> Result<ValueTypeId, DsError> {
        let pos = dst.len();
        dst.put_u8(0); // Placeholder for type ID
        let type_id = Self::encode_payload(value, dst)?;
        dst[pos] = type_id as u8;
        Ok(type_id)
    }

    fn encode_payload(value: &DsValue, dst: &mut BytesMut) -> Result<ValueTypeId, DsError> {
        match value {
            DsValue::Null => Ok(ValueTypeId::Null),
            DsValue::Bool(b) => {
                dst.put_u8(if *b { 1 } else { 0 });
                Ok(ValueTypeId::Bool)
            }
            DsValue::Int(i) => {
                dst.put_i64(*i);
                Ok(ValueTypeId::Int)
            }
            DsValue::Float(f) => {
                dst.put_f64(*f);
                Ok(ValueTypeId::Float)
            }
            DsValue::Text(s) => {
                dst.put_u32(s.len() as u32);
                dst.put_slice(s.as_bytes());
                Ok(ValueTypeId::Text)
            }
            DsValue::Bytes(b) => {
                dst.put_u32(b.len() as u32);
                dst.put_slice(b);
                Ok(ValueTypeId::Bytes)
            }
            DsValue::Binary(b) => {
                dst.put_u32(b.len() as u32);
                dst.put_slice(b);
                Ok(ValueTypeId::Binary)
            }
            DsValue::Uuid(u) => {
                dst.put_slice(u.as_bytes());
                Ok(ValueTypeId::Uuid)
            }
            DsValue::Decimal(d) => {
                let s = d.to_string();
                dst.put_u32(s.len() as u32);
                dst.put_slice(s.as_bytes());
                Ok(ValueTypeId::Decimal)
            }
            DsValue::Date(d) => {
                dst.put_i32(d.num_days_from_ce());
                Ok(ValueTypeId::Date)
            }
            DsValue::Time(t) => {
                let nanos =
                    t.num_seconds_from_midnight() as u64 * 1_000_000_000 + t.nanosecond() as u64;
                dst.put_u64(nanos);
                Ok(ValueTypeId::Time)
            }
            DsValue::DateTime(dt) => {
                dst.put_i64(dt.timestamp());
                dst.put_u32(dt.timestamp_subsec_nanos());
                Ok(ValueTypeId::DateTime)
            }
            DsValue::Vector(v) => {
                dst.put_u32(v.0.len() as u32);
                for &f in &v.0 {
                    dst.put_f32(f);
                }
                Ok(ValueTypeId::Vector)
            }
            DsValue::List(l) => {
                dst.put_u32(l.len() as u32);
                let table_pos = dst.len();
                for _ in 0..l.len() {
                    dst.put_u32(0);
                }
                let mut offsets = Vec::with_capacity(l.len());
                let base_offset = dst.len() as u32;
                for item in l {
                    offsets.push(dst.len() as u32 - base_offset);
                    Self::encode_value(item, dst)?;
                }
                for (i, &offset) in offsets.iter().enumerate() {
                    let pos = table_pos + i * 4;
                    let mut slice = &mut dst[pos..pos + 4];
                    slice.put_u32(offset);
                }
                Ok(ValueTypeId::List)
            }
            DsValue::Dict(d) => {
                dst.put_u32(d.len() as u32);
                let mut entries: Vec<_> = d.iter().collect();
                entries.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
                let index_pos = dst.len();
                for _ in 0..d.len() {
                    dst.put_u64(0); // Hash
                    dst.put_u32(0); // Key offset
                }
                let mut hash_offsets = Vec::with_capacity(d.len());
                let base_offset = dst.len() as u32;
                for (key, value) in &entries {
                    let key_offset = dst.len() as u32 - base_offset;
                    dst.put_u32(key.len() as u32);
                    dst.put_slice(key.as_bytes());
                    Self::encode_value(value, dst)?;
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    use std::hash::{Hash, Hasher as _};
                    key.hash(&mut hasher);
                    hash_offsets.push((hasher.finish(), key_offset));
                }
                for (i, (hash, key_off)) in hash_offsets.iter().enumerate() {
                    let pos = index_pos + i * 12;
                    let mut slice = &mut dst[pos..pos + 12];
                    slice.put_u64(*hash);
                    slice.put_u32(*key_off);
                }
                Ok(ValueTypeId::Dict)
            }
            DsValue::Object { schema_id, fields } => {
                dst.put_u32(*schema_id);
                dst.put_u32(fields.len() as u32);
                let mut sorted_fields: Vec<_> = fields.iter().collect();
                sorted_fields.sort_by_key(|&(&tag, _)| tag);
                let map_pos = dst.len();
                for _ in 0..fields.len() {
                    dst.put_u32(0); // Tag
                    dst.put_u32(0); // Offset
                }
                let mut tag_offsets = Vec::with_capacity(fields.len());
                let base_offset = dst.len() as u32;
                for (&tag, value) in sorted_fields {
                    let offset = dst.len() as u32 - base_offset;
                    tag_offsets.push((tag, offset));
                    Self::encode_value(value, dst)?;
                }
                for (i, (tag, offset)) in tag_offsets.iter().enumerate() {
                    let pos = map_pos + i * 8;
                    let mut slice = &mut dst[pos..pos + 8];
                    slice.put_u32(*tag);
                    slice.put_u32(*offset);
                }
                Ok(ValueTypeId::Object)
            }
            DsValue::Ext(name, data) => {
                dst.put_u32(name.len() as u32);
                dst.put_slice(name.as_bytes());
                dst.put_u32(data.len() as u32);
                dst.put_slice(data);
                Ok(ValueTypeId::Ext)
            }
            DsValue::File(file) => {
                dst.put_slice(file.file_id.as_bytes());
                dst.put_u32(file.name.len() as u32);
                dst.put_slice(file.name.as_bytes());
                dst.put_u64(file.size);
                dst.put_u32(file.mime_type.len() as u32);
                dst.put_slice(file.mime_type.as_bytes());
                Ok(ValueTypeId::File)
            }
        }
    }
}

pub struct DsValueDecoder;

impl DsValueDecoder {
    pub fn decode(src: &mut Bytes) -> Result<DsValue, DsError> {
        let header = ValueHeader::decode(src)?;
        src.advance(ValueHeader::SIZE);
        Self::decode_payload(ValueTypeId::from_u8(header.value_type), src)
    }

    fn decode_payload(type_id: ValueTypeId, src: &mut Bytes) -> Result<DsValue, DsError> {
        match type_id {
            ValueTypeId::Null => Ok(DsValue::Null),
            ValueTypeId::Bool => {
                let b = src.get_u8() != 0;
                Ok(DsValue::Bool(b))
            }
            ValueTypeId::Int => Ok(DsValue::Int(src.get_i64())),
            ValueTypeId::Float => Ok(DsValue::Float(src.get_f64())),
            ValueTypeId::Text => {
                let len = src.get_u32() as usize;
                let data = src.split_to(len);
                let s = String::from_utf8(data.to_vec())
                    .map_err(|e| DsError::serialization_with_format("binary", e.to_string()))?;
                Ok(DsValue::Text(s))
            }
            ValueTypeId::Bytes => {
                let len = src.get_u32() as usize;
                let data = src.split_to(len);
                Ok(DsValue::Bytes(data))
            }
            ValueTypeId::Binary => {
                let len = src.get_u32() as usize;
                let data = src.split_to(len);
                Ok(DsValue::Binary(data))
            }
            ValueTypeId::Uuid => {
                let mut bytes = [0u8; 16];
                src.copy_to_slice(&mut bytes);
                Ok(DsValue::Uuid(Uuid::from_bytes(bytes)))
            }
            ValueTypeId::Decimal => {
                let len = src.get_u32() as usize;
                let data = src.split_to(len);
                let s = String::from_utf8(data.to_vec())
                    .map_err(|e| DsError::serialization_with_format("binary", e.to_string()))?;
                use std::str::FromStr;
                let d = rust_decimal::Decimal::from_str(&s)
                    .map_err(|e| DsError::serialization_with_format("binary", e.to_string()))?;
                Ok(DsValue::Decimal(d))
            }
            ValueTypeId::Date => {
                let days = src.get_i32();
                let d = NaiveDate::from_num_days_from_ce_opt(days)
                    .ok_or_else(|| DsError::serialization_with_format("binary", "Invalid date"))?;
                Ok(DsValue::Date(d))
            }
            ValueTypeId::Time => {
                let nanos = src.get_u64();
                let secs = nanos / 1_000_000_000;
                let n = (nanos % 1_000_000_000) as u32;
                let t = NaiveTime::from_num_seconds_from_midnight_opt(secs as u32, n)
                    .ok_or_else(|| DsError::serialization_with_format("binary", "Invalid time"))?;
                Ok(DsValue::Time(t))
            }
            ValueTypeId::DateTime => {
                let secs = src.get_i64();
                let n = src.get_u32();
                let dt = Utc.timestamp_opt(secs, n)
                    .single()
                    .ok_or_else(|| DsError::serialization_with_format("binary", "Invalid datetime"))?;
                Ok(DsValue::DateTime(dt))
            }
            ValueTypeId::Vector => {
                let len = src.get_u32() as usize;
                let mut v = Vec::with_capacity(len);
                for _ in 0..len {
                    v.push(src.get_f32());
                }
                Ok(DsValue::Vector(crate::Vector(v)))
            }
            ValueTypeId::List => {
                let len = src.get_u32() as usize;
                let mut offsets = Vec::with_capacity(len);
                for _ in 0..len {
                    offsets.push(src.get_u32());
                }
                let mut list = Vec::with_capacity(len);
                for _ in 0..len {
                    list.push(Self::decode_value(src)?);
                }
                Ok(DsValue::List(list))
            }
            ValueTypeId::Dict => {
                let len = src.get_u32() as usize;
                let mut index = Vec::with_capacity(len);
                for _ in 0..len {
                    let hash = src.get_u64();
                    let offset = src.get_u32();
                    index.push((hash, offset));
                }
                let mut dict = std::collections::BTreeMap::new();
                for _ in 0..len {
                    let key_len = src.get_u32() as usize;
                    let key_data = src.split_to(key_len);
                    let key = String::from_utf8(key_data.to_vec())
                        .map_err(|e| DsError::serialization_with_format("binary", e.to_string()))?;
                    let value = Self::decode_value(src)?;
                    dict.insert(key, value);
                }
                Ok(DsValue::Dict(dict))
            }
            ValueTypeId::Object => {
                let schema_id = src.get_u32();
                let len = src.get_u32() as usize;
                let mut tag_index = Vec::with_capacity(len);
                for _ in 0..len {
                    let tag = src.get_u32();
                    let offset = src.get_u32();
                    tag_index.push((tag, offset));
                }
                let mut fields = std::collections::BTreeMap::new();
                for (tag, _) in tag_index {
                    let value = Self::decode_value(src)?;
                    fields.insert(tag, value);
                }
                Ok(DsValue::Object { schema_id, fields })
            }
            ValueTypeId::Ext => {
                let name_len = src.get_u32() as usize;
                let name_data = src.split_to(name_len);
                let name = String::from_utf8(name_data.to_vec())
                    .map_err(|e| DsError::serialization_with_format("binary", e.to_string()))?;
                let data_len = src.get_u32() as usize;
                let data = src.split_to(data_len).to_vec();
                Ok(DsValue::Ext(name, data))
            }
            ValueTypeId::File => {
                let mut file_id_bytes = [0u8; 16];
                src.copy_to_slice(&mut file_id_bytes);
                let file_id = Uuid::from_bytes(file_id_bytes);
                let name_len = src.get_u32() as usize;
                let name_data = src.split_to(name_len);
                let name = String::from_utf8(name_data.to_vec())
                    .map_err(|e| DsError::serialization_with_format("binary", e.to_string()))?;
                let size = src.get_u64();
                let mime_len = src.get_u32() as usize;
                let mime_data = src.split_to(mime_len);
                let mime_type = String::from_utf8(mime_data.to_vec())
                    .map_err(|e| DsError::serialization_with_format("binary", e.to_string()))?;
                Ok(DsValue::File(crate::DsFile {
                    file_id,
                    name,
                    size,
                    mime_type,
                }))
            }
        }
    }

    pub fn decode_value(src: &mut Bytes) -> Result<DsValue, DsError> {
        let type_id_u8 = src.get_u8();
        let type_id = ValueTypeId::from_u8(type_id_u8);
        Self::decode_payload(type_id, src)
    }
}

impl ValueTypeId {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => ValueTypeId::Null,
            1 => ValueTypeId::Bool,
            2 => ValueTypeId::Int,
            3 => ValueTypeId::Float,
            4 => ValueTypeId::Text,
            5 => ValueTypeId::Bytes,
            6 => ValueTypeId::Uuid,
            7 => ValueTypeId::Decimal,
            8 => ValueTypeId::Date,
            9 => ValueTypeId::Time,
            10 => ValueTypeId::DateTime,
            12 => ValueTypeId::Vector,
            13 => ValueTypeId::List,
            14 => ValueTypeId::Dict,
            15 => ValueTypeId::Object,
            16 => ValueTypeId::Ext,
            17 => ValueTypeId::Binary,
            18 => ValueTypeId::File,
            _ => ValueTypeId::Null,
        }
    }
}

/// DsValue Binary Encoder for DsValue.
pub struct DsValueBinaryEncoder;

impl DsValueBinaryEncoder {
    /// Encode a DsValue into its binary representation.
    pub fn encode(value: &crate::DsValue) -> Result<bytes::Bytes, crate::DsError> {
        DsValueEncoder::encode(value)
    }

    /// Get the magic bytes used for binary format.
    pub fn magic() -> [u8; 2] {
        VALUE_MAGIC
    }
}

