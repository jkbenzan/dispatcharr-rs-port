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
  - [x] Integrate `m3u_account.priority` as the base score for stream sorting.
  - [x] Refactor `bulk_sort_streams` logic to support priority-first selection.

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
- [ ] **Stream Checker Diagnostics**
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
- [ ] **GitHub Logo Library Sync**
  Implement a background task to clone and periodically pull external logo repositories (e.g., `iptv-org/logos`) to provide a searchable library for channel icon procurement.

---

## Integrations

- [ ] **Trakt.tv Integration**
  Explore and implement integration with Trakt.tv for scrobbling live TV/VOD playback, syncing watch history, and importing user lists/collections into the VOD dashboard.
