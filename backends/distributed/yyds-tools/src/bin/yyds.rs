use clap::{Parser, Subcommand};
use yyds_types::DsValue;

#[derive(Parser)]
#[command(name = "yyds")]
#[command(about = "yyds ecosystem CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show version information
    Version,
    /// Run yyds server
    Server {
        #[arg(short, long, default_value = "127.0.0.1:8889")]
        addr: String,
    },
    /// Tenant management
    Tenant {
        #[command(subcommand)]
        subcommand: TenantCommands,
    },
    /// Cluster management
    Cluster {
        #[command(subcommand)]
        subcommand: ClusterCommands,
    },
    /// Import data from a file
    Import {
        /// Path to the file to import
        #[arg(short, long)]
        path: String,
        /// Format of the file (dsv, jsonl)
        #[arg(short, long)]
        format: String,
        /// Target table name
        #[arg(short, long)]
        table: String,
        /// Delimiter for DSV (default is ',')
        #[arg(short, long)]
        delimiter: Option<char>,
        /// Whether the DSV file has a header row
        #[arg(long)]
        has_header: bool,
        /// Optional schema ID for validation
        #[arg(long)]
        schema_id: Option<u32>,
        /// Optional path to a JSON schema file for validation
        #[arg(long)]
        schema_file: Option<String>,
        /// Path to save/load import state for resumable import
        #[arg(long)]
        state_file: Option<String>,
        /// YYDS server address
        #[arg(long, default_value = "127.0.0.1:8889")]
        addr: String,
        /// Tenant ID
        #[arg(long)]
        tenant_id: Option<String>,
        /// Secret key for authentication
        #[arg(long)]
        secret: Option<String>,
        /// Strategy for schema validation errors
        #[arg(long, value_enum, default_value_t = yyds_tools::import::ErrorStrategy::Skip)]
        schema_error_strategy: yyds_tools::import::ErrorStrategy,
        /// Strategy for insertion errors
        #[arg(long, value_enum, default_value_t = yyds_tools::import::RetryStrategy::Retry)]
        insertion_error_strategy: yyds_tools::import::RetryStrategy,
    },
}

#[derive(Subcommand)]
enum TenantCommands {
    /// Register a new tenant
    Register {
        #[arg(short, long)]
        name: String,
        #[arg(short, long, default_value = "truth")]
        sdr: String,
    },
    /// List all tenants
    List,
    /// Show tenant details
    Show {
        #[arg(short, long)]
        id: String,
    },
}

#[derive(Subcommand)]
enum ClusterCommands {
    /// Show cluster status
    Status,
    /// List cluster nodes
    Nodes,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Version => {
            println!("yyds version {}", yyds::version());
        }
        Commands::Server { addr } => {
            println!("Starting yyds server on {}...", addr);
            if let Err(e) = yyds_server::run().await {
                eprintln!("Server error: {:?}", e);
                std::process::exit(1);
            }
        }
        Commands::Tenant { subcommand } => match subcommand {
            TenantCommands::Register { name, sdr } => {
                println!("Registering tenant '{}' with SDR level '{}'...", name, sdr);
                // TODO: Call yyds_tenant::register
            }
            TenantCommands::List => {
                println!("Listing tenants...");
            }
            TenantCommands::Show { id } => {
                println!("Showing details for tenant {}...", id);
            }
        },
        Commands::Cluster { subcommand } => match subcommand {
            ClusterCommands::Status => {
                println!("Cluster Status: Healthy");
                println!("Nodes: 1 active");
            }
            ClusterCommands::Nodes => {
                println!("Node ID | Address | Role | Status");
                println!("1 | 127.0.0.1:8889 | Leader | Active");
            }
        },
        Commands::Import {
            path,
            format,
            table,
            delimiter,
            has_header,
            schema_id,
            schema_file,
            state_file,
            addr,
            tenant_id,
            secret,
            schema_error_strategy,
            insertion_error_strategy,
        } => {
            use yyds_loader::{DsvLoader, JsonlLoader, Loader};
            use yyds_tools::import::Importer;
            use std::path::PathBuf;
            use uuid::Uuid;
            use yyds_types::DsSchema;

            let loader: Box<dyn Loader> = match format.to_lowercase().as_str() {
                "dsv" | "csv" | "tsv" => {
                    let d = delimiter.map(|c| c as u8).unwrap_or(b',');
                    Box::new(DsvLoader::new(d, *has_header))
                }
                "jsonl" => Box::new(JsonlLoader),
                _ => {
                    eprintln!("Unsupported format: {}", format);
                    std::process::exit(1);
                }
            };

            // Connect to YYDS
            let tenant_uuid = tenant_id.as_ref()
                .and_then(|id| Uuid::parse_str(id).ok())
                .unwrap_or_else(Uuid::nil);
            
            let secret_bytes = secret.as_ref().map(|s| s.as_bytes()).unwrap_or(&[]);

            println!("Connecting to YYDS at {}...", addr);
            let mut client = yyds::Client::connect(addr, tenant_uuid, secret_bytes).await?;

            // Load schema
            let mut schema = None;
            if let Some(sf) = schema_file {
                let content = std::fs::read_to_string(sf)?;
                let s: DsSchema = serde_json::from_str(&content)?;
                schema = Some(s);
            } else if let Some(sid) = schema_id {
                println!("Fetching schema {} from server...", sid);
                // In YYDS, schema might be stored as a special key or via a specific API
                // Assuming it's stored under "schema:{id}" for now
                let key = format!("schema:{}", sid);
                match client.get(&key).await? {
                    Some(DsValue::Text(json)) => {
                        let s: DsSchema = serde_json::from_str(&json)?;
                        schema = Some(s);
                    }
                    Some(_) => {
                        eprintln!("Invalid schema format in storage for ID {}", sid);
                        std::process::exit(1);
                    }
                    None => {
                        eprintln!("Schema {} not found on server", sid);
                        std::process::exit(1);
                    }
                }
            }

            let mut importer = Importer::new(
                loader,
                schema,
                state_file.as_ref().map(PathBuf::from),
                client,
                table.clone(),
                schema_error_strategy.clone(),
                insertion_error_strategy.clone(),
            );

            if let Err(e) = importer.run(std::path::Path::new(path)).await {
                eprintln!("Import failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
