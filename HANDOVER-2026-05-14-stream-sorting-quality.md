# Stream Sorting Quality Handoff - 2026-05-14

## Context

Video streaming had been stabilized, and the active troubleshooting focus moved to channel stream ordering from stream sorting rules. The live `stream_sorting_rule` table was empty during investigation, so previous sort runs were effectively provider-priority-only.

## Completed

- Changed new M3U/XC provider priority default from `1` to `0`.
- Added built-in stream scoring in `src/stream_checker/checker.rs` so sorting works even without custom rules.
- Preserved provider priority as an optional additive bias, but quality/reliability weights are intentionally larger by default.
- Added deterministic tie handling by reading existing channel-stream order and using `order`/`id` as stable tie breakers.
- Expanded the Svelte sorting-rule property dropdown to expose the actual saved stream stats: `status`, `reachable`, `height`, `width`, `fps`, `bitrate`, codecs, audio channels, and `consecutive_failures`.
- Kept legacy `resolution_height` and `resolution_width` rule properties working by mapping them to saved `height` and `width` stats with fallback to the old property names.
- Added focused Rust tests for built-in sorting priorities and rule alias behavior.

## Scoring Behavior

The built-in score prioritizes:

1. Reliability: online and reachable streams score highest; known offline streams are heavily penalized.
2. Stability: repeated failures, frozen video, and black-screen checks lower the score.
3. Quality: higher resolution height, FPS, and bitrate increase the score.
4. Codec preference: HEVC/H.265 and H.264/AVC receive small positive modifiers.
5. Manual tuning: provider priority and custom sorting rules remain additive controls.

Untested streams sort behind confirmed-good streams but ahead of known-dead streams.

## Verification

- `cargo test stream_checker::checker::tests` passed with 8 tests.

## Follow-Up

- Run full backend/frontend verification before final merge if more sorting UI changes are added.
- Consider exposing a read-only score breakdown in the UI later if sorting decisions need to be debugged from the browser.
