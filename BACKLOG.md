# Dispatcharr Backlog

Items queued for future implementation. Add notes and priority as needed.

---

## UI

- [ ] **Finalize SvelteKit Migration**
  Complete the transition from the legacy React/Angular interfaces to the unified SvelteKit frontend. Priority areas include finalizing the VOD infinite scroll, DVR scheduling UI, and the Plugin management interface.

---

## Backend / Rust

- [ ] **HLS Streaming Endpoint**
  Implement `/api/streams/:id?format=hls` to transcode or repackage MPEG-TS streams into HLS for better mobile/native compatibility. Use `ffmpeg-sidecar` for the process.
- [ ] **VOD Category Filtering**
  Update `get_vod_movies` and `get_vod_series` to support filtering by `category_id`.

---

## Testing & QA

- [ ] **Automated Settings Validation**
  Implement backend integration tests to verify the persistence and retrieval of all `core_settings` categories. Add Playwright E2E tests for the Settings UI to ensure live reactivity (e.g. table size, date formatting).
- [ ] **Media Pipeline Testing**
  Develop a testing suite for the Proxy, Cache Engine, and DVR/Comskip workflows once the Media Player is finalized. This should include stress tests for multiplexing and failover scenarios.
