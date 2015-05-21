# mysql-pyo3

PyO3 binding for `mysql-core`.

Exports orthogonal primitives only (`max_packet_payload`, `encode_packet`,
`decode_packet`, `protocol_version`). No binary, no server loop and no CLI: the
Python host owns those.
