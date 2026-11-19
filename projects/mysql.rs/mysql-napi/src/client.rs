//! Node-API handles over the upstream `database.client::mysql` wire client.
//!
//! The startup handshake, password authentication and both statement paths
//! (text `COM_QUERY` and prepared `COM_STMT_PREPARE` / `COM_STMT_EXECUTE`) run
//! in the upstream client; this module only moves values across the ABI
//! boundary and restores declared column types into JS values. No protocol
//! bytes are interpreted here and no second state machine grows on this side.

use std::{
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use database::client::mysql::{Cell, ClientConfig, ColumnInfo, Connection, Param, QueryResult, mysql_type};
use napi::{
    Env, Error, Result,
    bindgen_prelude::{AsyncTask, BigInt, Buffer, Either, Either4, Task},
};
use napi_derive::napi;

fn map_database_error(error: database::types::Error) -> Error {
    Error::from_reason(format!("{error}"))
}

/// Handshake settings mirrored onto the upstream [`ClientConfig`].
#[napi(object)]
pub struct MysqlClientConnectOptions {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: Option<String>,
    pub database: Option<String>,
    /// Connect and per-read timeout in milliseconds; the upstream default
    /// applies when absent.
    pub timeout_ms: Option<u32>,
}

/// One column of a client result set, as declared by the server.
#[napi(object)]
pub struct ClientColumn {
    pub name: String,
    pub type_code: u32,
    pub unsigned: bool,
    pub binary: bool,
}

/// A restored cell: `null`, `BigInt`, `number`, text or raw octets.
pub type ClientValue = Option<Either4<BigInt, f64, String, Buffer>>;

/// Outcome of one client statement.
#[napi(object)]
pub struct ClientResult {
    pub columns: Vec<ClientColumn>,
    pub rows: Vec<Vec<ClientValue>>,
    pub affected_rows: BigInt,
    pub last_insert_id: BigInt,
}

/// Opens one remote session: TCP connect, startup handshake and password
/// authentication run on the thread pool through the upstream client, so a
/// wrong password or an unreachable server fails the returned promise.
#[napi]
pub fn mysql_client_connect(options: MysqlClientConnectOptions) -> AsyncTask<ConnectTask> {
    AsyncTask::new(ConnectTask { options })
}

pub struct ConnectTask {
    options: MysqlClientConnectOptions,
}

impl Task for ConnectTask {
    type Output = MysqlClientSession;
    type JsValue = MysqlClientSession;

    fn compute(&mut self) -> Result<Self::Output> {
        let mut config = ClientConfig::new(self.options.host.clone(), self.options.port, self.options.user.clone());
        // The official client answers with an empty auth response for an empty
        // password; the upstream config models that as "no password", so an
        // empty string maps onto it here rather than scrambling `""`.
        if let Some(password) = self.options.password.as_ref().filter(|value| !value.is_empty()) {
            config = config.with_password(password.clone());
        }
        if let Some(database) = &self.options.database {
            config = config.with_database(database.clone());
        }
        if let Some(timeout_ms) = self.options.timeout_ms {
            config = config.with_timeout(Duration::from_millis(u64::from(timeout_ms)));
        }
        let connection = Connection::open(&config).map_err(map_database_error)?;
        Ok(MysqlClientSession { inner: Arc::new(Mutex::new(Some(connection))) })
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

/// One live client session owning the upstream TCP connection. Calls are
/// serialized through the mutex, which is exactly the single-physical-session
/// semantics the wire protocol requires.
#[napi(js_name = "MysqlClientSession")]
pub struct MysqlClientSession {
    inner: Arc<Mutex<Option<Connection>>>,
}

#[napi]
impl MysqlClientSession {
    /// Runs one text-protocol statement (`COM_QUERY`).
    #[napi]
    pub fn query(&self, sql: String) -> AsyncTask<QueryTask> {
        AsyncTask::new(QueryTask { session: Arc::clone(&self.inner), sql, params: None })
    }

    /// Runs one prepared statement; every `?` binds through the binary
    /// protocol, `null` binds SQL `NULL` and `Uint8Array` binds raw octets.
    #[napi]
    pub fn query_with_params(&self, sql: String, params: Vec<Option<Either<String, Buffer>>>) -> AsyncTask<QueryTask> {
        AsyncTask::new(QueryTask { session: Arc::clone(&self.inner), sql, params: Some(params) })
    }

    /// Drops the TCP connection. Repeating close is a no-op.
    #[napi]
    pub fn close(&mut self) {
        if let Ok(mut guard) = self.inner.lock() {
            *guard = None;
        }
    }
}

pub struct QueryTask {
    session: Arc<Mutex<Option<Connection>>>,
    sql: String,
    params: Option<Vec<Option<Either<String, Buffer>>>>,
}

impl Task for QueryTask {
    type Output = ClientResult;
    type JsValue = ClientResult;

    fn compute(&mut self) -> Result<Self::Output> {
        let mut guard = self.session.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let connection = guard.as_mut().ok_or_else(|| Error::from_reason("MySQL client session is closed"))?;
        let result = match &self.params {
            None => connection.query(&self.sql),
            Some(params) => {
                let bound: Vec<Param<'_>> = params
                    .iter()
                    .map(|param| match param {
                        None => Param::text(None),
                        Some(Either::A(text)) => Param::text(Some(text.as_str())),
                        Some(Either::B(bytes)) => Param::binary(&bytes[..]),
                    })
                    .collect();
                connection.query_with_params(&self.sql, &bound)
            }
        }
        .map_err(map_database_error)?;
        Ok(into_client_result(result))
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

fn into_client_result(result: QueryResult) -> ClientResult {
    let columns = result.column_info.iter().map(ClientColumn::from).collect::<Vec<_>>();
    let rows = result
        .rows
        .into_iter()
        .map(|row| row.into_iter().enumerate().map(|(index, cell)| restore_cell(cell, result.column_info.get(index))).collect())
        .collect();
    ClientResult {
        columns,
        rows,
        affected_rows: BigInt::from(result.affected_rows),
        last_insert_id: BigInt::from(result.insert_id),
    }
}

impl From<&ColumnInfo> for ClientColumn {
    fn from(column: &ColumnInfo) -> Self {
        Self {
            name: column.name.clone(),
            type_code: u32::from(column.type_code),
            unsigned: column.unsigned(),
            binary: column.binary(),
        }
    }
}

/// Restores one upstream cell against its declared column.
///
/// The upstream client widens integers, floats, dates and times into the text
/// the text protocol would show and keeps binary-collation columns as raw
/// octets. This turns integer text back into `BigInt` (exact for the full
/// signed and unsigned ranges), float text into `number`, keeps raw octets as
/// `Uint8Array` and everything else as text. Exact decimals stay strings so no
/// precision is lost in the JS number type; a value that does not parse as its
/// declared number shape falls back to text instead of guessing.
fn restore_cell(cell: Cell, column: Option<&ColumnInfo>) -> ClientValue {
    let Cell::Bytes(bytes) = cell
    else {
        return None;
    };
    let Some(column) = column
    else {
        return Some(Either4::C(String::from_utf8_lossy(&bytes).into_owned()));
    };
    if column.binary() {
        return Some(Either4::D(Buffer::from(bytes)));
    }
    match column.type_code {
        mysql_type::TINY | mysql_type::SHORT | mysql_type::LONG | mysql_type::INT24 | mysql_type::LONGLONG => {
            let text = String::from_utf8_lossy(&bytes);
            let widened = if column.unsigned() {
                u64::from_str(&text).ok().map(BigInt::from)
            }
            else {
                i64::from_str(&text).ok().map(BigInt::from)
            };
            widened.map(Either4::A).or(Some(Either4::C(text.into_owned())))
        }
        mysql_type::FLOAT | mysql_type::DOUBLE => {
            let text = String::from_utf8_lossy(&bytes);
            match f64::from_str(&text) {
                Ok(value) => Some(Either4::B(value)),
                Err(_) => Some(Either4::C(text.into_owned())),
            }
        }
        _ => Some(Either4::C(String::from_utf8_lossy(&bytes).into_owned())),
    }
}
