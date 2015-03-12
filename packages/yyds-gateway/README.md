# Unified Entrance Gateway

`yyds-gateway` is the unified entrance gateway for the YYDS ecosystem.

## Features

- **Multi-Protocol Access**: Supports proxy access for the Trust protocol, HTTP/JSON, and native database protocols (e.g., MySQL/PG).
- **Tenant Routing**: Distributes requests to corresponding backend execution nodes based on the `TenantID` in the request header.
- **Quota Control**: Integrates with `yyds-quota` for real-time rate limiting and resource consumption management.
- **Security Audit**: Records logs for all requests entering the system, supporting compliance auditing.
