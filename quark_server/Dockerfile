# Use the official Rust image as a base
FROM rust:1.85 as builder

# Set the working directory
WORKDIR /app/quark_server

# Copy the Cargo.toml files and workspace Cargo.lock
COPY quark_server/Cargo.toml ./
COPY quark_core/Cargo.toml ../quark_core/
COPY quark_bot/Cargo.toml ../quark_bot/
COPY quark_consumer/Cargo.toml ../quark_consumer/
COPY Cargo.toml ../

# Copy the source code
COPY quark_server/src/ ./src/
COPY quark_core/src/ ../quark_core/src/
COPY quark_bot/src/ ../quark_bot/src/
COPY quark_consumer/src/ ../quark_consumer/src/

# Build the application
RUN cargo build --release --bin quark_server

# Create a new, smaller image for the final application
FROM debian:bookworm-slim

# Install required system dependencies INCLUDING GPG
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    gnupg \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app/quark_server

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/quark_server .

# Copy GPG keys if they exist
COPY *.asc ./
COPY *.asc ./
COPY *.gpg ./

# Expose the port the app runs on
EXPOSE 8000

# Run the application
CMD ["./quark_server"] 