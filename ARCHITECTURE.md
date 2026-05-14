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
- **Category Save → Refresh Status Lifecycle**:
    - When a user saves category selections, `update_m3u_group_settings` sets the account status to `"pending"` and broadcasts a non-terminal progress event (`step: "groups_saved"`, `progress: 10`). It must NOT set `"success"` or broadcast `"completed"` because the caller (`saveImportSelections`) immediately queues a refresh via `refreshM3UAccount`. Setting `"success"` prematurely would cause the Streams page to stop polling (`forgetProgress`) before the refresh task starts, leaving the UI with stale stream counts.
    - The correct lifecycle is: `pending` (groups saved) → `queued` (refresh task spawned) → `fetching` (parsing in progress) → `success` (import complete). The Streams page polling loop remains active throughout because none of the intermediate statuses trigger `forgetProgress`.
    - Missing group-to-account mappings during `update_m3u_group_settings` are logged at `WARN` level to surface edge cases where a user's toggle would be silently lost.
- **Import Diagnostic Tracing**:
    - Both `parse_m3u_from_file` (M3U) and `fetch_and_parse_xc` (XC) log an import summary at `INFO` level upon completion. The summary includes: total eligible streams, newly inserted, hash-dedup skipped (already in DB), disabled-group skipped, and filter-skipped counts.
    - Example: `[M3U Sync] account_id=5: 12450 eligible streams, 340 inserted (new), 12100 skipped (hash dedup / already in DB), 10 skipped (disabled groups), 0 skipped (filters), is_initial=false`
    - This makes diagnosing "stream count didn't change" issues trivial from backend logs without requiring database inspection.
- **Stream Checker & Maintenance**:
    - Manual stream checks, bulk checks, and background maintenance share the same health bookkeeping path.
    - Stream diagnostics are stored in each stream row's `custom_properties.stream_stats` and `stream_stats_updated_at` fields.
    - Auto-pruning marks streams as stale after `maintenance_settings.auto_prune_failed_count` consecutive failed checks. The default threshold is `3`, and `0` disables stale marking.
    - **Stream Settings Management**: Settings are stored in `core_settings` under the `stream_settings` and `maintenance_settings` JSON objects. Missing defaults (e.g., `buffer_size`, `retry_count`, `default_user_agent`, `stream_checker_parallel_providers`) are injected dynamically on startup.
    - **Stream Settings UI**: Stream Checker and Maintenance parameters have been unified into a dedicated "Stream Checker" tab on the global Settings page. The UI provides an "Enable Auto-Prune" toggle that smoothly maps the UI boolean state to the numeric threshold (`0` for disabled).
    - **Sorting Rules**: CRUD endpoints (`/api/stream-checker/sorting-rules/`) manage `stream_sorting_rule` rows that define property-operator-value conditions with score modifiers. The `POST /api/stream-checker/sort-streams/` endpoint accepts channel IDs and reorders each channel's streams by cumulative rule score. Rule logic is validated via internal pure-function Rust unit tests.
    - **Sorting Rules UI**: The Stream Checker page (`/stream-checker`) uses a tabbed interface (Testing | Sorting Rules). The Sorting Rules tab provides an inline CRUD table with property/operator dropdowns, priority sequencing, score modifier input, and two-click delete confirmation. The table supports both inline editing of existing rules and inline creation of new rules.
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
    - The Streams page shows non-custom M3U/XC providers in fixed alphabetical order. The built-in custom provider is hidden from provider-card management because it has no refresh, category, VOD, or stream-ingestion workflow.
    - The Streams page listens for provider websocket progress/system events and polls provider state while refresh work is queued or running, so provider cards and the queue banner reflect active/queued refresh status without requiring a page reload. Refresh requests also create immediate local progress entries for accepted providers so cards do not appear idle while waiting for backend progress events.
    - Provider cards render a bottom refresh progress bar with elapsed time and ETA. M3U/XC cards use websocket percentage updates when available, while EPG/status-only refreshes fall back to status and elapsed-time estimates instead of hiding activity.
    - Channel category rows are selectable cards: clicking a visible category toggles import selection, Auto-Sync remains a separate per-category control, and "Select visible" / "Deselect visible" bulk actions only affect the currently filtered rows.
    - Provider identity/details and import-selection mappings have separate save flows. Saving Channel Categories, VOD Movies, or VOD Series persists only those selections, keeps the modal open, and queues a provider refresh so the stream inventory reflects the changed mapping.
    - Channel categories use lightweight country/region detection when provider group names look country-based. The UI shows detected flags on category cards and exposes a stable, compact, alphabetized detected-country filter while preserving plain text search for ambiguous or mixed category sets.
    - **Stream Statistics**: Displays human-readable stream counts (e.g., "{n} streams found") for all categories, retrieved from persisted `custom_properties`.
    - **Sync Overlay**: Provides real-time feedback during initial provider synchronization.
    - **Reactive Tabs**: Groups and Categories are filtered based on the selected provider and discovered in real-time.
- **Build Health**:
    - `npm run check` passes with zero Svelte diagnostics.
    - `npm run build` completes successfully and writes the static frontend to `../dist`.
    - The frontend includes explicit Node typings via `@types/node` for the SvelteKit generated TypeScript configuration.
- **Playback**:
    - The TV Guide player requests `/api/streams/:id?format=hls`. The backend serves a lightweight HLS manifest that points clients at the existing MPEG-TS proxy stream while preserving token or username/password query authentication.
- **Channel Manager** (`/channels`):
    - Two-pane resizable layout: **ChannelsPane** (left) shows a Group → Channel → Stream hierarchy; **StreamsPane** (right) shows M3U Account → Category → Stream hierarchy for assignment.
    - **Drag-and-drop assignment**: Streams are dragged from StreamsPane and dropped onto a channel row. StreamsPane encodes one or more stream IDs as `application/json` in `dataTransfer`; ChannelsPane merges them into the channel's existing stream list via `PATCH /api/channels/channels/:id/` with a full `streams: [id, …]` replacement array.
    - **Stream sorting**: `/api/channels/bulk-sort-streams/` applies a built-in score before custom rule modifiers. Confirmed online/reachable streams outrank untested streams, which outrank known offline streams. Among reliable streams, higher resolution height, FPS, bitrate, preferred codecs, lower consecutive failures, and clean frozen/black-screen checks sort higher. `m3u_account.priority` remains as a manual additive bias but new providers default to `0` so quality and reliability drive the order by default. If the optional `stream_sorting_rule` table is temporarily unreadable, the API logs a warning, returns an empty rules list, and continues sorting with the built-in score.
    - **Stream Checker tree**: `/stream-checker` builds its group → channel → streams tree from the same `GET /api/channels/channels/?page_size=5000` payload as Channel Manager. This keeps assigned streams enumerated under each channel using the backend `channel_stream.order` values, including immediately after bulk sorting. The UI displays stream order as one-based numbers while preserving the zero-based stored order; if an older response omits `order`, the rendered index is used as a fallback.
    - **Multi-select drag**: StreamsPane checkboxes (click/Space) accumulate selected IDs. Dragging any selected stream drags all selected IDs as a batch. A per-group "Select all/none" header button is provided.
    - **Stream sub-list**: Expanding a channel row reveals its assigned streams with drag handles for in-channel reorder (`reorderChannelStreams`) and ✕ remove buttons (calls `updateChannel` with filtered ID array).
    - **Stream count badge**: Each channel row shows a badge with the number of assigned streams.
    - **Kebab menu**: Each channel exposes Edit (opens `CreateChannelModal` in edit mode) and Delete (first click → confirm state, second click → `DELETE /api/channels/channels/:id/`).
    - **Reactive reloads**: After any mutation (create, edit, delete, assign, remove, reorder), both panes are refreshed via exported `reload()` functions. `window.location.reload()` is never used.
    - **Unassigned-only filter**: StreamsPane header toggle hides already-assigned streams so operators see only candidates for assignment.
    - **Create/Edit modal unification**: `CreateChannelModal.svelte` accepts `channelId` and `initialData` props. When `channelId` is non-null the modal is in edit mode: form is pre-filled from `initialData`, the submit button reads "Save Changes", and the API call routes to `PATCH` instead of `POST`.
    - **In-app floating video player**: `FloatingPlayer.svelte` wraps `VideoPlayer.svelte` in a fixed-position, draggable overlay. Both ChannelsPane (Play button per channel → uses the Dispatcharr stream proxy URL `/api/streams/:uuid`) and StreamsPane (Play button per stream → uses the raw stream URL) open the player by calling `onPlayStream` prop. The player supports minimize, Escape to close, and displays a live-dot indicator with stream title and provider name.
    - **Player persistence**: The player state is managed by a global Svelte 5 runes store (`$lib/player.svelte.ts`). The `FloatingPlayer` component is rendered in the root `+layout.svelte`, not inside individual route pages. This ensures the player survives SvelteKit page navigations. Any page can import `playerStore` and call `.open(info)` / `.close()` without prop drilling.
    - **Category vs Channel Group**: Streams carry a `group_title` field from the M3U provider's `#EXTINF group-title` attribute — this is the **Channel Category** (provider-side grouping). The `channel_group` FK on the stream table points to the curated **Channel Group** (user-created organizational containers in Channel Manager). The StreamsPane groups streams by Category (`group_title`), while ChannelsPane groups channels by Channel Group. The backend surfaces `group_title` from `custom_properties` in the stream JSON response.
    - **DESIGN CHOICE - Channel Groups vs Categories**: Throughout the application (e.g., Channel Manager, Stream Checker), the top-level organizational hierarchy MUST ALWAYS be "Channel Groups" (where `is_custom = true`), NOT "Channel Categories". The "vomit" of categories imported from M3U providers should strictly be contained to the right-hand M3U streams pane during assignment, and should never clutter the curated left-hand channel group views. Any future changes to this organizational hierarchy require double confirmation from the user.
    - **Provider filter**: StreamsPane header includes a `<select>` dropdown to filter streams by M3U provider. Default is "All Providers".
    - **Global Select All**: StreamsPane provides a header-level Select All / Deselect All toggle that applies to all currently visible (filtered) streams.
    - **Stream enumeration**: Each stream in a channel's expanded sub-list shows a 1-based position number (e.g., `1.`, `2.`, `3.`) and a Play button to preview individual streams.
    - **Stream health display**: Stream sub-list rows show health status through row shading (subtle red tint for offline/frozen/black_screen, no shading for online, gray dot indicator for untested). A condensed view shows key stats (resolution, video codec) as inline chips. Clicking a stats chevron expands to a 3-row detail view showing status, resolution, video/audio codecs, FPS, and bitrate. All stat values are null-safe (blank instead of literal "null").
    - **Sort by Health action**: The kebab menu includes a "Sort by Health" action that calls `POST /api/stream-checker/sort-streams/` with the channel's ID, reordering its streams by cumulative sorting rule score. The action is disabled when a channel has fewer than 2 streams.
    - **DELETE endpoint**: `DELETE /api/channels/channels/:id/` removes channel-stream join rows first (FK cleanup), then deletes the channel row. Returns `204 No Content`. Both trailing-slash and non-trailing-slash routes are registered.

## Telemetry & Logging
- **System Events**: Stored in `core_systemevent`. M3U Provider creations, deletions, and manual refreshes explicitly log their status (success, info, or error) to this table.
- **Sync Visibility**: Both manual and background refreshes are recorded in the activity log. Events include an `is_background` flag in the payload to distinguish automated tasks.
- **Errors**: All sync errors are propagated using the `?` operator to avoid silent failures. The `handle_sync_error` helper automatically updates the account `status` to `"failed"` and persists the error message for visibility in the Provider Grid.
- **Custom Provider Health**: The built-in custom provider is excluded from failed M3U provider counts because it is a manual stream container, not a refreshable provider.
- **Frontend Error Propagation**: The Svelte frontend strictly bubbles up `errorData.error` from JSON API responses to ensure any backend failures (such as database constraint violations) are correctly exposed rather than being masked as generic 500 errors.
- **Activity Log Rendering**: The Svelte activity page renders existing system event detail shapes (`message`, `event`, `status`, and `error`) into readable summaries while preserving the expandable raw JSON details for diagnostics.
- **Channel Manager Events**: All channel lifecycle mutations emit `record_event()` calls: `channel_created` (with channel name, number), `channel_updated` (metadata changes), `streams_assigned` (stream assignment/reorder with count), `channel_deleted` (with channel name as warning), `channel_group_created` (with group name), and `channels_bulk_updated`. Events fire only on success paths using fire-and-forget (`let _ =`) to avoid blocking the HTTP response.
- **Stream Checker Events**: `stream_check_completed` is emitted for every individual stream test (includes `is_single: true` flag for filtering), `bulk_check_started` when a bulk check launches (with stream/provider counts), `bulk_check_completed` when it finishes (with pass/fail/cancelled stats), and `bulk_check_cancelled` when the user cancels mid-run.
- **Sorting Rule Events**: `sorting_rule_created`, `sorting_rule_updated`, and `sorting_rule_deleted` fire on success paths for sorting rule CRUD operations. Create events include rule name, property, and operator for auditability.
- **Settings Events**: `settings_updated` and `settings_created` fire when `core_settings` rows are modified or created via the Settings page. Values are intentionally omitted from the event payload for security (some settings contain proxy URLs, network ACLs, and user limits).
- **Diagnostic Logging**: Backend sync tasks log discovered category and stream counts to stdout/stderr for operational monitoring and system integrity verification.

## Active Connections (Stats Dashboard)
- **Dashboard Integration**: Active connection monitoring lives as a tab on the Dashboard page (`/` -> "Active Connections" tab), not a separate route. This avoids sidebar clutter while keeping the stats easily accessible.
- **Sidebar Shortcut**: A persistent green Radio icon in the sidebar footer (next to theme controls) links directly to `/#connections`, allowing quick access from any page.
- **Connection Lifecycle**: Connection lifetimes are meticulously tracked via a `ClientDropGuard` that is bound into the inner HTTP stream fold to guarantee metrics align strictly with client connection drops or aborts, avoiding premature or orphaned connections in the telemetry logic.
- **Enriched Status Endpoint**: `GET /proxy/ts/status` batch-fetches channel metadata (name, number, logo URL) for all active channel UUIDs in a single DB query, eliminating the need for separate frontend lookups on each poll cycle.
- **Stop Controls**: Two DELETE endpoints provide connection management:
  - `DELETE /proxy/ts/stop/:channel_id` — Stops all streaming for a channel (removes clients, aborts broadcaster pumper task).
  - `DELETE /proxy/ts/stop/:channel_id/:client_id` — Disconnects a single client without stopping the broadcaster. The client's byte stream terminates on the next read cycle.
- **Polling Architecture**: The connections tab uses a configurable `setInterval` polling loop (default 5s, persisted in localStorage). Polling starts when the tab is activated and stops when switching away, preventing unnecessary network traffic.
- **ConnectionCard Component**: Each active channel renders as a card (`ConnectionCard.svelte`) showing: channel logo/name/number, LIVE indicator with pulse animation, uptime timer (HH:MM:SS), data transferred (auto-scaled B/KB/MB/GB), profile badges (stream profile + M3U provider), and a client table with per-client disconnect buttons.
- **Optimistic Updates**: Stop actions immediately remove the card/client from the UI for instant feedback, then re-fetch from the server to confirm the actual state.
- **Two-Click Stop**: The "Stop Channel" button requires two clicks within 3 seconds (first click → confirm state with red highlight and shake animation, second click → execute). Auto-resets if not confirmed.

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

## Proxy Stream Stabilization
- **Chunked Transfer EOF Prevention**: The roadcaster_pumper filters out empty  -length byte chunks from the upstream M3U provider. This prevents the Axum HTTP response from interpreting empty chunks as chunked transfer encoding EOF signals, which previously caused mpegts.js to silently freeze on the last decoded frame without throwing an error.
- **Diagnostic Pipeline**: The fprobe and fmpeg stream checkers use explicitly injected HTTP headers (-headers "User-Agent: VLC/3.0.0\r\n") instead of the unreliable -user_agent flag. This ensures universal compatibility across all demuxers (HTTP and HLS) and prevents upstream providers from blocking the diagnostic requests.
