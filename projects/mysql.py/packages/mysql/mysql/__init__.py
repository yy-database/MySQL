"""MySQL protocol primitives and the composed loopback server.

The compiled extension is built by `maturin` from `mysql-pyo3` and is
gitignored; the framing primitives re-export the Rust functions unchanged, and
`start_server` composes the native handles around the upstream gateway.

This distribution owns the top-level `mysql` package. Python installers do not
check cross-distribution top-level collisions, so another distribution that
ships `mysql/` makes this module ambiguous. A conflicting owner is rejected at
import time instead of failing silently later.
"""

from importlib.metadata import distributions
from pathlib import Path

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

_DISTRIBUTION = "yydb-mysql"
_NAMESPACE = "mysql"


def _conflicting_owners() -> list[str]:
    """Distributions besides this one that ship files under `mysql/`."""
    owners: set[str] = set()
    if (Path(__file__).resolve().parent / "connector").is_dir():
        # Left behind by an uninstall or by an install that overwrote this file.
        owners.add("an unregistered mysql/connector directory")
    for dist in distributions():
        name = (dist.metadata["Name"] or "").strip()
        if not name or name.lower().replace("_", "-") == _DISTRIBUTION:
            continue
        files = dist.files or ()
        if any(str(item).replace("\\", "/").startswith(f"{_NAMESPACE}/") for item in files):
            owners.add(name)
    return sorted(owners)


_conflicts = _conflicting_owners()
if _conflicts:
    raise ImportError(
        f"yydb-mysql provides the top-level `mysql` package, but it collides with: {', '.join(_conflicts)}. "
        "Python installers do not check cross-distribution top-level collisions, so the distribution "
        "installed last owns `mysql/__init__.py`. Run `pip uninstall <distribution>` for one of them, "
        "then reinstall `yydb-mysql` (the shared `__init__.py` is removed with the other distribution), "
        "or install the two in separate virtualenvs."
    )

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
