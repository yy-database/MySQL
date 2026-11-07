//! MySQL classic-protocol primitives.
//!
//! This crate ships library primitives only: no binary, no server loop and no
//! CLI. Hosts (TypeScript and Python in this repository) own the command line.
//!
//! * [`packet`] — the classic 4-byte header framing, re-exported from
//!   `database-protocol-mysql`
//! * [`PROTOCOL_VERSION`] — the version byte a MySQL server announces
//! * [`server`] — node-directory composition (`.yyds` catalog, `.yykv` shard,
//!   persisted schema registry, node lease) handed to the upstream
//!   `database.server::yyds::mysql` gateway. The listener itself is upstream.
//!
//! Reuse rule: the handshake, authentication, `COM_*` dispatch, `OK`/`ERR`/`EOF`
//! frames and length-encoded integers live upstream. Nothing here re-implements
//! them; search `database.server::yyds::mysql` before adding protocol semantics.

#![warn(missing_docs)]

/// Classic 4-byte header framing re-exported from `database-protocol-mysql`.
pub mod packet {
    pub use database_protocol_mysql::*;
}

/// Node-directory composition and the upstream MySQL listener.
#[cfg(feature = "server")]
pub mod server;

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

    // `encode_packet` / `decode_packet` are re-exports of
    // `database-protocol-mysql`, which owns the framing tests in
    // `database.server/tests/mysql`. These two cases assert the re-export
    // contract only: the pinned revision still exposes the same framing
    // semantics, so a pin bump cannot silently change the binding surface.

    #[test]
    fn reexport_framing_roundtrips_one_payload() {
        let framed = encode_packet(0, b"SELECT 1").expect("payload fits one packet");
        let packet = decode_packet(&framed, MAX_PACKET_PAYLOAD).expect("framed bytes are complete").expect("one packet");
        assert_eq!(packet.sequence, 0);
        assert_eq!(packet.payload, b"SELECT 1");
    }

    #[test]
    fn reexport_framing_rejects_oversized_payload() {
        let oversized = vec![0_u8; MAX_PACKET_PAYLOAD + 1];
        assert!(matches!(encode_packet(0, &oversized), Err(PacketError::PayloadTooLarge)));
    }
}
