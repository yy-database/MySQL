use crate::{DsError, DsFile, DsValue};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use chrono::{Datelike, TimeZone, Timelike};
use crc32fast::Hasher;
use uuid::Uuid;

/// YYValue Binary Header (16 bytes minimum)
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
                    Self::encode_value(value, dst)?;
                    tag_offsets.push((tag, offset));
                }
                for (i, (tag, offset)) in tag_offsets.iter().enumerate() {
                    let pos = map_pos + i * 8;
                    let mut slice = &mut dst[pos..pos + 8];
                    slice.put_u32(*tag);
                    slice.put_u32(*offset);
                }
                Ok(ValueTypeId::Object)
            }
            DsValue::Ext(t, b) => {
                let type_id: u8 = t.parse().unwrap_or(255);
                dst.put_u8(type_id);
                dst.put_u32(b.len() as u32);
                dst.put_slice(b);
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

pub struct YYValueDecoder;

impl YYValueDecoder {
    pub fn decode(src: &Bytes) -> Result<DsValue, DsError> {
        let mut buf = src.clone();
        Self::decode_from_stream(&mut buf)
    }

    pub fn decode_from_stream(src: &mut Bytes) -> Result<DsValue, DsError> {
        let header = ValueHeader::decode(src)?;
        let payload_start = header.payload_offset as usize;
        let total_len = header.length as usize;
        if src.len() < total_len {
            return Err(DsError::serialization_with_format(
                "binary",
                "Incomplete value data",
            ));
        }
        let full_data = src.split_to(total_len);
        let payload = &full_data[payload_start..total_len];
        let mut hasher = Hasher::new();
        hasher.update(payload);
        if hasher.finalize() != header.checksum {
            return Err(DsError::serialization_with_format(
                "binary",
                "Checksum mismatch",
            ));
        }
        let mut payload_buf = full_data.slice(payload_start..total_len);
        Self::decode_value_with_type(header.value_type, &mut payload_buf)
    }

    pub fn decode_value(src: &mut Bytes) -> Result<DsValue, DsError> {
        if src.is_empty() {
            return Err(DsError::serialization_with_format(
                "binary",
                "Unexpected end of data",
            ));
        }
        let type_id = src.get_u8();
        Self::decode_value_with_type(type_id, src)
    }

    pub fn decode_value_with_type(type_id: u8, src: &mut Bytes) -> Result<DsValue, DsError> {
        match type_id {
            0 => Ok(DsValue::Null),
            1 => Ok(DsValue::Bool(src.get_u8() != 0)),
            2 => Ok(DsValue::Int(src.get_i64())),
            3 => Ok(DsValue::Float(src.get_f64())),
            4 => {
                let len = src.get_u32() as usize;
                let data = src.split_to(len);
                let s = String::from_utf8(data.to_vec())
                    .map_err(|e| DsError::serialization_with_format("utf8", e.to_string()))?;
                Ok(DsValue::Text(s))
            }
            5 => {
                let len = src.get_u32() as usize;
                let data = src.split_to(len);
                Ok(DsValue::Bytes(data))
            }
            6 => {
                let mut bytes = [0u8; 16];
                bytes.copy_from_slice(&src.split_to(16));
                Ok(DsValue::Uuid(Uuid::from_bytes(bytes)))
            }
            7 => {
                let len = src.get_u32() as usize;
                let data = src.split_to(len);
                let s = String::from_utf8(data.to_vec())
                    .map_err(|e| DsError::serialization_with_format("utf8", e.to_string()))?;
                let d = s.parse().map_err(|e: rust_decimal::Error| {
                    DsError::type_err("Decimal", e.to_string())
                })?;
                Ok(DsValue::Decimal(d))
            }
            8 => {
                let days = src.get_i32();
                let d = chrono::NaiveDate::from_num_days_from_ce_opt(days)
                    .ok_or_else(|| DsError::type_err("NaiveDate", "Invalid date"))?;
                Ok(DsValue::Date(d))
            }
            9 => {
                let nanos = src.get_u64();
                let t = chrono::NaiveTime::from_num_seconds_from_midnight_opt(
                    (nanos / 1_000_000_000) as u32,
                    (nanos % 1_000_000_000) as u32,
                )
                .ok_or_else(|| DsError::type_err("NaiveTime", "Invalid time"))?;
                Ok(DsValue::Time(t))
            }
            10 => {
                let secs = src.get_i64();
                let nanos = src.get_u32();
                let dt = chrono::Utc
                    .timestamp_opt(secs, nanos)
                    .single()
                    .ok_or_else(|| DsError::type_err("DateTime", "Invalid timestamp"))?;
                Ok(DsValue::DateTime(dt))
            }
            12 => {
                let len = src.get_u32() as usize;
                let mut v = Vec::with_capacity(len);
                for _ in 0..len {
                    v.push(src.get_f32());
                }
                Ok(DsValue::Vector(crate::Vector(v)))
            }
            13 => {
                let len = src.get_u32() as usize;
                // Skip offset table (len * 4 bytes)
                let _offsets_buf = src.split_to(len * 4);
                let mut v = Vec::with_capacity(len);
                for _ in 0..len {
                    v.push(Self::decode_value(src)?);
                }
                Ok(DsValue::List(v))
            }
            14 => {
                let len = src.get_u32() as usize;
                // Skip index table (len * 12 bytes)
                let _index_buf = src.split_to(len * 12);
                let mut d = std::collections::BTreeMap::new();
                for _ in 0..len {
                    let key_len = src.get_u32() as usize;
                    let key_data = src.split_to(key_len);
                    let key = String::from_utf8(key_data.to_vec())
                        .map_err(|e| DsError::serialization_with_format("utf8", e.to_string()))?;
                    let val = Self::decode_value(src)?;
                    d.insert(key, val);
                }
                Ok(DsValue::Dict(d))
            }
            15 => {
                let schema_id = src.get_u32();
                let len = src.get_u32() as usize;
                let map_buf = src.split_to(len * 8);
                let mut tags = Vec::with_capacity(len);
                for i in 0..len {
                    let tag = u32::from_be_bytes([
                        map_buf[i * 8],
                        map_buf[i * 8 + 1],
                        map_buf[i * 8 + 2],
                        map_buf[i * 8 + 3],
                    ]);
                    tags.push(tag);
                }

                let mut fields = std::collections::BTreeMap::new();
                for tag in tags {
                    let val = Self::decode_value(src)?;
                    fields.insert(tag, val);
                }
                Ok(DsValue::Object { schema_id, fields })
            }
            16 => {
                let type_id_str = src.get_u8().to_string();
                let len = src.get_u32() as usize;
                let data = src.split_to(len);
                Ok(DsValue::Ext(type_id_str, data.to_vec()))
            }
            17 => {
                let len = src.get_u32() as usize;
                let data = src.split_to(len);
                Ok(DsValue::Binary(data))
            }
            18 => {
                let mut uuid_bytes = [0u8; 16];
                uuid_bytes.copy_from_slice(&src.split_to(16));
                let file_id = Uuid::from_bytes(uuid_bytes);
                let name_len = src.get_u32() as usize;
                let name_data = src.split_to(name_len);
                let name = String::from_utf8(name_data.to_vec())
                    .map_err(|e| DsError::serialization_with_format("utf8", e.to_string()))?;
                let size = src.get_u64();
                let mime_len = src.get_u32() as usize;
                let mime_data = src.split_to(mime_len);
                let mime_type = String::from_utf8(mime_data.to_vec())
                    .map_err(|e| DsError::serialization_with_format("utf8", e.to_string()))?;
                Ok(DsValue::File(DsFile {
                    file_id,
                    name,
                    size,
                    mime_type,
                }))
            }
            _ => Err(DsError::serialization_with_format(
                "binary",
                format!("Unsupported type id: {}", type_id),
            )),
        }
    }
}
