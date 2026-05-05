//! Channel name parsing and fuzzy match scoring.
//!
//! This module ports the channel name parsing and match scoring logic from the
//! ChannelIdentifiarr Python project into idiomatic Rust. It provides:
//!
//! - `parse_channel_name()` — extracts country, resolution, and a cleaned name
//!   from raw IPTV channel names (e.g. "US: ESPN HD" → clean="ESPN", country="USA", resolution="HDTV")
//! - `calculate_match_score()` — computes a 0.0–1.0 similarity score between a
//!   parsed channel name and a Gracenote station record using Jaro-Winkler distance
//!   plus contextual bonuses for resolution, country, and logo presence.

use regex::Regex;
use serde::Serialize;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Result of parsing a raw channel name.
#[derive(Debug, Clone, Serialize)]
pub struct ParsedChannelName {
    /// The cleaned channel name with country, resolution, and noise removed.
    pub clean_name: String,
    /// ISO 3166-1 alpha-3 country code detected in the name, if any.
    pub country: Option<String>,
    /// Detected resolution category: "HDTV", "SDTV", or "UHDTV".
    pub resolution: Option<String>,
    /// The original, unmodified channel name.
    pub original: String,
}

/// Minimal station data needed for match scoring.
/// Matches the shape returned by `channel_db::StationSearchResult`.
pub struct StationForScoring<'a> {
    pub name: Option<&'a str>,
    pub call_sign: Option<&'a str>,
    pub video_types: Option<&'a str>,
    pub country: Option<&'a str>,
    pub has_logo: bool,
}

// ---------------------------------------------------------------------------
// Country detection table
// ---------------------------------------------------------------------------

/// (canonical_code, patterns_to_match)
/// Order matters: first match wins, so put longer/more-specific patterns before
/// shorter ambiguous ones (e.g. "AUSTRALIA" before "AU").
const COUNTRY_PATTERNS: &[(&str, &[&str])] = &[
    ("USA", &["US", "USA", "UNITED STATES"]),
    ("GBR", &["UK", "GBR", "BRITAIN", "ENGLAND"]),
    ("CAN", &["CA", "CAN", "CANADA"]),
    ("AUS", &["AU", "AUS", "AUSTRALIA"]),
    ("DEU", &["DE", "DEU", "GERMANY", "DEUTSCH"]),
    ("FRA", &["FR", "FRA", "FRANCE", "FRENCH"]),
    ("ITA", &["IT", "ITA", "ITALY", "ITALIAN"]),
    ("ESP", &["ES", "ESP", "SPAIN", "SPANISH"]),
    ("NLD", &["NL", "NLD", "NETHERLANDS", "DUTCH"]),
    ("BEL", &["BE", "BEL", "BELGIUM", "BELGIAN"]),
    ("CHE", &["CH", "CHE", "SWITZERLAND", "SWISS"]),
    ("AUT", &["AT", "AUT", "AUSTRIA", "AUSTRIAN"]),
    ("SWE", &["SE", "SWE", "SWEDEN", "SWEDISH"]),
    ("NOR", &["NO", "NOR", "NORWAY", "NORWEGIAN"]),
    ("DNK", &["DK", "DNK", "DENMARK", "DANISH"]),
    ("FIN", &["FI", "FIN", "FINLAND", "FINNISH"]),
    ("JPN", &["JP", "JPN", "JAPAN", "JAPANESE"]),
    ("KOR", &["KR", "KOR", "KOREA", "KOREAN"]),
    ("CHN", &["CN", "CHN", "CHINA", "CHINESE"]),
    ("IND", &["IN", "IND", "INDIA", "INDIAN"]),
    ("BRA", &["BR", "BRA", "BRAZIL", "BRAZILIAN"]),
    ("MEX", &["MX", "MEX", "MEXICO", "MEXICAN"]),
    ("ARG", &["AR", "ARG", "ARGENTINA", "ARGENTINIAN"]),
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Check if `word` appears as a whole word in `text` (case-insensitive).
fn word_exists(text: &str, word: &str) -> bool {
    // Build a pattern like \bWORD\b — since Rust regex doesn't have \b for
    // Unicode by default, we use a simple approach with word-char boundaries.
    let pattern = format!(r"(?i)\b{}\b", regex::escape(word));
    Regex::new(&pattern)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

/// Remove the first whole-word occurrence of `word` from `text` (case-insensitive)
/// and normalize whitespace.
fn remove_word(text: &str, word: &str) -> String {
    let pattern = format!(r"(?i)\b{}\b", regex::escape(word));
    let result = Regex::new(&pattern)
        .map(|re| re.replace(text, " ").to_string())
        .unwrap_or_else(|_| text.to_string());
    normalize_whitespace(&result)
}

/// Collapse runs of whitespace into single spaces and trim.
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------------------
// Core functions
// ---------------------------------------------------------------------------

/// Parse a raw channel name to extract country, resolution, and produce a
/// cleaned version suitable for matching against the Gracenote database.
///
/// # Examples
/// ```
/// let parsed = parse_channel_name("US: ESPN HD");
/// assert_eq!(parsed.clean_name, "ESPN");
/// assert_eq!(parsed.country, Some("USA".to_string()));
/// assert_eq!(parsed.resolution, Some("HDTV".to_string()));
/// ```
pub fn parse_channel_name(channel_name: &str) -> ParsedChannelName {
    let original = channel_name.to_string();
    let mut clean = channel_name.to_uppercase();
    let mut detected_country: Option<String> = None;
    let mut detected_resolution: Option<String> = None;

    // ── Step 0: Replace special character separators with spaces ──────────
    let separator_re = Regex::new(r"[\\|★◉:►▶→»≫—–=〉〈⟩⟨◆♦◊⬥●•.]").unwrap();
    if separator_re.is_match(&clean) {
        clean = separator_re.replace_all(&clean, " ").to_string();
        clean = normalize_whitespace(&clean);
    }

    // ── Step 1: Country detection ─────────────────────────────────────────
    'country: for &(code, patterns) in COUNTRY_PATTERNS {
        for &pat in patterns {
            if word_exists(&clean, pat) {
                detected_country = Some(code.to_string());
                clean = remove_word(&clean, pat);
                break 'country;
            }
        }
    }

    // ── Step 2: Resolution detection ──────────────────────────────────────
    let ultra_hd_re = Regex::new(r"(?i)Ultra\s*HD").unwrap();
    let numeric_res_re = Regex::new(r"\b(1080[ip]?|720[ip]?)\b").unwrap();
    let sd_res_re = Regex::new(r"\b480[ip]?\b").unwrap();

    if ["4K", "UHD", "UHDTV"].iter().any(|t| word_exists(&clean, t))
        || ultra_hd_re.is_match(&clean)
    {
        detected_resolution = Some("UHDTV".to_string());
        for term in &["4K", "UHD", "UHDTV"] {
            clean = remove_word(&clean, term);
        }
        clean = ultra_hd_re.replace_all(&clean, " ").to_string();
    } else if word_exists(&clean, "FHD") || numeric_res_re.is_match(&clean) {
        detected_resolution = Some("HDTV".to_string());
        clean = remove_word(&clean, "FHD");
        clean = numeric_res_re.replace_all(&clean, " ").to_string();
    } else if word_exists(&clean, "HD") {
        // Only treat standalone "HD" as resolution if no digits are present
        // (avoids false positives like "HD5" channel names)
        let has_digit = Regex::new(r"\d").unwrap();
        if !has_digit.is_match(&clean.replace("HD", "")) {
            detected_resolution = Some("HDTV".to_string());
            clean = remove_word(&clean, "HD");
        }
    } else if word_exists(&clean, "SD") || sd_res_re.is_match(&clean) {
        detected_resolution = Some("SDTV".to_string());
        clean = remove_word(&clean, "SD");
        clean = sd_res_re.replace_all(&clean, " ").to_string();
    }

    // ── Step 3: General cleanup ───────────────────────────────────────────
    // Remove common prefixes
    let prefix_re = Regex::new(
        r"(?i)^(CHANNEL|CH|NETWORK|NET|TV|TELEVISION|DIGITAL|CABLE|SATELLITE|STREAM|LIVE|24/7|24-7)\s+",
    ).unwrap();
    clean = prefix_re.replace(&clean, "").to_string();

    // Remove common suffixes
    let suffix_re = Regex::new(
        r"(?i)\s+(CHANNEL|CH|NETWORK|NET|TV|TELEVISION|DIGITAL|LIVE|STREAM|PLUS|\+|24/7|24-7)$",
    ).unwrap();
    clean = suffix_re.replace(&clean, "").to_string();

    // Replace underscores and hyphens with spaces
    clean = clean.replace('_', " ").replace('-', " ");

    // Remove special characters
    let special_char_re = Regex::new(r"[(){}\[\]<>#@$%^&*]").unwrap();
    clean = special_char_re.replace_all(&clean, "").to_string();

    // Remove trailing punctuation
    let trailing_punct_re = Regex::new(r"[^\w\s]$").unwrap();
    clean = trailing_punct_re.replace(&clean, "").to_string();

    // Remove generic/noise words
    let generic_terms = [
        "THE", "A", "AN", "AND", "OR", "OF", "IN", "ON", "AT", "TO", "FOR",
        "WITH", "OFFICIAL", "ORIGINAL", "PREMIUM", "EXCLUSIVE", "ONLINE",
        "DIGITAL", "STREAMING", "BROADCAST",
    ];
    for term in &generic_terms {
        clean = remove_word(&clean, term);
    }

    // Final whitespace normalization
    clean = normalize_whitespace(&clean);

    // ── Step 4: Restore casing heuristic ──────────────────────────────────
    if !clean.is_empty() {
        let is_original_mixed = channel_name != channel_name.to_uppercase()
            && channel_name != channel_name.to_lowercase();
        let is_original_lower = channel_name == channel_name.to_lowercase();

        if is_original_mixed {
            // Title case
            clean = clean
                .split_whitespace()
                .map(|w| {
                    let mut chars = w.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => {
                            let upper: String = c.to_uppercase().collect();
                            let lower: String = chars.as_str().to_lowercase();
                            format!("{}{}", upper, lower)
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
        } else if is_original_lower {
            clean = clean.to_lowercase();
        }
        // else: keep uppercase (original was all-caps)
    }

    // If cleaning removed everything, fall back to the original name
    if clean.is_empty() {
        clean = channel_name.to_string();
    }

    ParsedChannelName {
        clean_name: clean,
        country: detected_country,
        resolution: detected_resolution,
        original,
    }
}

/// Calculate a match score (0.0–1.0) between a channel name and a station record.
///
/// Scoring components:
/// - **Base score**: Jaro-Winkler similarity of cleaned channel name vs station name/call sign
/// - **Exact match bonus**: 1.0 base score if either matches exactly
/// - **Resolution bonus/penalty**: +0.15 if resolution matches, −0.20 if it conflicts
/// - **Country bonus**: +0.10 if detected country matches station's country
/// - **Logo bonus**: +0.05 if the station has a logo
pub fn calculate_match_score(
    channel_name: &str,
    station: &StationForScoring<'_>,
    parsed: Option<&ParsedChannelName>,
) -> f64 {
    // Parse the channel name if not already provided
    let owned_parsed;
    let parsed = match parsed {
        Some(p) => p,
        None => {
            owned_parsed = parse_channel_name(channel_name);
            &owned_parsed
        }
    };

    let clean_upper = parsed.clean_name.to_uppercase();
    let station_name_upper = station
        .name
        .unwrap_or("")
        .to_uppercase();
    let call_sign_upper = station
        .call_sign
        .unwrap_or("")
        .to_uppercase();

    // ── Base score: best of name vs call_sign similarity ──────────────────
    let name_score = if station_name_upper.is_empty() {
        0.0
    } else {
        strsim::jaro_winkler(&clean_upper, &station_name_upper)
    };

    let call_sign_score = if call_sign_upper.is_empty() {
        0.0
    } else {
        strsim::jaro_winkler(&clean_upper, &call_sign_upper)
    };

    // Exact match overrides
    let name_score = if clean_upper == station_name_upper {
        1.0
    } else {
        name_score
    };
    let call_sign_score = if clean_upper == call_sign_upper {
        1.0
    } else {
        call_sign_score
    };

    let mut score = name_score.max(call_sign_score);

    // ── Resolution bonus/penalty ──────────────────────────────────────────
    if let Some(ref detected_res) = parsed.resolution {
        if let Some(video_types) = station.video_types {
            if video_types.contains(detected_res.as_str()) {
                score += 0.15; // Resolution matches
            } else if !video_types.is_empty() {
                score -= 0.20; // Resolution conflict
            }
        }
    }

    // ── Country bonus ─────────────────────────────────────────────────────
    if let (Some(ref detected_country), Some(station_country)) =
        (&parsed.country, station.country)
    {
        if detected_country == station_country {
            score += 0.10;
        }
    }

    // ── Logo bonus ────────────────────────────────────────────────────────
    if station.has_logo {
        score += 0.05;
    }

    // Clamp to [0.0, 1.0]
    score.clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_us_espn_hd() {
        let parsed = parse_channel_name("US: ESPN HD");
        assert_eq!(parsed.country, Some("USA".to_string()));
        assert_eq!(parsed.resolution, Some("HDTV".to_string()));
        // Clean name should be "ESPN" (or title-cased "Espn" depending on original casing)
        assert!(
            parsed.clean_name.to_uppercase().contains("ESPN"),
            "Expected ESPN in clean_name, got: {}",
            parsed.clean_name
        );
    }

    #[test]
    fn test_parse_plain_name() {
        let parsed = parse_channel_name("Discovery Channel");
        assert_eq!(parsed.country, None);
        assert_eq!(parsed.resolution, None);
        assert!(
            parsed.clean_name.to_uppercase().contains("DISCOVERY"),
            "Expected DISCOVERY in clean_name, got: {}",
            parsed.clean_name
        );
    }

    #[test]
    fn test_parse_4k_channel() {
        let parsed = parse_channel_name("UK | BBC One 4K");
        assert_eq!(parsed.country, Some("GBR".to_string()));
        assert_eq!(parsed.resolution, Some("UHDTV".to_string()));
        assert!(
            parsed.clean_name.to_uppercase().contains("BBC"),
            "Expected BBC in clean_name, got: {}",
            parsed.clean_name
        );
    }

    #[test]
    fn test_exact_match_score() {
        let station = StationForScoring {
            name: Some("ESPN"),
            call_sign: Some("ESPN"),
            video_types: Some("HDTV"),
            country: Some("USA"),
            has_logo: true,
        };
        let parsed = parse_channel_name("US: ESPN HD");
        let score = calculate_match_score("US: ESPN HD", &station, Some(&parsed));
        // Exact name match (1.0) + resolution match (+0.15) + country match (+0.10) + logo (+0.05) = 1.0 (clamped)
        assert!(score > 0.9, "Expected high score, got: {}", score);
    }

    #[test]
    fn test_no_match_score() {
        let station = StationForScoring {
            name: Some("CNN International"),
            call_sign: Some("CNNI"),
            video_types: Some("HDTV"),
            country: Some("USA"),
            has_logo: false,
        };
        let score = calculate_match_score("BBC One", &station, None);
        assert!(score < 0.5, "Expected low score, got: {}", score);
    }
}
