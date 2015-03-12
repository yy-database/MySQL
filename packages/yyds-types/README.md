# Core Types System

`yyds-types` provides the core type system for the YYDS ecosystem.

## Features

- **YYValue**: A unified multi-modal value type supporting 16 basic and complex data types.
- **Zero-Copy Layout**: Binary header-based zero-copy data access, allowing field reads without full deserialization.
- **YYError**: A unified error model supporting error propagation and mapping across components.
- **SDR Levels**: Software-Defined Redundancy levels (Single, Double, Truth) for controlling data replica strategies.

## Core Components

- `lib.rs`: Defines the `YYValue` enum and basic conversion logic.
- `layout.rs`: Implements binary layout encoding/decoding, supporting efficient memory-mapped access.
