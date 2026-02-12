use crate::codec::{BackendMessage, PgCodec};
use crate::message::Message;
use crate::transaction::PostgresTransaction;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::{debug, info};
use yykv_types::{
    ColumnInfo, DsError, DsResult, DsValue, EnumInfo, SchemaInspector, TableInfo,
};

#[async_trait::async_trait]
impl SchemaInspector for PostgresConnection {
    async fn introspect(&self, schema: Option<&str>) -> DsResult<(Vec<TableInfo>, Vec<EnumInfo>)> {
        let schema = schema.unwrap_or("public");

        // 1. Fetch enums
        let enum_sql = format!(
            "SELECT 
                t.typname as enum_name,
                e.enumlabel as enum_variant
             FROM pg_type t
             JOIN pg_enum e ON t.oid = e.enumtypid
             JOIN pg_catalog.pg_namespace n ON n.oid = t.typnamespace
             WHERE n.nspname = '{}'
             ORDER BY t.typname, e.enumsortorder",
            schema
        );

        let enum_rows = self.query(&enum_sql, &[]).await?;
        let mut enums = Vec::new();
        let mut current_enum: Option<EnumInfo> = None;

        for row in enum_rows {
            if let DsValue::List(fields) = row {
                let name = match fields.first() {
                    Some(DsValue::Text(s)) => s.clone(),
                    _ => continue,
                };
                let variant = match fields.get(1) {
                    Some(DsValue::Text(s)) => s.clone(),
                    _ => continue,
                };

                if let Some(ref mut e) = current_enum {
                    if e.name == name {
                        e.variants.push(variant);
                        continue;
                    } else {
                        enums.push(current_enum.take().unwrap());
                    }
                }
                current_enum = Some(EnumInfo {
                    name,
                    variants: vec![variant],
                });
            }
        }
        if let Some(e) = current_enum {
            enums.push(e);
        }

        // 2. Fetch tables
        let sql = format!(
            "SELECT 
                c.relname as table_name,
                pg_catalog.obj_description(c.oid, 'pg_class') as table_comment
             FROM pg_catalog.pg_class c
             JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
             WHERE n.nspname = '{}' AND c.relkind = 'r' AND c.relname NOT LIKE 'pg_%' AND c.relname NOT LIKE 'sql_%'",
            schema
        );

        let table_rows = self.query(&sql, &[]).await?;
        let mut tables = Vec::new();

        for row in table_rows {
            if let DsValue::List(fields) = row {
                let table_name = match fields.first() {
                    Some(DsValue::Text(s)) => s.clone(),
                    _ => continue,
                };
                let description = match fields.get(1) {
                    Some(DsValue::Text(s)) => Some(s.clone()),
                    _ => None,
                };

                let mut columns = Vec::new();
                let col_sql = format!(
                    "SELECT 
                        column_name, 
                        data_type, 
                        is_nullable,
                        column_default,
                        (SELECT pg_catalog.col_description(c.oid, a.attnum)
                         FROM pg_catalog.pg_class c
                         JOIN pg_catalog.pg_attribute a ON c.oid = a.attrelid
                         WHERE c.relname = '{}' AND a.attname = column_name) as column_comment
                     FROM information_schema.columns 
                     WHERE table_schema = '{}' AND table_name = '{}'
                     ORDER BY ordinal_position",
                    table_name, schema, table_name
                );

                let col_rows = self.query(&col_sql, &[]).await?;
                for col_row in col_rows {
                    if let DsValue::List(cfields) = col_row {
                        let name = match cfields.first() {
                            Some(DsValue::Text(s)) => s.clone(),
                            _ => continue,
                        };
                        let data_type = match cfields.get(1) {
                            Some(DsValue::Text(s)) => s.clone(),
                            _ => continue,
                        };
                        let is_nullable = match cfields.get(2) {
                            Some(DsValue::Text(s)) => s == "YES",
                            _ => false,
                        };
                        let default = match cfields.get(3) {
                            Some(DsValue::Text(s)) => Some(s.clone()),
                            _ => None,
                        };
                        let col_description = match cfields.get(4) {
                            Some(DsValue::Text(s)) => Some(s.clone()),
                            _ => None,
                        };

                        columns.push(ColumnInfo {
                            name,
                            data_type,
                            is_nullable,
                            is_primary_key: false, // Will fill later
                            is_enum: false,        // Will fill later
                            foreign_key: None,
                            default,
                            description: col_description,
                        });
                    }
                }

                tables.push(TableInfo {
                    name: table_name,
                    columns,
                    description,
                });
            }
        }

        Ok((tables, enums))
    }
}

type Result<T> = std::result::Result<T, DsError>;

pub struct PostgresConnection {
    pub url: String,
}

struct ConnectionConfig {
    host: String,
    port: u16,
    user: String,
    password: Option<String>,
    database: String,
}

impl PostgresConnection {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    fn parse_url(&self) -> Result<ConnectionConfig> {
        let url = self
            .url
            .strip_prefix("postgres://")
            .or_else(|| self.url.strip_prefix("postgresql://"))
            .ok_or_else(|| DsError::protocol("Invalid postgres URL scheme"))?;

        let (auth_host, database) = match url.find('/') {
            Some(i) => (&url[..i], url[i + 1..].to_string()),
            None => (url, "postgres".to_string()),
        };

        let (auth, host_port) = match auth_host.find('@') {
            Some(i) => (&auth_host[..i], &auth_host[i + 1..]),
            None => ("postgres", auth_host),
        };

        let (user, password) = match auth.find(':') {
            Some(i) => (auth[..i].to_string(), Some(auth[i + 1..].to_string())),
            None => (auth.to_string(), None),
        };

        let (host, port) = match host_port.find(':') {
            Some(i) => (
                host_port[..i].to_string(),
                host_port[i + 1..]
                    .parse()
                    .map_err(|_| DsError::protocol("Invalid port"))?,
            ),
            None => (host_port.to_string(), 5432),
        };

        Ok(ConnectionConfig {
            host,
            port,
            user,
            password,
            database,
        })
    }

    async fn connect(&self) -> Result<Framed<TcpStream, PgCodec>> {
        let config = self.parse_url()?;
        let addr = format!("{}:{}", config.host, config.port);
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| DsError::protocol(format!("Failed to connect to postgres: {}", e)))?;

        let mut framed = Framed::new(stream, PgCodec::new());

        // 1. Startup
        let params = vec![
            ("user".to_string(), config.user.clone()),
            ("database".to_string(), config.database.clone()),
            ("client_encoding".to_string(), "UTF8".to_string()),
        ];

        framed.send(Message::Startup { params }).await?;

        // 2. Authentication & Ready
        loop {
            let msg = framed
                .next()
                .await
                .ok_or_else(|| DsError::protocol("Connection closed during startup"))??;

            match msg {
                BackendMessage::AuthenticationOk => {
                    debug!("Postgres authentication OK");
                }
                BackendMessage::AuthenticationCleartextPassword => {
                    let pass = config.password.as_ref().ok_or_else(|| {
                        DsError::protocol("Password required by server but not provided")
                    })?;
                    framed.send(Message::Password(pass.clone())).await?;
                }
                BackendMessage::AuthenticationMD5Password { salt } => {
                    let user = config.user.clone();
                    let pass = config.password.as_ref().ok_or_else(|| {
                        DsError::protocol("Password required by server but not provided")
                    })?;

                    // md5(md5(password + username) + salt)
                    let hash1 = md5::compute(format!("{}{}", pass, user));
                    let hash1_hex = hex::encode(hash1.0);

                    let mut hash2_input = Vec::new();
                    hash2_input.extend_from_slice(hash1_hex.as_bytes());
                    hash2_input.extend_from_slice(&salt);

                    let hash2 = md5::compute(hash2_input);
                    let response = format!("md5{}", hex::encode(hash2.0));

                    framed.send(Message::Password(response)).await?;
                }
                BackendMessage::ParameterStatus { name, value } => {
                    debug!("Postgres parameter status: {} = {}", name, value);
                }
                BackendMessage::BackendKeyData { .. } => {}
                BackendMessage::ReadyForQuery { .. } => {
                    break;
                }
                BackendMessage::ErrorResponse { fields } => {
                    let msg = fields
                        .iter()
                        .find(|(t, _)| *t == b'M')
                        .map(|(_, m)| m.clone())
                        .unwrap_or_default();
                    return Err(DsError::protocol(format!(
                        "Postgres startup error: {}",
                        msg
                    )));
                }
                _ => {
                    debug!("Received unexpected message during startup: {:?}", msg);
                }
            }
        }

        Ok(framed)
    }

    pub async fn execute(&self, sql: &str, params: &[DsValue]) -> Result<u64> {
        info!("Postgres executing: {}", sql);
        let mut framed = self.connect().await?;

        if params.is_empty() {
            // Use Simple Query for no parameters
            framed.send(Message::Query(sql.to_string())).await?;
        } else {
            // Use Extended Query for parameters
            framed
                .send(Message::Parse {
                    name: "".to_string(),
                    query: sql.to_string(),
                    param_types: vec![0; params.len()], // Ask Postgres to infer types for all parameters
                })
                .await?;

            framed
                .send(Message::Bind {
                    portal: "".to_string(),
                    statement: "".to_string(),
                    params: params.to_vec(),
                })
                .await?;

            framed
                .send(Message::Execute {
                    portal: "".to_string(),
                    max_rows: 0,
                })
                .await?;

            framed.send(Message::Sync).await?;
        }

        let mut affected_rows = 0;
        loop {
            let msg = framed
                .next()
                .await
                .ok_or_else(|| DsError::protocol("Connection closed during execute"))??;

            match msg {
                BackendMessage::CommandComplete { tag } => {
                    if let Some(s) = tag.split_whitespace().last() {
                        affected_rows = s.parse().unwrap_or(0);
                    }
                }
                BackendMessage::ReadyForQuery { .. } => break,
                BackendMessage::ErrorResponse { fields } => {
                    let msg = fields
                        .iter()
                        .find(|(t, _)| *t == b'M')
                        .map(|(_, m)| m.clone())
                        .unwrap_or_default();
                    return Err(DsError::protocol(format!(
                        "Postgres execute error: {}",
                        msg
                    )));
                }
                _ => {}
            }
        }

        Ok(affected_rows)
    }

    pub async fn query(&self, sql: &str, params: &[DsValue]) -> Result<Vec<DsValue>> {
        info!("Postgres querying: {}", sql);

        let mut framed = self.connect().await?;

        if params.is_empty() {
            // Use Simple Query for no parameters
            framed.send(Message::Query(sql.to_string())).await?;
        } else {
            // Use Extended Query for parameters
            framed
                .send(Message::Parse {
                    name: "".to_string(),
                    query: sql.to_string(),
                    param_types: vec![0; params.len()], // Ask Postgres to infer types for all parameters
                })
                .await?;

            framed
                .send(Message::Bind {
                    portal: "".to_string(),
                    statement: "".to_string(),
                    params: params.to_vec(),
                })
                .await?;

            framed
                .send(Message::Execute {
                    portal: "".to_string(),
                    max_rows: 0,
                })
                .await?;

            framed.send(Message::Sync).await?;
        }

        let mut results = Vec::new();
        loop {
            let msg = framed
                .next()
                .await
                .ok_or_else(|| DsError::protocol("Connection closed during query"))??;

            match msg {
                BackendMessage::RowDescription { .. } => {}
                BackendMessage::DataRow { values } => {
                    debug!("Postgres received row with {} columns", values.len());
                    let mut row = Vec::with_capacity(values.len());
                    for val in values {
                        match val {
                            Some(bytes) => {
                                let s = String::from_utf8_lossy(&bytes).to_string();
                                row.push(DsValue::Text(s));
                            }
                            None => row.push(DsValue::Null),
                        }
                    }
                    results.push(DsValue::List(row));
                }
                BackendMessage::ReadyForQuery { .. } => break,
                BackendMessage::ErrorResponse { fields } => {
                    let msg = fields
                        .iter()
                        .find(|(t, _)| *t == b'M')
                        .map(|(_, m)| m.clone())
                        .unwrap_or_default();
                    return Err(DsError::protocol(format!("Postgres query error: {}", msg)));
                }
                _ => {}
            }
        }

        Ok(results)
    }

    pub async fn begin_transaction(self) -> Result<PostgresTransaction> {
        Ok(PostgresTransaction::new(self))
    }
}
