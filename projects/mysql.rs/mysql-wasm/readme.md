# mysql-wasm

Browser WebAssembly binding over `database-protocol-mysql`.

Exports the classic-protocol codec only (`max_packet_payload`,
`encode_packet`, `decode_packet`). Storage and the server loop stay native.
