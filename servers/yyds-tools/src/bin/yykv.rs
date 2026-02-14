use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "yykv")]
#[command(about = "YYKV storage engine CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show version information
    Version,
    /// Run benchmark
    Bench,
    /// Run yykv server
    Server,
    /// Performance profiling and analysis
    Profile {
        #[command(subcommand)]
        command: yyds_tools::profile::ProfileCommands,
    },
    /// Inspect storage file
    Inspect {
        #[arg(short, long)]
        path: String,
    },
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Version => {
            println!("yykv version 0.1.0");
        }
        Commands::Bench => {
            yyds_tools::bench::run().await?;
        }
        Commands::Server => {
            yykv_server::run().await?;
        }
        Commands::Profile { command } => {
            yyds_tools::profile::run(command.clone())
                .await
                .map_err(|e| e.to_string())?;
        }
        Commands::Inspect { path } => {
            println!("Inspecting storage file: {}", path);
            // TODO: Call yykv_storage::inspect(path)
        }
    }

    Ok(())
}
