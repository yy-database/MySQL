# 部署指南：安装与运行 YYDS

本指南将介绍如何部署一个 YYDS (獨一分布式存储) 实例。

## 1. 系统要求

YYDS 针对裸硬件性能进行了优化，推荐以下环境：
- **操作系统**：Linux (内核 5.10+) 或 Windows Server 2022+。
- **磁盘**：推荐 NVMe SSD 以获得极致性能，也支持 SATA SSD 和 HDD。
- **内存**：最小 2GB，推荐 8GB 以上（用于 Page Cache 优化）。

## 2. 快速部署 (Docker)

这是启动测试实例最快的方式：

```bash
docker run -d \
  --name yyds-server \
  -p 22122:22122 \
  -v /mnt/data:/var/lib/yyds \
  yydb/yyds:latest
```

## 3. 二进制部署

1. **下载二进制文件**：
   前往 [GitHub Releases](https://github.com/yydb/yydb/releases) 下载对应平台的 `yyds-server`。

2. **初始化配置**：
   ```bash
   ./yyds-server init --path ./data
   ```
   这将扫描你的硬件并生成最优的调度配置。

3. **启动服务**：
   ```bash
   ./yyds-server start --config ./config.toml
   ```

## 4. 关键配置项

在 `config.toml` 中，你可以调整以下核心参数：

- `storage.path`: 原始数据存放路径。
- `network.listen`: 服务监听地址。
- `scheduler.io_priority`: 是否启用硬件感知的 I/O 优先级调度。
- `tenant.default_quota`: 默认租户的资源配额。

## 5. 验证部署

启动后，你可以通过日志或内置的健康检查接口验证状态：

```bash
curl http://localhost:22122/health
```

如果返回 `{"status": "UP"}`，说明你的 YYDS 实例已就绪。

---

**下一步**：
- 了解如何 [连接到实例](./connect)。
- 深入研究 [架构设计](./architecture) 以了解 YYDS 的内部工作原理。
