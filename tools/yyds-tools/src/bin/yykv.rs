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
            println!("Running benchmark...");
            // TODO: Call yykv_bench::run()
        }
        Commands::Inspect { path } => {
            println!("Inspecting storage file: {}", path);
            // TODO: Call yykv_storage::inspect(path)
        }
    }

    Ok(())
}
