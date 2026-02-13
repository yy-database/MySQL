# YYKV Storage Server

The standalone storage server program for the YYKV storage engine.

## Features

- **Core Storage**: Provides direct access to the underlying `yykv` storage kernel.
- **High Performance**: Optimized for low-latency IO operations and high throughput.
- **Trust Protocol**: Supports the Trust binary protocol for efficient data communication.
- **Minimalist Design**: Lightweight server process focused on storage operations.

## Running

Start the server via `cargo run -p yykv-server`. It listens on `127.0.0.1:8889` by default.
