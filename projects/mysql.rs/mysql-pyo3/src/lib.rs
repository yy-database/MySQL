//! Python bindings: thin wrappers over the `mysql-core` primitives.
//!
//! No global state, no CLI and no second listener: the Python host composes the
//! exported handles, and the accept loop stays in the upstream
//! `database.server::yyds::mysql` gateway.

use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{PyBytes, PyModule},
};

mod server;

fn map_error(error: mysql_core::PacketError) -> PyErr {
    PyValueError::new_err(format!("{error:?}"))
}

/// Largest payload one classic packet may carry.
#[pyfunction]
fn max_packet_payload() -> usize {
    mysql_core::max_packet_payload()
}

/// Frames one payload with the classic 4-byte header.
#[pyfunction]
fn encode_packet(sequence: u8, payload: &[u8]) -> PyResult<Vec<u8>> {
    mysql_core::encode_packet(sequence, payload).map_err(map_error)
}

/// Decodes the first complete packet as `(sequence, payload)`.
#[pyfunction]
fn decode_packet<'py>(py: Python<'py>, input: &[u8]) -> PyResult<Option<(u8, Bound<'py, PyBytes>)>> {
    match mysql_core::decode_packet(input, mysql_core::MAX_PACKET_PAYLOAD).map_err(map_error)? {
        Some(packet) => Ok(Some((packet.sequence, PyBytes::new(py, packet.payload)))),
        None => Ok(None),
    }
}

/// Protocol version announced in the initial handshake.
#[pyfunction]
fn protocol_version() -> u8 {
    mysql_core::PROTOCOL_VERSION
}

/// The native module behind `mysql.mysql_pyo3`.
#[pymodule]
fn mysql_pyo3(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(max_packet_payload, module)?)?;
    module.add_function(wrap_pyfunction!(encode_packet, module)?)?;
    module.add_function(wrap_pyfunction!(decode_packet, module)?)?;
    module.add_function(wrap_pyfunction!(protocol_version, module)?)?;
    module.add("MAX_PACKET_PAYLOAD", mysql_core::MAX_PACKET_PAYLOAD)?;
    server::register(module)?;
    Ok(())
}
