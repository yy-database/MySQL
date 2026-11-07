//! Node-API handles over the upstream MySQL gateway.
//!
//! Each handle is a thin conversion over `database.server::yyds::mysql::instance`:
//! no protocol bytes, no command dispatch and no listener implementation lives
//! here. The TypeScript host composes these primitives into a server command.

use std::sync::Arc;

use database::{
    cluster::control::NodeLease, execution::cluster::KeyValueExecutor, query::SqlSchemaRegistry,
    server::yyds::mysql::instance::MysqlInstance,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;

fn map_string(error: String) -> Error {
    Error::from_reason(error)
}

/// Publishes single-node shard routing into a `.yyds` catalog.
#[napi]
pub fn mysql_publish_routing(catalog_path: String, epoch: u32, shard_ids: Vec<String>) -> Result<()> {
    database::server::yyds::mysql::instance::publish_routing(&catalog_path, u64::from(epoch), &shard_ids).map_err(map_string)
}

/// Exclusive node lease for one MySQL data directory.
#[napi]
pub struct MysqlLease {
    inner: Arc<NodeLease>,
}

#[napi]
impl MysqlLease {
    /// Acquires the directory lease, creating the data directory when absent.
    #[napi(constructor)]
    pub fn acquire(data_dir: String, cluster_id: String, node_id: String) -> Result<Self> {
        let lease =
            database::server::yyds::mysql::instance::acquire_lease(&data_dir, &cluster_id, &node_id).map_err(map_string)?;
        Ok(Self { inner: Arc::new(lease) })
    }

    /// Data directory held by this lease.
    #[napi]
    pub fn data_dir(&self) -> String {
        self.inner.directory().display().to_string()
    }
}

/// Local KV executor over the catalog's `.yykv` shards.
#[napi]
pub struct MysqlExecutor {
    pub(crate) inner: Arc<dyn KeyValueExecutor>,
}

#[napi]
impl MysqlExecutor {
    /// Opens the executor from published `.yyds` routing.
    #[napi(factory)]
    pub fn open(data_dir: String, catalog_path: String) -> Result<Self> {
        let inner = database::server::yyds::mysql::instance::open_executor(&data_dir, &catalog_path).map_err(map_string)?;
        Ok(Self { inner })
    }
}

/// Persisted SQL schema registry shared by DDL and DML.
#[napi]
pub struct MysqlSchema {
    pub(crate) inner: Arc<SqlSchemaRegistry>,
}

#[napi]
impl MysqlSchema {
    /// Opens `<data-dir>/sql-schema.v0.json`, starting empty when absent.
    #[napi(factory)]
    pub fn open(path: String) -> Result<Self> {
        let inner = database::server::yyds::mysql::instance::open_schema(path).map_err(map_string)?;
        Ok(Self { inner })
    }

    /// Current schema revision, bumped by every applied DDL.
    #[napi]
    pub fn revision(&self) -> u32 {
        self.inner.revision()
    }
}

/// Loopback MySQL listener owned by the upstream `database-server` gateway.
#[napi]
pub struct MysqlService {
    inner: Option<MysqlInstance>,
}

#[napi]
impl MysqlService {
    /// Starts the listener on `127.0.0.1:<port>`; port `0` binds a free port.
    #[napi(factory)]
    pub fn start(lease: &MysqlLease, executor: &MysqlExecutor, schema: &MysqlSchema, port: u16) -> Result<Self> {
        let inner =
            MysqlInstance::start(Arc::clone(&lease.inner), Arc::clone(&executor.inner), Arc::clone(&schema.inner), port)
                .map_err(map_string)?;
        Ok(Self { inner: Some(inner) })
    }

    /// Bound address, including the assigned port when started with `0`.
    #[napi]
    pub fn address(&self) -> Result<String> {
        self.inner.as_ref().and_then(|server| server.address()).map(|address| address.to_string()).ok_or_else(missing)
    }

    /// Whether the accept worker has exited.
    #[napi]
    pub fn is_finished(&self) -> Result<bool> {
        Ok(self.inner.as_ref().is_some_and(|server| server.is_finished()))
    }

    /// Stops the listener and waits for its worker to exit.
    #[napi]
    pub fn stop(&mut self) -> Result<()> {
        let mut server = self.inner.take().ok_or_else(missing)?;
        server.stop().map_err(map_string)
    }
}

fn missing() -> Error {
    Error::from_reason("MySQL service is not running")
}
