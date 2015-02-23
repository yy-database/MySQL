# we-trust-s3 (S3 Object Storage Adapter)

`we-trust-s3` 是 YYKV 的对象存储适配层。它通过模拟标准 Amazon S3 RESTful API，使得现有的云原生应用与工具（如 `aws-cli`, `rclone`）能够将 YYKV 作为高性能的对象存储后端使用。

## 核心功能

### 📦 S3 API 完整适配
- **对象操作**: 支持 `PutObject`, `GetObject`, `DeleteObject`, `HeadObject`。
- **桶管理**: 支持 `CreateBucket`, `ListBuckets`, `DeleteBucket`。
- **分片上传 (Multipart Upload)**: 支持大文件的并行上传与合并。

### 🗺️ 物理映射机制
- **Bucket to Tenant**: 将 S3 的 Bucket 映射为 YYKV 的独立租户空间。
- **Object to Pages**: 利用 YYKV 的变长布局（`yykv-layout`），将对象数据高效地分散存储在物理页面中。

### 🛡️ 认证与授权
- **V4 签名支持**: 实现了 AWS Signature Version 4 校验算法。
- **访问控制**: 集成 YYKV 的多租户权限体系。

## 核心组件

- `S3Auth`: 负责处理复杂的 S3 V4 签名验证逻辑。
- `ObjectStorageAdapter`: 将 HTTP REST 请求转换为 YYKV 的存算算子。
- `Models`: 定义符合 S3 规范的 XML 响应结构。

## 使用示例

```bash
# 使用 AWS CLI 访问 YYKV S3
aws --endpoint-url http://localhost:8080 s3 cp myfile.txt s3://mybucket/
```

## 技术优势
- **存算一体**: 允许直接在 S3 对象上运行 `yykv-compute` 算子（如对存储在 S3 中的 CSV/Parquet 进行过滤）。
- **极速响应**: 绕过标准文件系统，利用 Direct IO 实现极低的对象读写延迟。
