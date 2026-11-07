# yydb-mysql

Python package for the MySQL classic-protocol primitives and the loopback
server composed from them.

```shell
cd projects/mysql.py/packages/mysql
maturin develop
```

```python
from mysql import start_server

server = start_server("./data", port=0)
print(server.address)
server.stop()
```

The listener itself is the upstream `database.server::yyds::mysql` gateway.
`mysql-pyo3` only exports handles for the `.yyds` catalog, the `.yykv` shard,
the persisted schema registry and the node lease.

## Top-level namespace `mysql`

`yydb-mysql` provides the top-level `mysql` package. Python installers do not
check cross-distribution top-level name collisions, so another distribution
that ships `mysql/` (for example `mysql-connector-python`) cannot coexist with
this one: whichever distribution was installed last owns `mysql/__init__.py`.

`import mysql` detects a conflicting owner and raises `ImportError` naming it.
Fix it by removing one of them and reinstalling this package:

```shell
pip uninstall mysql-connector-python   # or: pip uninstall yydb-mysql
pip install --force-reinstall yydb-mysql
```

The reinstall is not optional: both distributions list `mysql/__init__.py` in
their `RECORD`, so uninstalling the other one also deletes this file and leaves
an empty namespace package behind.

If both are needed, install them in separate virtualenvs. The reverse install
order (the other distribution installed after this one) replaces this
`__init__.py`, so no detection can run from this package; that order is
unsupported.
