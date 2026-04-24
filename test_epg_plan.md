1. **Edit `src/api.rs`**:
   - Add `get_epg_grid` function to handle GET requests for `/api/epg/grid/`. This function will fetch programs from `now - 1h` to `now + 24h` and generate standard and custom dummy programs for channels without data or with a dummy EPG source.
   - Add `get_current_programs` function to handle POST requests for `/api/epg/current-programs/`. This function will fetch currently airing programs for the provided `channel_uuids` (or all channels if null).
2. **Edit `src/main.rs`**:
   - Add the routes `.route("/api/epg/grid/", get(api::get_epg_grid))` and `.route("/api/epg/current-programs/", post(api::get_current_programs))` to the router.
3. **Verify Code Edits**:
   - Use `cargo check` to verify that `src/main.rs` and `src/api.rs` compile without syntax or type errors.
4. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
5. **Run the relevant tests (e.g., `cargo test`) to ensure the new EPG endpoints work and no regressions were introduced.**
