# Cloudflare D1 Compatibility Protocol

**we-trust-d1** is designed to allow developers to operate YYKV storage clusters directly using Cloudflare D1's HTTP query interface.

## 1. Protocol Features

- **HTTP/JSON Carrier**: Utilizes a lightweight JSON format to transmit SQL statements and result sets.
- **Stateless Connections**: Ideal for Serverless environments, as it does not require maintaining persistent connections.
- **D1 SDK Compatibility**: Developers can use the D1 interface definitions from `@cloudflare/workers-types` to call YYKV.

## 2. Use Cases

- **Serverless Functions**: Low-latency access to YYKV storage within Cloudflare Workers or Vercel Functions.
