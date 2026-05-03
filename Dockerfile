# STEP 1a: Build the React Frontend
FROM node:20-slim AS frontend-builder

WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# STEP 1b: Build the Angular Channel Manager (mini-app)
FROM node:20-slim AS angular-builder

WORKDIR /app/angular-frontend
COPY angular-frontend/package*.json ./
RUN npm ci
COPY angular-frontend/ ./
RUN npx ng build

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

# Copy the React frontend (main app)
# Vite is configured to output to ../dist (relative to frontend/)
COPY --from=frontend-builder /app/dist /app/dist

# Copy the Angular channel manager mini-app to /channel-manager/
COPY --from=angular-builder /app/dist/browser /app/dist/channel-manager

EXPOSE 8080

# Start the application
CMD ["dispatcharr-rs"]