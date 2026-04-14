# Dispatcharr-RS

A high-performance Rust rewrite of the Dispatcharr IPTV middleware.

## Features
- Zero-copy stream proxying
- Failover support
- Async SQLite with SeaORM
- API compatibility with existing React frontend

## Development
1. `cp .env.example .env`
2. `cargo run`

## Docker
`docker build -t dispatcharr-rs .`
