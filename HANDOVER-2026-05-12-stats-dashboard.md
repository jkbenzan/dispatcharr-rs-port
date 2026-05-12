# Handover — 2026-05-12 Stats Dashboard Implementation

## Session Objective

Implement the Active Connections (Stats) dashboard as a tab on the Dashboard page, with live polling, connection cards, and stop controls.

## What Was Done

### Backend (`src/proxy.rs`, `src/main.rs`)

1. **Enriched `handle_ts_status`**: Batch-fetches channel name, number, and logo URL for all active channel UUIDs in a single DB query. Non-UUID keys (stream hashes) are safely skipped. The response now includes `channel_name`, `channel_number`, and `logo_url` fields per channel.

2. **`handle_stop_channel`** (`DELETE /proxy/ts/stop/:channel_id`): Removes all client tracking, removes the broadcaster from the dashmap, and aborts the pumper task.

3. **`handle_stop_client`** (`DELETE /proxy/ts/stop/:channel_id/:client_id`): Removes a single client, decrements subscriber count, and cleans up empty channel entries.

4. All routes registered with both trailing-slash and non-trailing-slash variants.

### Frontend

1. **Dashboard Tab System** (`svelte-frontend/src/routes/+page.svelte`): Added Overview and Active Connections tabs. The connections tab starts polling on activation and stops on deactivation.

2. **ConnectionCard Component** (`svelte-frontend/src/lib/components/stats/ConnectionCard.svelte`): Dark card with:
   - Channel logo/name/number + LIVE pulse indicator
   - Uptime, data transferred, client count stats bar
   - Profile badges (stream profile, M3U provider)
   - Client table with per-client disconnect buttons
   - Two-click "Stop Channel" button with confirmation

3. **Sidebar Shortcut** (`+layout.svelte`): Green Radio icon in sidebar footer links to `/#connections`.

4. **API Methods** (`api.ts`): `getActiveConnections()`, `stopChannel()`, `stopClient()`.

### Documentation

- `ARCHITECTURE.md`: New "Active Connections (Stats Dashboard)" section
- `BACKLOG.md`: Stats item marked complete
- This handoff document

## Build Status

- `cargo check`: ✅ Clean
- `svelte-check`: ✅ 0 errors, 2 pre-existing a11y warnings (unrelated)

## Known Limitations / Phase 2

- **No bitrate history graphs** — would require tracking `total_bytes` deltas over time
- **No EPG "Now Playing" overlay** — needs `get_current_programs` integration
- **No VOD connection cards** — backend returns empty stub
- **No stream switch dropdown** — no endpoint to change active stream mid-broadcast
- **Sidebar shortcut uses hash routing** (`/#connections`) — SvelteKit doesn't natively handle hash-based tab state; the Dashboard page checks `window.location.hash` on mount

## Pending Backlog Items (Priority Order)

1. Stream Checker ↔ Channel Manager Integration
2. ~~Stream Checker Event Logging~~ ✅ Completed (commit `ff9a43b`)
3. ~~Settings Change Events~~ ✅ Completed (commit `ff9a43b`)
4. ~~Sorting Rule Change Events~~ ✅ Completed (commit `ff9a43b`)

## Advanced Logging (commit `ff9a43b`)

Added 9 `record_event()` calls across 2 backend files:

**Stream Checker** (`checker.rs`):
- `stream_check_completed` (single check, `is_single: true` for Activity page filtering)
- `bulk_check_started` / `bulk_check_completed` / `bulk_check_cancelled`

**Sorting Rules** (`checker.rs`):
- `sorting_rule_created` / `sorting_rule_updated` / `sorting_rule_deleted`

**Settings** (`settings.rs`):
- `settings_created` / `settings_updated` (values omitted from payload for security)

**Frontend** (`activity/+page.svelte`):
- All 9 event types render with structured human-readable summaries
