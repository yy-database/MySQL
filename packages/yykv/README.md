# yykv

YYKV 高性能 Key-Value 存储引擎。

## 功能特性

- **分层存储 (Tiering)**: 支持 SSD, HDD 等不同介质的自动冷热数据迁移。
- **Direct IO**: 针对 Windows 平台优化的 `FILE_FLAG_NO_BUFFERING` 直接 IO 访问。
- **LSM-Tree 架构**: 高效的写入性能与优化的压缩算法。
- **存储级 UDF**: 支持在存储引擎内部直接运行 WASM 算子进行数据预过滤。
- **硬件抽象 (HAL)**: 统一的块设备抽象接口，易于适配不同操作系统与硬件。

## 子项目说明

- `yykv-io`: 底层异步 Direct IO 实现。
- `yykv-allocator`: 专为数据库设计的页面分配器。
- `yykv-tiering`: 冷热数据检测与自动迁移逻辑。
- `yykv-udf`: 基于 WIT 定义的存储级 UDF 接口。
