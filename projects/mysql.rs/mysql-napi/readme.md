# mysql-napi

Node-API binding for `mysql-core`.

Exports orthogonal primitives only (`maxPacketPayload`, `encodePacket`,
`decodePacket`). No binary, no server loop and no CLI: the TypeScript host owns
those.
