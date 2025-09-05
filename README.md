# Rust gRPC Fileserver

A secure, configurable file server built with Rust and gRPC that provides remote access to specific directories with granular permissions.

## Features

- **Directory-based access control**: Configure which directories clients can access
- **Permission management**: Set read-only or read-write permissions per directory
- **IP whitelisting**: Control which IP addresses can connect to the server
- **Streaming operations**: Efficient handling of large files through streaming
- **Path validation**: Prevents directory traversal attacks
- **gRPC protocol**: Modern, efficient communication protocol

## Quick Start

### Build the project
```bash
cargo build --workspace
```

### Start the server
```bash
cd server
cargo run
```

### Use the client
```bash
cd client

# Health check
cargo run -- health-check

# List directory contents
cargo run -- list documents

# Read a file
cargo run -- read-text documents/sample.txt

# Write to workspace (read-write directory)
cargo run -- write workspace/test.txt "Hello, World!"
```

## Configuration

See `CLAUDE.md` for detailed configuration instructions and architecture documentation.

## License

GPL