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

## Logging Configuration
The application uses the `tracing` framework for logging. You can configure the log level and specificity using the `RUST_LOG` environment variable. The variable accepts multiple values separated by commas.

### Examples

**1. Basic Module-specific logging:**
```env
RUST_LOG=info,dispatcharr_rs=debug
```
*Sets the global log level to `info` to reduce noise from dependencies, but sets your application code (`dispatcharr_rs`) to `debug`.*

**2. Highly specific module logging:**
```env
RUST_LOG=info,dispatcharr_rs::stream_checker=trace,sea_orm=warn
```
*Global level is `info`, but the `stream_checker` module will output `trace` (all details), while `sea_orm` (database queries) will only show warnings and errors.*

**3. Target a specific function:**
```env
RUST_LOG=info,dispatcharr_rs::api[get_channels]=debug
```
*Only shows debug logs originating from inside the `get_channels` function, while keeping the rest of the application at `info`.*
