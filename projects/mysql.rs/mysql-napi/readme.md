# mysql-napi

Node-API binding over `database` and `database-protocol-mysql`.

Exports orthogonal primitives only (`maxPacketPayload`, `encodePacket`,
`decodePacket`). No binary, no server loop and no CLI: the TypeScript host owns
those.
