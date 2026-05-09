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
    - **Enable VOD Toggle**: Providers can opt-out of VOD ingestion via a custom property `enable_vod`. **Disabled by default** to minimize unintended data ingestion.
    - **Mandatory Account Type**: New providers require an explicit choice between M3U and XC types; no system default is assumed.
    - **Activity Log**: Sync events are recorded in `core_systemevent` for both manual and background refreshes.
- **Stream Counting & Statistics**:
    - **Initial Prefetch**: During provider setup, a "full fetch" is performed to tally streams per category, allowing the UI to display "{n} streams found" before categories are selected.
    - **Periodic Updates**: Background sync tasks for Live, VOD, and Series now include a tallying phase that updates category `stream_count` in the database `custom_properties` field.
    - **Global Metrics**: Counts are updated for all categories (groups) provided by the source, ensuring the management UI always reflects current provider content.

## Frontend (Svelte)
- **State Management**: Svelte 5 Runes ($state, $derived, $effect).
- **M3U Provider Management**:
    - Modal for adding/editing providers with tabbed interface (**Channel Categories**, VOD Movies, VOD Series).
    - **Stream Statistics**: Displays human-readable stream counts (e.g., "{n} streams found") for all categories, retrieved from persisted `custom_properties`.
    - **Sync Overlay**: Provides real-time feedback during initial provider synchronization.
    - **Reactive Tabs**: Groups and Categories are filtered based on the selected provider and discovered in real-time.

## Telemetry & Logging
- **System Events**: Stored in `core_systemevent`.
- **Sync Visibility**: Both manual and background refreshes are recorded in the activity log. Events include an `is_background` flag in the payload to distinguish automated tasks.
- **Errors**: All sync errors are logged with detailed context for troubleshooting.
