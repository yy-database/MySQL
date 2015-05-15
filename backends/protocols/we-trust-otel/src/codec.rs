use crate::{LogData, MetricData, OtelMessage, TraceData};
use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use yykv_types::DsError;

pub struct OtelCodec;

impl OtelCodec {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OtelCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for OtelCodec {
    type Item = OtelMessage;
    type Error = DsError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Header: [Type: u8][Length: u32] = 5 bytes
        if src.len() < 5 {
            return Ok(None);
        }

        let msg_type = src[0];
        let payload_len = u32::from_be_bytes([src[1], src[2], src[3], src[4]]) as usize;

        if src.len() < 5 + payload_len {
            src.reserve(5 + payload_len - src.len());
            return Ok(None);
        }

        src.advance(5);
        let mut payload = src.split_to(payload_len);

        match msg_type {
            1 => {
                // Trace
                if payload.len() < 48 + 2 {
                    return Err(DsError::protocol("Invalid Trace payload"));
                }
                let mut trace_id = [0u8; 16];
                payload.copy_to_slice(&mut trace_id);
                let mut span_id = [0u8; 8];
                payload.copy_to_slice(&mut span_id);
                let mut parent_span_id = [0u8; 8];
                payload.copy_to_slice(&mut parent_span_id);
                let start_time = payload.get_u64();
                let end_time = payload.get_u64();
                let name_len = payload.get_u16() as usize;
                let name = String::from_utf8_lossy(&payload.split_to(name_len)).to_string();

                Ok(Some(OtelMessage::Trace(TraceData {
                    trace_id,
                    span_id,
                    parent_span_id,
                    start_time_unix_nano: start_time,
                    end_time_unix_nano: end_time,
                    name,
                })))
            }
            2 => {
                // Metric
                if payload.len() < 16 + 2 {
                    return Err(DsError::protocol("Invalid Metric payload"));
                }
                let timestamp = payload.get_u64();
                let value = payload.get_f64();
                let name_len = payload.get_u16() as usize;
                let name = String::from_utf8_lossy(&payload.split_to(name_len)).to_string();

                Ok(Some(OtelMessage::Metric(MetricData {
                    name,
                    value,
                    timestamp,
                })))
            }
            3 => {
                // Log
                if payload.len() < 8 + 1 + 2 {
                    return Err(DsError::protocol("Invalid Log payload"));
                }
                let timestamp = payload.get_u64();
                let severity = payload.get_u8();
                let body_len = payload.get_u16() as usize;
                let body = String::from_utf8_lossy(&payload.split_to(body_len)).to_string();

                Ok(Some(OtelMessage::Log(LogData {
                    timestamp,
                    severity,
                    body,
                })))
            }
            _ => Err(DsError::protocol(format!(
                "Unknown binary OTEL type: {}",
                msg_type
            ))),
        }
    }
}

impl Encoder<OtelMessage> for OtelCodec {
    type Error = DsError;

    fn encode(&mut self, item: OtelMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            OtelMessage::Trace(t) => {
                let name_bytes = t.name.as_bytes();
                dst.put_u8(1); // Type
                dst.put_u32((48 + 2 + name_bytes.len()) as u32); // Length
                dst.extend_from_slice(&t.trace_id);
                dst.extend_from_slice(&t.span_id);
                dst.extend_from_slice(&t.parent_span_id);
                dst.put_u64(t.start_time_unix_nano);
                dst.put_u64(t.end_time_unix_nano);
                dst.put_u16(name_bytes.len() as u16);
                dst.extend_from_slice(name_bytes);
            }
            OtelMessage::Metric(m) => {
                let name_bytes = m.name.as_bytes();
                dst.put_u8(2);
                dst.put_u32((16 + 2 + name_bytes.len()) as u32);
                dst.put_u64(m.timestamp);
                dst.put_f64(m.value);
                dst.put_u16(name_bytes.len() as u16);
                dst.extend_from_slice(name_bytes);
            }
            OtelMessage::Log(l) => {
                let body_bytes = l.body.as_bytes();
                dst.put_u8(3);
                dst.put_u32((8 + 1 + 2 + body_bytes.len()) as u32);
                dst.put_u64(l.timestamp);
                dst.put_u8(l.severity);
                dst.put_u16(body_bytes.len() as u16);
                dst.extend_from_slice(body_bytes);
            }
        }
        Ok(())
    }
}
