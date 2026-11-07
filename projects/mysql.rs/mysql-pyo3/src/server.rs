//! PyO3 handles over the upstream MySQL gateway.
//!
//! Each handle is a thin conversion over `mysql_core::server`: no protocol
//! bytes, no command dispatch and no listener implementation lives here. The
//! Python host composes these primitives into a server command.
//!
//! Reuse rule: before adding any command or frame semantics here, search
//! `database.server::yyds::mysql`. If it exists upstream, wire it instead of
//! writing a second implementation.

use std::sync::Arc;

use pyo3::{exceptions::PyRuntimeError, prelude::*};

fn map_error(error: String) -> PyErr {
    PyRuntimeError::new_err(error)
}

/// Publishes single-node shard routing into a `.yyds` catalog.
#[pyfunction]
pub fn publish_routing(catalog_path: &str, epoch: u64, shard_ids: Vec<String>) -> PyResult<()> {
    mysql_core::server::publish_routing(catalog_path, epoch, &shard_ids).map_err(map_error)
}

/// Exclusive node lease for one MySQL data directory.
#[pyclass(module = "mysql.mysql_pyo3")]
pub struct MysqlLease {
    pub(crate) inner: Arc<mysql_core::server::NodeLease>,
}

#[pymethods]
impl MysqlLease {
    /// Acquires the directory lease, creating the data directory when absent.
    #[new]
    fn acquire(data_dir: String, cluster_id: String, node_id: String) -> PyResult<Self> {
        let lease = mysql_core::server::acquire_lease(&data_dir, &cluster_id, &node_id).map_err(map_error)?;
        Ok(Self { inner: Arc::new(lease) })
    }

    /// Data directory held by this lease.
    fn data_dir(&self) -> String {
        self.inner.directory().display().to_string()
    }
}

/// Local KV executor over the catalog's `.yykv` shards.
#[pyclass(module = "mysql.mysql_pyo3")]
pub struct MysqlExecutor {
    pub(crate) inner: Arc<dyn mysql_core::server::KeyValueExecutor>,
}

#[pymethods]
impl MysqlExecutor {
    /// Opens the executor from published `.yyds` routing.
    #[staticmethod]
    fn open(data_dir: String, catalog_path: String) -> PyResult<Self> {
        let inner = mysql_core::server::open_executor(&data_dir, &catalog_path).map_err(map_error)?;
        Ok(Self { inner })
    }
}

/// Persisted SQL schema registry shared by DDL and DML.
#[pyclass(module = "mysql.mysql_pyo3")]
pub struct MysqlSchema {
    pub(crate) inner: Arc<mysql_core::server::SqlSchemaRegistry>,
}

#[pymethods]
impl MysqlSchema {
    /// Opens `<data-dir>/sql-schema.v0.json`, starting empty when absent.
    #[staticmethod]
    fn open(path: String) -> PyResult<Self> {
        let inner = mysql_core::server::open_schema(path).map_err(map_error)?;
        Ok(Self { inner })
    }

    /// Current schema revision, bumped by every applied DDL.
    fn revision(&self) -> u64 {
        self.inner.revision() as u64
    }
}

/// Loopback MySQL listener owned by the upstream `database-server` gateway.
#[pyclass(module = "mysql.mysql_pyo3")]
pub struct MysqlService {
    inner: Option<mysql_core::server::MysqlServer>,
}

#[pymethods]
impl MysqlService {
    /// Starts the listener on `127.0.0.1:<port>`; port `0` binds a free port.
    #[staticmethod]
    fn start(
        lease: PyRef<'_, MysqlLease>,
        executor: PyRef<'_, MysqlExecutor>,
        schema: PyRef<'_, MysqlSchema>,
        port: u16,
    ) -> PyResult<Self> {
        let inner = mysql_core::server::MysqlServer::start(
            Arc::clone(&lease.inner),
            Arc::clone(&executor.inner),
            Arc::clone(&schema.inner),
            port,
        )
        .map_err(map_error)?;
        Ok(Self { inner: Some(inner) })
    }

    /// Bound address, including the assigned port when started with `0`.
    fn address(&self) -> PyResult<String> {
        self.inner.as_ref().and_then(|server| server.address()).map(|address| address.to_string()).ok_or_else(missing)
    }

    /// Whether the accept worker has exited.
    fn is_finished(&self) -> bool {
        self.inner.as_ref().is_none_or(|server| server.is_finished())
    }

    /// Stops the listener and waits for its worker to exit.
    fn stop(&mut self) -> PyResult<()> {
        let mut server = self.inner.take().ok_or_else(missing)?;
        server.stop().map_err(map_error)
    }
}

fn missing() -> PyErr {
    PyRuntimeError::new_err("MySQL service is not running")
}

/// Registers the server handles on the `mysql.mysql_pyo3` module.
pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(publish_routing, module)?)?;
    module.add_class::<MysqlLease>()?;
    module.add_class::<MysqlExecutor>()?;
    module.add_class::<MysqlSchema>()?;
    module.add_class::<MysqlService>()?;
    module.add("LOCAL_SHARD_ID", mysql_core::server::LOCAL_SHARD_ID)?;
    module.add("CATALOG_FILE", mysql_core::server::CATALOG_FILE)?;
    module.add("SCHEMA_FILE", mysql_core::server::SCHEMA_FILE)?;
    Ok(())
}
