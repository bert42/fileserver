mod auth;
mod config;
mod file_handler;
mod privilege;
mod service;

use auth::AuthService;
use config::ServerConfig;
use privilege::PrivilegeManager;
use service::FileServiceImpl;
use common::file_service_server::FileServiceServer;
use clap::Parser;
use std::net::SocketAddr;
use tonic::transport::Server;
use tracing::info;

#[derive(Parser)]
#[command(name = "fileserver-server")]
#[command(about = "A gRPC fileserver with secure remote file access")]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "/etc/fileserver.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    
    info!("Loading configuration from: {}", args.config);
    let config = ServerConfig::load_from_file(&args.config)?;
    
    // Handle privilege dropping if user/group specified
    let privilege_manager = PrivilegeManager::new();
    privilege_manager.validate_user_group(
        config.server.user.as_deref(),
        config.server.group.as_deref()
    )?;
    
    privilege_manager.drop_privileges(
        config.server.user.as_deref(),
        config.server.group.as_deref()
    )?;
    
    let addr: SocketAddr = format!("0.0.0.0:{}", config.server.port).parse()?;
    info!("Starting fileserver on {}", addr);
    
    let auth_service = AuthService::new(config.clone());
    let file_service = FileServiceImpl::new(auth_service);
    
    info!("Configured directories:");
    for dir in &config.directories {
        info!("  - {}: {} ({})", dir.name, dir.path, dir.permissions);
    }
    
    info!("Allowed IPs: {:?}", config.server.allowed_ips);

    Server::builder()
        .add_service(FileServiceServer::new(file_service))
        .serve(addr)
        .await?;

    Ok(())
}