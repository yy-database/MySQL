# yykv

YYKV 高性能键值存储引擎客户端库。

> **注意**：本仓库是 **客户端库**，用于在你的 Rust 应用中集成 YYKV 存储能力。如果你需要部署运行 YYKV/YYDS 服务端，请使用 [yyds-tools](../../servers/yyds-tools)。

## 特性

- **分层存储 (Tiering)**：支持 SSD 和 HDD 等不同介质间冷热数据的自动迁移。
- **Direct IO**：针对 Windows 平台优化的 `FILE_FLAG_NO_BUFFERING` 直接 IO 访问。
- **LSM-Tree 架构**：高效的写入性能和优化的合并算法。
- **存储级 UDF**：支持在存储引擎内部直接运行 WASM 算子进行数据预过滤。
- **硬件抽象层 (HAL)**：统一的块设备抽象接口，易于适配不同操作系统和硬件。

## 子项目

- `yykv-io`: 底层异步 Direct IO 实现。
- `yykv-allocator`: 专门为数据库设计的页面分配器。
- `yykv-tiering`: 冷热数据检测与自动迁移逻辑。
- `yykv-udf`: 基于 WIT 定义的存储级 UDF 接口。
