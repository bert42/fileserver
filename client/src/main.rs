mod client;
mod config;
mod operations;

use client::FileServerClient;
use config::ClientConfig;
use operations::FileOperations;
use clap::{Parser, Subcommand};
use tracing::error;

#[derive(Parser)]
#[command(name = "fileserver-client")]
#[command(about = "A gRPC fileserver client")]
struct Args {
    #[arg(short, long, default_value = "config.toml")]
    config: String,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Connect,
    HealthCheck,
    Stat { path: String },
    List { path: String },
    Read { path: String },
    ReadText { path: String },
    Write { path: String, content: String },
    WriteFile { path: String, file: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    
    let config = ClientConfig::load_from_file(&args.config)?;
    let client_id = format!("client-{}", uuid::Uuid::new_v4());
    let client = FileServerClient::new(config, client_id).await?;
    let mut operations = FileOperations::new(client);

    let result: Result<(), Box<dyn std::error::Error>> = match args.command {
        Commands::Connect => {
            operations.connect().await?;
            Ok(())
        }
        Commands::HealthCheck => {
            operations.health_check().await?;
            Ok(())
        }
        Commands::Stat { path } => {
            operations.stat(&path).await?;
            Ok(())
        }
        Commands::List { path } => {
            operations.list(&path).await?;
            Ok(())
        }
        Commands::Read { path } => {
            let data = operations.read(&path).await?;
            println!("Read {} bytes", data.len());
            Ok(())
        }
        Commands::ReadText { path } => {
            operations.read_text(&path).await?;
            Ok(())
        }
        Commands::Write { path, content } => {
            operations.write(&path, &content).await?;
            Ok(())
        }
        Commands::WriteFile { path, file } => {
            operations.write_file(&path, &file).await?;
            Ok(())
        }
    };

    if let Err(e) = result {
        error!("Operation failed: {}", e);
        std::process::exit(1);
    }

    Ok(())
}