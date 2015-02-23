use crate::{ArrowField, ArrowMessage, ArrowRecordBatch, ArrowSchema};
use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use yykv_types::DsError;

pub struct ArrowCodec {
    // 标记是否已经读取过 Schema
    has_read_schema: bool,
}

impl Default for ArrowCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl ArrowCodec {
    pub fn new() -> Self {
        Self {
            has_read_schema: false,
        }
    }
}

impl Decoder for ArrowCodec {
    type Item = ArrowMessage;
    type Error = DsError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }

        // 读取长度前缀 (Little Endian)
        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&src[..4]);
        let length = u32::from_le_bytes(len_bytes) as usize;

        if src.len() < 4 + length {
            src.reserve(length);
            return Ok(None);
        }

        // 跳过长度字段
        src.advance(4);
        let mut data = src.split_to(length);

        if !self.has_read_schema {
            // 解析 Schema (手工简化实现)
            let field_count = data.get_u32_le() as usize;
            let mut fields = Vec::with_capacity(field_count);
            for _ in 0..field_count {
                let name_len = data.get_u32_le() as usize;
                let name = String::from_utf8_lossy(&data.split_to(name_len)).to_string();
                let type_len = data.get_u32_le() as usize;
                let data_type = String::from_utf8_lossy(&data.split_to(type_len)).to_string();
                fields.push(ArrowField { name, data_type });
            }
            self.has_read_schema = true;
            Ok(Some(ArrowMessage::Schema(ArrowSchema { fields })))
        } else {
            // 解析 RecordBatch
            let row_count = data.get_u32_le() as usize;
            let col_count = data.get_u32_le() as usize;
            let mut columns = Vec::with_capacity(col_count);
            for _ in 0..col_count {
                let col_len = data.get_u32_le() as usize;
                columns.push(data.split_to(col_len).to_vec());
            }
            Ok(Some(ArrowMessage::Data(ArrowRecordBatch {
                row_count,
                columns,
            })))
        }
    }
}

impl Encoder<ArrowMessage> for ArrowCodec {
    type Error = DsError;

    fn encode(&mut self, item: ArrowMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            ArrowMessage::Schema(schema) => {
                let mut body = BytesMut::new();
                body.put_u32_le(schema.fields.len() as u32);
                for field in schema.fields {
                    body.put_u32_le(field.name.len() as u32);
                    body.put_slice(field.name.as_bytes());
                    body.put_u32_le(field.data_type.len() as u32);
                    body.put_slice(field.data_type.as_bytes());
                }
                dst.put_u32_le(body.len() as u32);
                dst.put_slice(&body);
            }
            ArrowMessage::Data(batch) => {
                let mut body = BytesMut::new();
                body.put_u32_le(batch.row_count as u32);
                body.put_u32_le(batch.columns.len() as u32);
                for col in batch.columns {
                    body.put_u32_le(col.len() as u32);
                    body.put_slice(&col);
                }
                dst.put_u32_le(body.len() as u32);
                dst.put_slice(&body);
            }
        }
        Ok(())
    }
}
