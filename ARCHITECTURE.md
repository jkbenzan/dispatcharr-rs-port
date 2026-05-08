# Dispatcharr Architecture

## Overview
Dispatcharr is a high-performance M3U/XC/EPG proxy and management system built with Rust (Backend) and Svelte (Frontend).

## Backend (Rust)
- **Framework**: Axum
- **Database**: PostgreSQL (via SeaORM)
- **Background Worker**: 
    - Handles periodic M3U and EPG refreshes.
    - **Staggering**: Refreshes are staggered across accounts with a 60s delay and a stable per-account jitter (±30s) to prevent thundering herd issues.
    - **Throttling**: Accounts are only refreshed if the `refresh_interval` has passed since the last successful update.
- **M3U/XC Ingestion**:
    - Supports standard M3U playlists and Xtream Codes API.
    - **Case-Insensitive Account Detection**: Correctly handles "XC", "xc", and "xtream" types.
    - **VOD Support**: Segmented ingestion for Movies and Series.
    - **Enable VOD Toggle**: Providers can opt-out of VOD ingestion via a custom property `enable_vod`.
    - **Activity Log**: Sync events are recorded in `core_systemevent`. Routine background successes are suppressed from the log to reduce noise.

## Frontend (Svelte)
- **State Management**: Svelte 5 Runes ($state, $derived, $effect).
- **M3U Provider Management**:
    - Modal for adding/editing providers with tabbed interface (General, Channel Categories, VOD Movies, VOD Series).
    - **Sync Overlay**: Provides real-time feedback during initial provider synchronization.
    - **Reactive Tabs**: Groups and Categories are filtered based on the selected provider and discovered in real-time.

## Telemetry & Logging
- **System Events**: Stored in `core_systemevent`.
- **Background Suppression**: Routine successful background tasks do not create event entries.
- **Errors**: All sync errors are logged regardless of background status.
