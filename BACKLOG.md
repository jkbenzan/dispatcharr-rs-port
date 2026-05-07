# Dispatcharr Backlog

Items queued for future implementation. Add notes and priority as needed.

---

## UI

- [ ] **Migrate UI to Angular/ Taiga UI**
  Migrate the current React/Vite frontend to Angular. Evaluate Angular 18+ (standalone components, signals). Retain existing API service layer and Mantine-equivalent component library (e.g., Angular Material or PrimeNG).
- [ ] **Channel DB Settings UI**
  During the Angular settings migration, build the form component for `channel_db_settings` (including `download_url` and `auto_check` toggle) and add a new tab to the Settings page.

---

## Backend / Rust

- [ ] **HLS Streaming Endpoint**
  Implement `/api/streams/:id?format=hls` to transcode or repackage MPEG-TS streams into HLS for better mobile/native compatibility. Use `ffmpeg-sidecar` for the process.
- [ ] **VOD Category Filtering**
  Update `get_vod_movies` and `get_vod_series` to support filtering by `category_id`.

---

## Infrastructure

*(none yet)*
