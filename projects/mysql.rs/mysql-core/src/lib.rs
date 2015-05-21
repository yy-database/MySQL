//! MySQL classic-protocol primitives.
//!
//! This crate ships library primitives only: no binary, no server loop and no
//! CLI. Hosts (TypeScript and Python in this repository) own the socket loop,
//! the transaction state and the command line.
//!
//! * [`packet`] — the classic 4-byte header framing, re-exported from
//!   `database-protocol-mysql`
//! * [`PROTOCOL_VERSION`] — the version byte a MySQL server announces
//!
//! The `.yyds` catalog and `.yykv` shard store land here in later rounds, so the
//! bindings never grow an engine of their own: they only convert types and map
//! errors.

#![warn(missing_docs)]

/// Classic 4-byte header framing re-exported from `database-protocol-mysql`.
pub mod packet {
    pub use database_protocol_mysql::*;
}

pub use database_protocol_mysql::{MAX_PACKET_PAYLOAD, Packet, PacketError, decode_packet, encode_packet};

/// Protocol version announced in the initial handshake.
pub const PROTOCOL_VERSION: u8 = 10;

/// Largest payload one classic packet may carry.
pub const fn max_packet_payload() -> usize {
    MAX_PACKET_PAYLOAD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_roundtrips_one_payload() {
        let framed = encode_packet(0, b"SELECT 1").expect("payload fits one packet");
        let packet = decode_packet(&framed, MAX_PACKET_PAYLOAD).expect("framed bytes are complete").expect("one packet");
        assert_eq!(packet.sequence, 0);
        assert_eq!(packet.payload, b"SELECT 1");
    }

    #[test]
    fn oversized_payload_is_rejected() {
        let oversized = vec![0_u8; MAX_PACKET_PAYLOAD + 1];
        assert!(matches!(encode_packet(0, &oversized), Err(PacketError::PayloadTooLarge)));
    }
}
