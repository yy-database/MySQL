//! Node-directory composition and the upstream MySQL listener.
//!
//! Every handshake byte, command dispatch, `OK`/`ERR`/`EOF` frame and SQL
//! runtime decision lives upstream in `database.server::yyds::mysql`. This
//! module only opens the `.yyds` catalog, the `.yykv` shard, the persisted SQL
//! schema registry and the node lease, then hands them to `MysqlService`.
//!
//! Reuse rule: before adding any command or frame semantics here, search
//! `database.server::yyds::mysql`. If it exists upstream, wire it instead of
//! writing a second implementation.

use std::{
    collections::HashMap,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

pub use database::{
    cluster::control::NodeLease, execution::cluster::KeyValueExecutor, query::SqlSchemaRegistry,
    server::yyds::mysql::service::MysqlService,
};
use database::{
    cluster::{catalog::Catalog, control::NodeIdentity},
    execution::cluster::LocalExecutor,
    storage::cluster_kv::FileShard,
    types::cluster::{ShardEpoch, ShardId},
};

/// Single-node shard id used by a MySQL data directory.
pub const LOCAL_SHARD_ID: &str = "local-0";
/// `.yyds` catalog file name inside a data directory.
pub const CATALOG_FILE: &str = "catalog.yyds";
/// Persisted SQL schema registry file name inside a data directory.
pub const SCHEMA_FILE: &str = "sql-schema.v0.json";

fn map_error(context: &str, error: impl std::fmt::Display) -> String {
    format!("{context}: {error}")
}

/// Publishes the single-node shard routing into a `.yyds` catalog.
///
/// Re-running with the same epoch and shard list is a no-op so a restart can
/// boot the same data directory without republishing.
pub fn publish_routing(catalog_path: impl AsRef<Path>, epoch: u64, shards: &[String]) -> Result<(), String> {
    let mut catalog = Catalog::open(catalog_path).map_err(|error| map_error("open catalog", error))?;
    let wanted: Vec<ShardId> = shards.iter().cloned().map(ShardId).collect();
    if let Some(existing) = catalog.routing().map_err(|error| map_error("read catalog routing", error))? {
        if existing.epoch().0 == epoch && existing.shards() == wanted.as_slice() {
            return Ok(());
        }
    }
    catalog.publish_routing(ShardEpoch(epoch), wanted).map_err(|error| map_error("publish catalog routing", error))?;
    catalog.flush().map_err(|error| map_error("flush catalog", error))
}

/// Acquires the exclusive node lease for one data directory.
pub fn acquire_lease(data_dir: impl AsRef<Path>, cluster_id: &str, node_id: &str) -> Result<NodeLease, String> {
    let identity =
        NodeIdentity::new(cluster_id.to_string(), node_id.to_string()).map_err(|error| map_error("node identity", error))?;
    NodeLease::acquire(data_dir, identity).map_err(|error| map_error("acquire node lease", error))
}

/// Opens the local KV executor backed by the catalog's `.yykv` shards.
///
/// The catalog must already carry published routing. There is no silent
/// `local-0` fallback: a missing or unpublished catalog is an error.
pub fn open_executor(data_dir: impl AsRef<Path>, catalog_path: impl AsRef<Path>) -> Result<Arc<dyn KeyValueExecutor>, String> {
    let directory = data_dir.as_ref();
    let catalog = Catalog::open(catalog_path.as_ref()).map_err(|error| map_error("open catalog", error))?;
    let routing = catalog
        .routing()
        .map_err(|error| map_error("read catalog routing", error))?
        .ok_or_else(|| String::from("catalog has no published routing; publish routing first"))?;
    let mut shards = HashMap::new();
    for shard_id in routing.shards() {
        let path = directory.join(format!("{}.yykv", shard_id.0));
        let shard = FileShard::open(&path).map_err(|error| map_error("open .yykv shard", error))?;
        shards.insert(shard_id.clone(), shard);
    }
    let executor = LocalExecutor::new(routing, shards, Vec::new()).map_err(|error| map_error("build local executor", error))?;
    Ok(Arc::new(executor))
}

/// Opens the persisted SQL schema registry, creating empty state when absent.
pub fn open_schema(path: impl Into<PathBuf>) -> Result<Arc<SqlSchemaRegistry>, String> {
    SqlSchemaRegistry::open(path).map(Arc::new).map_err(|error| format!("open SQL schema: {}", error.message))
}

/// Loopback MySQL listener handed to the upstream gateway.
#[derive(Debug)]
pub struct MysqlServer {
    inner: Option<MysqlService>,
}

impl MysqlServer {
    /// Starts the upstream listener on `127.0.0.1:<port>`.
    ///
    /// Passing port `0` lets the operating system pick a free port; read the
    /// bound address back through [`MysqlServer::address`].
    pub fn start(
        lease: Arc<NodeLease>,
        executor: Arc<dyn KeyValueExecutor>,
        schema: Arc<SqlSchemaRegistry>,
        port: u16,
    ) -> Result<Self, String> {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        let service = MysqlService::start_with_registry(lease, address, executor, schema)
            .map_err(|error| map_error("start MySQL service", error))?;
        Ok(Self { inner: Some(service) })
    }

    /// Bound address, including the assigned port when started with `0`.
    pub fn address(&self) -> Option<SocketAddr> {
        self.inner.as_ref().map(MysqlService::address)
    }

    /// Whether the accept worker has exited.
    pub fn is_finished(&self) -> bool {
        self.inner.as_ref().is_none_or(MysqlService::is_finished)
    }

    /// Stops the listener and waits for its worker to exit.
    pub fn stop(&mut self) -> Result<(), String> {
        let service = self.inner.take().ok_or_else(|| String::from("MySQL service is not running"))?;
        service.stop().map_err(|error| map_error("stop MySQL service", error))
    }
}
