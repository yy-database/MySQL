#![warn(missing_docs)]

use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::sync::Arc;
use tokio_util::codec::{Decoder, Encoder};
use uuid::Uuid;
use yykv_types::{DsError, DsValue, Redundancy};

use std::path::PathBuf;

type Result<T> = std::result::Result<T, DsError>;
use yykv_wal::{OpType, WalManager};

pub mod adapter;
pub mod connection;
pub mod transaction;

pub use adapter::RedisAdapter;
pub use connection::RedisConnection;
pub use transaction::RedisTransaction;

#[derive(Debug, Clone, PartialEq)]
pub enum RedisFrame {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Option<Bytes>),
    Array(Option<Vec<RedisFrame>>),
}

use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::{error, info};

pub async fn handle_connection(stream: TcpStream, service: Arc<RedisService>) -> Result<()> {
    let mut framed = Framed::new(stream, RedisCodec);

    while let Some(result) = framed.next().await {
        let frame = match result {
            Ok(f) => f,
            Err(e) => {
                error!("Decode error: {}", e);
                break;
            }
        };

        // Parse command from frame
        let cmd = match RedisCommand::from_frame(frame) {
            Ok(c) => c,
            Err(e) => {
                let err_frame = RedisFrame::Error(format!("ERR {}", e));
                let _ = framed.send(err_frame).await;
                continue;
            }
        };

        // Handle command via RedisService (YY ecosystem)
        let response = service.handle_command(cmd).await?;

        // Send response back
        framed.send(response).await?;
    }

    info!("Connection closed");
    Ok(())
}

pub struct RedisCodec;

impl Decoder for RedisCodec {
    type Item = RedisFrame;
    type Error = DsError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if src.is_empty() {
            return Ok(None);
        }

        match src[0] {
            b'+' => self.decode_simple_string(src),
            b'-' => self.decode_error(src),
            b':' => self.decode_integer(src),
            b'$' => self.decode_bulk_string(src),
            b'*' => self.decode_array(src),
            _ => Err(DsError::protocol(format!(
                "Invalid Redis frame type: {}",
                src[0] as char
            ))),
        }
    }
}

impl RedisCodec {
    fn decode_simple_string(&mut self, src: &mut BytesMut) -> Result<Option<RedisFrame>> {
        if let Some(i) = self.find_crlf(src) {
            let line = src.split_to(i + 2);
            let s = String::from_utf8(line[1..i].to_vec())?;
            Ok(Some(RedisFrame::SimpleString(s)))
        } else {
            Ok(None)
        }
    }

    fn decode_error(&mut self, src: &mut BytesMut) -> Result<Option<RedisFrame>> {
        if let Some(i) = self.find_crlf(src) {
            let line = src.split_to(i + 2);
            let s = String::from_utf8(line[1..i].to_vec())?;
            Ok(Some(RedisFrame::Error(s)))
        } else {
            Ok(None)
        }
    }

    fn decode_integer(&mut self, src: &mut BytesMut) -> Result<Option<RedisFrame>> {
        if let Some(i) = self.find_crlf(src) {
            let line = src.split_to(i + 2);
            let s = std::str::from_utf8(&line[1..i])?;
            let n = s.parse::<i64>()?;
            Ok(Some(RedisFrame::Integer(n)))
        } else {
            Ok(None)
        }
    }

    fn decode_bulk_string(&mut self, src: &mut BytesMut) -> Result<Option<RedisFrame>> {
        if let Some(i) = self.find_crlf(src) {
            let line = &src[..i];
            let len = std::str::from_utf8(&line[1..])?.parse::<isize>()?;

            if len == -1 {
                src.advance(i + 2);
                return Ok(Some(RedisFrame::BulkString(None)));
            }

            let bulk_len = len as usize;
            if src.len() < i + 2 + bulk_len + 2 {
                return Ok(None);
            }

            src.advance(i + 2);
            let data = src.split_to(bulk_len).freeze();
            src.advance(2); // CRLF
            Ok(Some(RedisFrame::BulkString(Some(data))))
        } else {
            Ok(None)
        }
    }

    fn decode_array(&mut self, src: &mut BytesMut) -> Result<Option<RedisFrame>> {
        if let Some(i) = self.find_crlf(src) {
            let line = &src[..i];
            let len = std::str::from_utf8(&line[1..])?.parse::<isize>()?;

            if len == -1 {
                src.advance(i + 2);
                return Ok(Some(RedisFrame::Array(None)));
            }

            let array_len = len as usize;
            src.advance(i + 2);

            let mut frames = Vec::with_capacity(array_len);
            for _ in 0..array_len {
                match self.decode(src)? {
                    Some(frame) => frames.push(frame),
                    None => return Ok(None), // Should technically buffer or error
                }
            }
            Ok(Some(RedisFrame::Array(Some(frames))))
        } else {
            Ok(None)
        }
    }

    fn find_crlf(&self, src: &[u8]) -> Option<usize> {
        src.windows(2).position(|w| w == b"\r\n")
    }
}

impl Encoder<RedisFrame> for RedisCodec {
    type Error = DsError;

    fn encode(&mut self, item: RedisFrame, dst: &mut BytesMut) -> Result<()> {
        match item {
            RedisFrame::SimpleString(s) => {
                dst.reserve(1 + s.len() + 2);
                dst.put_u8(b'+');
                dst.put_slice(s.as_bytes());
                dst.put_slice(b"\r\n");
            }
            RedisFrame::Error(s) => {
                dst.reserve(1 + s.len() + 2);
                dst.put_u8(b'-');
                dst.put_slice(s.as_bytes());
                dst.put_slice(b"\r\n");
            }
            RedisFrame::Integer(n) => {
                let s = n.to_string();
                dst.reserve(1 + s.len() + 2);
                dst.put_u8(b':');
                dst.put_slice(s.as_bytes());
                dst.put_slice(b"\r\n");
            }
            RedisFrame::BulkString(opt_data) => match opt_data {
                Some(data) => {
                    let len_s = data.len().to_string();
                    dst.reserve(1 + len_s.len() + 2 + data.len() + 2);
                    dst.put_u8(b'$');
                    dst.put_slice(len_s.as_bytes());
                    dst.put_slice(b"\r\n");
                    dst.put_slice(&data);
                    dst.put_slice(b"\r\n");
                }
                None => {
                    dst.put_slice(b"$-1\r\n");
                }
            },
            RedisFrame::Array(opt_frames) => match opt_frames {
                Some(frames) => {
                    let len_s = frames.len().to_string();
                    dst.reserve(1 + len_s.len() + 2);
                    dst.put_u8(b'*');
                    dst.put_slice(len_s.as_bytes());
                    dst.put_slice(b"\r\n");
                    for frame in frames {
                        self.encode(frame, dst)?;
                    }
                }
                None => {
                    dst.put_slice(b"*-1\r\n");
                }
            },
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum RedisCommand {
    Ping,
    Get(String),
    Set(String, Bytes),
    Del(Vec<String>),
    Exists(Vec<String>),
    // Hash commands
    HSet(String, String, Bytes),
    HGet(String, String),
    HDel(String, Vec<String>),
    // Atomic increments
    Incr(String),
    Decr(String),
    IncrBy(String, i64),
    // TTL commands
    Expire(String, u64),
    Ttl(String),
    // Pattern matching
    Keys(String),
}

impl RedisCommand {
    pub fn from_frame(frame: RedisFrame) -> Result<Self> {
        match frame {
            RedisFrame::Array(Some(frames)) => {
                if frames.is_empty() {
                    return Err(DsError::protocol("Empty command array"));
                }

                let cmd_name = match &frames[0] {
                    RedisFrame::BulkString(Some(data)) => {
                        String::from_utf8_lossy(data).to_uppercase()
                    }
                    _ => {
                        return Err(DsError::protocol("Command name must be a bulk string"));
                    }
                };

                match cmd_name.as_str() {
                    "PING" => Ok(RedisCommand::Ping),
                    "GET" => {
                        if frames.len() != 2 {
                            return Err(DsError::protocol("GET requires exactly 1 argument"));
                        }
                        let key = match &frames[1] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).to_string()
                            }
                            _ => {
                                return Err(DsError::protocol("GET key must be a bulk string"));
                            }
                        };
                        Ok(RedisCommand::Get(key))
                    }
                    "SET" => {
                        if frames.len() != 3 {
                            return Err(DsError::protocol("SET requires exactly 2 arguments"));
                        }
                        let key = match &frames[1] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).to_string()
                            }
                            _ => {
                                return Err(DsError::protocol("SET key must be a bulk string"));
                            }
                        };
                        let value = match &frames[2] {
                            RedisFrame::BulkString(Some(data)) => data.clone(),
                            _ => {
                                return Err(DsError::protocol("SET value must be a bulk string"));
                            }
                        };
                        Ok(RedisCommand::Set(key, value))
                    }
                    "DEL" => {
                        let mut keys = Vec::new();
                        for frame in frames.iter().skip(1) {
                            match frame {
                                RedisFrame::BulkString(Some(data)) => {
                                    keys.push(String::from_utf8_lossy(data).to_string())
                                }
                                _ => {
                                    return Err(DsError::protocol("DEL key must be a bulk string"));
                                }
                            }
                        }
                        Ok(RedisCommand::Del(keys))
                    }
                    "EXISTS" => {
                        let mut keys = Vec::new();
                        for frame in frames.iter().skip(1) {
                            match frame {
                                RedisFrame::BulkString(Some(data)) => {
                                    keys.push(String::from_utf8_lossy(data).to_string())
                                }
                                _ => {
                                    return Err(DsError::protocol(
                                        "EXISTS key must be a bulk string",
                                    ));
                                }
                            }
                        }
                        Ok(RedisCommand::Exists(keys))
                    }
                    "HSET" => {
                        if frames.len() != 4 {
                            return Err(DsError::protocol("HSET requires exactly 3 arguments"));
                        }
                        let key = match &frames[1] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).to_string()
                            }
                            _ => {
                                return Err(DsError::protocol("HSET key must be a bulk string"));
                            }
                        };
                        let field = match &frames[2] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).to_string()
                            }
                            _ => {
                                return Err(DsError::protocol("HSET field must be a bulk string"));
                            }
                        };
                        let value = match &frames[3] {
                            RedisFrame::BulkString(Some(data)) => data.clone(),
                            _ => {
                                return Err(DsError::protocol("HSET value must be a bulk string"));
                            }
                        };
                        Ok(RedisCommand::HSet(key, field, value))
                    }
                    "HGET" => {
                        if frames.len() != 3 {
                            return Err(DsError::protocol("HGET requires exactly 2 arguments"));
                        }
                        let key = match &frames[1] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).to_string()
                            }
                            _ => {
                                return Err(DsError::protocol("HGET key must be a bulk string"));
                            }
                        };
                        let field = match &frames[2] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).to_string()
                            }
                            _ => {
                                return Err(DsError::protocol("HGET field must be a bulk string"));
                            }
                        };
                        Ok(RedisCommand::HGet(key, field))
                    }
                    "HDEL" => {
                        if frames.len() < 3 {
                            return Err(DsError::protocol("HDEL requires at least 2 arguments"));
                        }
                        let key = match &frames[1] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).to_string()
                            }
                            _ => {
                                return Err(DsError::protocol("HDEL key must be a bulk string"));
                            }
                        };
                        let mut fields = Vec::new();
                        for frame in frames.iter().skip(2) {
                            match frame {
                                RedisFrame::BulkString(Some(data)) => {
                                    fields.push(String::from_utf8_lossy(data).to_string())
                                }
                                _ => {
                                    return Err(DsError::protocol(
                                        "HDEL field must be a bulk string",
                                    ));
                                }
                            }
                        }
                        Ok(RedisCommand::HDel(key, fields))
                    }
                    "INCR" => {
                        if frames.len() != 2 {
                            return Err(DsError::protocol("INCR requires exactly 1 argument"));
                        }
                        let key = match &frames[1] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).to_string()
                            }
                            _ => {
                                return Err(DsError::protocol("INCR key must be a bulk string"));
                            }
                        };
                        Ok(RedisCommand::Incr(key))
                    }
                    "DECR" => {
                        if frames.len() != 2 {
                            return Err(DsError::protocol("DECR requires exactly 1 argument"));
                        }
                        let key = match &frames[1] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).to_string()
                            }
                            _ => {
                                return Err(DsError::protocol("DECR key must be a bulk string"));
                            }
                        };
                        Ok(RedisCommand::Decr(key))
                    }
                    "INCRBY" => {
                        if frames.len() != 3 {
                            return Err(DsError::protocol("INCRBY requires exactly 2 arguments"));
                        }
                        let key = match &frames[1] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).to_string()
                            }
                            _ => {
                                return Err(DsError::protocol("INCRBY key must be a bulk string"));
                            }
                        };
                        let amount = match &frames[2] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).parse::<i64>().map_err(|e| {
                                    DsError::protocol(format!("Invalid INCRBY amount: {}", e))
                                })?
                            }
                            _ => {
                                return Err(DsError::protocol(
                                    "INCRBY amount must be a bulk string",
                                ));
                            }
                        };
                        Ok(RedisCommand::IncrBy(key, amount))
                    }
                    "EXPIRE" => {
                        if frames.len() != 3 {
                            return Err(DsError::protocol("EXPIRE requires exactly 2 arguments"));
                        }
                        let key = match &frames[1] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).to_string()
                            }
                            _ => {
                                return Err(DsError::protocol("EXPIRE key must be a bulk string"));
                            }
                        };
                        let seconds = match &frames[2] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).parse::<u64>().map_err(|e| {
                                    DsError::protocol(format!("Invalid EXPIRE seconds: {}", e))
                                })?
                            }
                            _ => {
                                return Err(DsError::protocol(
                                    "EXPIRE seconds must be a bulk string",
                                ));
                            }
                        };
                        Ok(RedisCommand::Expire(key, seconds))
                    }
                    "TTL" => {
                        if frames.len() != 2 {
                            return Err(DsError::protocol("TTL requires exactly 1 argument"));
                        }
                        let key = match &frames[1] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).to_string()
                            }
                            _ => {
                                return Err(DsError::protocol("TTL key must be a bulk string"));
                            }
                        };
                        Ok(RedisCommand::Ttl(key))
                    }
                    "KEYS" => {
                        if frames.len() != 2 {
                            return Err(DsError::protocol("KEYS requires exactly 1 argument"));
                        }
                        let pattern = match &frames[1] {
                            RedisFrame::BulkString(Some(data)) => {
                                String::from_utf8_lossy(data).to_string()
                            }
                            _ => {
                                return Err(DsError::protocol(
                                    "KEYS pattern must be a bulk string",
                                ));
                            }
                        };
                        Ok(RedisCommand::Keys(pattern))
                    }
                    _ => Err(DsError::protocol(format!(
                        "Unsupported command: {}",
                        cmd_name
                    ))),
                }
            }
            _ => Err(DsError::protocol("Invalid command frame: must be an array")),
        }
    }
}

pub struct RedisService {
    wal: Arc<WalManager>,
    tenant_id: Uuid,
    // In-memory state for the example to actually work
    kv: Arc<tokio::sync::RwLock<std::collections::HashMap<String, Bytes>>>,
    hash: Arc<
        tokio::sync::RwLock<
            std::collections::HashMap<String, std::collections::HashMap<String, Bytes>>,
        >,
    >,
    ttl: Arc<tokio::sync::RwLock<std::collections::HashMap<String, u64>>>,
}

impl RedisService {
    pub async fn new() -> Result<Self> {
        let wal = Arc::new(
            WalManager::new(PathBuf::from("wal_redis"))
                .await
                .map_err(|e| DsError::storage(e.to_string()))?,
        );
        // let index = Arc::new(SearchIndexManager::new_in_memory()?);

        Ok(Self {
            wal,
            tenant_id: Uuid::new_v4(),
            kv: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            hash: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            ttl: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        })
    }

    pub async fn handle_command(&self, cmd: RedisCommand) -> Result<RedisFrame> {
        match cmd {
            RedisCommand::Ping => Ok(RedisFrame::SimpleString("PONG".to_string())),
            RedisCommand::Set(key, value) => {
                // 1. Update in-memory state
                {
                    let mut kv = self.kv.write().await;
                    kv.insert(key.clone(), value.clone());
                }

                // 2. Log to WAL (Simplified)
                self.wal
                    .write(
                        self.tenant_id,
                        "redis_kv".to_string(),
                        key.clone(),
                        OpType::Insert,
                        Redundancy::SINGLE,
                        DsValue::Binary(value),
                    )
                    .await?;

                Ok(RedisFrame::SimpleString("OK".to_string()))
            }
            RedisCommand::Get(key) => {
                let kv = self.kv.read().await;
                match kv.get(&key) {
                    Some(val) => Ok(RedisFrame::BulkString(Some(val.clone()))),
                    None => Ok(RedisFrame::BulkString(None)),
                }
            }
            RedisCommand::Del(keys) => {
                let mut kv = self.kv.write().await;
                let mut count = 0;
                for key in keys {
                    if kv.remove(&key).is_some() {
                        count += 1;
                        self.wal
                            .write(
                                self.tenant_id,
                                "redis_kv".to_string(),
                                key.clone(),
                                OpType::Delete,
                                Redundancy::SINGLE,
                                DsValue::Text(key),
                            )
                            .await?;
                    }
                }
                Ok(RedisFrame::Integer(count))
            }
            RedisCommand::Exists(keys) => {
                let kv = self.kv.read().await;
                let mut count = 0;
                for key in keys {
                    if kv.contains_key(&key) {
                        count += 1;
                    }
                }
                Ok(RedisFrame::Integer(count))
            }
            RedisCommand::HSet(key, field, value) => {
                let mut hash = self.hash.write().await;
                let entry = hash
                    .entry(key)
                    .or_insert_with(std::collections::HashMap::new);
                entry.insert(field, value);
                Ok(RedisFrame::Integer(1))
            }
            RedisCommand::HGet(key, field) => {
                let hash = self.hash.read().await;
                match hash.get(&key).and_then(|m| m.get(&field)) {
                    Some(val) => Ok(RedisFrame::BulkString(Some(val.clone()))),
                    None => Ok(RedisFrame::BulkString(None)),
                }
            }
            RedisCommand::HDel(key, fields) => {
                let mut hash = self.hash.write().await;
                let mut count = 0;
                if let Some(m) = hash.get_mut(&key) {
                    for field in fields {
                        if m.remove(&field).is_some() {
                            count += 1;
                        }
                    }
                }
                Ok(RedisFrame::Integer(count))
            }
            RedisCommand::Incr(key) => {
                let mut kv = self.kv.write().await;
                let val = kv
                    .get(&key)
                    .map(|v| String::from_utf8_lossy(v).parse::<i64>().unwrap_or(0))
                    .unwrap_or(0);
                let new_val = val + 1;
                kv.insert(key, Bytes::from(new_val.to_string()));
                Ok(RedisFrame::Integer(new_val))
            }
            RedisCommand::Decr(key) => {
                let mut kv = self.kv.write().await;
                let val = kv
                    .get(&key)
                    .map(|v| String::from_utf8_lossy(v).parse::<i64>().unwrap_or(0))
                    .unwrap_or(0);
                let new_val = val - 1;
                kv.insert(key, Bytes::from(new_val.to_string()));
                Ok(RedisFrame::Integer(new_val))
            }
            RedisCommand::IncrBy(key, amount) => {
                let mut kv = self.kv.write().await;
                let val = kv
                    .get(&key)
                    .map(|v| String::from_utf8_lossy(v).parse::<i64>().unwrap_or(0))
                    .unwrap_or(0);
                let new_val = val + amount;
                kv.insert(key, Bytes::from(new_val.to_string()));
                Ok(RedisFrame::Integer(new_val))
            }
            RedisCommand::Expire(key, seconds) => {
                let mut ttl = self.ttl.write().await;
                ttl.insert(key, seconds);
                Ok(RedisFrame::Integer(1))
            }
            RedisCommand::Ttl(key) => {
                let ttl = self.ttl.read().await;
                match ttl.get(&key) {
                    Some(s) => Ok(RedisFrame::Integer(*s as i64)),
                    None => Ok(RedisFrame::Integer(-1)),
                }
            }
            RedisCommand::Keys(pattern) => {
                let kv = self.kv.read().await;
                let keys: Vec<RedisFrame> = kv
                    .keys()
                    .filter(|k| k.contains(&pattern.replace("*", "")))
                    .map(|k| RedisFrame::BulkString(Some(Bytes::copy_from_slice(k.as_bytes()))))
                    .collect();
                Ok(RedisFrame::Array(Some(keys)))
            }
        }
    }
}
