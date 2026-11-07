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
