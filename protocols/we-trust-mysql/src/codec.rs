use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use yykv_types::DsError;

/// MySQL 协议包
#[derive(Debug, Clone)]
pub struct MySqlPacket {
    pub sequence_id: u8,
    pub payload: Bytes,
}

/// we-trust-mysql 为 YYKV 提供的 MySQL 协议编解码器
pub struct MySqlCodec;

impl MySqlCodec {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MySqlCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for MySqlCodec {
    type Item = MySqlPacket;
    type Error = DsError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }

        let payload_len = (src[0] as usize) | ((src[1] as usize) << 8) | ((src[2] as usize) << 16);
        let sequence_id = src[3];

        if src.len() < 4 + payload_len {
            src.reserve(payload_len);
            return Ok(None);
        }

        src.advance(4);
        let payload = src.split_to(payload_len).freeze();

        Ok(Some(MySqlPacket {
            sequence_id,
            payload,
        }))
    }
}

impl Encoder<MySqlPacket> for MySqlCodec {
    type Error = DsError;

    fn encode(&mut self, item: MySqlPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let payload_len = item.payload.len();
        if payload_len > 0xFFFFFF {
            return Err(DsError::protocol(
                "Payload too large for single MySQL packet",
            ));
        }

        dst.put_u8((payload_len & 0xFF) as u8);
        dst.put_u8(((payload_len >> 8) & 0xFF) as u8);
        dst.put_u8(((payload_len >> 16) & 0xFF) as u8);
        dst.put_u8(item.sequence_id);
        dst.put_slice(&item.payload);

        Ok(())
    }
}
