use clap::{Parser, Subcommand};

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
        /// Delimiter for DSV (default is ',')
        #[arg(short, long)]
        delimiter: Option<char>,
        /// Whether the DSV file has a header row
        #[arg(long)]
        has_header: bool,
        /// Optional schema ID for validation
        #[arg(long)]
        schema_id: Option<u32>,
        /// Path to save/load import state for resumable import
        #[arg(long)]
        state_file: Option<String>,
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
    }

    Ok(())
}
