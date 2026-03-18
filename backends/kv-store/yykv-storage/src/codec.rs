use bytes::{Buf, BufMut, BytesMut};
use yykv_types::{DsError, DsResult, DsValue};

pub struct Codec;

impl Codec {
    pub fn encode_value(value: &DsValue, buf: &mut BytesMut) {
        match value {
            DsValue::Null => buf.put_u8(0),
            DsValue::Bool(b) => {
                buf.put_u8(1);
                buf.put_u8(if *b { 1 } else { 0 });
            }
            DsValue::Int(i) => {
                buf.put_u8(2);
                buf.put_i64(*i);
            }
            DsValue::Float(f) => {
                buf.put_u8(3);
                buf.put_f64(*f);
            }
            DsValue::Text(s) => {
                buf.put_u8(4);
                let bytes = s.as_bytes();
                buf.put_u32_le(bytes.len() as u32);
                buf.put_slice(bytes);
            }
            DsValue::Binary(b) => {
                buf.put_u8(5);
                buf.put_u32_le(b.len() as u32);
                buf.put_slice(b);
            }
            // Add other types as needed
            _ => {
                // Fallback or todo
                buf.put_u8(255);
            }
        }
    }

    pub fn decode_value(data: &mut &[u8]) -> DsResult<DsValue> {
        if data.is_empty() {
            return Err(DsError::storage("Empty data for decoding"));
        }
        let tag = data.get_u8();
        match tag {
            0 => Ok(DsValue::Null),
            1 => Ok(DsValue::Bool(data.get_u8() != 0)),
            2 => Ok(DsValue::Int(data.get_i64())),
            3 => Ok(DsValue::Float(data.get_f64())),
            4 => {
                let len = data.get_u32_le() as usize;
                let s = std::str::from_utf8(&data[..len])
                    .map_err(|e| DsError::storage(e.to_string()))?
                    .to_string();
                data.advance(len);
                Ok(DsValue::Text(s))
            }
            5 => {
                let len = data.get_u32_le() as usize;
                let b = data[..len].to_vec();
                data.advance(len);
                Ok(DsValue::Binary(b.into()))
            }
            _ => Err(DsError::storage(format!("Unknown tag: {}", tag))),
        }
    }
}
