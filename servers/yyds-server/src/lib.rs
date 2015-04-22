#![warn(missing_docs)]

use bytes::Bytes;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};
use uuid::Uuid;

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio_util::codec::Framed;
use we_trust::{MessageType, TrustCodec, TrustMessage};
use yyds_billing::{BillingManager, ResourceType};
use yyds_gateway::YydsGateway;
use yyds_quota::{Quota, QuotaManager};
use yyds_task::TaskManager;
use yyds_tenant::TenantManager;
use yyds_types::{DeviceConfig, DsError, DsResult, DsValue, MediumType, Redundancy, StorageConfig};
use yykv_compute::MockWasmRuntime;
use yykv_executor::YykvExecutor as YydsExecutor;
use yykv_index::SearchIndexManager;
use yykv_operators::OpsGraph;
use yykv_storage::BTree as YykvStorage;
use yykv_wal::{OpType, WalManager};

use std::time::Duration;
use tokio::time;

type Result<T> = std::result::Result<T, DsError>;

/// Global state for the YYDS Server.
pub struct ServerState {
    pub tenant_manager: Arc<RwLock<TenantManager>>,
    pub billing_manager: Arc<Mutex<BillingManager>>,
    pub quota_manager: Arc<RwLock<QuotaManager>>,
    pub task_manager: Arc<Mutex<TaskManager>>,
    pub wal: Arc<WalManager>,
    pub storage: Arc<YykvStorage>,
    pub executor: Arc<YydsExecutor>,
    pub index: Arc<SearchIndexManager>,
}

impl ServerState {
    pub async fn handle_insert(
        &self,
        tenant_id: Uuid,
        table: String,
        key: String,
        value: DsValue,
        redundancy: Redundancy,
    ) -> DsResult<()> {
        let tenant_internal_id = {
            let tm = self.tenant_manager.read().await;
            tm.get_internal_id(tenant_id)
                .ok_or(DsError::internal("Tenant not found"))?
        };

        // Calculate storage size (rough estimate)
        let replicas = redundancy.replicas() as u64;
        let value_bytes = yyds_types::layout::DsValueEncoder::encode(&value)?;
        let storage_size = (value_bytes.len() as u64) * replicas;

        // Quota & Billing
        {
            let qm = self.quota_manager.read().await;
            let mut bm = self.billing_manager.lock().await;
            let current_usage = bm.get_total_usage(tenant_internal_id, ResourceType::Storage);
            if !qm.check_storage(tenant_internal_id, current_usage + storage_size) {
                return Err(DsError::permission("storage_put", "quota_exceeded"));
            }
            bm.record_usage(tenant_internal_id, ResourceType::Storage, storage_size);
        }

        // Storage
        self.storage
            .put(tenant_internal_id, key.clone(), value.clone(), redundancy)
            .await
            .map_err(|e| DsError::storage_with_op("put", format!("Storage error: {:?}", e)))?;

        // WAL
        self.wal
            .write(tenant_id, table, key, OpType::Insert, redundancy, value)
            .await?;

        Ok(())
    }
}

use yyds_types::FromValue;
use yyds_types::layout::DsValueDecoder;

#[async_trait]
impl yyds_gateway::GatewayBackend for ServerState {
    async fn execute_query(&self, tenant_id: Uuid, graph: OpsGraph) -> DsResult<Vec<DsValue>> {
        self.executor.execute_local(graph, tenant_id).await
    }

    async fn execute_insert(
        &self,
        tenant_id: Uuid,
        table: String,
        key: DsValue,
        value: DsValue,
    ) -> DsResult<()> {
        let key_str = match key {
            DsValue::Text(s) => s,
            DsValue::Int(i) => i.to_string(),
            DsValue::Uuid(u) => u.to_string(),
            _ => {
                return Err(DsError::execution_with_op(
                    "insert",
                    "Unsupported key type for PG insert",
                ));
            }
        };
        self.handle_insert(tenant_id, table, key_str, value, Redundancy::TRUTH)
            .await
    }

    async fn execute_delete(
        &self,
        _tenant_id: Uuid,
        _table: String,
        _selection: Option<yykv_operators::Expr>,
    ) -> DsResult<()> {
        // Selection handling is complex, for now we just support simple delete or return error
        // In a real implementation, we would translate Expr to storage predicates
        Err(DsError::execution_with_op(
            "delete",
            "Complex DELETE via PG gateway not yet fully implemented",
        ))
    }
}

/// WAE Pipeline for processing events
pub struct WaePipeline {
    pub wal: Arc<WalManager>,
    pub task_manager: Arc<Mutex<TaskManager>>,
    pub tenant_manager: Arc<RwLock<TenantManager>>,
    pub index: Arc<SearchIndexManager>,
}

impl WaePipeline {
    pub fn new(
        wal: Arc<WalManager>,
        task_manager: Arc<Mutex<TaskManager>>,
        tenant_manager: Arc<RwLock<TenantManager>>,
        index: Arc<SearchIndexManager>,
    ) -> Self {
        Self {
            wal,
            task_manager,
            tenant_manager,
            index,
        }
    }

    pub async fn start(self: Arc<Self>) {
        let mut rx = self.wal.subscribe();
        info!("WAE Pipeline started");

        while let Ok(event) = rx.recv().await {
            info!(
                "WAE Pipeline: Processing event {} for tenant {}",
                event.id(),
                event.tenant_id()
            );

            // 1. Audit Task Creation
            if event.op_type() == OpType::Insert {
                let tm = self.task_manager.lock().await;
                let _ = tm
                    .submit(
                        event.tenant_id(),
                        format!("Audit:Insert:{}", event.table()),
                        event.value().cloned().unwrap_or(DsValue::Null),
                    )
                    .await;
            }

            // 2. Real-time Indexing
            if event.table().starts_with("search:") || event.table() == "sys:docs" {
                if let Some(yyds_types::DsValue::Text(content)) = event.value() {
                    let _ = self
                        .index
                        .index_document(event.id(), event.tenant_id(), content.clone())
                        .await;
                }
            }
        }
    }
}

const BILLING_FILE: &str = "persistence/billing_data.json";

pub async fn run() -> std::result::Result<(), DsError> {
    tracing_subscriber::fmt::init();

    // Ensure persistence directory exists
    std::fs::create_dir_all("persistence")
        .map_err(|e| DsError::io(format!("Failed to create persistence directory: {}", e)))?;

    let storage_config = StorageConfig {
        devices: vec![DeviceConfig {
            id: 1,
            path: "persistence/yyds_data.db".into(),
            medium: MediumType::NvmeSSD,
        }],
        page_size: 4096,
        wal_path: Some("persistence/yyds_wal.bin".into()),
    };

    let device_config = &storage_config.devices[0];
    // Ensure file exists for WindowsDirectIo (which uses OPEN_EXISTING for now)
    if !std::path::Path::new(&device_config.path).exists() {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&device_config.path)
            .map_err(|e| DsError::io(format!("Failed to create storage file: {}", e)))?;
        file.set_len(10 * 1024 * 1024) // Initial 10MB
            .map_err(|e| DsError::io(format!("Failed to set initial size: {}", e)))?;
    }

    let device = Arc::new(yykv_io::WindowsDirectIo::open(
        &device_config.path,
        device_config.medium,
    )?);
    let pager =
        Arc::new(yykv_storage::Pager::new(device, 1024).await.map_err(|e| {
            DsError::storage_with_op("init", format!("Failed to init pager: {:?}", e))
        })?);
    let storage = Arc::new(YykvStorage::open(pager).await.map_err(|e| {
        DsError::storage_with_op("init", format!("Failed to init storage: {:?}", e))
    })?);

    // Initialize WAL
    let wal = Arc::new(
        WalManager::new(std::path::PathBuf::from("persistence/yyds_wal.bin"))
            .await
            .map_err(|e| DsError::storage_with_op("wal_init", format!("{:?}", e)))?,
    );

    // Load billing data if exists
    let billing_manager = BillingManager::load(BILLING_FILE).unwrap_or_else(|e| {
        error!(
            "Failed to load billing data: {:?}. Starting with fresh state.",
            e
        );
        BillingManager::new()
    });

    // Initialize Search Index
    let index = Arc::new(SearchIndexManager::new_in_memory()?);

    // Initialize Compute Engine (Mock for now)
    let compute = Arc::new(MockWasmRuntime);

    // Initialize Executor
    let executor = Arc::new(YydsExecutor::new(Uuid::new_v4(), compute, Arc::clone(&wal)));

    let tenant_manager_arc = Arc::new(RwLock::new(TenantManager::new()));
    let task_manager_arc = Arc::new(Mutex::new(TaskManager::new(Arc::clone(&wal))));

    let state = Arc::new(ServerState {
        tenant_manager: Arc::clone(&tenant_manager_arc),
        billing_manager: Arc::new(Mutex::new(billing_manager)),
        quota_manager: Arc::new(RwLock::new(QuotaManager::new())),
        task_manager: Arc::clone(&task_manager_arc),
        wal: Arc::clone(&wal),
        storage,
        executor,
        index: Arc::clone(&index),
    });

    // Start WAE Pipeline
    let wae_pipeline = Arc::new(WaePipeline::new(
        Arc::clone(&wal),
        Arc::clone(&task_manager_arc),
        Arc::clone(&tenant_manager_arc),
        Arc::clone(&index),
    ));
    tokio::spawn(async move {
        wae_pipeline.start().await;
    });

    // Spawn billing persistence task
    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60)); // Save every minute
        loop {
            interval.tick().await;
            let bm = state_clone.billing_manager.lock().await;
            if let Err(e) = bm.save(BILLING_FILE) {
                error!("Failed to save billing data: {:?}", e);
            } else {
                info!("Billing data saved to {}", BILLING_FILE);
            }
        }
    });

    // Register a default test tenant
    {
        let mut tm = state.tenant_manager.write().await;
        let mut qm = state.quota_manager.write().await;

        let test_uuid = Uuid::nil(); // Using nil UUID for simple testing
        let test_secret = b"yyds-secret-key-2026".to_vec();

        tm.register_tenant(
            test_uuid,
            1,
            1,
            1,
            "System Admin".into(),
            Redundancy::TRUTH,
            test_secret,
        )
        .expect("Failed to register default tenant");

        let test_internal_id = tm.get_internal_id(test_uuid).unwrap();

        qm.set_quota(Quota {
            tenant_id: test_internal_id,
            max_storage: 10 * 1024 * 1024 * 1024, // 10GB
            cpu_burst_limit: 1000,
            max_memory: 2 * 1024 * 1024 * 1024, // 2GB
        });
    }

    let addr = "127.0.0.1:8889";
    let pg_gateway_addr = "127.0.0.1:5432";

    // Start Postgres Gateway
    let gateway = YydsGateway::new(Arc::clone(&state) as Arc<dyn yyds_gateway::GatewayBackend>);
    let pg_gateway_addr_clone = pg_gateway_addr.to_string();
    tokio::spawn(async move {
        info!("Starting Postgres Gateway on {}", pg_gateway_addr_clone);
        if let Err(e) = gateway.listen_postgres(&pg_gateway_addr_clone).await {
            error!("Postgres Gateway error: {:?}", e);
        }
    });

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| DsError::io(format!("Failed to bind to {}: {}", addr, e)))?;
    info!("YYDS Server started on {}", addr);

    loop {
        let (socket, _) = listener.accept().await?;
        let state = Arc::clone(&state);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, state).await {
                error!("Connection error: {:?}", e);
            }
        });
    }
}

async fn handle_connection(socket: TcpStream, state: Arc<ServerState>) -> Result<()> {
    let mut framed = Framed::new(socket, TrustCodec);

    // Simple loop to handle requests
    while let Some(msg) = framed.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to decode message: {:?}", e);
                break;
            }
        };

        let header = msg.header;
        let payload = msg.payload;

        info!(
            "Received request type {} from tenant UUID {}",
            header.msg_type, header.tenant_id
        );

        // 1. Authentication & Verification
        let tenant_secret = {
            let tm = state.tenant_manager.read().await;
            let tenant = match tm.get_by_uuid(&header.tenant_id) {
                Some(t) => t,
                None => {
                    error!("Tenant not found: {}", header.tenant_id);
                    // Send error response
                    let err_msg = TrustMessage::new(
                        MessageType::Error,
                        header.tenant_id,
                        Bytes::from("Tenant not found"),
                    );
                    let _ = framed.send(err_msg).await;
                    continue;
                }
            };

            if !tenant.active {
                error!("Tenant {} is inactive", header.tenant_id);
                let err_msg = TrustMessage::new(
                    MessageType::Error,
                    header.tenant_id,
                    Bytes::from("Tenant inactive"),
                );
                let _ = framed.send(err_msg).await;
                continue;
            }

            // Trust Protocol Signature Verification
            if !header.verify(&tenant.secret_key) {
                error!("Invalid signature for tenant {}", header.tenant_id);
                let err_msg = TrustMessage::new(
                    MessageType::Error,
                    header.tenant_id,
                    Bytes::from("Invalid signature"),
                );
                let _ = framed.send(err_msg).await;
                continue;
            }

            tenant.secret_key.clone()
        };

        let tenant_internal_id = {
            let tm = state.tenant_manager.read().await;
            tm.get_by_uuid(&header.tenant_id).unwrap().internal_id
        };

        // Handle Handshake/Auth explicitly if needed, but here it's implicit per message
        if header.msg_type == MessageType::Auth as u8 {
            let mut resp = TrustMessage::new(
                MessageType::Response,
                header.tenant_id,
                Bytes::from("auth-ok"),
            );
            resp.header.sign(&tenant_secret);
            framed.send(resp).await?;
            continue;
        }

        // 3. Payload Handling & YYKV Operations
        match header.msg_type {
            t if t == MessageType::Put as u8 => {
                // PUT
                use bytes::Buf;
                let mut p = payload;
                if p.len() < 4 {
                    let err_msg = TrustMessage::new(
                        MessageType::Error,
                        header.tenant_id,
                        Bytes::from("Invalid PUT payload"),
                    );
                    framed.send(err_msg).await?;
                    continue;
                }
                let key_len = p.get_u32() as usize;
                if p.len() < key_len {
                    let err_msg = TrustMessage::new(
                        MessageType::Error,
                        header.tenant_id,
                        Bytes::from("Invalid key length"),
                    );
                    framed.send(err_msg).await?;
                    continue;
                }
                let key = String::from_utf8(p.copy_to_bytes(key_len).to_vec())?;
                let value = yyds_types::layout::DsValueDecoder::decode(&mut p)
                    .map_err(|e| DsError::serialization(format!("Value decode error: {:?}", e)))?;

                state
                    .handle_insert(
                        header.tenant_id,
                        "default".into(),
                        key,
                        value,
                        header.sdr_level(),
                    )
                    .await?;

                let mut resp =
                    TrustMessage::new(MessageType::Response, header.tenant_id, Bytes::new());
                resp.header.sign(&tenant_secret);
                framed.send(resp).await?;
                info!("PUT successful for tenant {}", header.tenant_id);
            }
            t if t == MessageType::Get as u8 => {
                // GET
                use bytes::Buf;
                let mut p = payload;
                if p.len() < 4 {
                    let err_msg = TrustMessage::new(
                        MessageType::Error,
                        header.tenant_id,
                        Bytes::from("Invalid GET payload"),
                    );
                    framed.send(err_msg).await?;
                    continue;
                }
                let key_len = p.get_u32() as usize;
                if p.len() < key_len {
                    let err_msg = TrustMessage::new(
                        MessageType::Error,
                        header.tenant_id,
                        Bytes::from("Invalid key length"),
                    );
                    framed.send(err_msg).await?;
                    continue;
                }
                let key = String::from_utf8(p.copy_to_bytes(key_len).to_vec())?;

                let result = state
                    .storage
                    .get(tenant_internal_id.0, &key)
                    .await
                    .map_err(|e| {
                        DsError::storage_with_op("get", format!("YYKV Get error: {:?}", e))
                    })?;

                if let Some(val) = result {
                    let resp_data =
                        yyds_types::layout::DsValueEncoder::encode(&val).map_err(|e| {
                            DsError::serialization(format!("Value encode error: {:?}", e))
                        })?;

                    let mut resp =
                        TrustMessage::new(MessageType::Response, header.tenant_id, resp_data);
                    resp.header.sign(&tenant_secret);
                    framed.send(resp).await?;
                } else {
                    let mut resp = TrustMessage::new(
                        MessageType::Error,
                        header.tenant_id,
                        Bytes::from("Not found"),
                    );
                    resp.header.flags = 404;
                    resp.header.sign(&tenant_secret);
                    framed.send(resp).await?;
                }
            }
            t if t == MessageType::Delete as u8 => {
                // DELETE
                use bytes::Buf;
                let mut p = payload;
                if p.len() < 4 {
                    let err_msg = TrustMessage::new(
                        MessageType::Error,
                        header.tenant_id,
                        Bytes::from("Invalid DELETE payload"),
                    );
                    framed.send(err_msg).await?;
                    continue;
                }
                let key_len = p.get_u32() as usize;
                let key = String::from_utf8(p.copy_to_bytes(key_len).to_vec())?;

                state
                    .storage
                    .delete(tenant_internal_id.0, key.clone())
                    .await
                    .map_err(|e| {
                        DsError::storage_with_op("delete", format!("YYKV Delete error: {:?}", e))
                    })?;

                // WAE for Delete
                state
                    .wal
                    .delete(header.tenant_id, "default".into(), key)
                    .await?;

                let mut resp =
                    TrustMessage::new(MessageType::Response, header.tenant_id, Bytes::new());
                resp.header.sign(&tenant_secret);
                framed.send(resp).await?;
            }
            t if t == MessageType::Query as u8 => {
                // QUERY
                let mut p = payload;
                let graph_value = DsValueDecoder::decode(&mut p).map_err(|e| {
                    DsError::serialization(format!("OpsGraph decode error: {:?}", e))
                })?;
                let graph = OpsGraph::from_value(graph_value).map_err(|e| {
                    DsError::serialization(format!("OpsGraph parse error: {:?}", e))
                })?;

                info!(
                    "Executing query graph {} for tenant {}",
                    graph.id, header.tenant_id
                );

                let results = state
                    .executor
                    .execute_local(graph, header.tenant_id)
                    .await
                    .map_err(|e| {
                        DsError::execution_with_op("query", format!("Executor error: {:?}", e))
                    })?;

                let result_value = yyds_types::DsValue::List(results);
                let resp_data = yyds_types::layout::DsValueEncoder::encode(&result_value)
                    .map_err(|e| DsError::serialization(format!("Value encode error: {:?}", e)))?;

                let mut resp =
                    TrustMessage::new(MessageType::QueryResp, header.tenant_id, resp_data);
                resp.header.sign(&tenant_secret);
                framed.send(resp).await?;
                info!("QUERY successful for tenant {}", header.tenant_id);
            }
            t if t == MessageType::Heartbeat as u8 => {
                // Heartbeat
                let mut resp =
                    TrustMessage::new(MessageType::Response, header.tenant_id, Bytes::from("pong"));
                resp.header.sign(&tenant_secret);
                framed.send(resp).await?;
            }
            _ => {
                error!("Unsupported message type: {}", header.msg_type);
                let err_msg = TrustMessage::new(
                    MessageType::Error,
                    header.tenant_id,
                    Bytes::from("Unsupported message type"),
                );
                framed.send(err_msg).await?;
            }
        }
    }
    Ok(())
}
