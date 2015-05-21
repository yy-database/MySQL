"""MySQL classic-protocol primitives for Python.

The compiled extension is built by `maturin` from `mysql-pyo3` and is
gitignored; everything public re-exports the Rust functions unchanged.
"""

from .mysql_pyo3 import (  # noqa: F401
    MAX_PACKET_PAYLOAD,
    decode_packet,
    encode_packet,
    max_packet_payload,
    protocol_version,
)

__all__ = [
    "MAX_PACKET_PAYLOAD",
    "decode_packet",
    "encode_packet",
    "max_packet_payload",
    "protocol_version",
]
