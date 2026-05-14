# End-of-Day Handoff - 2026-05-14

## Session Summary

Video streaming had already been stabilized before this session. Today's work focused on stream sorting and Stream Checker regressions: sorting quality, sorting-rule table access, Stream Checker parity with Channel Manager, visible stream order numbers, and Sorting Rules save/value-picker behavior.

The working branch is `develop`. All completed code changes listed below were committed and pushed to origin.

## Pushed Commits

- `cb1c133` - `Improve stream sorting quality scoring`
- `61e8510` - `Handle unavailable stream sorting rules`
- `4445133` - `Align stream checker channel tree`
- `f859ab2` - `Show stream order in stream checker`
- `e39dbf5` - `Fix sorting rule saves and value pickers`

## What Changed

### Stream Sorting Quality

- New M3U/XC provider priority now defaults to `0`.
- Provider priority remains supported as an optional manual additive bias, but the built-in sort score is intentionally dominated by quality and reliability.
- Bulk stream sorting now scores streams before custom rule modifiers using:
  - online/reachable status,
  - offline/unreachable penalties,
  - consecutive failure penalties,
  - frozen-video and black-screen penalties,
  - resolution height,
  - FPS,
  - bitrate,
  - preferred codecs.
- Untested streams sort behind confirmed-good streams but ahead of known-dead streams.
- Sorting tie handling is deterministic by preserving existing channel-stream order and stream id as stable fallbacks.

### Sorting Rule Resilience

- `list_sorting_rules` now degrades gracefully if `stream_sorting_rule` is unreadable.
- Instead of causing HTTP 500s, the rules endpoint returns an empty rules list and sorting continues with built-in scoring.
- Rule create/update now normalizes and validates payloads server-side:
  - trims strings,
  - rejects empty names/values,
  - rejects unsupported properties/operators,
  - rejects non-numeric values for numeric comparison operators,
  - clamps negative priority to `0`.
- Rule creation assigns ids explicitly instead of relying on the PostgreSQL sequence default. This avoids a likely permission regression where the table has grants but `stream_sorting_rule_id_seq` does not have `USAGE`.

### Sorting Rules UI

- The Stream Checker Sorting Rules form again uses property-specific operators and value controls.
- Most fields now have defined pick lists:
  - `status`: online, offline, frozen, black screen,
  - `reachable`: yes/no,
  - `height` / `resolution_height`: 2160, 1080, 720, 480,
  - `width` / `resolution_width`: 3840, 1920, 1280, 720,
  - `resolution`: common WxH values,
  - `fps`: 60, 50, 30, 25, 24,
  - `video_codec`: h264, hevc, mpeg2video,
  - `audio_codec`: aac, ac3, eac3, mp2,
  - `audio_channels`: 2, 6,
  - `consecutive_failures`: 0, 1, 2, 3.
- `bitrate` remains a numeric input because provider values vary widely.
- The UI normalizes payloads before save so numbers are sent as numbers and text fields are trimmed.

### Stream Checker Channel Tree

- Stream Checker now builds its group/channel/stream tree from the same `getChannels({ page_size: 5000 })` payload used by Channel Manager.
- Expanded channels now enumerate their assigned streams under the channel instead of showing only channel rows.
- After bulk sort completes, Stream Checker reloads the channel tree so persisted order changes are visible immediately.
- Stream rows now show one-based order numbers (`1.`, `2.`, `3.`) before each assigned stream, matching the Channel Manager experience.
- The displayed order uses persisted zero-based `stream.order + 1` when available and falls back to the rendered row index for older payloads.

## Docs Updated

- `ARCHITECTURE.md`
  - Stream sorting behavior, Stream Checker tree behavior, and Sorting Rules save/value-picker behavior were documented.
- `BACKLOG.md`
  - Completed items were checked off under stream sorting and Stream Checker integration.
- `HANDOVER-2026-05-14-stream-sorting-quality.md`
  - Updated throughout the day with the stream sorting and Stream Checker fixes.

## Verification Completed

Backend:

- `cargo test stream_checker::checker::tests`
  - Passed with 10 tests after the Sorting Rules save validation work.
- `cargo check`
  - Passed.
- Earlier in the stream sorting slice:
  - `cargo test stream_checker::checker::tests` passed with 8 tests before the validation tests were added.

Frontend:

- `npm run check`
  - Passed with 0 errors.
  - Existing warnings remain in `svelte-frontend/src/routes/stream-checker/+page.svelte`:
    - clickable non-interactive `div` rows without keyboard handlers/roles,
    - unused CSS selectors for old table/header styles.
- `npm run build`
  - Passed and wrote the static site to `dist`.
  - Existing build warnings remain:
    - same Stream Checker Svelte warnings,
    - large chunk warning,
    - plugin timing warnings.

Database/Runtime Notes:

- A previous direct privilege probe showed the `.env` database URL connects as `gemini`.
- At that point, `stream_sorting_rule` was readable and empty.
- The original runtime error was:
  - `permission denied for table stream_sorting_rule`
- Later save failures may also have been caused by missing sequence permission, so create now avoids depending on the sequence default.

## Not Fully Verified In Browser

The code/build checks passed, but the final Sorting Rules save/value-picker flow was not manually verified in the running browser during this closeout. Tomorrow's first hands-on verification should be:

1. Open Stream Checker.
2. Go to the Sorting Rules tab.
3. Create a rule such as:
   - name: `Prefer online`
   - property: `Status`
   - operator: `==`
   - value: `Online`
   - score modifier: `100`
4. Confirm the row saves and reloads.
5. Edit that rule to a different property with a picker, save, and confirm it persists.
6. Delete the test rule.
7. Expand a channel with multiple streams and confirm order numbers are visible.
8. Run a small channel sort and confirm the stream order updates in Stream Checker after the sort.

## Current Repo State

- `develop` is pushed through commit `e39dbf5`.
- There is one unrelated untracked file:
  - `debug_server.log`
- Do not include `debug_server.log` in future commits unless it becomes intentionally useful.

## Suggested Next Work

1. Browser-verify Sorting Rules create/edit/delete with the live backend.
2. Browser-verify that predefined value pickers cover the rules the user expects.
3. Consider a small read-only score breakdown in the UI for debugging why a stream sorted above another stream.
4. Address existing Stream Checker accessibility warnings separately:
   - convert clickable tree rows to buttons or add keyboard handlers and roles.
5. Consider a proper database migration/grant fix for `stream_sorting_rule` and its sequence if deployment permission issues continue.

## Resume Point

Start tomorrow by verifying the live Stream Checker UI. The highest-risk remaining area is not compilation; it is whether the live app can create/edit/delete Sorting Rules against the real database and whether the restored value pickers match the user's expected rule list.
