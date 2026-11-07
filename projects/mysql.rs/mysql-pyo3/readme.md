# mysql-pyo3

PyO3 binding over `database` and `database-protocol-mysql`.

Exports orthogonal primitives only (`max_packet_payload`, `encode_packet`,
`decode_packet`, `protocol_version`) plus the server handles (`MysqlLease`,
`MysqlExecutor`, `MysqlSchema`, `MysqlService`, `publish_routing`). No binary,
no server loop and no CLI: the Python host composes those, and the accept loop
stays in the upstream `database.server::yyds::mysql` gateway.
