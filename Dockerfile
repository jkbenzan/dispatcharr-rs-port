# STEP 1: Build the Rust Binary
FROM rust:1.88-slim AS builder

# Install OpenSSL development headers and pkg-config
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
RUN cargo build --release

# STEP 2: Create the slim Production Image
FROM debian:bookworm-slim

# Install runtime OpenSSL (needed for the binary to actually run)
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/dispatcharr-rs /usr/local/bin/

# Copy your React frontend files
COPY dist /app/dist

EXPOSE 8080

# Start the application
CMD ["dispatcharr-rs"]