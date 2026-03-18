# S3 Compatibility Protocol

**we-trust-s3** implements an Amazon S3-compatible object storage interface, allowing YYKV to serve as a storage backend for massive unstructured data.

## 1. Architectural Implementation

Unlike traditional file systems, YYKV builds the S3 protocol layer directly on top of the **HAL (Hardware Abstraction Layer)**:
- **RESTful Interface**: Provides standard S3 APIs via HTTP/HTTPS.
- **Virtual Buckets**: Maps the S3 `Bucket` concept to YYKV's tenant-level logical namespaces.
- **Object Mapping**: `Key` is mapped to the storage engine's keys, and `Body` utilizes `yykv-io`'s Direct I/O for large-block continuous writes.

## 2. Supported APIs

- **Service Operations**: `ListBuckets`
- **Bucket Operations**: `GetBucket (List Objects)`, `HeadBucket`
- **Object Operations**:
  - `PutObject`: Supports Multipart Upload optimization.
  - `GetObject`: Supports Range requests, enabling physical-level offset reads.
  - `DeleteObject`: Atomic deletion.
  - `HeadObject`: Retrieves metadata.

## 3. Storage Optimization: Large Object Slicing

For S3 scenarios, YYKV employs specific storage strategies:
- **Automatic Sharding**: Objects larger than 4MB are automatically sliced into multiple physical Pages.
- **Hybrid Storage**: Object metadata is stored in high-performance SSDs (Blaze tier), while object body data automatically migrates between SSDs and high-capacity HDDs based on access frequency.

## 4. Security & Authentication

Compatible with the standard S3 V4 signature algorithm (`AWS4-HMAC-SHA256`):
- Clients sign requests using `AccessKey` and `SecretKey`.
- YYKV's internal authentication module verifies signatures and associates them with the corresponding `TenantID`.

## 5. Use Cases

- **Cloud-Native Backup**: Directly sync backup data to YYKV using `aws-cli` or `rclone`.
- **Multi-Modal Applications**: The same tenant can operate structured metadata under `we-trust-postgres` while storing associated large media files under `we-trust-s3`.
