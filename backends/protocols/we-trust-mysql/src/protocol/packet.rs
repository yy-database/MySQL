use bytes::{Buf, BufMut, Bytes, BytesMut};
use crate::Result;

pub struct Packet {
    pub sequence_id: u8,
    pub payload: Bytes,
}

impl Packet {
    pub fn new(sequence_id: u8, payload: Bytes) -> Self {
        Self {
            sequence_id,
            payload,
        }
    }

    pub fn decode(src: &mut BytesMut) -> Result<Option<Self>> {
        if src.len() < 4 {
            return Ok(None);
        }

        let payload_len = (src[0] as usize) | ((src[1] as usize) << 8) | ((src[2] as usize) << 16);
        let sequence_id = src[3];

        if src.len() < 4 + payload_len {
            return Ok(None);
        }

        src.advance(4);
        let payload = src.split_to(payload_len).freeze();

        Ok(Some(Packet {
            sequence_id,
            payload,
        }))
    }

    pub fn encode(&self, dst: &mut BytesMut) -> Result<()> {
        let payload_len = self.payload.len();
        dst.put_u8((payload_len & 0xFF) as u8);
        dst.put_u8(((payload_len >> 8) & 0xFF) as u8);
        dst.put_u8(((payload_len >> 16) & 0xFF) as u8);
        dst.put_u8(self.sequence_id);
        dst.put_slice(&self.payload);
        Ok(())
    }
}
