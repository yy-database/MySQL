#![warn(missing_docs)]

use bytes::{BufMut, Bytes, BytesMut};
use crc32fast::Hasher;
use yyds_types::DsValue;

pub const HEADER_SIZE: usize = 32;
pub const MAGIC: [u8; 2] = *b"YY";

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Raw = 0,
    List = 1,
    Dict = 2,
    Object = 3,
}

#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub magic: [u8; 2],
    pub value_type: ValueType,
    pub flags: u8,
    pub length: u32,
    pub checksum: u32,
    pub payload_offset: u32,
    pub reserved: [u8; 16],
}

impl Header {
    pub fn new(value_type: ValueType, length: u32, payload_offset: u32) -> Self {
        Self {
            magic: MAGIC,
            value_type,
            flags: 0,
            length,
            checksum: 0,
            payload_offset,
            reserved: [0; 16],
        }
    }

    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..2].copy_from_slice(&self.magic);
        buf[2] = self.value_type as u8;
        buf[3] = self.flags;
        buf[4..8].copy_from_slice(&self.length.to_le_bytes());
        buf[8..12].copy_from_slice(&self.checksum.to_le_bytes());
        buf[12..16].copy_from_slice(&self.payload_offset.to_le_bytes());
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Option<Header> {
        if data.len() < HEADER_SIZE || data[0..2] != MAGIC {
            return None;
        }
        let vt = match data[2] {
            1 => ValueType::List,
            2 => ValueType::Dict,
            3 => ValueType::Object,
            _ => ValueType::Raw,
        };
        Some(Self {
            magic: MAGIC,
            value_type: vt,
            flags: data[3],
            length: u32::from_le_bytes(data[4..8].try_into().unwrap()),
            checksum: u32::from_le_bytes(data[8..12].try_into().unwrap()),
            payload_offset: u32::from_le_bytes(data[12..16].try_into().unwrap()),
            reserved: [0; 16],
        })
    }
}

pub struct ListLayout;
impl ListLayout {
    /// Layout: [Header] | [ItemCount(u32)] | [OffsetTable(u32 * ItemCount)] | [Item1][Item2]...
    pub fn encode(items: Vec<Bytes>) -> Bytes {
        let count = items.len() as u32;
        let mut offset_table = Vec::with_capacity(items.len());
        let mut current_offset = 4 + (4 * count); // Skip count and table

        let mut payload = BytesMut::new();
        payload.put_u32_le(count);

        for item in &items {
            offset_table.push(current_offset);
            current_offset += item.len() as u32;
        }

        for offset in offset_table {
            payload.put_u32_le(offset);
        }

        for item in items {
            payload.put(item);
        }

        let mut header = Header::new(
            ValueType::List,
            payload.len() as u32 + HEADER_SIZE as u32,
            HEADER_SIZE as u32,
        );
        let mut hasher = Hasher::new();
        hasher.update(&payload);
        header.checksum = hasher.finalize();

        let mut full = BytesMut::with_capacity(HEADER_SIZE + payload.len());
        full.put_slice(&header.to_bytes());
        full.put(payload);
        full.freeze()
    }
}

pub struct DictLayout;
impl DictLayout {
    /// Layout: [Header] | [KeyCount(u32)] | [SortedHashIndex(Hash:u32, Offset:u32) * KeyCount] | [KeyData] | [ValueData]
    pub fn encode(mut entries: Vec<(String, Bytes)>) -> Bytes {
        // Sort by key hash for O(log N) lookup
        entries.sort_by_key(|(k, _)| {
            let mut h = Hasher::new();
            h.update(k.as_bytes());
            h.finalize()
        });

        let count = entries.len() as u32;
        let mut index_data = BytesMut::new();
        let mut kv_data = BytesMut::new();

        let mut current_offset = 4 + (8 * count); // Skip count and index table

        for (key, val) in entries {
            let mut h = Hasher::new();
            h.update(key.as_bytes());
            let hash = h.finalize();

            index_data.put_u32_le(hash);
            index_data.put_u32_le(current_offset);

            // Write Key (len + data) and Value (len + data)
            let val_len = val.len() as u32;
            kv_data.put_u32_le(key.len() as u32);
            kv_data.put_slice(key.as_bytes());
            kv_data.put_u32_le(val_len);
            kv_data.put(val);

            current_offset += 4 + key.len() as u32 + 4 + val_len;
        }

        let mut payload = BytesMut::new();
        payload.put_u32_le(count);
        payload.put(index_data);
        payload.put(kv_data);

        let mut header = Header::new(
            ValueType::Dict,
            payload.len() as u32 + HEADER_SIZE as u32,
            HEADER_SIZE as u32,
        );
        let mut hasher = Hasher::new();
        hasher.update(&payload);
        header.checksum = hasher.finalize();

        let mut full = BytesMut::with_capacity(HEADER_SIZE + payload.len());
        full.put_slice(&header.to_bytes());
        full.put(payload);
        full.freeze()
    }
}

pub struct ObjectLayout;
impl ObjectLayout {
    /// Layout: [Header] | [SchemaID(u32)] | [TagCount(u32)] | [TagMap(Tag:u32, Offset:u32) * TagCount] | [FieldData]
    pub fn encode(schema_id: u32, mut fields: Vec<(u32, Bytes)>) -> Bytes {
        fields.sort_by_key(|(tag, _)| *tag);

        let count = fields.len() as u32;
        let mut tag_map = BytesMut::new();
        let mut field_data = BytesMut::new();

        let mut current_offset = 8 + (8 * count); // Skip schema_id, count and tag map

        for (tag, data) in fields {
            tag_map.put_u32_le(tag);
            tag_map.put_u32_le(current_offset);

            let data_len = data.len() as u32;
            field_data.put_u32_le(data_len);
            field_data.put(data);

            current_offset += 4 + data_len;
        }

        let mut payload = BytesMut::new();
        payload.put_u32_le(schema_id);
        payload.put_u32_le(count);
        payload.put(tag_map);
        payload.put(field_data);

        let mut header = Header::new(
            ValueType::Object,
            payload.len() as u32 + HEADER_SIZE as u32,
            HEADER_SIZE as u32,
        );
        let mut hasher = Hasher::new();
        hasher.update(&payload);
        header.checksum = hasher.finalize();

        let mut full = BytesMut::with_capacity(HEADER_SIZE + payload.len());
        full.put_slice(&header.to_bytes());
        full.put(payload);
        full.freeze()
    }
}

pub struct LayoutManager;

impl LayoutManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutManager {
    #[allow(clippy::only_used_in_recursion)]
    pub fn serialize(&self, value: &DsValue) -> Bytes {
        match value {
            DsValue::List(items) => {
                let serialized_items: Vec<Bytes> =
                    items.iter().map(|v| self.serialize(v)).collect();
                ListLayout::encode(serialized_items)
            }
            DsValue::Dict(entries) => {
                let serialized_entries: Vec<(String, Bytes)> = entries
                    .iter()
                    .map(|(k, v)| (k.clone(), self.serialize(v)))
                    .collect();
                DictLayout::encode(serialized_entries)
            }
            DsValue::Object { schema_id, fields } => {
                let serialized_fields: Vec<(u32, Bytes)> = fields
                    .iter()
                    .map(|(tag, v)| (*tag, self.serialize(v)))
                    .collect();
                ObjectLayout::encode(*schema_id, serialized_fields)
            }
            _ => {
                // Fallback to simple binary encoding for primitive types
                let payload = yykv_types::layout::DsValueEncoder::encode(value)
                    .unwrap_or_else(|_| Bytes::new());
                let mut header = Header::new(
                    ValueType::Raw,
                    payload.len() as u32 + HEADER_SIZE as u32,
                    HEADER_SIZE as u32,
                );
                let mut hasher = Hasher::new();
                hasher.update(&payload);
                header.checksum = hasher.finalize();

                let mut full = BytesMut::with_capacity(HEADER_SIZE + payload.len());
                full.put_slice(&header.to_bytes());
                full.put(payload);
                full.freeze()
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    pub fn deserialize(&self, data: &[u8]) -> Option<DsValue> {
        let header = Header::from_bytes(data)?;
        let payload = &data[HEADER_SIZE..];

        // Verify checksum
        let mut hasher = Hasher::new();
        hasher.update(payload);
        if hasher.finalize() != header.checksum {
            return None;
        }

        match header.value_type {
            ValueType::List => {
                let count = u32::from_le_bytes(payload[0..4].try_into().ok()?) as usize;
                let offset_table_start = 4;
                let mut items = Vec::with_capacity(count);

                for i in 0..count {
                    let start = offset_table_start + (i * 4);
                    let offset =
                        u32::from_le_bytes(payload[start..start + 4].try_into().ok()?) as usize;

                    // The item ends at the next offset or end of payload
                    let end = if i + 1 < count {
                        u32::from_le_bytes(payload[start + 4..start + 8].try_into().ok()?) as usize
                    } else {
                        payload.len()
                    };

                    let item_data = &payload[offset..end];
                    items.push(self.deserialize(item_data)?);
                }
                Some(DsValue::List(items))
            }
            ValueType::Dict => {
                let count = u32::from_le_bytes(payload[0..4].try_into().ok()?) as usize;
                let mut entries = std::collections::BTreeMap::new();
                let index_start = 4;

                for i in 0..count {
                    let entry_start = index_start + (i * 8);
                    // Skip hash (first 4 bytes of entry)
                    let offset = u32::from_le_bytes(
                        payload[entry_start + 4..entry_start + 8].try_into().ok()?,
                    ) as usize;

                    let mut curr = offset;
                    let key_len =
                        u32::from_le_bytes(payload[curr..curr + 4].try_into().ok()?) as usize;
                    curr += 4;
                    let key = String::from_utf8(payload[curr..curr + key_len].to_vec()).ok()?;
                    curr += key_len;

                    let val_len =
                        u32::from_le_bytes(payload[curr..curr + 4].try_into().ok()?) as usize;
                    curr += 4;
                    let val_data = &payload[curr..curr + val_len];
                    entries.insert(key, self.deserialize(val_data)?);
                }
                Some(DsValue::Dict(entries))
            }
            ValueType::Object => {
                let schema_id = u32::from_le_bytes(payload[0..4].try_into().ok()?);
                let count = u32::from_le_bytes(payload[4..8].try_into().ok()?) as usize;
                let mut fields = std::collections::BTreeMap::new();
                let map_start = 8;

                for i in 0..count {
                    let entry_start = map_start + (i * 8);
                    let tag =
                        u32::from_le_bytes(payload[entry_start..entry_start + 4].try_into().ok()?);
                    let offset = u32::from_le_bytes(
                        payload[entry_start + 4..entry_start + 8].try_into().ok()?,
                    ) as usize;

                    let val_len =
                        u32::from_le_bytes(payload[offset..offset + 4].try_into().ok()?) as usize;
                    let val_data = &payload[offset + 4..offset + 4 + val_len];
                    fields.insert(tag, self.deserialize(val_data)?);
                }
                Some(DsValue::Object { schema_id, fields })
            }
            ValueType::Raw => {
                let mut data = Bytes::copy_from_slice(payload);
                yyds_types::layout::DsValueDecoder::decode(&mut data).ok()
            }
        }
    }
}
