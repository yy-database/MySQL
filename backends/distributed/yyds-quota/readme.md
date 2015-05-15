# Resource Quota and Flow Control System

`yyds-quota` is the resource quota and flow control system for the YYDS ecosystem.

## Features

- **Multi-Dimensional Quotas**: Supports quota limits for tenants across multiple dimensions, including QPS, storage capacity, and compute units.
- **Real-Time Flow Control**: Millisecond-level implementation of request token bucket/leaky bucket algorithms to ensure system stability.
- **Dynamic Adjustment**: Supports dynamic adjustment of tenant quotas without restarting services.
