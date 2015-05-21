# mysql-core

MySQL classic-protocol primitives shared by `mysql-napi`, `mysql-pyo3` and
`mysql-wasm`.

The crate carries the wire framing today and the `.yyds` / `.yykv` store later.
It exposes no binary, no server loop and no CLI: hosts compose the primitives.
