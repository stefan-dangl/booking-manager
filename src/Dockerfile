# Use the official Rust image for version 1.88
FROM rust:1.88.0-bookworm as builder

# Install build dependencies for Diesel with PostgreSQL
RUN apt-get update && \
    apt-get install -y \
    libpq-dev \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty shell project
RUN USER=root cargo new --bin booking_manager
WORKDIR /booking_manager

# Copy your manifests
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock

# Install Diesel CLI (if needed for migrations)
RUN cargo install diesel_cli --no-default-features --features postgres

# This build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# Copy your source tree
COPY ./src ./src
# COPY ./migrations ./migrations  # If you use Diesel migrations

# Build for release
RUN rm ./target/release/deps/booking_manager*
RUN cargo build --release

# Final base
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the build artifacts from the build stage
COPY --from=builder /booking_manager/target/release/booking_manager .
COPY ./.env .
COPY ./frontend ./frontend
COPY ./migrations ./migrations
# COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel  # If you need diesel CLI


# Set the startup command to run your binary
