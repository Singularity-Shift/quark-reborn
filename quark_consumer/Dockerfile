# Use the official Rust image as the base
FROM rust:1.85 as builder

# Set the working directory
WORKDIR /app

# Copy the workspace Cargo.toml and Cargo.lock
COPY Cargo.toml Cargo.lock ./

# Copy all workspace members (required for workspace build)
COPY quark_core/ ./quark_core/
COPY quark_bot/ ./quark_bot/
COPY quark_server/ ./quark_server/
COPY quark_consumer/ ./quark_consumer/

# Build the application
RUN cargo build --release --bin quark_consumer

# Runtime stage
FROM debian:bookworm-slim

# Install ca-certificates for HTTPS connections
# Install required system dependencies INCLUDING GPG
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    gnupg \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 consumer

# Set working directory
WORKDIR /app

# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/quark_consumer /usr/local/bin/quark_consumer

# Copy the assets folder from the builder stage
COPY --from=builder /app/quark_consumer/assets/ ./assets/

# Copy GPG keys if they exist
COPY *.asc ./
COPY *.asc ./
COPY *.gpg ./


# Change ownership of the binary and assets
RUN chown consumer:consumer /usr/local/bin/quark_consumer && \
    chown -R consumer:consumer /app/assets

# Switch to the non-root user
USER consumer

# Set the command to run the application
CMD ["quark_consumer"] 