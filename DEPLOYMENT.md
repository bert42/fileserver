# Deployment Guide - Rust gRPC Fileserver

## Overview

This guide covers deploying the Rust gRPC fileserver in a production environment with proper security configurations, privilege dropping, and systemd integration.

## Prerequisites

- Linux system with systemd (for native deployment)
- Docker and Docker Compose (for container deployment)
- Rust toolchain (for building from source)
- Root access for initial setup (native deployment)

## Docker Deployment (Recommended)

### Docker Configuration

Docker deployments often use numeric UID/GID values for security and compatibility. The fileserver supports both username/group names and numeric IDs.

#### Example Dockerfile

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --workspace

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create fileserver user with specific UID/GID
RUN groupadd -r fileserver -g 1000 && \
    useradd -r -g fileserver -u 1000 fileserver

COPY --from=builder /app/target/release/fileserver-server /usr/local/bin/
COPY server/config.prod.toml /etc/fileserver.toml

# Create data directories
RUN mkdir -p /srv/fileserver/{documents,uploads,shared,workspace} && \
    chown -R 1000:1000 /srv/fileserver

EXPOSE 50051
USER 1000:1000

CMD ["/usr/local/bin/fileserver-server"]
```

#### Docker Compose Example

```yaml
version: '3.8'

services:
  fileserver:
    build: .
    container_name: fileserver
    restart: unless-stopped
    ports:
      - "50051:50051"
    volumes:
      # Configuration
      - ./config/fileserver.toml:/etc/fileserver.toml:ro
      # Data directories
      - ./data/documents:/srv/fileserver/documents:ro
      - ./data/uploads:/srv/fileserver/uploads:rw
      - ./data/shared:/srv/fileserver/shared:ro
      - ./data/workspace:/srv/fileserver/workspace:rw
    environment:
      - RUST_LOG=info
      - RUST_BACKTRACE=1
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    cap_add:
      - SETUID
      - SETGID
    user: "1000:1000"
    networks:
      - fileserver-net

networks:
  fileserver-net:
    driver: bridge
```

#### Configuration for Docker

When running in Docker, you can use numeric UIDs/GIDs in your configuration:

```toml
# config/fileserver.toml
[server]
port = 50051
allowed_ips = ["127.0.0.1", "10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"]

# Use numeric IDs for Docker compatibility
user = "1000"    # Numeric UID
group = "1000"   # Numeric GID

[[directories]]
name = "documents"
path = "/srv/fileserver/documents"
permissions = "read-only"

[[directories]]
name = "uploads"
path = "/srv/fileserver/uploads"
permissions = "read-write"

[[directories]]
name = "shared"
path = "/srv/fileserver/shared"
permissions = "read-only"

[[directories]]
name = "workspace"
path = "/srv/fileserver/workspace"
permissions = "read-write"
```

#### Running with Docker

```bash
# Build and run with Docker Compose
docker-compose up -d

# Or run directly with Docker
docker build -t fileserver .
docker run -d \
  --name fileserver \
  --restart unless-stopped \
  -p 50051:50051 \
  -v $(pwd)/config/fileserver.toml:/etc/fileserver.toml:ro \
  -v $(pwd)/data:/srv/fileserver:rw \
  --user 1000:1000 \
  --cap-drop ALL \
  --cap-add SETUID \
  --cap-add SETGID \
  --security-opt no-new-privileges:true \
  fileserver
```

### Docker Security Features

- **Non-root execution**: Runs as user 1000:1000
- **Minimal capabilities**: Only SETUID/SETGID for privilege dropping
- **No new privileges**: Prevents privilege escalation
- **Read-only config**: Configuration mounted as read-only
- **Network isolation**: Uses custom Docker network

---

## Native Installation Steps

### 1. Create System User and Group

Create a dedicated user and group for the fileserver service:

```bash
# Create fileserver group
sudo groupadd --system fileserver

# Create fileserver user with restricted shell and no home directory
sudo useradd --system --gid fileserver --shell /usr/sbin/nologin \
    --home-dir /opt/fileserver --create-home fileserver

# Set proper ownership
sudo chown -R fileserver:fileserver /opt/fileserver
```

### 2. Create Directory Structure

Set up the required directory structure:

```bash
# Create application directories
sudo mkdir -p /opt/fileserver/{bin,logs}

# Create data directories (adjust paths as needed)
sudo mkdir -p /srv/fileserver/{documents,uploads,shared,workspace}

# Set permissions
sudo chown -R fileserver:fileserver /opt/fileserver
sudo chown -R fileserver:fileserver /srv/fileserver

# Set appropriate permissions for data directories
sudo chmod 755 /srv/fileserver/{documents,shared}      # Read-only directories
sudo chmod 775 /srv/fileserver/{uploads,workspace}     # Read-write directories
```

### 3. Build and Install Binary

Build the application in release mode:

```bash
# From project root
cargo build --release --workspace

# Copy binary to installation directory
sudo cp target/release/fileserver-server /opt/fileserver/bin/
sudo chown fileserver:fileserver /opt/fileserver/bin/fileserver-server
sudo chmod 755 /opt/fileserver/bin/fileserver-server
```

### 4. Install Configuration

Copy and customize the production configuration:

```bash
# Copy production config to system config directory
sudo cp server/config.prod.toml /etc/fileserver.toml

# Set proper ownership and permissions
sudo chown root:fileserver /etc/fileserver.toml
sudo chmod 640 /etc/fileserver.toml
```

Edit the configuration file to match your environment:

```toml
[server]
port = 50051
allowed_ips = ["127.0.0.1", "10.0.0.0/8", "192.168.0.0/16"]
user = "fileserver"
group = "fileserver"

[[directories]]
name = "documents"
path = "/srv/fileserver/documents"
permissions = "read-only"

[[directories]]
name = "uploads"
path = "/srv/fileserver/uploads"
permissions = "read-write"
```

### 5. Install Systemd Service

Install and enable the systemd service:

```bash
# Copy service file
sudo cp fileserver.service /etc/systemd/system/

# Reload systemd configuration
sudo systemctl daemon-reload

# Enable service to start on boot
sudo systemctl enable fileserver.service
```

## Service Management

### Starting the Service

```bash
# Start the service
sudo systemctl start fileserver.service

# Check status
sudo systemctl status fileserver.service

# View logs
sudo journalctl -u fileserver.service -f
```

### Configuration Validation

The service will automatically validate:
- Port availability
- Directory existence and permissions
- User/group existence (when running as root)
- IP address/CIDR format validation

### Security Features

#### Privilege Dropping

When started as root, the service will:
1. Bind to the configured port
2. Validate user/group existence
3. Drop privileges to the specified user/group
4. Verify privileges were successfully dropped

#### Systemd Security

The service unit includes comprehensive security hardening:

- **Filesystem isolation**: `ProtectSystem=strict`, `ProtectHome=true`
- **Namespace restrictions**: `RestrictNamespaces=true`
- **Memory protection**: `MemoryDenyWriteExecute=true`
- **Network restrictions**: `RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6`
- **Capability restrictions**: Minimal required capabilities

## Firewall Configuration

Configure your firewall to allow the gRPC port:

```bash
# For firewalld
sudo firewall-cmd --permanent --add-port=50051/tcp
sudo firewall-cmd --reload

# For ufw
sudo ufw allow 50051/tcp

# For iptables
sudo iptables -A INPUT -p tcp --dport 50051 -j ACCEPT
```

## Monitoring and Logging

### Log Files

The service logs to systemd journal. View logs with:

```bash
# View recent logs
sudo journalctl -u fileserver.service --since "1 hour ago"

# Follow logs in real-time
sudo journalctl -u fileserver.service -f

# View logs with specific log level
sudo journalctl -u fileserver.service -p info
```

### Health Checks

The service provides a health check endpoint. You can create monitoring scripts:

```bash
# Example health check script
#!/bin/bash
if systemctl is-active --quiet fileserver.service; then
    echo "Fileserver is running"
    exit 0
else
    echo "Fileserver is not running"
    exit 1
fi
```

## Backup and Recovery

### Configuration Backup

```bash
# Backup configuration
sudo cp /etc/fileserver.toml /etc/fileserver.toml.backup.$(date +%Y%m%d)
```

### Data Backup

```bash
# Backup data directories
sudo tar -czf /backup/fileserver-data-$(date +%Y%m%d).tar.gz /srv/fileserver/
```

## Troubleshooting

### Common Issues

1. **Permission Denied Errors**
   - Verify file/directory ownership: `ls -la /opt/fileserver/ /srv/fileserver/`
   - Check SELinux contexts if applicable
   - Ensure firewall allows the configured port

2. **Privilege Drop Failures**
   - Verify user/group exist: `id fileserver`
   - Check if running as root initially
   - Review service logs for detailed error messages

3. **Configuration Validation Errors**
   - Verify directory paths exist and are accessible
   - Check IP address/CIDR format in allowed_ips
   - Validate port availability: `ss -tlnp | grep :50051`

### Service Debugging

```bash
# Stop service
sudo systemctl stop fileserver.service

# Run manually for debugging
sudo -u fileserver /opt/fileserver/bin/fileserver-server --config /etc/fileserver.toml

# Check service environment
sudo systemd-analyze verify /etc/systemd/system/fileserver.service
```

## Updates and Maintenance

### Updating the Binary

```bash
# Stop service
sudo systemctl stop fileserver.service

# Build new version
cargo build --release --workspace

# Backup current binary
sudo cp /opt/fileserver/bin/fileserver-server /opt/fileserver/bin/fileserver-server.backup

# Install new binary
sudo cp target/release/fileserver-server /opt/fileserver/bin/
sudo chown fileserver:fileserver /opt/fileserver/bin/fileserver-server

# Start service
sudo systemctl start fileserver.service
```

### Configuration Updates

```bash
# Backup current config
sudo cp /etc/fileserver.toml /etc/fileserver.toml.backup

# Edit configuration
sudo nano /etc/fileserver.toml

# Test configuration (dry run)
sudo -u fileserver /opt/fileserver/bin/fileserver-server --config /etc/fileserver.toml --help

# Restart service to apply changes
sudo systemctl restart fileserver.service
```

## Security Recommendations

1. **Network Security**
   - Use TLS/SSL termination proxy (nginx, HAProxy) for external access
   - Restrict `allowed_ips` to known networks only
   - Use VPN or private networks when possible

2. **File Permissions**
   - Regularly audit directory permissions
   - Use read-only permissions where possible
   - Implement file integrity monitoring

3. **System Security**
   - Keep system packages updated
   - Monitor service logs regularly
   - Implement log rotation and retention policies
   - Use fail2ban or similar for brute force protection

4. **Application Security**
   - Regularly update Rust dependencies
   - Monitor for security advisories
   - Implement rate limiting at the proxy level

## Performance Tuning

### Resource Limits

Adjust systemd service limits based on your needs:

```ini
# In fileserver.service [Service] section
LimitNOFILE=65536        # File descriptor limit
LimitNPROC=4096          # Process limit
MemoryMax=1G             # Memory limit
```

### Network Optimization

For high-throughput scenarios:

```bash
# Increase network buffer sizes
echo 'net.core.rmem_max = 16777216' >> /etc/sysctl.conf
echo 'net.core.wmem_max = 16777216' >> /etc/sysctl.conf
sudo sysctl -p
```

This deployment guide ensures a secure, production-ready installation of the Rust gRPC fileserver with proper privilege separation and system integration.