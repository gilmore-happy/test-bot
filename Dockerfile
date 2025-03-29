# Multi-stage Dockerfile for Solana HFT Bot
# Stage 1: Build environment
FROM rust:1.76-slim AS builder

# Set build arguments
ARG RUSTFLAGS="-C target-cpu=native"
ENV RUSTFLAGS=${RUSTFLAGS}

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libudev-dev \
    clang \
    cmake \
    git \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy Cargo.toml and Cargo.lock files
COPY solana-hft-bot/Cargo.toml ./
COPY solana-hft-bot/crates ./crates/

# Build dependencies to cache them (this will be cached if dependencies don't change)
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy the rest of the source code
COPY solana-hft-bot .

# Build the application
RUN cargo build --release

# Stage 2: Development image
FROM debian:bookworm-slim AS development

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl-dev \
    libudev-dev \
    procps \
    iproute2 \
    net-tools \
    curl \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/solana-hft-bot /usr/local/bin/

# Copy configuration files
COPY solana-hft-bot/config.json /app/config/config.json
COPY solana-hft-bot/logging-config.json /app/config/logging-config.json
COPY solana-hft-bot/metrics-config.json /app/config/metrics-config.json

# Create necessary directories with proper permissions
RUN mkdir -p /app/logs /app/metrics /app/keys /app/data && \
    chmod 755 /app/logs /app/metrics /app/keys /app/data

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash solana
RUN chown -R solana:solana /app

# Set environment variables
ENV CONFIG_PATH=/app/config/config.json
ENV LOGGING_CONFIG_PATH=/app/config/logging-config.json
ENV METRICS_CONFIG_PATH=/app/config/metrics-config.json
ENV RUST_BACKTRACE=1
ENV RUST_LOG=info

# Add debugging tools for development
RUN apt-get update && apt-get install -y \
    gdb \
    strace \
    htop \
    vim \
    && rm -rf /var/lib/apt/lists/*

# Switch to non-root user
USER solana

# Set the entrypoint
ENTRYPOINT ["solana-hft-bot"]
CMD ["--config", "/app/config/config.json"]

# Stage 3: Production image (minimal)
FROM debian:bookworm-slim AS production

# Install only necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl-dev \
    libudev-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/solana-hft-bot /usr/local/bin/

# Copy configuration files
COPY solana-hft-bot/config.json /app/config/config.json
COPY solana-hft-bot/logging-config.json /app/config/logging-config.json
COPY solana-hft-bot/metrics-config.json /app/config/metrics-config.json

# Create necessary directories with proper permissions
RUN mkdir -p /app/logs /app/metrics /app/keys /app/data && \
    chmod 755 /app/logs /app/metrics /app/keys /app/data

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash solana
RUN chown -R solana:solana /app

# Set environment variables
ENV CONFIG_PATH=/app/config/config.json
ENV LOGGING_CONFIG_PATH=/app/config/logging-config.json
ENV METRICS_CONFIG_PATH=/app/config/metrics-config.json
ENV RUST_BACKTRACE=1
ENV RUST_LOG=info

# Switch to non-root user
USER solana

# Set the entrypoint
ENTRYPOINT ["solana-hft-bot"]
CMD ["--config", "/app/config/config.json"]