mod client;
mod config;
mod operations;

use client::FileServerClient;
use config::{ClientConfig, ServerSettings, ClientSettings};
use operations::FileOperations;
use clap::{Parser, Subcommand};
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "fileserver-client")]
#[command(about = "A gRPC fileserver client with flexible configuration")]
struct Args {
    /// Path to configuration file (optional if --server and --port are provided)
    #[arg(short, long)]
    config: Option<String>,

    /// Server host address (overrides config file)
    #[arg(short, long)]
    server: Option<String>,

    /// Server port (overrides config file)  
    #[arg(short, long)]
    port: Option<u16>,

    /// Connection timeout in seconds
    #[arg(long, default_value = "30")]
    timeout: u64,

    /// Number of retry attempts
    #[arg(long, default_value = "3")]
    retries: u32,
    
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
    Delete { path: String },
}

fn create_config_from_args(args: &Args) -> Result<ClientConfig, Box<dyn std::error::Error>> {
    // If both server and port are provided via CLI, create config from CLI args
    if let (Some(server), Some(port)) = (&args.server, &args.port) {
        info!("Using server configuration from command line: {}:{}", server, port);
        
        let config = ClientConfig {
            server: ServerSettings {
                host: server.clone(),
                port: *port,
            },
            client: ClientSettings {
                timeout_seconds: args.timeout,
                retry_attempts: args.retries,
            },
        };
        
        config.validate()?;
        return Ok(config);
    }
    
    // If config file is specified, try to load it
    if let Some(config_path) = &args.config {
        info!("Loading configuration from file: {}", config_path);
        let mut config = ClientConfig::load_from_file(config_path)?;
        
        // Override with CLI parameters if provided
        if let Some(server) = &args.server {
            config.server.host = server.clone();
        }
        if let Some(port) = &args.port {
            config.server.port = *port;
        }
        
        // Always override timeout and retries if specified
        config.client.timeout_seconds = args.timeout;
        config.client.retry_attempts = args.retries;
        
        config.validate()?;
        return Ok(config);
    }
    
    // Try default config file if it exists
    let default_config = "config.toml";
    if std::fs::metadata(default_config).is_ok() {
        info!("Using default configuration file: {}", default_config);
        let mut config = ClientConfig::load_from_file(default_config)?;
        
        // Override with CLI parameters if provided
        if let Some(server) = &args.server {
            config.server.host = server.clone();
        }
        if let Some(port) = &args.port {
            config.server.port = *port;
        }
        
        config.client.timeout_seconds = args.timeout;
        config.client.retry_attempts = args.retries;
        
        config.validate()?;
        return Ok(config);
    }
    
    // Error: neither CLI args nor config file available
    return Err("Either provide --server and --port, or specify a config file with --config".into());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    
    let config = create_config_from_args(&args)?;
    let client_id = format!("client-{}", uuid::Uuid::now_v7());
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
        Commands::Delete { path } => {
            operations.delete(&path).await?;
            Ok(())
        }
    };

    if let Err(e) = result {
        error!("Operation failed: {}", e);
        std::process::exit(1);
    }

    Ok(())
}