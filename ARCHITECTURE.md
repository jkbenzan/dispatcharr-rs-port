# Dispatcharr Architecture

## Overview
Dispatcharr is a high-performance M3U/XC/EPG proxy and management system built with Rust (Backend) and Svelte (Frontend).

## Backend (Rust)
- **Framework**: Axum
- **Database**: External PostgreSQL via SeaORM. Runtime configuration is provided through `DATABASE_URL`, and production/deployed environments are expected to point at the external Postgres instance. SeaORM is compiled with both PostgreSQL and SQLite features because a separate read-only SQLite sidecar is used for channel database metadata.
- **Channel Model**: Channels are curated playback outputs. Provider M3U/XC entries are stored as streams and assigned to channels through `dispatcharr_channels_channelstream`; a single channel can have multiple streams for proxy failover, quality selection, concurrency limits, and buffering behavior. Stream rows should not be treated as user-facing channels. The historical auto-channel-sync path can generate channels from streams, but it is a legacy workflow and is not practical for normal provider-scale usage.
- **Background Worker**: 
    - Handles periodic M3U and EPG refreshes.
    - **Staggering**: Refreshes are staggered across accounts with a 60s delay and a stable per-account jitter (±30s) to prevent thundering herd issues.
    - **Throttling**: Accounts are only refreshed if the `refresh_interval` has passed since the last successful update.
    - **Global Provider Refresh Queue**: M3U/XC and EPG refresh jobs share a single process-wide refresh semaphore. The default concurrency is `1`, so simultaneous manual, bulk, setup, VOD, and background provider refreshes queue and run sequentially. The limit can be raised through `stream_settings.provider_refresh_concurrency` or overridden at startup with `DISPATCHARR_PROVIDER_REFRESH_CONCURRENCY`; values are clamped to `1..=8`.
- **M3U/XC Ingestion**:
    - Supports standard M3U playlists and Xtream Codes API.
    - Provider entries are ingested as `dispatcharr_channels_stream` rows first. The legacy auto-channel-sync path can promote unmapped streams into generated channels, and those generated channels are tied back to the provider through `auto_created_by_id` for cleanup, but normal operation should curate channels separately and assign one or more streams to them.
    - Legacy auto-channel-sync is disabled unless `DISPATCHARR_ENABLE_LEGACY_AUTO_CHANNEL_SYNC=true` is set, and it refuses to generate more than `DISPATCHARR_AUTO_CHANNEL_SYNC_LIMIT` channels in one sync (default `100`).
    - Locked/custom providers are excluded from manual, bulk, and background refresh paths. Custom-provider detection is case-insensitive, and bulk refresh skips providers already in `fetching` so an operator cannot queue duplicate work for an in-flight account.
    - Provider refresh state is self-healing: startup and the background scheduler reset accounts left in `fetching` longer than `DISPATCHARR_M3U_FETCHING_STALE_MINUTES` (default `120`) to `failed` with a clear message.
    - Provider account API responses never return stored usernames or passwords. Responses expose `has_username` and `has_password` booleans so edit forms can preserve existing credentials unless the user enters replacements.
    - The Svelte provider modal treats provider credentials as write-only: edit forms leave credential inputs blank, show saved-credential hints, and only submit replacements when the user types them.
    - The Svelte provider UI reads M3U account types case-insensitively (`XC`, `xc`, and `xtream`) while continuing to submit normalized lowercase values.
    - Provider status responses include normalized status, failure state, stream count, and last refresh timestamp; dashboard health treats both `error` and `failed` provider states as warnings.
    - The Provider Grid displays normalized status, provider stream count, last refresh timestamp, and safe status messages so operators can triage provider health without inspecting Postgres.
    - **Case-Insensitive Account Detection**: A centralized `is_xc_account()` helper in `api.rs` normalizes all XC type comparisons. This prevents routing failures caused by case mismatches between the frontend (`"xc"`) and backend (`"XC"`) — ensuring provider creation, refresh, and background sync all correctly identify XC accounts regardless of case.
    - **VOD Support**: Segmented ingestion for Movies and Series.
    - **Enable VOD Toggle**: Providers can opt into VOD ingestion via a custom property `enable_vod`. **Disabled by default** in backend responses and in the Svelte provider modal to minimize unintended data ingestion.
    - **Mandatory Account Type**: New providers require an explicit choice between M3U and XC types; no system default is assumed.
    - **URL Normalization**: Robust XC base URL extraction logic preserves subpaths while stripping specific IPTV filenames (e.g., `get.php`, `player_api.php`) and query strings.
    - **Centralized Error Handling**: All synchronization routines (Live, VOD, Series) utilize a stabilized `async { ... }.await` Result-block pattern. This architectural pattern guarantees that all errors (connectivity, parsing, or API timeouts) are properly captured and propagated to a centralized `handle_sync_error` routine. 
    - **Trait Satisfaction**: Backend routines are strictly enforced to satisfy `Send + Sync` requirements for multi-threaded async execution, ensuring reliable background operation and stable container builds.
    - **Activity Log**: Sync events are recorded in `core_systemevent` for both manual and background refreshes, providing a clear audit trail of ingestion health.
- **Stream Counting & Statistics**:
    - **Initial Prefetch**: During provider setup, a "full fetch" is performed to tally streams per category, allowing the UI to display "{n} streams found" before categories are selected.
    - **Periodic Updates**: Background sync tasks for Live, VOD, and Series now include a tallying phase that updates category `stream_count` in the database `custom_properties` field.
    - **Global Metrics**: Counts are updated for all categories (groups) provided by the source, ensuring the management UI always reflects current provider content.
- **Stream Checker & Maintenance**:
    - Manual stream checks, bulk checks, and background maintenance share the same health bookkeeping path.
    - Stream diagnostics are stored in each stream row's `custom_properties.stream_stats` and `stream_stats_updated_at` fields.
    - Auto-pruning marks streams as stale after `maintenance_settings.auto_prune_failed_count` consecutive failed checks. The default threshold is `3`, and `0` disables stale marking.
    - Maintenance settings are stored in `core_settings` under `maintenance_settings` and surfaced in the Svelte Settings page.
- **Channel Matching**:
    - Channel name parsing removes provider noise, country prefixes, resolution markers, and generic terms before scoring.
    - Match scoring uses Jaro-Winkler similarity plus resolution, country, and logo context.
    - A token-overlap guard caps unrelated short-name matches below medium confidence when the channel name shares no significant token with the station name or call sign.
    - Applying a match now persists selected station metadata to the external PostgreSQL channel row, including station ID, channel name, TVG-ID, and logo lookup/creation.
- **Logo Libraries**:
    - A background worker syncs external logo repositories into `data/logo-libraries` on startup and every 24 hours.
    - Default repositories are `tv-logo/tv-logos` and `iptv-org/logos`; local logo-library search is exposed through `/api/channels/logos/search-libraries/`.

## Frontend (Svelte)
- **State Management**: Svelte 5 Runes ($state, $derived, $effect).
- **M3U Provider Management**:
    - Modal for adding/editing providers with tabbed interface (**Channel Categories**, VOD Movies, VOD Series).
    - Provider delete and bulk-refresh actions use an in-app confirmation dialog instead of native browser prompts. The dialog keeps queued-refresh language aligned with the backend provider queue and guards against double-submit while the confirmed request is in flight.
    - **Stream Statistics**: Displays human-readable stream counts (e.g., "{n} streams found") for all categories, retrieved from persisted `custom_properties`.
    - **Sync Overlay**: Provides real-time feedback during initial provider synchronization.
    - **Reactive Tabs**: Groups and Categories are filtered based on the selected provider and discovered in real-time.
- **Build Health**:
    - `npm run check` passes with zero Svelte diagnostics.
    - `npm run build` completes successfully and writes the static frontend to `../dist`.
    - The frontend includes explicit Node typings via `@types/node` for the SvelteKit generated TypeScript configuration.
- **Playback**:
    - The TV Guide player requests `/api/streams/:id?format=hls`. The backend serves a lightweight HLS manifest that points clients at the existing MPEG-TS proxy stream while preserving token or username/password query authentication.

## Telemetry & Logging
- **System Events**: Stored in `core_systemevent`. M3U Provider creations, deletions, and manual refreshes explicitly log their status (success, info, or error) to this table.
- **Sync Visibility**: Both manual and background refreshes are recorded in the activity log. Events include an `is_background` flag in the payload to distinguish automated tasks.
- **Errors**: All sync errors are propagated using the `?` operator to avoid silent failures. The `handle_sync_error` helper automatically updates the account `status` to `"failed"` and persists the error message for visibility in the Provider Grid.
- **Frontend Error Propagation**: The Svelte frontend strictly bubbles up `errorData.error` from JSON API responses to ensure any backend failures (such as database constraint violations) are correctly exposed rather than being masked as generic 500 errors.
- **Activity Log Rendering**: The Svelte activity page renders existing system event detail shapes (`message`, `event`, `status`, and `error`) into readable summaries while preserving the expandable raw JSON details for diagnostics.
- **Diagnostic Logging**: Backend sync tasks log discovered category and stream counts to stdout/stderr for operational monitoring and system integrity verification.

## Verification Status
- `cargo check` passes with local Rust warning debt cleared; SeaORM is upgraded to the 1.1 line, which resolves the prior `sqlx-postgres v0.7.4` future-incompatibility notice.
- `cargo test` passes all current unit tests.
- Runtime startup has been verified against the external PostgreSQL database and the downloaded channel-data SQLite sidecar; the backend connects and listens on port 8080.
- The optional channel-data sidecar is stored locally at `data/channel_data.db`, is ignored by git, and can be replaced or redirected with `CHANNEL_DB_PATH` when needed.
- Channel DB match application has been smoke-tested against an existing external-PostgreSQL channel row by applying and restoring name/TVG-ID fields.
- External PostgreSQL channel cleanup was performed after a bad auto-created-channel flood: `52,744` orphan generated channels were backed up to `cleanup_backup_bad_auto_channels_20260509_171135` and removed, leaving `316` curated/manual channels.
- `npm run check` passes with zero warnings.
- `npm run build` passes when the process can write the repository-root `dist` directory.
- `npm install` currently reports low-severity audit findings; do not run `npm audit fix --force` without checking for breaking package changes.
