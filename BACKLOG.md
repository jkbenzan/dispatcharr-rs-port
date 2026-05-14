# Dispatcharr Backlog

Items queued for future implementation and audit phases.

---

## Completed API Parity Work [DONE]

- [x] **Core Notifications API**
  Replaced mock notification handlers with database-backed endpoints for active notifications and notification counts.
- [x] **Notification Visibility Rules**
  Filter notifications by active status, expiry, dismissals, and admin-only visibility for non-admin users.
- [x] **Notification Response Shape**
  Return DRF-compatible list responses with `count`, `next`, `previous`, and `results`, plus `{ "count": X }` for the count endpoint.

---

## Phase 2: Data Ingestion & Streaming Infrastructure [DONE]

- [x] **Background Worker Hardening**:
  - [x] Implement randomized jitter (±30 mins) for M3U/EPG refreshes.
  - [x] Implement staggered sequential processing (60s delay) between providers.
- [x] **Health Monitoring & Dashboard**:
  - [x] Surface failed provider counts in dashboard stats.
  - [x] Add visual health alerts and "Refresh All" troubleshooting buttons.
- [x] **Intelligent Scoring Engine**:
  - [x] Integrate `m3u_account.priority` as an optional manual bias for stream sorting.
  - [x] Refactor `bulk_sort_streams` logic to prioritize reliability and quality before provider priority.
  - [x] Default new M3U/XC provider priority to `0` so built-in stream health drives ordering unless explicitly overridden.
  - [x] Add built-in sorting score for reachable/online status, resolution height, FPS, bitrate, codecs, failure count, frozen video, and black-screen detection.
  - [x] Keep custom sorting rules additive for special-case tuning and map legacy `resolution_height` / `resolution_width` rules to saved `height` / `width` stats.
  - [x] Treat the sorting-rule table as optional at runtime so permission drift falls back to built-in scoring instead of breaking channel views or bulk sort requests.

---

## Phase 3: Processing & Mapping (Channel Manager & Stream Checker)

- [x] **Stream Checker: Extended Performance Testing**
  Add an option to run longer tests (2-5 minutes) to monitor for buffering or bit-rate drops after the initial connection.
- [x] **Stream Checker: Static/Fake Stream Detection**
  Integrate FFmpeg/FFprobe filters to detect frozen images or black screens to identify non-functional but "active" streams.
- [x] **Stream Checker: Auto-Pruning**
  Implement logic to automatically delete or disable streams that consistently fail health checks or are flagged as fake.
- [ ] **Channel Manager UX Refinement**
  Verify drag-and-drop assignments between the Stream Pane and Channel Pane. Ensure smooth performance with large lists.
- [ ] **Group & Bulk Operations**
  Audit group management logic and bulk stream assignments to channels.
- [ ] **Logo & Metadata Resolution**
  Audit the fuzzy matching logic and logo URL resolution from the `channel_data.db` sidecar.
- [x] **Stream Checker Diagnostics**
  Ensure real-time `ffprobe` and `ffmpeg` results are correctly visualized in the diagnostic dashboard.

---

## Phase 4: Consumption Interfaces (Playback & UI)

- [x] **UI Polish: Navigation Icons**
  Update the sidebar navigation to use more appropriate icons (e.g., change Stream Checker to `ticket-check`).
- [x] **Finalize SvelteKit Migration**
  Complete the remaining UI skeletons for DVR scheduling and the Plugin management interface.
- [x] **EPG Timeline Accuracy**
  Audit vertical scrolling synchronization and the "Jump to Now" behavior in the TV Guide.
- [ ] **TMDB VOD Integration**
  Verify metadata retrieval for VOD posters and descriptions using TMDB IDs.
- [ ] **Multi-Browser Playback Stress Test**
  Conduct a final stress test of the Hybrid Video Player across Safari, Chrome, and Firefox to resolve any remaining "Format Unsupported" issues.
- [x] **HLS Streaming Endpoint**
  Implement `/api/streams/:id?format=hls` for better mobile/native compatibility.
- [x] **VOD Category Filtering**
  Update backend `get_vod` handlers to support filtering by `category_id`.
- [ ] **Streams Tab Refresh UX**
  Add a toggleable alternate grid view that keeps the same provider actions, metadata, and progress indicators while reducing card crowding.
- [ ] **Streams Category Country/Region Detection**
  Replace the current small alias-only category detector with a data-backed classifier that handles provider prefix formats such as `US |`, `|US|`, and `┃DE┃`, while keeping ambiguous prefixes like `AR`, `AF`, `CH`, `IR`, `IS`, `IN`, `LA`, `EU`, `LAT`, and `EXYU` out of unsafe country matches. Add separate labels/filters for non-country clusters such as Sports, PPV, Live Events, League packages, 24/7, and Movies. The May 11 audit of 8,821 configured category mappings showed the current UI detector matching only 22.2%, improving to 63.2% with conservative structured-prefix handling.

---

## Phase 5: Observability & Event Coverage

- [x] **Channel CRUD Events**
  Add `record_event()` calls to `create_channel`, `update_channel`, `delete_channel`, `create_channel_group`, and `bulk_update_channels` so Channel Manager operations appear in the Activity page.
- [x] **Stream Checker Event Logging**
  Emits `stream_check_completed` (with `is_single` flag for filtering), `bulk_check_started`, `bulk_check_completed`, and `bulk_check_cancelled` events.
- [x] **Sorting Rule Change Events**
  Emits `sorting_rule_created`, `sorting_rule_updated`, `sorting_rule_deleted` on success paths.
- [x] **Settings Change Events**
  Emits `settings_updated` and `settings_created` events. Values omitted from payload for security.
- [x] **Stats / Active Connections Page**
  Implemented as a Dashboard tab ("Active Connections") with connection cards, live polling (configurable interval), stop channel/client controls, and a sidebar shortcut icon. Phase 2 (bitrate graphs, EPG overlay, VOD cards) deferred.
- [x] **Stream Checker ↔ Channel Manager Integration**
  Sorting Rules management UI added to `/stream-checker` (Sorting Rules tab). ChannelsPane now shows stream health via row shading (red for offline, gray dot for untested) and condensed/expanded performance stats. Kebab menu includes "Sort by Health" action. Frontend API extended with full CRUD for sorting rules and bulk sort trigger.
  - [x] Align Stream Checker channel tree loading with Channel Manager so streams enumerate under expanded channels from the same `getChannels({ page_size: 5000 })` payload and order.
  - [x] Show one-based stream order numbers in Stream Checker rows so the visible ordering matches Channel Manager.

---

## Testing & QA

- [ ] **Automated Settings Validation**
  Implement backend integration tests for `core_settings` persistence and Playwright E2E tests for UI reactivity.
- [ ] **Media Pipeline Testing**
  Develop a testing suite for Proxy, Cache Engine, and DVR/Comskip workflows once the Media Player is finalized.

---

## Infrastructure

- [ ] **Database Initialization Script**
  Create a DB initialization script to automate schema setup and initial permission configuration for new installs.
- [x] **GitHub Logo Library Sync**
  Implement a background task to clone and periodically pull external logo repositories (e.g., `iptv-org/logos`) to provide a searchable library for channel icon procurement.

---

## Integrations

- [ ] **Trakt.tv Integration**
  Explore and implement integration with Trakt.tv for scrobbling live TV/VOD playback, syncing watch history, and importing user lists/collections into the VOD dashboard.
