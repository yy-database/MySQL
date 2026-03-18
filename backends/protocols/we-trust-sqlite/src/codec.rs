use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use yykv_types::DsError;

#[derive(Debug, Clone)]
pub enum SqliteCommand {
    Execute(String),
    Query(String),
}

#[derive(Debug, Clone)]
pub enum SqliteResponse {
    Ok,
    Row(Vec<u8>),
    Error(String),
}

pub struct SqliteCodec;

impl Decoder for SqliteCodec {
    type Item = SqliteCommand;
    type Error = DsError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 5 {
            return Ok(None);
        }

        let tag = src[0];
        let len = u32::from_le_bytes([src[1], src[2], src[3], src[4]]) as usize;

        if src.len() < 5 + len {
            src.reserve(len);
            return Ok(None);
        }

        src.advance(5);
        let data = src.split_to(len);
        let sql = String::from_utf8_lossy(&data).to_string();

        match tag {
            1 => Ok(Some(SqliteCommand::Execute(sql))),
            2 => Ok(Some(SqliteCommand::Query(sql))),
            _ => Err(DsError::protocol("Invalid SQLite command tag")),
        }
    }
}

impl Encoder<SqliteResponse> for SqliteCodec {
    type Error = DsError;

    fn encode(&mut self, item: SqliteResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            SqliteResponse::Ok => {
                dst.put_u8(0);
                dst.put_u32_le(0);
            }
            SqliteResponse::Row(data) => {
                dst.put_u8(1);
                dst.put_u32_le(data.len() as u32);
                dst.put_slice(&data);
            }
            SqliteResponse::Error(e) => {
                dst.put_u8(255);
                let bytes = e.as_bytes();
                dst.put_u32_le(bytes.len() as u32);
                dst.put_slice(bytes);
            }
        }
        Ok(())
    }
}
