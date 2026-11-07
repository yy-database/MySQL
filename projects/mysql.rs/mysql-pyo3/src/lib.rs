//! Python bindings: thin wrappers over `database-protocol-mysql` framing and the
//! upstream `database.server::yyds::mysql` gateway handles.
//!
//! No global state, no CLI and no second listener: the Python host composes the
//! exported handles, and the accept loop stays upstream.

use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{PyBytes, PyModule},
};

mod server;

fn map_error(error: database_protocol_mysql::PacketError) -> PyErr {
    PyValueError::new_err(format!("{error:?}"))
}

/// Largest payload one classic packet may carry.
#[pyfunction]
fn max_packet_payload() -> usize {
    database_protocol_mysql::MAX_PACKET_PAYLOAD
}

/// Frames one payload with the classic 4-byte header.
#[pyfunction]
fn encode_packet(sequence: u8, payload: &[u8]) -> PyResult<Vec<u8>> {
    database_protocol_mysql::encode_packet(sequence, payload).map_err(map_error)
}

/// Decodes the first complete packet as `(sequence, payload)`.
#[pyfunction]
fn decode_packet<'py>(py: Python<'py>, input: &[u8]) -> PyResult<Option<(u8, Bound<'py, PyBytes>)>> {
    match database_protocol_mysql::decode_packet(input, database_protocol_mysql::MAX_PACKET_PAYLOAD).map_err(map_error)? {
        Some(packet) => Ok(Some((packet.sequence, PyBytes::new(py, packet.payload)))),
        None => Ok(None),
    }
}

/// Protocol version announced in the initial handshake.
#[pyfunction]
fn protocol_version() -> u8 {
    database_protocol_mysql::PROTOCOL_VERSION
}

/// The native module behind `mysql.mysql_pyo3`.
#[pymodule]
fn mysql_pyo3(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(max_packet_payload, module)?)?;
    module.add_function(wrap_pyfunction!(encode_packet, module)?)?;
    module.add_function(wrap_pyfunction!(decode_packet, module)?)?;
    module.add_function(wrap_pyfunction!(protocol_version, module)?)?;
    module.add("MAX_PACKET_PAYLOAD", database_protocol_mysql::MAX_PACKET_PAYLOAD)?;
    server::register(module)?;
    Ok(())
}
