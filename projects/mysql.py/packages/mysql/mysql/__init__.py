"""MySQL classic-protocol primitives and the composed loopback server.

The compiled extension is built by `maturin` from `mysql-pyo3` and is
gitignored; the framing primitives re-export the Rust functions unchanged, and
`start_server` composes the native handles around the upstream gateway.
"""

from .mysql_pyo3 import (  # noqa: F401
    CATALOG_FILE,
    LOCAL_SHARD_ID,
    MAX_PACKET_PAYLOAD,
    SCHEMA_FILE,
    MysqlExecutor,
    MysqlLease,
    MysqlSchema,
    MysqlService,
    decode_packet,
    encode_packet,
    max_packet_payload,
    protocol_version,
    publish_routing,
)
from .server import Server, start_server

__all__ = [
    "CATALOG_FILE",
    "LOCAL_SHARD_ID",
    "MAX_PACKET_PAYLOAD",
    "SCHEMA_FILE",
    "MysqlExecutor",
    "MysqlLease",
    "MysqlSchema",
    "MysqlService",
    "Server",
    "decode_packet",
    "encode_packet",
    "max_packet_payload",
    "protocol_version",
    "publish_routing",
    "start_server",
]
