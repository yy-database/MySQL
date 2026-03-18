# YYDS Distributed Database Server

The main server program for the YYDS distributed database.

## Features

- **Integrated Core**: Integrates the storage engine (`yykv`), compute engine (`yyds-compute`), consensus protocol (`yyds-consensus`), and executor (`yyds-executor`).
- **Multi-Tenant Support**: Deeply integrates with `yyds-tenant`, `yyds-billing`, and `yyds-quota` to achieve tenant isolation and resource billing.
- **Trust Protocol Access**: Native support for the high-performance Trust binary communication protocol.
- **State Management**: Unified management of global tenant metadata, quotas, and storage handles.

## Running

Start the server via `cargo run -p yyds-server`. It listens on `127.0.0.1:8888` by default.
