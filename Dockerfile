# STEP 1: Build the React Frontend
FROM node:20-slim AS frontend-builder

WORKDIR /app/frontend
COPY Dispatcharr-main/frontend/package*.json ./
RUN npm ci
COPY Dispatcharr-main/frontend/ ./
RUN npm run build

# STEP 2: Build the Rust Binary
FROM rust:bookworm AS backend-builder

# Install OpenSSL development headers and pkg-config
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
RUN cargo build --release

# STEP 3: Create the slim Production Image
FROM debian:bookworm-slim

# Install runtime OpenSSL and ffmpeg
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary from the Rust builder stage
COPY --from=backend-builder /app/target/release/dispatcharr-rs /usr/local/bin/

# Copy the compiled frontend from the Node builder stage
COPY --from=frontend-builder /app/frontend/dist /app/dist

EXPOSE 8080

# Start the application
CMD ["dispatcharr-rs"]