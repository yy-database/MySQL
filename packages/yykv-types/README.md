# yykv-types (Common Type System)

`yykv-types` 是 YYKV 生态系统的基石，定义了所有组件通用的基础类型、错误处理机制、核心标识符以及物理介质抽象。

## 核心组件

### 🆔 核心标识符
- **PageId**: 存储页面的逻辑唯一标识。
- **DeviceId**: 物理设备的唯一 ID。
- **TenantId**: 租户 UUID，用于实现物理与逻辑上的隔离。

### 💾 介质抽象 (Medium Types)
定义了系统支持的所有物理存储介质，用于指导 `yykv-allocator` 和 `yykv-tiering` 的决策：
- `NVM`: 非易失性内存。
- `NvmeSSD`: 高性能 NVMe 固态硬盘。
- `SataSSD`: 标准 SATA 固态硬盘。
- `HDD`: 机械硬盘。

### 📊 统一值模型 (DsValue)
定义了 YYKV 支持的数据类型，采用零拷贝友好的枚举设计：
- `Null`, `Bool`, `Int64`, `Float64`, `Text`, `Binary`。
- `Vector`: 支持高性能向量检索。

### 🛡️ 错误处理 (DsError)
提供统一的错误处理框架：
- **IO Error**: 物理层读写异常。
- **Storage Error**: 逻辑空间分配或管理异常。
- **Serialization Error**: 二进制协议编解码错误。
- **Internal Error**: 系统内部逻辑错误。

## 核心特性

- **零依赖 (Core-first)**: 尽量减少外部依赖，确保作为最底层包的编译速度与稳定性。
- **二进制安全**: 所有类型均设计为易于序列化，符合跨语言通信要求。
- **多租户友好**: 在基础类型设计中优先考虑租户 ID 的嵌入。
