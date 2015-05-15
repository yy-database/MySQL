#![warn(missing_docs)]

use bytes::{Buf, BufMut, Bytes, BytesMut};
use uuid::Uuid;
use yykv_types::layout::{DsValueDecoder, DsValueEncoder};
pub use yykv_types::{DsError, DsValue, Redundancy};

/// 数据库后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseBackend {
    /// Limbo/SQLite 后端
    Limbo,
    /// PostgreSQL 后端
    Postgres,
    /// MySQL 后端
    MySql,
}

/// 数据库连接结果类型
pub type DatabaseResult<T> = Result<T, DsError>;

/// 数据库连接 trait
#[async_trait::async_trait]
pub trait DatabaseConnection {
    /// 获取数据库后端类型
    fn backend(&self) -> DatabaseBackend;
    
    /// 执行查询
    async fn query(&self, sql: &str) -> DatabaseResult<Box<dyn RowIterator>>;
}

/// 行迭代器 trait
#[async_trait::async_trait]
pub trait RowIterator: Send {
    /// 获取下一行
    async fn next(&mut self) -> DatabaseResult<Option<Box<dyn Row>>>;
}

/// 行 trait
pub trait Row: Send {
    /// 获取指定索引的字符串值
    fn get_string(&self, index: usize) -> DatabaseResult<String>;
    
    /// 获取指定索引的整数值
    fn get_i64(&self, index: usize) -> DatabaseResult<i64>;
    
    /// 获取指定索引的布尔值
    fn get_bool(&self, index: usize) -> DatabaseResult<bool>;
    
    /// 获取指定索引的可选字符串值
    fn get_option_string(&self, index: usize) -> DatabaseResult<Option<String>>;
}

// 导出 schema 模块
pub mod schema;

use crc32fast::Hasher;
use futures::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

/// Connection options for WeTrust.
#[derive(Debug, Clone)]
pub struct ConnectionOptions {
    pub addr: SocketAddr,
    pub tenant_id: Uuid,
    pub secret_key: Vec<u8>,
}

impl FromStr for ConnectionOptions {
    type Err = DsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut options = ConnectionOptions {
            addr: "127.0.0.1:8889".parse().unwrap(),
            tenant_id: Uuid::nil(),
            secret_key: b"yykv-secret-key-2026".to_vec(),
        };

        for part in s.split(';') {
            let kv: Vec<&str> = part.split('=').collect();
            if kv.len() == 2 {
                match kv[0].to_lowercase().as_str() {
                    "server" | "host" => {
                        let host = kv[1];
                        options.addr = format!("{}:8889", host)
                            .parse()
                            .map_err(|e| DsError::internal(format!("Invalid host: {}", e)))?;
                    }
                    "port" => {
                        let port: u16 = kv[1]
                            .parse()
                            .map_err(|e| DsError::internal(format!("Invalid port: {}", e)))?;
                        let mut addr = options.addr;
                        addr.set_port(port);
                        options.addr = addr;
                    }
                    "tenantid" => {
                        options.tenant_id = Uuid::parse_str(kv[1])
                            .map_err(|e| DsError::internal(format!("Invalid TenantID: {}", e)))?;
                    }
                    "secretkey" => {
                        options.secret_key = kv[1].as_bytes().to_vec();
                    }
                    _ => {}
                }
            }
        }

        Ok(options)
    }
}

/// Trust protocol magic bytes: "YY"
pub const MAGIC: [u8; 2] = *b"YY";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Put = 1,
    Get = 2,
    Delete = 3,
    Query = 4, // New: Query using OpsGraph
    Rbq = 5,
    Response = 6,
    Auth = 7,
    Value = 8,
    Push = 9,
    Pull = 10,
    Heartbeat = 11,
    Kql = 12,

    PutResp = 101,
    GetResp = 102,
    DeleteResp = 103,
    QueryResp = 104,

    Error = 255,
}

impl From<u8> for MessageType {
    fn from(v: u8) -> Self {
        match v {
            1 => MessageType::Put,
            2 => MessageType::Get,
            3 => MessageType::Delete,
            4 => MessageType::Query,
            5 => MessageType::Rbq,
            6 => MessageType::Response,
            7 => MessageType::Auth,
            8 => MessageType::Value,
            9 => MessageType::Push,
            10 => MessageType::Pull,
            11 => MessageType::Heartbeat,
            12 => MessageType::Kql,
            101 => MessageType::PutResp,
            102 => MessageType::GetResp,
            103 => MessageType::DeleteResp,
            104 => MessageType::QueryResp,
            _ => MessageType::Error,
        }
    }
}

/// Trust Message Header (80 bytes)
/// | Magic (2B) | Version (1B) | Type (1B) | Flags (4B) | Length (4B) | Checksum (4B) |
/// | RequestID (16B) | TenantID (16B) | Signature (32B) |
/// Flags:
/// - Bits 0-7: SDR Level
#[derive(Debug, Clone, PartialEq)]
pub struct TrustHeader {
    pub version: u8,
    pub msg_type: u8,
    pub flags: u32,
    pub length: u32,
    pub checksum: u32,
    pub request_id: Uuid,
    pub tenant_id: Uuid,
    pub signature: [u8; 32],
}

impl TrustHeader {
    pub const SIZE: usize = 80;

    pub fn sdr_level(&self) -> Redundancy {
        Redundancy::from_u8((self.flags & 0xFF) as u8)
    }

    pub fn set_sdr_level(&mut self, level: Redundancy) {
        self.flags = (self.flags & !0xFF) | (level.0 as u32 & 0xFF);
    }

    pub fn sign(&mut self, secret: &[u8]) {
        let mut hasher = Sha256::new();
        hasher.update(secret);
        hasher.update(self.request_id.as_bytes());
        hasher.update(self.tenant_id.as_bytes());
        hasher.update(self.checksum.to_be_bytes());
        hasher.update(self.flags.to_be_bytes());
        let hash = hasher.finalize();
        self.signature.copy_from_slice(&hash);
    }

    pub fn verify(&self, secret: &[u8]) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(secret);
        hasher.update(self.request_id.as_bytes());
        hasher.update(self.tenant_id.as_bytes());
        hasher.update(self.checksum.to_be_bytes());
        hasher.update(self.flags.to_be_bytes());
        let hash = hasher.finalize();
        self.signature == hash.as_slice()
    }

    pub fn encode<B: BufMut>(&self, mut dst: B) {
        dst.put_slice(&MAGIC);
        dst.put_u8(self.version);
        dst.put_u8(self.msg_type);
        dst.put_u32(self.flags);
        dst.put_u32(self.length);
        dst.put_u32(self.checksum);
        dst.put_slice(self.request_id.as_bytes());
        dst.put_slice(self.tenant_id.as_bytes());
        dst.put_slice(&self.signature);
    }

    pub fn decode(src: &mut BytesMut) -> Result<Self, DsError> {
        if src.len() < Self::SIZE {
            return Err(DsError::protocol("Insufficient data for header"));
        }

        let magic = [src[0], src[1]];
        if magic != MAGIC {
            return Err(DsError::protocol(format!("Invalid magic: {:?}", magic)));
        }

        let version = src[2];
        let msg_type = src[3];
        let flags = u32::from_be_bytes([src[4], src[5], src[6], src[7]]);
        let length = u32::from_be_bytes([src[8], src[9], src[10], src[11]]);
        let checksum = u32::from_be_bytes([src[12], src[13], src[14], src[15]]);

        let request_id = Uuid::from_slice(&src[16..32])
            .map_err(|e| DsError::protocol(format!("Invalid request ID: {}", e)))?;
        let tenant_id = Uuid::from_slice(&src[32..48])
            .map_err(|e| DsError::protocol(format!("Invalid tenant ID: {}", e)))?;

        let mut signature = [0u8; 32];
        signature.copy_from_slice(&src[48..80]);

        src.advance(Self::SIZE);

        Ok(Self {
            version,
            msg_type,
            flags,
            length,
            checksum,
            request_id,
            tenant_id,
            signature,
        })
    }
}

/// A Trust protocol message
#[derive(Debug)]
pub struct TrustMessage {
    pub header: TrustHeader,
    pub payload: Bytes,
}

impl TrustMessage {
    pub fn new(msg_type: MessageType, tenant_id: Uuid, payload: Bytes) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(&payload);
        let checksum = hasher.finalize();

        TrustMessage {
            header: TrustHeader {
                version: 1,
                msg_type: msg_type as u8,
                flags: 0,
                length: payload.len() as u32,
                checksum,
                request_id: Uuid::new_v4(),
                tenant_id,
                signature: [0u8; 32],
            },
            payload,
        }
    }

    pub fn encode<B: BufMut>(&self, mut dst: B) {
        self.header.encode(&mut dst);
        dst.put(self.payload.clone());
    }
}

pub struct TrustCodec;

impl tokio_util::codec::Decoder for TrustCodec {
    type Item = TrustMessage;
    type Error = DsError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < TrustHeader::SIZE {
            return Ok(None);
        }

        // Peek payload length without consuming
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[8..12]);
        let payload_len = u32::from_be_bytes(length_bytes) as usize;
        let total_length = TrustHeader::SIZE + payload_len;

        if src.len() < total_length {
            src.reserve(total_length - src.len());
            return Ok(None);
        }

        let header = TrustHeader::decode(src)?;
        let payload = src.split_to(payload_len).freeze();

        // Verify payload checksum
        let mut hasher = Hasher::new();
        hasher.update(&payload);
        if hasher.finalize() != header.checksum {
            return Err(DsError::protocol("Payload checksum mismatch"));
        }

        Ok(Some(TrustMessage { header, payload }))
    }
}

impl tokio_util::codec::Encoder<TrustMessage> for TrustCodec {
    type Error = DsError;

    fn encode(&mut self, item: TrustMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.encode(dst);
        Ok(())
    }
}

/// Codec for encoding/decoding DsValue with TrustHeader.
pub struct DsValueCodec;

impl DsValueCodec {
    pub fn encode(
        value: &DsValue,
        tenant_id: Uuid,
        sdr_level: Redundancy,
    ) -> Result<Bytes, DsError> {
        let mut result = BytesMut::new();
        // Reserve space for TrustHeader
        result.resize(TrustHeader::SIZE, 0);

        // Encode DsValue directly into result after TrustHeader
        DsValueEncoder::encode_to_buf(value, &mut result)?;

        let total_len = result.len();

        // Calculate checksum on the encoded payload
        let mut hasher = Hasher::new();
        hasher.update(&result[TrustHeader::SIZE..]);
        let checksum = hasher.finalize();

        let mut header = TrustHeader {
            version: 1,
            msg_type: MessageType::Value as u8,
            flags: 0,
            length: total_len as u32,
            checksum,
            request_id: Uuid::new_v4(),
            tenant_id,
            signature: [0u8; 32],
        };
        header.set_sdr_level(sdr_level);

        // Encode header into the reserved space at the beginning
        let mut header_part = &mut result[..TrustHeader::SIZE];
        header.encode(&mut header_part);

        Ok(result.freeze())
    }

    pub fn decode(mut data: BytesMut) -> Result<(DsValue, TrustHeader), DsError> {
        let header = TrustHeader::decode(&mut data)?;
        let mut payload = data.freeze();
        let value = DsValueDecoder::decode(&mut payload)?;
        Ok((value, header))
    }
}

/// A client for the WeTrust protocol, similar to the JDBC implementation.
pub struct WeTrustClient {
    framed: Framed<TcpStream, TrustCodec>,
    tenant_id: Uuid,
    secret_key: Vec<u8>,
}

impl WeTrustClient {
    pub async fn connect(
        addr: SocketAddr,
        tenant_id: Uuid,
        secret_key: Vec<u8>,
    ) -> Result<Self, DsError> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| DsError::io_raw(e, Some(addr.to_string().into())))?;
        let mut framed = Framed::new(stream, TrustCodec);

        // Handshake: Send Auth message
        let mut auth_msg = TrustMessage::new(MessageType::Auth, tenant_id, Bytes::from("auth-v1"));
        auth_msg.header.sign(&secret_key);

        framed.send(auth_msg).await?;

        // Wait for Auth response
        if let Some(resp) = framed.next().await {
            let resp = resp?;
            if resp.header.msg_type != MessageType::Response as u8 {
                return Err(DsError::protocol(
                    "Unexpected message type during handshake",
                ));
            }
            if !resp.header.verify(&secret_key) {
                return Err(DsError::protocol("Handshake signature verification failed"));
            }
        } else {
            return Err(DsError::protocol("Connection closed during handshake"));
        }

        Ok(Self {
            framed,
            tenant_id,
            secret_key,
        })
    }

    pub async fn send_request(
        &mut self,
        msg_type: MessageType,
        payload: Bytes,
    ) -> Result<TrustMessage, DsError> {
        let mut msg = TrustMessage::new(msg_type, self.tenant_id, payload);
        msg.header.sign(&self.secret_key);

        self.framed.send(msg).await?;

        if let Some(resp) = self.framed.next().await {
            let resp = resp?;
            if !resp.header.verify(&self.secret_key) {
                return Err(DsError::protocol("Message signature verification failed"));
            }
            Ok(resp)
        } else {
            Err(DsError::protocol("Connection closed by server"))
        }
    }

    pub async fn send_query(&mut self, sql: &str) -> Result<Vec<Vec<DsValue>>, DsError> {
        let _resp = self
            .send_request(MessageType::Kql, Bytes::copy_from_slice(sql.as_bytes()))
            .await?;

        // In a real implementation, we would decode the payload into rows.
        // For now, we return a mock success result.
        Ok(vec![vec![DsValue::Text(format!("Executed: {}", sql))]])
    }

    pub async fn put(&mut self, key: &str, value: DsValue) -> Result<(), DsError> {
        let value_data = DsValueEncoder::encode(&value)?;
        let mut payload = BytesMut::with_capacity(4 + key.len() + value_data.len());
        payload.put_u32(key.len() as u32);
        payload.put_slice(key.as_bytes());
        payload.put(value_data);

        self.send_request(MessageType::Put, payload.freeze())
            .await?;
        Ok(())
    }

    pub async fn get(&mut self, key: &str) -> Result<Option<DsValue>, DsError> {
        let mut payload = BytesMut::with_capacity(4 + key.len());
        payload.put_u32(key.len() as u32);
        payload.put_slice(key.as_bytes());

        let resp = self
            .send_request(MessageType::Get, payload.freeze())
            .await?;
        if resp.header.msg_type == MessageType::Error as u8 {
            return Ok(None);
        }

        let mut data = resp.payload;
        if data.is_empty() {
            return Ok(None);
        }

        // Use DsValueDecoder to decode the value from the response
        let val = DsValueDecoder::decode(&mut data)?;
        Ok(Some(val))
    }

    pub async fn delete(&mut self, key: &str) -> Result<(), DsError> {
        let mut payload = BytesMut::with_capacity(4 + key.len());
        payload.put_u32(key.len() as u32);
        payload.put_slice(key.as_bytes());
        self.send_request(MessageType::Delete, payload.freeze())
            .await?;
        Ok(())
    }

    pub async fn kql(&mut self, query: &str) -> Result<DsValue, DsError> {
        let resp = self
            .send_request(MessageType::Kql, Bytes::copy_from_slice(query.as_bytes()))
            .await?;
        if resp.header.msg_type == MessageType::Error as u8 {
            return Err(DsError::query_with_sql(
                query,
                String::from_utf8_lossy(&resp.payload).to_string(),
            ));
        }
        let mut data = resp.payload;
        let value = DsValueDecoder::decode(&mut data)?;
        Ok(value)
    }

    pub async fn heartbeat(&mut self) -> Result<(), DsError> {
        self.send_request(MessageType::Heartbeat, Bytes::new())
            .await?;
        Ok(())
    }
}
