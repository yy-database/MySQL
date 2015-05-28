//! Node bindings: thin wrappers over the `mysql-core` framing primitives.
//!
//! No global state, no `start` entry and no CLI: the TypeScript host owns the
//! socket loop, the transaction state and the command line.

use napi::{Error, Result, Status, bindgen_prelude::Buffer};
use napi_derive::napi;

fn map_error(error: mysql_core::PacketError) -> Error {
    Error::new(Status::InvalidArg, format!("{error:?}"))
}

/// One decoded classic packet.
#[napi(object)]
pub struct DecodedPacket {
    /// Sequence byte from the 4-byte header.
    pub sequence: u32,
    /// Payload bytes the header announced.
    pub payload: Buffer,
}

/// Largest payload one classic packet may carry.
#[napi]
pub fn max_packet_payload() -> u32 {
    mysql_core::max_packet_payload() as u32
}

/// Protocol version announced in the initial handshake.
#[napi]
pub fn protocol_version() -> u8 {
    mysql_core::PROTOCOL_VERSION
}

/// Frames one payload with the classic 4-byte header.
#[napi]
pub fn encode_packet(sequence: u32, payload: Buffer) -> Result<Buffer> {
    let sequence = u8::try_from(sequence).map_err(|_| Error::new(Status::InvalidArg, "sequence must be 0..=255"))?;
    mysql_core::encode_packet(sequence, payload.as_ref()).map(Buffer::from).map_err(map_error)
}

/// Decodes the first complete packet, leaving the rest of the input untouched.
#[napi]
pub fn decode_packet(input: Buffer) -> Result<Option<DecodedPacket>> {
    match mysql_core::decode_packet(input.as_ref(), mysql_core::MAX_PACKET_PAYLOAD).map_err(map_error)? {
        Some(packet) => {
            Ok(Some(DecodedPacket { sequence: u32::from(packet.sequence), payload: Buffer::from(packet.payload.to_vec()) }))
        }
        None => Ok(None),
    }
}
