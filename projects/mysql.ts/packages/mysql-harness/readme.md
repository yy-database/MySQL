# @yydb/mysql-harness

Private acceptance harness for the MySQL product. It never re-implements the
wire protocol: it starts the real server through `@yydb/mysql`, connects with
the real `mysql2` driver and checks the `.yyds` catalog, the `.yykv` shard and
the persisted schema registry on disk.

- `cases/*.json` — declarative cases (`steps`, `columns`, `rows`, `affectedRows`,
  `rowCount`, `valueIn`, `restart`).
- `fixtures/` — expected result shapes for this batch.
- `corpus/` — frozen captures from a real MySQL instance (batch 3).
- `stages/node.mjs` — runs every case through `@yydb/mysql` with `mysql2`,
  including a real process stop and a fresh process on the same data directory.
- `stages/install.mjs` — packs `@yydb/mysql`, the platform package and this
  harness into a system temporary directory, installs them and runs the
  acceptance stage with plain `node`.
- `stages/python.mjs` — builds the `yydb-mysql` wheel with maturin, creates a
  clean virtualenv in a system temporary directory, installs the real `pymysql`
  driver and runs the same cases from inside the virtualenv.

```shell
node stages/node.mjs          # workspace build
node stages/install.mjs       # install-state acceptance (TypeScript)
node stages/python.mjs        # install-state acceptance (Python)
```
