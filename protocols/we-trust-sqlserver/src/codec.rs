use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use yykv_types::DsError;

/// TDS Packet Header
#[derive(Debug, Clone)]
pub struct TdsPacket {
    pub packet_type: u8,
    pub status: u8,
    pub length: u16,
    pub spid: u16,
    pub packet_id: u8,
    pub window: u8,
    pub payload: Vec<u8>,
}

pub struct TdsCodec;

impl Decoder for TdsCodec {
    type Item = TdsPacket;
    type Error = DsError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 8 {
            return Ok(None);
        }

        let length = u16::from_be_bytes([src[2], src[3]]) as usize;
        if src.len() < length {
            src.reserve(length);
            return Ok(None);
        }

        let packet_type = src[0];
        let status = src[1];
        let spid = u16::from_be_bytes([src[4], src[5]]);
        let packet_id = src[6];
        let window = src[7];

        src.advance(8);
        let payload = src.split_to(length - 8).to_vec();

        Ok(Some(TdsPacket {
            packet_type,
            status,
            length: length as u16,
            spid,
            packet_id,
            window,
            payload,
        }))
    }
}

impl Encoder<TdsPacket> for TdsCodec {
    type Error = DsError;

    fn encode(&mut self, item: TdsPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.put_u8(item.packet_type);
        dst.put_u8(item.status);
        dst.put_u16(8 + item.payload.len() as u16);
        dst.put_u16(item.spid);
        dst.put_u8(item.packet_id);
        dst.put_u8(item.window);
        dst.put_slice(&item.payload);
        Ok(())
    }
}
