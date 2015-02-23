use crate::transaction::RedisTransaction;
use crate::{RedisCommand, RedisFrame, RedisService};
use bytes::Bytes;
use std::sync::Arc;
use yykv_types::{DsError, DsValue};

type Result<T> = std::result::Result<T, DsError>;

pub struct RedisConnection {
    service: Arc<RedisService>,
}

impl RedisConnection {
    pub fn new(service: Arc<RedisService>) -> Self {
        Self { service }
    }

    pub async fn execute(&self, sql: &str, _params: &[DsValue]) -> Result<u64> {
        // Simple mapping: "SET key value" -> RedisCommand::Set
        let parts: Vec<&str> = sql.split_whitespace().collect();
        if parts.is_empty() {
            return Err(DsError::query("Empty command"));
        }

        let cmd = match parts[0].to_uppercase().as_str() {
            "SET" if parts.len() == 3 => RedisCommand::Set(
                parts[1].to_string(),
                Bytes::copy_from_slice(parts[2].as_bytes()),
            ),
            "DEL" if parts.len() >= 2 => {
                RedisCommand::Del(parts.iter().skip(1).map(|s| s.to_string()).collect())
            }
            _ => {
                return Err(DsError::query(format!(
                    "Unsupported or invalid command for execute: {}",
                    sql
                )));
            }
        };

        match self.service.handle_command(cmd).await? {
            RedisFrame::SimpleString(s) if s == "OK" => Ok(1),
            RedisFrame::Integer(n) => Ok(n as u64),
            _ => Ok(0),
        }
    }

    pub async fn query(&self, sql: &str, _params: &[DsValue]) -> Result<Vec<DsValue>> {
        let parts: Vec<&str> = sql.split_whitespace().collect();
        if parts.is_empty() {
            return Err(DsError::query("Empty command"));
        }

        let cmd = match parts[0].to_uppercase().as_str() {
            "GET" if parts.len() == 2 => RedisCommand::Get(parts[1].to_string()),
            "KEYS" if parts.len() == 2 => RedisCommand::Keys(parts[1].to_string()),
            _ => {
                return Err(DsError::query(format!(
                    "Unsupported or invalid command for query: {}",
                    sql
                )));
            }
        };

        match self.service.handle_command(cmd).await? {
            RedisFrame::BulkString(Some(data)) => Ok(vec![DsValue::Binary(data)]),
            RedisFrame::BulkString(None) => Ok(vec![DsValue::Null]),
            RedisFrame::Array(Some(frames)) => {
                let mut results = Vec::new();
                for frame in frames {
                    match frame {
                        RedisFrame::BulkString(Some(data)) => results.push(DsValue::Binary(data)),
                        RedisFrame::Integer(n) => results.push(DsValue::Int(n)),
                        _ => {}
                    }
                }
                Ok(results)
            }
            _ => Ok(vec![]),
        }
    }

    pub async fn begin_transaction(self) -> Result<RedisTransaction> {
        Ok(RedisTransaction::new(self))
    }
}
