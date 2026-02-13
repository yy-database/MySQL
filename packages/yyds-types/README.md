# yyds-types

YYDS 核心类型系统。

## 功能特性

- **YYValue**: 多模态统一值类型，支持 16 种基础与复杂数据类型。
- **Zero-Copy Layout**: 基于二进制头部的零拷贝数据访问，无需完全反序列化即可读取字段。
- **YYError**: 统一的错误模型，支持跨组件的错误传递与映射。
- **SDR Levels**: 软件定义冗余级别（Single, Double, Truth），控制数据的副本策略。

## 核心组件

- `lib.rs`: 定义 `YYValue` 枚举及基础转换逻辑。
- `layout.rs`: 实现二进制布局的编解码，支持高效的内存映射访问。
