# Handover: Streams Provider Refresh and Category Country Audit

Date: 2026-05-11
Branch: `develop`
Latest pushed commit before this handover: `5cb0a3f Stabilize stream provider refresh UI`

## What Was Shipped

- Replaced native provider confirmation prompts with in-app confirmation dialogs.
- Improved provider activity messages so refresh events are readable instead of only showing "Raw Event Data".
- Added Streams provider-card refresh progress bars with elapsed time and ETA.
- Added immediate queued progress state so refresh cards show activity before backend progress events arrive.
- Kept M3U/XC provider cards in stable alphabetical order with keyed Svelte loops.
- Hid the built-in `Custom` provider from Streams provider-card management.
- Excluded `Custom` from failed M3U provider counts.
- Returned `queued_account_ids` from bulk M3U refresh so the UI only marks accepted providers as queued.
- Split provider save and category/VOD selection save behavior so selection saves keep the modal open and queue refreshes.
- Updated `ARCHITECTURE.md` for the provider-card, progress, custom-provider, and country-filter behavior.

## User Testing Signal

The user confirmed the same configured providers work in original Dispatcharr and return new streams when new categories are added. That strongly suggests any remaining "refresh completes but stream count does not increase" issue is in this Rust/Svelte port's category-selection, mapping, or import path rather than in the upstream provider data.

## Country/Region Audit

The app database currently points at:

- `DATABASE_URL=postgres://...@192.168.0.36:5432/dispatcharr-rs`

Do not print credentials in chat output.

The audit queried non-custom M3U/XC provider category mappings from:

- `m3u_m3uaccount`
- `dispatcharr_channels_channelgroupm3uaccount`
- `dispatcharr_channels_channelgroup`

Audit totals:

- Total configured provider/category mappings: `8,821`
- Current frontend alias detector matches: `1,960` / `8,821` = `22.2%`
- Conservative structured-prefix enhancement estimate: `63.2%`
- Loose structured-prefix estimate: about `75.3%`, but this is too risky without ambiguity handling.

Important ambiguity:

- `AR | ...` often means Arabic, not Argentina.
- `AF | ...` often means Africa, not Afghanistan.
- `CH | ...` can mean China or Switzerland depending on provider.
- `IR | ...` can mean Ireland or Iran depending on provider.
- `IS | ...` can mean Israel or Iceland depending on provider.
- `IN | ...` can mean India, but some providers use regional language buckets.
- `LA`, `EU`, `LAT`, and `EXYU` are regional buckets, not countries.

Current detector limitation:

- `M3UProviderModal.svelte` uses a small hardcoded `COUNTRY_OPTIONS` alias list and `normalized.includes(alias)`.
- It catches full words like `United States`, `France`, and `Germany`.
- It misses dominant provider formats like `US |`, `|US|`, `┃DE┃`, `UK |`, and other compact prefixes.

## Per-Provider Strict Audit

Strict audit means current detector plus conservative structured prefixes only.

| Provider | Categories | Current Match | With Safe Prefixes |
|---|---:|---:|---:|
| B1G | 195 | 22.6% | 44.6% |
| Beagle | 730 | 10.0% | 68.9% |
| Dino | 346 | 32.7% | 53.2% |
| Dream | 812 | 13.1% | 71.7% |
| Eagle | 493 | 13.4% | 65.3% |
| Mega 2 | 318 | 58.8% | 58.8% |
| MEGA IPTV | 318 | 58.8% | 58.8% |
| Promax | 516 | 13.4% | 13.6% |
| Prosat | 276 | 11.2% | 46.4% |
| Strong 2 | 874 | 17.6% | 69.6% |
| Strong12 | 874 | 17.6% | 69.6% |
| Tivi1 | 709 | 28.3% | 71.1% |
| Trex | 808 | 24.9% | 68.1% |
| ULTRA | 776 | 24.1% | 67.8% |
| ULTRA 2 | 776 | 24.1% | 67.8% |

## Recommended Next Implementation Slice

1. Move country/category detection out of the modal into a shared utility, for example:
   - `svelte-frontend/src/lib/utils/categoryDetection.ts`
2. Return a structured classification:
   - `kind: 'country' | 'region' | 'sports' | 'event' | 'entertainment' | 'unknown'`
   - `code`
   - `label`
   - `confidence`
   - `reason`
3. Add conservative prefix parsing before alias matching.
4. Add explicit region handling for `EU`, `LAT`, `EXYU`, `AFR`, `ASIA`, and similar provider buckets.
5. Add non-country category filters for Sports, PPV, Live Events, League packages, 24/7, Movies, Kids, News, and Music.
6. Add tests with real examples from the audit:
   - `US | ABC`
   - `┃DE┃ PREMIUM HORROR`
   - `UK | Premier Sports GB`
   - `AR | beIN Entertainment`
   - `AF | DSTV Africa`
   - `LAT| MEXICO`
   - `LIVE | PPV Events`
   - `SPORTS | Tennis Channel`
7. Keep ambiguous prefixes from mapping to a country unless the rest of the text confirms it.

## Remaining Risk

The provider refresh UI is now much more visible, but the stream-import count issue still needs a backend-focused pass. The highest-value trace is:

1. Save a category selection in the modal.
2. Confirm `update_m3u_group_settings` updates `dispatcharr_channels_channelgroupm3uaccount.enabled`.
3. Confirm the refresh is queued for the selected account.
4. During `m3u.rs` import, confirm the selected group is not in `disabled_group_ids`.
5. Confirm rows are inserted into `streams_stream` with the expected `channel_group_id`.
6. Confirm provider `stream_count` reflects those inserted streams after `loadM3uProviders`.

## Backlog Update

Added `Streams Category Country/Region Detection` to `BACKLOG.md` under Phase 4.
