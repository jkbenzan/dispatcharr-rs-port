# Dispatcharr-RS Architecture

> **Living Document** — Update this file whenever a significant design decision is made.
> Last updated: 2026-05-04

---

## Overview

Dispatcharr-RS is a Rust rewrite of the Dispatcharr IPTV middleware. The backend is an Axum-based API server, and the frontend is a **dual-framework** setup: a React main app and an Angular channel manager mini-app.

---

## Project Structure

```
dispatcharr-rs-port/
├── src/                        # Rust backend (Axum)
│   ├── main.rs                 # Server setup, routes, background workers
│   ├── api.rs                  # REST API handlers
│   ├── proxy.rs                # Stream proxy / broadcaster
│   ├── middleware.rs            # Network access middleware
│   ├── entities/               # SeaORM entity models
│   └── ...
├── frontend/                   # React frontend (main app, production)
├── angular-frontend/           # Angular 18 channel manager (mini-app)
├── enhancedchannelmanager-main/ # ECM reference project (gitignored, local only)
├── Dispatcharr-main/           # Original Dispatcharr reference (gitignored, local only)
├── dist/                       # Built frontend output (gitignored)
├── Dockerfile                  # Multi-stage build (React + Angular + Rust)
└── ARCHITECTURE.md             # ← You are here
```

---

## Frontend Architecture

### Dual-Frontend Strategy

| Path | Framework | Source | Purpose |
|------|-----------|--------|---------|
| `/` | React | `frontend/` | Main application (dashboard, EPG, M3U, settings, etc.) |
| `/channel-manager/` | Angular 18 + Taiga UI | `angular-frontend/` | Channel manager mini-app (migration in progress) |

**Why two frameworks?** The channel manager is being incrementally migrated from React to Angular 18 with Taiga UI v5. Rather than a big-bang rewrite, it runs as a standalone mini-app at `/channel-manager/` while the rest of the React app remains untouched.

### How it works

- The **Dockerfile** has two Node.js build stages: one for React, one for Angular.
- React's output goes to `dist/` (served at `/`).
- Angular's output goes to `dist/channel-manager/` (served at `/channel-manager/`).
- Angular's `index.html` has `<base href="/channel-manager/">` so routing and assets resolve correctly.
- The Rust backend's `ServeDir::new("dist")` serves both — no special routing needed.

### Angular Channel Manager Components

```
angular-frontend/src/app/
├── channel-manager/
│   ├── channel-manager.component.*    # Orchestrator (resizable two-pane layout only)
│   ├── channels-pane/                 # Left pane: nested Group → Channel → Stream tree
│   │   ├── channels-pane.ts           # Self-contained: data fetching, selection, actions, drag reorder
│   │   ├── channels-pane.html         # Full nested tree template with checkboxes, kebab menus, drag handles
│   │   └── channels-pane.less         # Styles for tree, kebab, drag indicators, hover actions
│   ├── streams-pane/                  # Right pane: independent available stream catalog
│   │   ├── streams-pane.ts            # Fetches all streams, handles selection & drag
│   │   └── streams-pane.html          # Grouped stream list with checkboxes
│   ├── video-player/                  # In-app floating video player
│   │   └── video-player.component.ts  # Angular-native mpegts.js player (live) + HTML5 (VOD)
│   └── channel-list-item/             # Legacy: absorbed into channels-pane template
├── api.service.ts                     # HTTP client for all backend API calls
└── websocket.service.ts               # WebSocket client for real-time updates
```

### Layout & Interaction

- **Resizable panes**: Three-pane architecture. Left (Channels), Middle (Toolbar/Divider), Right (Streams). Default left pane width is 40%, clamped between 15% and 75%.
- **Collapsible Toolbar**: The divider between panes doubles as a collapsible action toolbar. When collapsed, it acts as a standard drag handle. When expanded, it takes up 10% of the screen and displays channel management buttons (e.g., Create Channel).
- **Nested tree**: Left pane renders **Group → Channel → Stream**. Only groups with channels are shown.
- **Expand/collapse**: Per-group and per-channel expand arrows. Global expand all / collapse all buttons in header.
- **Search**: Type-ahead search filters channels by name or number.
- **Group filter**: Multi-select dropdown showing only groups with channels. Acts as a datagrid filter.

### Group Row
- Expand arrow + group name + channel count badge
- **Kebab menu** (⋮) with:
  - **Test Channels**: Collects all stream IDs across all channels in the group, submits to `POST /api/streams/bulk-check/`, and opens the Stream Checker SheetDialog (see below).
- **Retrieval badge**: Pulsing icon appears when the SheetDialog is dismissed while a check is in progress. Click to re-open the sheet.

### Channel Row
- Checkbox + expand arrow + logo (resized to fit) + channel number + channel name + stream count badge
- **Channel logo resolution**: The channel entity stores `logo_id` (FK to `dispatcharr_channels_logo`). The `get_channel_json()` function resolves `logo_id` → `logo_url` by looking up the logo table, injecting the URL directly into the channel JSON for frontend display.
- **Kebab menu** (⋮) with:
  - **Play Channel**: Opens in-app video player at `/stream/{channel_uuid}/`
  - **Test Channel**: Submits channel's streams to `POST /api/streams/bulk-check/` and opens the Stream Checker SheetDialog.
- **Retrieval badge**: Same as group row — pulsing icon for dismissed sheets.

### Stream Row (sub-items under channel)
- Checkbox + enumerated number (1, 2, 3...) + drag handle (≡) + logo + 3-row info cell + hover actions
- **3-row info cell**: Stream name / Stats summary (resolution · codec · bitrate · status) / M3U account name
- **Drag reorder**: Drag handle allows reordering streams within a channel. New order is persisted via `PATCH /api/channels/channels/:id/`
- **Hover actions**: Preview stream (👁 opens in-app player) + Test stream (🔍 via `POST /api/streams/:id/check/`)

### Selection System
- Checkboxes on all channels and streams
- Shift-click range selection on channels
- Group filter acts as a datagrid filter (not selection)

### Drag-and-Drop Stream Assignment
- Streams in the Stream Pane are `draggable="true"` and set `application/json` data with selected stream IDs.
- Channel rows in the Channels Pane accept drops: `(dragover)` / `(dragleave)` / `(drop)` handlers.
- **Visual feedback**: A `box-shadow` inset border highlights the target channel during hover.
- **Duplicate prevention**: Streams already assigned to the target channel are filtered out before assignment.
- **Persistence**: The full stream ID list (existing + new) is sent via `PATCH /api/channels/channels/:id/` with `{ streams: [...] }`.
- **Internal reorder**: Within the Channels Pane, streams can also be reordered within a single channel via drag handles (separate drag type using `text/plain`).

### Stream Pane (Source View)
- **Hierarchy**: M3U Provider → M3U Group → Stream Name.
- **M3U Row**: Displays provider name, fetching status, last updated timestamp, and a manual "Refresh Now" button. All providers are shown even if they have 0 streams.
- **Provider/Group Filter**: Searchable multi-select dropdowns that filter the tree.
- **Group Name Resolution**: `channel_group` is a numeric ID in the stream API response. The frontend resolves it to a human-readable name via `GET /api/channels/groups/`.
- **Stream Preview**: Integrated "Preview Stream" button opens the in-app player.
- **3-Row Info Cell**: Displays Stream Name, (Reserved), and M3U Account Name.
- **Drag Handles**: Each stream row has a grab handle (`drag_indicator`) matching the Channels Pane style.
- **Expansion Persistence**: When the tree is rebuilt (e.g. after drag-and-drop), both panes preserve which groups/channels/M3Us were expanded.

### In-App Video Player
- Angular-native component using `mpegts.js` for live MPEG-TS stream playback
- Native HTML5 `<video>` for VOD content
- Displays as a centered modal overlay with loading spinner and error states
- Used for both "Play Channel" and "Preview Stream" actions

### Stream Checker SheetDialog

The stream checker is presented as a **Taiga UI SheetDialog** (from `@taiga-ui/addon-mobile`) that slides up from the bottom of the screen when any "Test" action is triggered. This replaces the previous approach of switching to the React Stream Checker tab.

- **Two stop levels**: Collapsed (~6rem shows progress bar + label), expanded (~14rem shows stats + workers). Fully draggable to see the live activity log.
- **Uses bulk-check backend**: All testing goes through `POST /api/streams/bulk-check/` which provides parallel testing by M3U provider, respects `stream_checker_parallel_providers` setting, and populates the shared `BulkCheckStatus` (visible in both Angular and React dashboards).
- **Polls** `GET /api/streams/bulk-check/status/` every 1s while running.
- **Cancel button**: Red stop button in the sheet header, visible while a check is running. Calls `POST /api/streams/bulk-check/cancel/` which sets a cooperative `AtomicBool` flag on the backend. Workers check this flag before each stream — the current in-progress ffprobe/ffmpeg call completes, but no new streams start. Shows a "Cancelling..." badge until workers finish.
- **Auto-sort**: After check completes, auto-sorts channels via `POST /api/channels/bulk-sort-streams/`.
- **Dismissible**: User can swipe/drag down to dismiss. A **pulsing retrieval badge** appears on the source row (group or channel) to re-open the sheet.
- **Persistence**: Badge and sheet state persist until another check is started or the component is destroyed (page refresh/reboot).
- **Edge case**: If a bulk check is already running (from React UI or another action), the sheet shows the existing check's progress instead of starting a new one.

### Taiga UI v5 Integration Notes

Taiga UI v5 significantly overhauled its dependency injection and provider system compared to v3/v4. To prevent fatal `NG0201: No provider found` errors during bootstrap:
- **`provideTaiga()`**: Must be included in the `providers` array in `app.config.ts` to supply core tokens like `TUI_OPTIONS`.
- **`@taiga-ui/addon-mobile`**: Required for `TuiSheetDialog` component used in the Stream Checker SheetDialog.
- **Form Inputs**: Certain complex structural components from older Taiga versions (like `<tui-textfield>`) require strict modular imports (`TuiTextfieldModule` or similar textfield providers) which can fail in a purely standalone component tree. As a workaround, standard native HTML `<input>` and `<select>` elements are used in place of `<tui-textfield>` wrappers. They integrate seamlessly with existing CSS classes (`search-input`, `filter-select`) while entirely bypassing the provider crash.

---

## Backend API Routes

> **Critical:** The Angular frontend calls the Rust backend directly. These URL paths must match exactly.

### Channel Endpoints

| Method | Path | Handler | Notes |
|--------|------|---------|-------|
| GET | `/api/channels/channels/` | `get_channels` | Paginated, includes `streams` array. Supports `?search=`, `?channel_group=`, `?ordering=`, `?page_size=` (default 50, max 5000) |
| GET | `/api/channels/channels/summary/` | `get_channels_summary` | Lightweight: id, name, logo_id, channel_number only |
| PATCH/PUT | `/api/channels/channels/:id/` | `update_channel` | Update channel fields including `streams` array |
| GET | `/api/channels/groups/` | `get_channel_groups` | Returns flat array of `{id, name}` |

### Stream Endpoints

| Method | Path | Handler | Notes |
|--------|------|---------|-------|
| GET | `/api/channels/streams/` | `get_streams` | Paginated. Supports `?m3u_account=`, `?channel_group=`, `?search=` |
| GET | `/api/channels/streams/filter-options/` | `get_stream_filter_options` | Filter metadata |

### Other Key Endpoints

| Method | Path | Handler | Notes |
|--------|------|---------|-------|
| GET | `/api/m3u/accounts/` | `get_m3u_accounts` | M3U provider accounts |
| GET | `/api/channels/logos/` | `get_logos` | Channel logos |

### Stream Checker & Sorting Endpoints

| Method | Path | Handler | Notes |
|--------|------|---------|-------|
| POST | `/api/streams/:id/check/` | `test_stream` | Test a single stream (ffprobe + ffmpeg). Returns updated `stream_stats`. |
| POST | `/api/streams/bulk-check/` | `start_bulk_check` | Start checking multiple streams. Body: `{ stream_ids: [] }` |
| GET | `/api/streams/bulk-check/status/` | `get_bulk_check_status` | Poll bulk check progress (is_running, completed, total, workers) |
| POST | `/api/streams/bulk-check/cancel/` | `cancel_bulk_check` | Cooperative cancel — sets `AtomicBool` flag, workers stop before next stream |
| POST | `/api/channels/bulk-sort-streams/` | `bulk_sort_streams` | Sort streams by scoring rules. Body: `{ channel_ids: [] }` |
| GET | `/api/stream-checker/sorting-rules/` | `list_sorting_rules` | List all sorting rules |
| POST | `/api/stream-checker/sorting-rules/` | `create_sorting_rule` | Create a sorting rule |

> ⚠️ **Common pitfall:** The original Django API used `/api/channels/` for channels. The Rust port uses `/api/channels/channels/` (double "channels"). Streams are at `/api/channels/streams/` not `/api/streams/`. Stream _checking_ routes are at `/api/streams/` (single).

---

## Drag and Drop (Stream → Channel Assignment)

The channel manager uses **native HTML5 drag and drop** (not Angular CDK).

### Flow

1. **Drag start** (`streams-pane.ts`): Sets `event.dataTransfer.setData('application/json', JSON.stringify(streamIds))`
2. **Drag over** (`channels-pane.ts`): Calls `event.preventDefault()` to allow drop
3. **Drop** (`channels-pane.ts`): Reads stream IDs from `getData('application/json')`, merges with existing channel streams, calls `PATCH /api/channels/channels/:id/`

### Why native instead of Angular CDK?

Angular CDK's `cdkDropList` is designed for list-to-list transfers within flat containers. The channel manager has a complex nested structure (groups → channels → streams) that CDK couldn't handle without excessive workarounds.

---

## Data Flow: Channel Grouping

The channels pane displays channels organized by group, matching the ECM layout.

1. **Fetch in parallel:** `GET /api/channels/groups/` and `GET /api/channels/channels/summary/`
2. **Client-side grouping:** Channels are grouped by `channel_group_id`, matched against group names
3. **Filter:** Only groups that contain at least one channel are displayed (empty groups are hidden)
4. **Search:** Full-text search across channels by name/number, displaying the matching channels and their corresponding groups.

---

## Docker Build

```dockerfile
# Stage 1a: React frontend (main app)
frontend/ → npm run build → dist/

# Stage 1b: Angular channel manager (mini-app)
angular-frontend/ → npx ng build --base-href /channel-manager/ → /app/dist/browser/ → /app/dist/channel-manager/

# Stage 2: Rust binary
cargo build --release

# Stage 3: Production image
debian:bookworm-slim + binary + both dist/ outputs
```

> ⚠️ **Critical:** The `--base-href /channel-manager/` flag is **required** in the Docker build. Without it, the Angular `index.html` emits `<script src="/main-XXX.js">` (root-relative), which the Rust server resolves to the React SPA fallback, returning HTML instead of JavaScript. The browser then rejects it with a MIME type error and the app shows a black screen.

---

## Future Work / Decisions Pending

- [ ] **Light/dark theme toggle** — CSS currently uses hardcoded dark colors. Extract to CSS variables for theme switching.
- [ ] **Full Angular migration** — Once the channel manager is stable, decide whether to migrate remaining React pages or keep the dual setup.
- [ ] **Bundle size** — Angular bundle exceeds the 500kB warning. Consider lazy loading or chunk splitting.
- [ ] **Toast notifications** — Replace `console.log` confirmations with Taiga UI notification service.
- [ ] **Advanced ECM features** — Bulk CSV import/export, EPG assignment modals, undo/redo history.
- [ ] **HLS DVR Segmentation** — Transition from single-file TS recording to FFmpeg HLS segmentation to enable "Watch While Recording" (parity with Dispatcharr 0.24.0).
- [ ] **Delta Stats Refresh** — Implement a lightweight "since" cursor endpoint for stream stats to reduce payload size and frontend re-renders (parity with Dispatcharr 0.24.0).
- [ ] **Richer EPG Metadata** — Extract Season/Episode (SxxExx) and program subtitles from XMLTV sources (parity with Dispatcharr 0.21.0).
- [ ] **API Key Authentication** — Add support for permanent API keys to allow programmatic access without JWT tokens (parity with Dispatcharr 0.20.0).

---

## Reference Projects

These are local-only reference copies (gitignored):

- **`Dispatcharr-main/`** — Original Python/Django Dispatcharr
- **`enhancedchannelmanager-main/`** — ECM project (React) — the target for channel manager parity

---

## Testing and Deployment Workflow

> **Note:** The user does not build and test locally.

The standardized workflow for testing changes is:
1. **Push** code changes to the Git repository.
2. **Build and Publish** the Docker image to Docker Hub (typically via CI/CD or remote build server).
3. **Update Image** on the Unraid server to pull the latest Docker Hub image and test the changes in the live environment.
