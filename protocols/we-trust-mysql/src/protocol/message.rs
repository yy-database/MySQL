use bytes::{Buf, Bytes};
use crate::Result;

#[derive(Debug)]
pub enum Message {
    Handshake {
        protocol_version: u8,
        server_version: String,
        connection_id: u32,
        auth_plugin_data: Vec<u8>,
        capability_flags: u32,
        character_set: u8,
        status_flags: u16,
        auth_plugin_name: Option<String>,
    },
    Ok {
        affected_rows: u64,
        last_insert_id: u64,
        status_flags: u16,
        warnings: u16,
        info: String,
    },
    Err {
        error_code: u16,
        sql_state: String,
        error_message: String,
    },
    CommandRequest {
        command: u8,
        payload: Bytes,
    },
}

impl Message {
    // Basic decode logic could go here
}
