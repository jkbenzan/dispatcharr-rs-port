# Dispatcharr-RS Architecture

> **Living Document** — Update this file whenever a significant design decision is made.
> Last updated: 2026-05-03

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
│   ├── channel-manager.component.*    # Orchestrator (resizable two-pane layout)
│   ├── channels-pane/                 # Left pane: grouped channel list
│   │   ├── channels-pane.ts           # Fetches groups + full channels (with streams), builds grouped views
│   │   ├── channels-pane.html         # Search, filtered group dropdown, expandable groups
│   │   └── channels-pane.less         # ECM-matching styles
│   ├── streams-pane/                  # Right pane: assigned + available streams
│   │   ├── streams-pane.ts            # Shows assigned streams for selected channel, fetches available streams
│   │   └── streams-pane.html          # Assigned streams section + grouped available stream list with checkboxes
│   └── channel-list-item/             # Reusable channel row component
├── api.service.ts                     # HTTP client for all backend API calls
└── websocket.service.ts               # WebSocket client for real-time updates
```

### Layout & Interaction

- **Resizable panes**: The two-pane layout uses a draggable divider. Default split is **40% channels / 60% streams**. The divider can be dragged between 15% and 75%.
- **Assigned streams**: Each channel in the left pane has an expand arrow. Clicking it reveals the streams currently assigned to that channel as sub-items below the channel row. The streams pane (right) is fully independent and always shows the full available stream catalog.
- **Group filter**: The group dropdown in the channels pane only shows groups that have at least one channel assigned. Empty groups are excluded to reduce noise.

### Taiga UI v5 Integration Notes

Taiga UI v5 significantly overhauled its dependency injection and provider system compared to v3/v4. To prevent fatal `NG0201: No provider found` errors during bootstrap:
- **`provideTaiga()`**: Must be included in the `providers` array in `app.config.ts` to supply core tokens like `TUI_OPTIONS`.
- **Form Inputs**: Certain complex structural components from older Taiga versions (like `<tui-textfield>`) require strict modular imports (`TuiTextfieldModule` or similar textfield providers) which can fail in a purely standalone component tree. As a workaround, standard native HTML `<input>` and `<select>` elements are used in place of `<tui-textfield>` wrappers. They integrate seamlessly with existing CSS classes (`search-input`, `filter-select`) while entirely bypassing the provider crash.

---

## Backend API Routes

> **Critical:** The Angular frontend calls the Rust backend directly. These URL paths must match exactly.

### Channel Endpoints

| Method | Path | Handler | Notes |
|--------|------|---------|-------|
| GET | `/api/channels/channels/` | `get_channels` | Paginated, includes `streams` array. Supports `?search=`, `?channel_group=`, `?ordering=` |
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

> ⚠️ **Common pitfall:** The original Django API used `/api/channels/` for channels. The Rust port uses `/api/channels/channels/` (double "channels"). Streams are at `/api/channels/streams/` not `/api/streams/`.

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
