# MySQL

MySQL classic-protocol product: wire framing, handshake and command primitives
for a server that persists into `.yyds` / `.yykv`.

## Layout

- `projects/mysql.rs/` — Rust workspace
  - `mysql-core` — protocol primitives shared by every binding, the only place
    where parsing and execution live
  - `mysql-napi` — Node-API binding, type conversion and error mapping only
  - `mysql-pyo3` — PyO3 binding, type conversion and error mapping only
  - `mysql-wasm` — browser WebAssembly binding, codec exports only
- `projects/mysql.ts/` — TypeScript packages, platform bindings, examples and
  homepage
- `projects/mysql.py/` — Python package assembled from `mysql-pyo3` and examples

## Build

```shell
cargo check --manifest-path projects/mysql.rs/Cargo.toml --workspace
pnpm install
pnpm run build:ts
```

Released under the Mozilla Public License 2.0. See `License.md`.
