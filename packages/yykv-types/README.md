# Common Type System

**yykv-types** is the cornerstone of the YYKV ecosystem, defining the basic types, error handling mechanisms, core identifiers, and physical media abstractions common to all components.

## 1. Core Components

### 🆔 Core Identifiers
- **PageId**: Logically unique identifier for a storage page.
- **DeviceId**: Unique ID for a physical device.
- **TenantId**: Tenant UUID, used to achieve physical and logical isolation.

### 💾 Media Abstraction
Defines all physical storage media supported by the system, used to guide decisions by `yykv-allocator` and `yykv-tiering`:
- `NVM`: Non-Volatile Memory.
- `NvmeSSD`: High-performance NVMe Solid State Drive.
- `SataSSD`: Standard SATA Solid State Drive.
- `HDD`: Hard Disk Drive.

### 📊 Unified Value Model (DsValue)
Defines the data types supported by YYKV, using a zero-copy friendly enum design:
- `Null`, `Bool`, `Int64`, `Float64`, `Text`, `Binary`.
- `Vector`: Supports high-performance vector retrieval.

### 🛡️ Error Handling (DsError)
Provides a unified error handling framework:
- **IO Error**: Physical layer read/write exceptions.
- **Storage Error**: Logical space allocation or management exceptions.
- **Serialization Error**: Binary protocol encoding/decoding errors.
- **Internal Error**: System internal logic errors.

## 2. Core Features

- **Zero Dependency (Core-first)**: Minimizes external dependencies to ensure compilation speed and stability as the lowest-level package.
- **Binary Safe**: All types are designed for easy serialization, meeting requirements for cross-language communication.
- **Multi-tenant Friendly**: Prioritizes embedding tenant IDs into basic type designs.
