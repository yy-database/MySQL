# 部署指南：安装与运行 YYDS

本指南将介绍如何部署一个 YYDS (獨一分布式存储) 实例。

> **重要提示**：YYDS 深度依赖裸硬件加速技术（如 Direct I/O, `io_uring`），**强烈不建议**在 Docker、虚拟机或任何经过虚拟化层包装的环境中运行。虚拟化会导致显著的 I/O 延迟抖动，并使硬件感知调度器失效。请始终在裸机（Bare Metal）上进行生产部署。

## 1. 系统要求

YYDS 针对裸硬件性能进行了极致优化，推荐以下环境：
- **操作系统**：Linux (内核 5.10+，推荐使用支持 `io_uring` 的最新稳定内核) 或 Windows Server 2022+。
- **磁盘**：推荐 NVMe SSD 以获得物理层极致性能。
- **内存**：最小 2GB，推荐 8GB 以上（用于 Page Cache 和计算下推缓存）。

## 2. 裸机部署步骤

1. **下载二进制文件**：
   前往 [GitHub Releases](https://github.com/yydb/yydb/releases) 下载对应平台的 `yyds-server` 压缩包并解压。

2. **硬件扫描与初始化**：
   ```bash
   ./yyds-server init --path /dev/nvme0n1
   ```
   > 注意：YYDS 可以直接接管块设备或挂载点。`init` 命令会扫描磁盘特性并生成针对该硬件最优的调度配置文件 `config.toml`。

3. **启动服务**：
   ```bash
   ./yyds-server start --config ./config.toml
   ```

## 3. 关键配置项

在生成的 `config.toml` 中，你可以根据实际需求微调以下核心参数：

- `storage.path`: 原始数据存放路径（块设备路径或目录路径）。
- `network.listen`: 服务监听地址，默认为 `0.0.0.0:22122`。
- `scheduler.io_priority`: 默认为 `true`。启用后将激活硬件感知的 I/O 优先级调度。
- `tenant.default_quota`: 默认租户的资源配额（I/O 带宽、存储上限等）。

## 4. 验证部署

启动后，你可以通过内置的健康检查接口验证状态：

```bash
curl http://localhost:22122/health
```

如果返回 `{"status": "UP"}`，说明你的 YYDS 实例已成功在裸机上运行。

---

**下一步**：
- 了解如何 [连接到实例](./connect)。
- 深入研究 [架构设计](./architecture) 以了解 YYDS 如何绕过文件系统直接驱动硬件。
