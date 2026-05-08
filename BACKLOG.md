# Dispatcharr Backlog

Items queued for future implementation and audit phases.

---

## Phase 2: Data Ingestion & Sources (Providers)

- [ ] **Provider Management Audit**
  Verify creation, editing, and deletion forms for M3U/Xtream providers. Ensure all backend fields (timeouts, buffer sizes) are correctly mapped.
- [ ] **Refresh Status Indicators**
  Implement/verify UI indicators for last sync time and active stream counts per provider on the `/streams` page.
- [ ] **Bulk Sync Triggering**
  Ensure the "Refresh All" button correctly triggers the backend background workers and provides live feedback.

---

## Phase 3: Processing & Mapping (Channel Manager & Stream Checker)

- [ ] **Stream Checker: Extended Performance Testing**
  Add an option to run longer tests (2-5 minutes) to monitor for buffering or bit-rate drops after the initial connection.
- [ ] **Stream Checker: Static/Fake Stream Detection**
  Integrate FFmpeg/FFprobe filters to detect frozen images or black screens to identify non-functional but "active" streams.
- [ ] **Stream Checker: Auto-Pruning**
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

- [ ] **UI Polish: Navigation Icons**
  Update the sidebar navigation to use more appropriate icons (e.g., change Stream Checker to `ticket-check`).
- [ ] **Finalize SvelteKit Migration**
  Complete the remaining UI skeletons for DVR scheduling and the Plugin management interface.
- [ ] **EPG Timeline Accuracy**
  Audit vertical scrolling synchronization and the "Jump to Now" behavior in the TV Guide.
- [ ] **TMDB VOD Integration**
  Verify metadata retrieval for VOD posters and descriptions using TMDB IDs.
- [ ] **Multi-Browser Playback Stress Test**
  Conduct a final stress test of the Hybrid Video Player across Safari, Chrome, and Firefox to resolve any remaining "Format Unsupported" issues.
- [ ] **HLS Streaming Endpoint**
  Implement `/api/streams/:id?format=hls` for better mobile/native compatibility.
- [ ] **VOD Category Filtering**
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
