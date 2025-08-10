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

# Copy manifests
COPY src/Cargo.toml ./Cargo.toml
COPY src/Cargo.lock ./Cargo.lock

# cache dependencies
RUN cargo build --release
RUN rm src/*.rs

# Copy source tree
COPY src/src ./src

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
COPY src/frontend ./frontend
