//! Browser WebAssembly binding: codec exports over `mysql-core`.
//!
//! Storage is native-only: the wasm build carries the classic framing so a
//! browser host can shape packets itself.

use wasm_bindgen::prelude::*;

/// One decoded classic packet.
#[wasm_bindgen]
pub struct DecodedPacket {
    sequence: u8,
    payload: Vec<u8>,
}

#[wasm_bindgen]
impl DecodedPacket {
    /// Sequence byte from the 4-byte header.
    #[wasm_bindgen(getter)]
    pub fn sequence(&self) -> u8 {
        self.sequence
    }

    /// Payload bytes the header announced.
    #[wasm_bindgen(getter)]
    pub fn payload(&self) -> Vec<u8> {
        self.payload.clone()
    }
}

/// Largest payload one classic packet may carry.
#[wasm_bindgen]
pub fn max_packet_payload() -> u32 {
    mysql_core::max_packet_payload() as u32
}

/// Frames one payload with the classic 4-byte header.
#[wasm_bindgen]
pub fn encode_packet(sequence: u8, payload: &[u8]) -> Result<Vec<u8>, JsValue> {
    mysql_core::encode_packet(sequence, payload).map_err(|error| JsValue::from_str(&format!("{error:?}")))
}

/// Decodes the first complete packet, leaving the rest of the input untouched.
#[wasm_bindgen]
pub fn decode_packet(input: &[u8]) -> Result<Option<DecodedPacket>, JsValue> {
    match mysql_core::decode_packet(input, mysql_core::MAX_PACKET_PAYLOAD) {
        Ok(Some(packet)) => Ok(Some(DecodedPacket { sequence: packet.sequence, payload: packet.payload.to_vec() })),
        Ok(None) => Ok(None),
        Err(error) => Err(JsValue::from_str(&format!("{error:?}"))),
    }
}
