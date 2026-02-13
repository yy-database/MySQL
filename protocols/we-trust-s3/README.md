# we-trust-s3 (S3 Object Storage Adapter)

`we-trust-s3` is the object storage adaptation layer for YYKV. By simulating the standard Amazon S3 RESTful API, it allows existing cloud-native applications and tools (such as `aws-cli`, `rclone`) to use YYKV as a high-performance object storage backend.

## Core Features

### 📦 Full S3 API Adaptation
- **Object Operations**: Supports `PutObject`, `GetObject`, `DeleteObject`, `HeadObject`.
- **Bucket Management**: Supports `CreateBucket`, `ListBuckets`, `DeleteBucket`.
- **Multipart Upload**: Supports parallel uploading and merging of large files.

### 🗺️ Physical Mapping Mechanism
- **Bucket to Tenant**: Maps S3 Buckets to independent YYKV tenant spaces.
- **Object to Pages**: Utilizes YYKV's variable-length layout (`yykv-layout`) to efficiently distribute and store object data in physical pages.

### 🛡️ Authentication & Authorization
- **V4 Signature Support**: Implements the AWS Signature Version 4 verification algorithm.
- **Access Control**: Integrates with YYKV's multi-tenant permission system.

## Core Components

- `S3Auth`: Responsible for handling complex S3 V4 signature verification logic.
- `ObjectStorageAdapter`: Converts HTTP REST requests into YYKV storage-compute operators.
- `Models`: Defines XML response structures compliant with S3 specifications.

## Usage Example

```bash
# Access YYKV S3 using AWS CLI
aws --endpoint-url http://localhost:8080 s3 cp myfile.txt s3://mybucket/
```

## Technical Advantages
- **Integrated Storage & Compute**: Allows running `yykv-compute` operators directly on S3 objects (e.g., filtering CSV/Parquet stored in S3).
- **Extreme Response Speed**: Bypasses the standard file system, utilizing Direct IO to achieve ultra-low latency for object R/W.
