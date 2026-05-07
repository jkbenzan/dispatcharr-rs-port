# Dispatcharr-RS Architecture

> **Living Document** — Update this file whenever a significant design decision is made.
> Last updated: 2026-05-07

---

## Overview

Dispatcharr-RS is a Rust rewrite of the Dispatcharr IPTV middleware. The backend is an Axum-based API server. The frontend is currently undergoing a major migration from a dual-framework setup (React/Angular) to a unified **SvelteKit** application with a high-performance, **Trakt-inspired** dark aesthetic.

---

## Project Structure

```
dispatcharr-rs-port/
├── src/                        # Rust backend (Axum)
│   ├── main.rs                 # Server setup, routes, background workers
│   ├── api.rs                  # REST API handlers
│   ├── proxy.rs                # Stream proxy / broadcaster
│   ├── entities/               # SeaORM entity models
│   └── ...
├── svelte-frontend/            # NEW: Unified SvelteKit frontend (Active Migration)
├── dist/                       # Built frontend output (gitignored)
├── Dockerfile                  # Multi-stage build
└── ARCHITECTURE.md             # ← You are here
```

---

## Frontend Migration (SvelteKit)

We are pivoting the entire frontend to **SvelteKit** to achieve a premium "Trakt-like" user experience. This work is primarily happening on the `feature/svelte-migration` branch.

### Media Consumption (Upcoming)
- **TV Guide (EPG)**: An absolute-positioned, CSS-grid-based timeline view at `/guide`. Features paginated, lazy-loaded channel/EPG data using Intersection Observers, ensuring we only fetch EPG blocks for channels currently rendered in the viewport, maintaining the "relative time" and efficiency guardrails.
- **VOD**: Trakt-inspired interface at `/vod` supporting infinite scroll pagination and dynamic categorization of Movies and Series. Includes TMDB ID resolving for rich poster metadata.
- **DVR**: A placeholder UI skeleton at `/dvr` outlining upcoming features like Series Pass and Comskip Integration.

### Design System (Trakt Aesthetic)
- **Framework**: SvelteKit 2 + Svelte 5 (Runes).
- **Styling**: Vanilla CSS/LESS with a custom-built design system.
- **Theme/Accent System**: Uses CSS variables bound to `document.documentElement` attributes (`data-theme` and `data-accent`). Fully supports dynamic switching between light/dark modes and configurable brand colors (Trakt Red `#ed1c24` / Dispatcharr Green `#158f76`). Preferences persist via `localStorage`.
- **Components**: Lightweight, native-feeling components (avoiding heavy UI libraries like Taiga).
  - *Primitives*: Reusable primitives like `Modal.svelte` handle backdrop blur, z-indexing, and keyboard escape.
- **Icons**: Lucide-Svelte.

- **Layout Architecture**:
  - **Root Layout**: A fixed sidebar navigation with a responsive main content area. Includes a custom branding header with a "Powered by Rust" flare and a collapsible mode replacing the text with a themeable Dispatcharr logo SVG. Now enhanced with a glassmorphism effect (frosted glass) over a subtle radial gradient.
  - **Channel Manager**: A split-pane interface with independent scrolling for Channels (left) and Streams (right), featuring a custom JS-based resizer.
  - **Create Channel**: A massive 2-panel modal component leveraging the `Modal` primitive, fully integrating the Channel Data sidecar fuzzy lookups, EPG assignments, and Logo uploads without navigating away from the Channel Manager.
  - **M3U / Streams Management**: A grid-based view at `/streams` for managing multiple M3U and XTREAM Codes providers. Includes robust creation/editing forms and handles bulk refresh operations.
  - **Stream Checker**: A diagnostic dashboard at `/stream-checker` utilizing the backend bulk check workers. It features live progress polling, dynamic stream selection via providers, and real-time visualization of `ffprobe` and `ffmpeg` results.
  - **Global Settings**: A modular sidebar-driven interface at `/settings` directly reading/writing to the backend `core_settings` table, mapping configuration JSON structures into bespoke UI controls (e.g. DVR padding, IP network CIDRs, Proxy failover thresholds).
  - **Activity & Logs**: A real-time terminal interface at `/activity` streaming live system events from the WebSocket backend, complete with a hybrid JSON-viewer for inspecting raw payload details.
  - **Integrations & Plugins**: Placeholder UI skeletons at `/integrations` and `/plugins` for future webhook, API token, and custom parser management.
- **State Management**: Reactive stores using Svelte 5 `$state` and `$effect` for real-time WebSocket events and system status.

### Build & Serving
- **Adapter**: `@sveltejs/adapter-static` configured in SPA mode.
- **Output**: Generates to the root `dist/` folder.
- **Rust Integration**: Served by Axum via `ServeDir::new("dist")` with `index.html` fallback for client-side routing.

---

## Backend Architecture

### Streaming Engine (Broadcaster)
The backend has evolved from a direct-pipe proxy to a multiplexing broadcaster:
- **Multiplexing**: Multiple users watching the same channel share a single upstream connection.
- **Failover**: Automatic stream failover based on user-defined sorting rules.
- **Caching**: Ring-buffer implementation for instant playback starts.

### Channel Data Sidecar
Enriches channels with metadata via a read-only SQLite database (`channel_data.db`).
- **Fuzzy Matching**: Jaro-Winkler string similarity for matching local channels to station metadata.
- **Graceful Degradation**: 503 response if the sidecar DB is missing or corrupted.

---

## Logo Management
- **Sync**: Background task clones/pulls a GitHub logo repository.
- **Resolution**: `get_channel_json()` resolves FKs to public URLs for the frontend.

---

## Docker Build Process
The build is being updated to prioritize the SvelteKit output:
1. **SvelteKit Stage**: `npm run build` outputs to `dist/`.
2. **Rust Stage**: `cargo build --release`.
3. **Final Stage**: Combines the binary with the `dist/` folder.

---

## Active Branch: `feature/svelte-migration`
Current focus: Porting the Channel Manager (Groups, Channels, Streams) and the Create Channel Dialog into Svelte components while maintaining functional parity with the legacy Angular/React versions.
