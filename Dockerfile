FROM rust:1.75 as builder

# Install protobuf compiler
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
RUN cargo build --release --bin fileserver-server --bin fileserver-client

FROM debian:bookworm-slim

# Install ca-certificates for TLS support
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create fileserver user with specific UID/GID for Docker compatibility
RUN groupadd -r fileserver -g 1000 && \
    useradd -r -g fileserver -u 1000 -m fileserver

# Copy binaries from builder
COPY --from=builder /app/target/release/fileserver-server /usr/local/bin/
COPY --from=builder /app/target/release/fileserver-client /usr/local/bin/

# Copy production config template (can be overridden with volume mount)
COPY server/config.prod.toml /etc/fileserver.toml

# Create data directories with proper ownership
RUN mkdir -p /srv/fileserver/{documents,uploads,shared,workspace} && \
    chown -R 1000:1000 /srv/fileserver

# Expose gRPC port
EXPOSE 50051

# Switch to non-root user
USER 1000:1000

# Set working directory
WORKDIR /srv/fileserver

# Run the fileserver
CMD ["/usr/local/bin/fileserver-server"]