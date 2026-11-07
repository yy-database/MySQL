"""Compose the upstream MySQL gateway from the native handles.

Every handshake byte, command dispatch, `OK`/`ERR`/`EOF` frame and SQL runtime
decision stays upstream in `database.server::yyds::mysql`. This module only
opens the data-directory primitives in order and hands them to that listener.
"""

from __future__ import annotations

import os
from dataclasses import dataclass

from . import mysql_pyo3

LOCAL_SHARD_ID = mysql_pyo3.LOCAL_SHARD_ID
CATALOG_FILE = mysql_pyo3.CATALOG_FILE
SCHEMA_FILE = mysql_pyo3.SCHEMA_FILE


@dataclass
class Server:
    """One running loopback listener."""

    address: str
    port: int
    data_dir: str
    catalog_path: str
    schema_path: str
    _service: object

    def stop(self) -> None:
        """Stops the listener and waits for its worker to exit."""
        self._service.stop()


def start_server(
    data_dir: str,
    *,
    port: int = 0,
    cluster_id: str = "mysql-local",
    node_id: str = "mysql-node-0",
) -> Server:
    """Starts the upstream listener on `127.0.0.1:<port>`.

    Port `0` binds a free port. The node id is deterministic because the lease
    pins one identity per data directory, so a restart must reuse it.
    """
    os.makedirs(data_dir, exist_ok=True)
    catalog_path = os.path.join(data_dir, CATALOG_FILE)
    schema_path = os.path.join(data_dir, SCHEMA_FILE)
    mysql_pyo3.publish_routing(catalog_path, 1, [LOCAL_SHARD_ID])
    lease = mysql_pyo3.MysqlLease(data_dir, cluster_id, node_id)
    executor = mysql_pyo3.MysqlExecutor.open(data_dir, catalog_path)
    schema = mysql_pyo3.MysqlSchema.open(schema_path)
    service = mysql_pyo3.MysqlService.start(lease, executor, schema, port)
    address = service.address()
    _host, _, port_text = address.rpartition(":")
    return Server(
        address=address,
        port=int(port_text),
        data_dir=data_dir,
        catalog_path=catalog_path,
        schema_path=schema_path,
        _service=service,
    )
