# CLAUDE.md - Rust gRPC Fileserver Project

## Project Overview

This is a Rust-based client-server fileserver application that provides secure, configurable remote file access using the gRPC protocol. The system allows controlled access to specific directories on the server with granular permission settings.

## Architecture

### Technology Stack
- **Language**: Rust
- **Communication Protocol**: gRPC (using `tonic` and `prost`)
- **Configuration**: TOML files for both server and client
- **Async Runtime**: Tokio

### Components
1. **Server**: Hosts file access services with configurable directory mappings and permissions
2. **Client**: Connects to the server and performs file operations
3. **Protocol Definitions**: Protobuf files defining the gRPC service interface

## Server Configuration

The server uses a `config.toml` file with the following structure:

```toml
# Server Configuration
[server]
port = 50051
allowed_ips = ["127.0.0.1", "192.168.1.100", "10.0.0.0/24"]

# Directory Sections
[[directories]]
name = "documents"
path = "/home/user/documents"
permissions = "read-only"

[[directories]]
name = "workspace"
path = "/home/user/workspace"
permissions = "read-write"

[[directories]]
name = "shared"
path = "/var/shared"
permissions = "read-only"
```

### Configuration Fields
- `port`: The port on which the server listens for gRPC connections
- `allowed_ips`: List of IP addresses or CIDR ranges allowed to connect
- `directories`: Array of directory configurations
  - `name`: Logical name for the directory (used by clients)
  - `path`: Actual filesystem path on the server
  - `permissions`: Either "read-only" or "read-write"

## Client Configuration

The client uses its own `config.toml`:

```toml
[server]
host = "192.168.1.10"
port = 50051

[client]
timeout_seconds = 30
retry_attempts = 3
```

## Client Operations

### Core Operations

#### 1. Connect
Establishes a connection to the server and performs authentication/authorization checks.
```rust
// Usage example
client.connect().await?;
```

#### 2. Health Check
Verifies the server is responsive and the connection is healthy.
```rust
// Returns server status and uptime
let status = client.healthcheck().await?;
```

#### 3. Stat
Retrieves metadata about a file or directory.
```rust
// Get file information
let metadata = client.stat("documents/report.pdf").await?;
// Returns: size, permissions, modified time, is_directory, etc.
```

#### 4. List
Lists contents of a directory.
```rust
// List directory contents
let entries = client.list("workspace/projects").await?;
// Returns: Vec<FileEntry> with names, types, sizes
```

#### 5. Read
Reads file contents from the server.
```rust
// Read entire file
let content = client.read("documents/data.txt").await?;

// Stream large files
let mut stream = client.read_stream("documents/large.bin").await?;
```

#### 6. Write
Writes data to a file (only for directories with "read-write" permissions).
```rust
// Write data to file
client.write("workspace/output.txt", data).await?;

// Stream write for large files
let mut stream = client.write_stream("workspace/large.dat").await?;
```

## Protocol Definition (proto/fileserver.proto)

```protobuf
syntax = "proto3";

package fileserver;

service FileService {
    rpc Connect(ConnectRequest) returns (ConnectResponse);
    rpc HealthCheck(Empty) returns (HealthStatus);
    rpc Stat(StatRequest) returns (FileMetadata);
    rpc List(ListRequest) returns (ListResponse);
    rpc Read(ReadRequest) returns (stream DataChunk);
    rpc Write(stream DataChunk) returns (WriteResponse);
}

message ConnectRequest {
    string client_id = 1;
}

message FileMetadata {
    string name = 1;
    uint64 size = 2;
    bool is_directory = 3;
    string permissions = 4;
    int64 modified_time = 5;
}

// Additional message definitions...
```

## Project Structure

```
fileserver/
├── Cargo.toml
├── README.md
├── CLAUDE.md
├── proto/
│   └── fileserver.proto
├── build.rs
├── server/
│   ├── Cargo.toml
│   ├── config.toml
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── service.rs
│       ├── auth.rs
│       └── file_handler.rs
├── client/
│   ├── Cargo.toml
│   ├── config.toml
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── client.rs
│       └── operations.rs
└── common/
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        └── generated/
            └── fileserver.rs (generated from proto)
```

## Security Considerations

1. **IP Whitelisting**: Only configured IP addresses can connect to the server
2. **Permission System**: Directories have explicit read-only or read-write permissions
3. **Path Validation**: All file paths are validated to prevent directory traversal attacks
4. **TLS Support**: Consider adding TLS encryption for production deployments
5. **Authentication**: Consider implementing token-based authentication for enhanced security

## Development Guidelines

### Building the Project

```bash
# Build all components
cargo build --workspace

# Build with optimizations
cargo build --release

# Run tests
cargo test --workspace
```

### Running the Server

```bash
cd server
cargo run -- --config config.toml
```

### Running the Client

```bash
cd client
cargo run -- --config config.toml <operation> [args]
```

### Example Client Commands

```bash
# Connect and check health
cargo run -- healthcheck

# List directory contents
cargo run -- list workspace/projects

# Get file information
cargo run -- stat documents/report.pdf

# Read a file
cargo run -- read documents/data.txt

# Write to a file
cargo run -- write workspace/output.txt "Hello, World!"
```

## Dependencies

Key Cargo dependencies:

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
tonic = "0.11"
prost = "0.12"
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

[build-dependencies]
tonic-build = "0.11"
```

## Error Handling

The system uses Result types with custom error enums:

```rust
#[derive(Debug, thiserror::Error)]
pub enum FileServerError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}
```

## Performance Considerations

1. **Streaming**: Large files are streamed in chunks to avoid memory issues
2. **Connection Pooling**: Client maintains a connection pool for efficiency
3. **Async I/O**: All operations are async using Tokio
4. **Buffer Management**: Configurable buffer sizes for file operations

## Future Enhancements

- [ ] Add compression support for data transfer
- [ ] Implement file watching/notification system
- [ ] Add support for symbolic links
- [ ] Implement quota management
- [ ] Add metrics and monitoring endpoints
- [ ] Support for partial file reads/writes
- [ ] Implement caching layer
- [ ] Add support for file versioning
- [ ] Implement audit logging

## Contributing

When contributing to this project:

1. Follow Rust coding conventions and idioms
2. Add tests for new functionality
3. Update this documentation for significant changes
4. Ensure all tests pass before submitting PR
5. Use `cargo fmt` and `cargo clippy` before commits

## License

GPL

---

*This CLAUDE.md file serves as the primary technical documentation for the Rust gRPC Fileserver project. It should be kept up-to-date as the project evolves.*
