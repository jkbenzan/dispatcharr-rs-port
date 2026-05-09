# Dispatcharr Architecture

## Overview
Dispatcharr is a high-performance M3U/XC/EPG proxy and management system built with Rust (Backend) and Svelte (Frontend).

## Backend (Rust)
- **Framework**: Axum
- **Database**: External PostgreSQL via SeaORM. Runtime configuration is provided through `DATABASE_URL`, and production/deployed environments are expected to point at the external Postgres instance. SeaORM is compiled with both PostgreSQL and SQLite features because a separate read-only SQLite sidecar is used for channel database metadata.
- **Background Worker**: 
    - Handles periodic M3U and EPG refreshes.
    - **Staggering**: Refreshes are staggered across accounts with a 60s delay and a stable per-account jitter (±30s) to prevent thundering herd issues.
    - **Throttling**: Accounts are only refreshed if the `refresh_interval` has passed since the last successful update.
- **M3U/XC Ingestion**:
    - Supports standard M3U playlists and Xtream Codes API.
    - **Case-Insensitive Account Detection**: A centralized `is_xc_account()` helper in `api.rs` normalizes all XC type comparisons. This prevents routing failures caused by case mismatches between the frontend (`"xc"`) and backend (`"XC"`) — ensuring provider creation, refresh, and background sync all correctly identify XC accounts regardless of case.
    - **VOD Support**: Segmented ingestion for Movies and Series.
    - **Enable VOD Toggle**: Providers can opt-out of VOD ingestion via a custom property `enable_vod`. **Disabled by default** to minimize unintended data ingestion.
    - **Mandatory Account Type**: New providers require an explicit choice between M3U and XC types; no system default is assumed.
    - **URL Normalization**: Robust XC base URL extraction logic preserves subpaths while stripping specific IPTV filenames (e.g., `get.php`, `player_api.php`) and query strings.
    - **Centralized Error Handling**: All synchronization routines (Live, VOD, Series) utilize a stabilized `async { ... }.await` Result-block pattern. This architectural pattern guarantees that all errors (connectivity, parsing, or API timeouts) are properly captured and propagated to a centralized `handle_sync_error` routine. 
    - **Trait Satisfaction**: Backend routines are strictly enforced to satisfy `Send + Sync` requirements for multi-threaded async execution, ensuring reliable background operation and stable container builds.
    - **Activity Log**: Sync events are recorded in `core_systemevent` for both manual and background refreshes, providing a clear audit trail of ingestion health.
- **Stream Counting & Statistics**:
    - **Initial Prefetch**: During provider setup, a "full fetch" is performed to tally streams per category, allowing the UI to display "{n} streams found" before categories are selected.
    - **Periodic Updates**: Background sync tasks for Live, VOD, and Series now include a tallying phase that updates category `stream_count` in the database `custom_properties` field.
    - **Global Metrics**: Counts are updated for all categories (groups) provided by the source, ensuring the management UI always reflects current provider content.
- **Channel Matching**:
    - Channel name parsing removes provider noise, country prefixes, resolution markers, and generic terms before scoring.
    - Match scoring uses Jaro-Winkler similarity plus resolution, country, and logo context.
    - A token-overlap guard caps unrelated short-name matches below medium confidence when the channel name shares no significant token with the station name or call sign.
    - Applying a match now persists selected station metadata to the external PostgreSQL channel row, including station ID, channel name, TVG-ID, and logo lookup/creation.

## Frontend (Svelte)
- **State Management**: Svelte 5 Runes ($state, $derived, $effect).
- **M3U Provider Management**:
    - Modal for adding/editing providers with tabbed interface (**Channel Categories**, VOD Movies, VOD Series).
    - **Stream Statistics**: Displays human-readable stream counts (e.g., "{n} streams found") for all categories, retrieved from persisted `custom_properties`.
    - **Sync Overlay**: Provides real-time feedback during initial provider synchronization.
    - **Reactive Tabs**: Groups and Categories are filtered based on the selected provider and discovered in real-time.
- **Build Health**:
    - `npm run check` passes with zero Svelte diagnostics.
    - `npm run build` completes successfully and writes the static frontend to `../dist`.
    - The frontend includes explicit Node typings via `@types/node` for the SvelteKit generated TypeScript configuration.

## Telemetry & Logging
- **System Events**: Stored in `core_systemevent`. M3U Provider creations, deletions, and manual refreshes explicitly log their status (success, info, or error) to this table.
- **Sync Visibility**: Both manual and background refreshes are recorded in the activity log. Events include an `is_background` flag in the payload to distinguish automated tasks.
- **Errors**: All sync errors are propagated using the `?` operator to avoid silent failures. The `handle_sync_error` helper automatically updates the account `status` to `"failed"` and persists the error message for visibility in the Provider Grid.
- **Frontend Error Propagation**: The Svelte frontend strictly bubbles up `errorData.error` from JSON API responses to ensure any backend failures (such as database constraint violations) are correctly exposed rather than being masked as generic 500 errors.
- **Diagnostic Logging**: Backend sync tasks log discovered category and stream counts to stdout/stderr for operational monitoring and system integrity verification.

## Verification Status
- `cargo check` passes with local Rust warning debt cleared; SeaORM is upgraded to the 1.1 line, which resolves the prior `sqlx-postgres v0.7.4` future-incompatibility notice.
- `cargo test` passes all current unit tests.
- Runtime startup has been verified against the external PostgreSQL database and the downloaded channel-data SQLite sidecar; the backend connects and listens on port 8080.
- The optional channel-data sidecar is stored locally at `data/channel_data.db`, is ignored by git, and can be replaced or redirected with `CHANNEL_DB_PATH` when needed.
- `npm run check` passes with zero warnings.
- `npm run build` passes when the process can write the repository-root `dist` directory.
- `npm install` currently reports low-severity audit findings; do not run `npm audit fix --force` without checking for breaking package changes.
