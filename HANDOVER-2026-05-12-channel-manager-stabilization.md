# Handoff Document — 2026-05-11

## Session Summary

This session focused on resolving the Channel Manager regression backlog identified during user testing. All planned phases were completed, verified, and pushed.

---

## What Was Completed (commit `c25b907` on `develop`)

### Files Modified (8 files, 186 insertions)

| File | Changes |
|------|---------|
| `svelte-frontend/src/lib/player.svelte.ts` | **[NEW]** Global Svelte 5 runes store for player state persistence across navigation |
| `svelte-frontend/src/routes/+layout.svelte` | Moved `FloatingPlayer` here so it survives page navigations |
| `svelte-frontend/src/routes/channels/+page.svelte` | Removed local player state; delegates to `playerStore.open()` |
| `svelte-frontend/src/lib/components/channel-manager/ChannelsPane.svelte` | Reduced indentation, added stream enumeration (1., 2., 3.), per-stream Play button |
| `svelte-frontend/src/lib/components/channel-manager/StreamsPane.svelte` | Added provider filter dropdown, global Select All/Deselect All, category grouping via `group_title` |
| `svelte-frontend/src/lib/components/channel-manager/CreateChannelModal.svelte` | Faster debounce (300ms), tooltip on channel data cards, enlarged EPG selector (300px) |
| `src/api.rs` | Backend: surface `group_title` from `custom_properties` in stream API responses |
| `ARCHITECTURE.md` | Documented player persistence, category vs channel group, provider filter, stream enumeration |

### Build Status
- `cargo check` ✅
- `svelte-check` ✅ (0 errors)
- Pushed to `develop`

---

## What Was Completed (late session — Activity Event Coverage)

### Files Modified

| File | Changes |
|------|---------|
| `src/api.rs` | Added 6 `record_event()` calls: `channel_created`, `channel_updated`, `streams_assigned`, `channel_deleted`, `channel_group_created`, `channels_bulk_updated` |
| `ARCHITECTURE.md` | Documented Channel Manager event types in Telemetry & Logging section |
| `BACKLOG.md` | Added Phase 5: Observability & Event Coverage with deferred items (stream checker events, sorting rule events, settings events, stats page, stream checker integration) |

### Build Status
- `cargo check` ✅

---

## What's Queued Next (Priority Order)

### 1. Activity Page — Event Coverage (Small, High Impact)
**Problem**: Channel Manager operations (create/edit/delete channel, assign/remove/reorder streams) produce no system events. The `record_event()` function is never called from channel CRUD handlers.

**Fix**: Add `record_event()` calls in `api.rs` at each channel mutation endpoint. Event types: `channel_created`, `channel_updated`, `channel_deleted`, `stream_assigned`, `stream_removed`, `stream_reordered`.

### 2. Stats / Active Connections Page (Large, High Impact)
**Problem**: No live connection monitoring exists. The Dashboard only shows static counters.

**Reference found**: Original Dispatcharr has a rich Stats page:
- `Dispatcharr/frontend/src/pages/Stats.jsx` (438 lines)
- `Dispatcharr/frontend/src/components/cards/StreamConnectionCard.jsx` (828 lines)
- `Dispatcharr/frontend/src/utils/pages/StatsUtils.js` (159 lines)

**Features to port**: Active connection cards with logo/uptime/bitrate/codec badges, live stream switching, client table with disconnect, EPG program progress, VOD connections, configurable refresh polling.

**Decision needed**: New route (`/stats`) vs embedding in Dashboard. Recommendation: separate `/stats` page.

### 3. Stream Checker ↔ Channel Manager Integration (Medium)
**Problem**: Stream checker works standalone but has no Channel Manager integration.

**Backend is complete**: `test_stream()`, `bulk_sort_streams()`, `internal_bulk_sort_streams()`, sorting rules CRUD, health tracking, automated maintenance — all exist.

**Frontend gaps**:
- No "Check streams" or "Sort by health" buttons per channel in ChannelsPane
- No sorting rules management UI on `/stream-checker`
- No stream health badges in channel sub-lists

### 4. Stream Profile Fallback (from earlier session)
Stream profiles not being honored during proxy relay. Needs backend investigation in `proxy.rs`.

---

## Still-Open User Questions

| Question | Context |
|----------|---------|
| Stats page location | Separate `/stats` route recommended. User to confirm. |
| Sorting rules UI | Should it live on `/stream-checker` or `/settings`? |
| Activity granularity | Should every stream assign/remove be logged, or only bulk operations? |

---

## Key Architecture Notes for Next Session

- **Player state**: `$lib/player.svelte.ts` — global runes store, imported by any page
- **Category vs Channel Group**: "Category" = M3U `group_title` (provider-side), "Channel Group" = user-created containers
- **Stream Checker backend**: `src/stream_checker/checker.rs` — all scoring/sorting/health logic is complete
- **Event system**: `src/events.rs` → `record_event()` — simple insert + FIFO cleanup based on `max_system_events` setting
- **Original Dispatcharr reference**: `Dispatcharr/` directory contains the full React/Mantine codebase for feature reference

---

## Known Issues (from 2026-05-11 testing)

### CreateChannelModal
- **Channel DB lookup false negative**: Search sometimes returns "No results found" too quickly, then finds results on retry. Likely a race condition or timeout issue with the SQLite sidecar query. *Not priority.*
- **Tooltip styling clash**: The native `title` attribute tooltip on channel data cards has no custom styling and visually clashes with the dark UI. Should be replaced with a styled Svelte tooltip component.

### StreamsPane — Category Grouping
- **Some providers still show "Ungrouped"**: Category extraction from `group_title` works for most providers but some streams have no `group_title` in their `custom_properties`. This may be due to providers that don't set `group-title` in their `#EXTINF` lines, or streams that were ingested before the backend fix to surface `group_title`. Needs investigation — may require a provider re-sync to populate missing values.

### General UI
- **Sluggish performance**: Channel Manager UI feels sluggish with large stream/channel counts. No optimization needed right now, but worth noting for future virtual scrolling or pagination work.

---

## Repository State
- Branch: `develop`
- Last commit: `c25b907` — "feat(channel-manager): persistent player, category grouping, stream controls, select-all, provider filter"
- No uncommitted changes
- No open PRs
