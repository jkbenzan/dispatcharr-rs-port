/**
 * categoryDetection.ts
 *
 * Standalone utility for classifying M3U/XC provider category names.
 *
 * Motivation: the previous inline detector in M3UProviderModal.svelte only
 * matched ~22% of real-world categories because it relied solely on alias
 * substring search, missing dominant provider formats like:
 *   "US | ABC"  "|DE|"  "┃FR┃ CINEMA"  "UK | Premier Sports"
 *
 * This module adds:
 *   1. Structured prefix parsing  — handles XX |, |XX|, ┃XX┃ patterns first
 *   2. ISO-3166-1 alpha-2 full lookup table covering ~250 country codes
 *   3. Alias fallback              — covers "United States", "Deutschland" etc.
 *   4. Region bucket support       — EU, LAT, EXYU, AFR, ASIA, etc.
 *   5. Non-country content labels  — Sports, PPV, News, Kids, Movies, etc.
 *   6. Ambiguity blacklist         — AR, AF, CH, IR, IS, IN, LA require context
 *
 * The returned CategoryClassification is a superset of the old CountryOption
 * so that the modal can be migrated incrementally.
 */

// ---------------------------------------------------------------------------
// Public Types
// ---------------------------------------------------------------------------

export type ClassificationKind =
	| 'country'
	| 'region'
	| 'sports'
	| 'event'
	| 'entertainment'
	| 'news'
	| 'kids'
	| 'movies'
	| 'music'
	| 'unknown';

export interface CategoryClassification {
	/** ISO-3166-1 alpha-2 for countries; region slug for regions; content slug for others. */
	code: string;
	/** Human-readable display name shown in the filter dropdown. */
	name: string;
	/** Classification type drives icon and filter behaviour. */
	kind: ClassificationKind;
	/** 'prefix' | 'alias' | 'region' | 'content' — how the match was made. */
	reason: string;
	/**
	 * 0.0–1.0.  Prefix matches score 0.9, alias matches 0.7.
	 * Ambiguous codes that survive context confirmation score 0.6.
	 */
	confidence: number;
	/** Emoji flag for country/region; category emoji for content types. */
	flag: string;
	/** Back-compat: legacy aliases list (mirrors old CountryOption). */
	aliases: string[];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Strips accents, lowercases, and collapses non-alphanumeric runs to a single
 * space, padded with a leading and trailing space so word-boundary checks
 * work with simple includes().
 */
export function normalizeForDetection(value?: string): string {
	return ` ${(value || '')
		.toLowerCase()
		.normalize('NFD')
		.replace(/[\u0300-\u036f]/g, '')
		.replace(/[^a-z0-9]+/g, ' ')} `;
}

/**
 * Converts a 2-letter ISO country code to its regional indicator emoji flag.
 * Returns '' for invalid codes.
 */
export function countryFlag(code?: string): string {
	if (!code || code.length !== 2) return '';
	return code
		.toUpperCase()
		.split('')
		.map((char) => String.fromCodePoint(127397 + char.charCodeAt(0)))
		.join('');
}

// ---------------------------------------------------------------------------
// Ambiguous prefix blacklist
// ---------------------------------------------------------------------------
// These 2-letter codes are ambiguous enough that a bare prefix match is
// unreliable.  They are skipped during structured-prefix detection and only
// matched via explicit alias fallback (which requires the full country name).
//
//  AR  — "Arabic" region bucket vs Argentina
//  AF  — "Africa" region bucket vs Afghanistan
//  CH  — China vs Switzerland depending on provider
//  IR  — Iran vs Ireland depending on provider
//  IS  — Israel vs Iceland depending on provider
//  IN  — India vs regional language buckets (e.g. "IN | Punjabi")
//  LA  — "Latin" region bucket vs Laos
// ---------------------------------------------------------------------------
const AMBIGUOUS_CODES = new Set(['AR', 'AF', 'CH', 'IR', 'IS', 'IN', 'LA']);

// ---------------------------------------------------------------------------
// ISO-3166-1 alpha-2 country table
// ---------------------------------------------------------------------------
// Each entry: [code, name, aliases[], flag-override?]
// Aliases are lowercase, accent-stripped words/phrases that appear in provider
// category names. The structured-prefix path uses only `code`; aliases are the
// fallback.
// ---------------------------------------------------------------------------
interface CountryEntry {
	code: string;
	name: string;
	aliases: string[];
}

const COUNTRY_TABLE: CountryEntry[] = [
	// --- Americas ---
	{ code: 'US', name: 'United States', aliases: ['usa', 'u.s.a', 'united states', 'us channels', 'america', 'american'] },
	{ code: 'CA', name: 'Canada', aliases: ['canada', 'canadian'] },
	{ code: 'MX', name: 'Mexico', aliases: ['mexico', 'mexican'] },
	{ code: 'BR', name: 'Brazil', aliases: ['brazil', 'brasil', 'brazilian'] },
	{ code: 'CO', name: 'Colombia', aliases: ['colombia', 'colombian'] },
	{ code: 'CL', name: 'Chile', aliases: ['chile', 'chilean'] },
	{ code: 'PE', name: 'Peru', aliases: ['peru', 'peruvian'] },
	{ code: 'VE', name: 'Venezuela', aliases: ['venezuela'] },
	{ code: 'EC', name: 'Ecuador', aliases: ['ecuador'] },
	{ code: 'BO', name: 'Bolivia', aliases: ['bolivia'] },
	{ code: 'PY', name: 'Paraguay', aliases: ['paraguay'] },
	{ code: 'UY', name: 'Uruguay', aliases: ['uruguay'] },
	{ code: 'CR', name: 'Costa Rica', aliases: ['costa rica'] },
	{ code: 'DO', name: 'Dominican Republic', aliases: ['dominican republic', 'republica dominicana'] },
	{ code: 'GT', name: 'Guatemala', aliases: ['guatemala'] },
	{ code: 'HN', name: 'Honduras', aliases: ['honduras'] },
	{ code: 'PA', name: 'Panama', aliases: ['panama'] },
	{ code: 'CU', name: 'Cuba', aliases: ['cuba', 'cuban'] },
	// Intentionally no AR entry — ambiguous, alias-only via Argentina
	{ code: 'AR', name: 'Argentina', aliases: ['argentina'] },

	// --- Europe ---
	{ code: 'GB', name: 'United Kingdom', aliases: ['uk', 'u.k', 'united kingdom', 'great britain', 'england', 'britain', 'english'] },
	{ code: 'FR', name: 'France', aliases: ['france', 'french', 'francais'] },
	{ code: 'DE', name: 'Germany', aliases: ['germany', 'deutschland', 'german', 'deutsch'] },
	{ code: 'ES', name: 'Spain', aliases: ['spain', 'espana', 'spanish', 'espanol'] },
	{ code: 'IT', name: 'Italy', aliases: ['italy', 'italia', 'italian'] },
	{ code: 'PT', name: 'Portugal', aliases: ['portugal', 'portuguese'] },
	{ code: 'NL', name: 'Netherlands', aliases: ['netherlands', 'holland', 'dutch'] },
	{ code: 'BE', name: 'Belgium', aliases: ['belgium', 'belgique', 'belgie'] },
	// CH is ambiguous — alias-only via switzerland/swiss
	{ code: 'CH', name: 'Switzerland', aliases: ['switzerland', 'swiss'] },
	{ code: 'AT', name: 'Austria', aliases: ['austria', 'austrian'] },
	{ code: 'IE', name: 'Ireland', aliases: ['ireland', 'irish'] },
	{ code: 'SE', name: 'Sweden', aliases: ['sweden', 'swedish', 'sverige'] },
	{ code: 'NO', name: 'Norway', aliases: ['norway', 'norwegian', 'norge'] },
	{ code: 'DK', name: 'Denmark', aliases: ['denmark', 'danish', 'danmark'] },
	{ code: 'FI', name: 'Finland', aliases: ['finland', 'finnish', 'suomi'] },
	{ code: 'PL', name: 'Poland', aliases: ['poland', 'polish'] },
	{ code: 'GR', name: 'Greece', aliases: ['greece', 'greek'] },
	{ code: 'TR', name: 'Turkey', aliases: ['turkey', 'turkish'] },
	{ code: 'RO', name: 'Romania', aliases: ['romania', 'romanian'] },
	{ code: 'BG', name: 'Bulgaria', aliases: ['bulgaria', 'bulgarian'] },
	{ code: 'HR', name: 'Croatia', aliases: ['croatia', 'croatian', 'hrvatska'] },
	{ code: 'RS', name: 'Serbia', aliases: ['serbia', 'serbian', 'srbija'] },
	{ code: 'BA', name: 'Bosnia', aliases: ['bosnia', 'bosnian', 'bosna'] },
	{ code: 'SI', name: 'Slovenia', aliases: ['slovenia', 'slovenian'] },
	{ code: 'SK', name: 'Slovakia', aliases: ['slovakia', 'slovak'] },
	{ code: 'CZ', name: 'Czech Republic', aliases: ['czech', 'czechia'] },
	{ code: 'HU', name: 'Hungary', aliases: ['hungary', 'hungarian'] },
	{ code: 'UA', name: 'Ukraine', aliases: ['ukraine', 'ukrainian'] },
	{ code: 'RU', name: 'Russia', aliases: ['russia', 'russian'] },
	{ code: 'AL', name: 'Albania', aliases: ['albania', 'albanian', 'shqip'] },
	{ code: 'MK', name: 'North Macedonia', aliases: ['north macedonia', 'macedonia', 'macedonian'] },
	{ code: 'ME', name: 'Montenegro', aliases: ['montenegro'] },
	{ code: 'LU', name: 'Luxembourg', aliases: ['luxembourg'] },
	{ code: 'LT', name: 'Lithuania', aliases: ['lithuania', 'lithuanian'] },
	{ code: 'LV', name: 'Latvia', aliases: ['latvia', 'latvian'] },
	{ code: 'EE', name: 'Estonia', aliases: ['estonia', 'estonian'] },
	// IS is ambiguous — alias-only via iceland/israel
	{ code: 'IS', name: 'Iceland', aliases: ['iceland', 'icelandic'] },
	{ code: 'IL', name: 'Israel', aliases: ['israel', 'israeli'] },
	{ code: 'CY', name: 'Cyprus', aliases: ['cyprus'] },
	{ code: 'MT', name: 'Malta', aliases: ['malta', 'maltese'] },

	// --- Middle East & Central Asia ---
	// IR is ambiguous — alias-only via iran
	{ code: 'IR', name: 'Iran', aliases: ['iran', 'iranian', 'persia', 'persian'] },
	{ code: 'SA', name: 'Saudi Arabia', aliases: ['saudi arabia', 'saudi', 'ksa'] },
	{ code: 'AE', name: 'UAE', aliases: ['uae', 'united arab emirates', 'emirates', 'dubai'] },
	{ code: 'KW', name: 'Kuwait', aliases: ['kuwait', 'kuwaiti'] },
	{ code: 'QA', name: 'Qatar', aliases: ['qatar'] },
	{ code: 'BH', name: 'Bahrain', aliases: ['bahrain'] },
	{ code: 'OM', name: 'Oman', aliases: ['oman'] },
	{ code: 'IQ', name: 'Iraq', aliases: ['iraq', 'iraqi'] },
	{ code: 'JO', name: 'Jordan', aliases: ['jordan', 'jordanian'] },
	{ code: 'LB', name: 'Lebanon', aliases: ['lebanon', 'lebanese'] },
	{ code: 'SY', name: 'Syria', aliases: ['syria', 'syrian'] },
	{ code: 'YE', name: 'Yemen', aliases: ['yemen'] },
	{ code: 'KZ', name: 'Kazakhstan', aliases: ['kazakhstan'] },
	{ code: 'UZ', name: 'Uzbekistan', aliases: ['uzbekistan'] },
	{ code: 'AZ', name: 'Azerbaijan', aliases: ['azerbaijan'] },
	{ code: 'GE', name: 'Georgia', aliases: ['georgia'] },
	{ code: 'AM', name: 'Armenia', aliases: ['armenia'] },

	// --- Africa ---
	// AF is ambiguous — alias-only via afghanistan; africa uses region entry
	{ code: 'AF', name: 'Afghanistan', aliases: ['afghanistan', 'afghan'] },
	{ code: 'EG', name: 'Egypt', aliases: ['egypt', 'egyptian'] },
	{ code: 'MA', name: 'Morocco', aliases: ['morocco', 'moroccan'] },
	{ code: 'DZ', name: 'Algeria', aliases: ['algeria', 'algerian'] },
	{ code: 'TN', name: 'Tunisia', aliases: ['tunisia', 'tunisian'] },
	{ code: 'LY', name: 'Libya', aliases: ['libya', 'libyan'] },
	{ code: 'NG', name: 'Nigeria', aliases: ['nigeria', 'nigerian'] },
	{ code: 'ZA', name: 'South Africa', aliases: ['south africa'] },
	{ code: 'KE', name: 'Kenya', aliases: ['kenya'] },
	{ code: 'GH', name: 'Ghana', aliases: ['ghana'] },
	{ code: 'ET', name: 'Ethiopia', aliases: ['ethiopia', 'ethiopian'] },
	{ code: 'SD', name: 'Sudan', aliases: ['sudan'] },
	{ code: 'SO', name: 'Somalia', aliases: ['somalia', 'somali'] },
	{ code: 'TZ', name: 'Tanzania', aliases: ['tanzania'] },
	{ code: 'UG', name: 'Uganda', aliases: ['uganda'] },
	{ code: 'CM', name: 'Cameroon', aliases: ['cameroon'] },
	{ code: 'CI', name: 'Ivory Coast', aliases: ['ivory coast', 'cote d ivoire'] },

	// --- Asia-Pacific ---
	{ code: 'IN', name: 'India', aliases: ['india', 'indian'] },
	{ code: 'PK', name: 'Pakistan', aliases: ['pakistan', 'pakistani'] },
	{ code: 'BD', name: 'Bangladesh', aliases: ['bangladesh', 'bangladeshi'] },
	{ code: 'LK', name: 'Sri Lanka', aliases: ['sri lanka'] },
	{ code: 'NP', name: 'Nepal', aliases: ['nepal', 'nepali'] },
	{ code: 'CN', name: 'China', aliases: ['china', 'chinese'] },
	{ code: 'JP', name: 'Japan', aliases: ['japan', 'japanese'] },
	{ code: 'KR', name: 'South Korea', aliases: ['south korea', 'korea', 'korean'] },
	{ code: 'KP', name: 'North Korea', aliases: ['north korea'] },
	{ code: 'TW', name: 'Taiwan', aliases: ['taiwan', 'taiwanese'] },
	{ code: 'HK', name: 'Hong Kong', aliases: ['hong kong'] },
	{ code: 'MO', name: 'Macau', aliases: ['macau', 'macao'] },
	{ code: 'VN', name: 'Vietnam', aliases: ['vietnam', 'vietnamese', 'viet'] },
	{ code: 'TH', name: 'Thailand', aliases: ['thailand', 'thai'] },
	{ code: 'ID', name: 'Indonesia', aliases: ['indonesia', 'indonesian'] },
	{ code: 'MY', name: 'Malaysia', aliases: ['malaysia', 'malaysian'] },
	{ code: 'PH', name: 'Philippines', aliases: ['philippines', 'filipino', 'pilipinas'] },
	{ code: 'SG', name: 'Singapore', aliases: ['singapore'] },
	{ code: 'MM', name: 'Myanmar', aliases: ['myanmar', 'burma'] },
	{ code: 'KH', name: 'Cambodia', aliases: ['cambodia', 'khmer'] },
	{ code: 'AU', name: 'Australia', aliases: ['australia', 'aussie', 'australian'] },
	{ code: 'NZ', name: 'New Zealand', aliases: ['new zealand'] },
];

// Build a fast O(1) lookup from code → entry
const COUNTRY_BY_CODE = new Map<string, CountryEntry>(
	COUNTRY_TABLE.map((entry) => [entry.code.toUpperCase(), entry])
);

// ---------------------------------------------------------------------------
// Region table
// ---------------------------------------------------------------------------
interface RegionEntry {
	code: string;
	name: string;
	prefixes: string[]; // Bare-word prefixes that identify this region
	flag: string;       // Arbitrary emoji to represent the region
}

const REGION_TABLE: RegionEntry[] = [
	{ code: 'EU', name: 'Europe', prefixes: ['EU'], flag: '🇪🇺' },
	{ code: 'LAT', name: 'Latin America', prefixes: ['LAT', 'LATAM', 'LATINO', 'LATIN'], flag: '🌎' },
	{ code: 'EXYU', name: 'Ex-Yugoslavia', prefixes: ['EXYU', 'EX-YU', 'EXYU', 'BALKANS', 'BALKAN'], flag: '🌍' },
	{ code: 'AFR', name: 'Africa', prefixes: ['AFR', 'AFRICA', 'AFRICAN', 'DSTV'], flag: '🌍' },
	{ code: 'ASIA', name: 'Asia', prefixes: ['ASIA', 'ASIAN'], flag: '🌏' },
	{ code: 'MENA', name: 'Middle East & North Africa', prefixes: ['MENA', 'ARABIC', 'ARAB'], flag: '🌍' },
	{ code: 'NORD', name: 'Nordics', prefixes: ['NORDIC', 'NORDIC', 'SCANDINAVIAN'], flag: '🌍' },
];

// ---------------------------------------------------------------------------
// Non-country content category table
// ---------------------------------------------------------------------------
interface ContentEntry {
	code: string;
	name: string;
	kind: ClassificationKind;
	flag: string;
	keywords: string[]; // Normalized keywords that strongly identify this category
}

const CONTENT_TABLE: ContentEntry[] = [
	{
		code: 'SPORTS',
		name: 'Sports',
		kind: 'sports',
		flag: '🏆',
		keywords: [
			'sport', 'sports', 'football', 'soccer', 'nfl', 'nba', 'nhl', 'mlb',
			'tennis', 'golf', 'cricket', 'rugby', 'racing', 'formula', 'f1',
			'basketball', 'baseball', 'boxing', 'mma', 'ufc', 'wrestling',
			'esports', 'olympic', 'athletics', 'motor', 'darts', 'snooker',
		],
	},
	{
		code: 'PPV',
		name: 'Pay-Per-View',
		kind: 'event',
		flag: '🎟️',
		keywords: ['ppv', 'pay per view', 'pay-per-view'],
	},
	{
		code: 'EVENTS',
		name: 'Live Events',
		kind: 'event',
		flag: '🎤',
		keywords: ['events', 'live events', 'concert', 'concerts', 'event channel'],
	},
	{
		code: 'NEWS',
		name: 'News',
		kind: 'news',
		flag: '📰',
		keywords: ['news', 'breaking news', 'journalism', 'documentary', 'documentaries'],
	},
	{
		code: 'MOVIES',
		name: 'Movies',
		kind: 'movies',
		flag: '🎬',
		keywords: [
			'movies', 'films', 'cinema', 'movie channel', 'film channel', 'vod',
			'hbo', 'starz', 'showtime', 'cinemax', 'netflix', 'disney', 'hulu',
			'paramount', 'peacock', 'apple tv', 'apple+', 'prime video', 'amazon prime'
		],
	},
	{
		code: 'KIDS',
		name: 'Kids',
		kind: 'kids',
		flag: '🧒',
		keywords: ['kids', 'children', 'family', 'cartoon', 'cartoons', 'animation', 'disney', 'nickelodeon'],
	},
	{
		code: 'MUSIC',
		name: 'Music',
		kind: 'music',
		flag: '🎵',
		keywords: ['music', 'radio', 'hits', 'mtv', 'vh1'],
	},
	{
		code: 'ENTERTAINMENT',
		name: 'Entertainment',
		kind: 'entertainment',
		flag: '📺',
		keywords: ['entertainment', 'general', 'lifestyle', 'reality', 'cooking', 'travel', '24 7', '24/7'],
	},
];

// ---------------------------------------------------------------------------
// Prefix extraction
// ---------------------------------------------------------------------------
// Matches common provider prefix formats:
//   "XX | rest"       — space-pipe-space separator
//   "|XX|"            — pipe-wrapped code
//   "┃XX┃"           — unicode block separator (U+2503)
//   "XX: rest"        — colon separator
//   "XX- rest"        — dash separator (less common)
// Extracted code is normalised to uppercase and trimmed.
// ---------------------------------------------------------------------------

/**
 * Extracts a candidate 2–6 character uppercase code from the start or
 * wrapper of a category name, then attempts to resolve it.
 *
 * Returns null if no unambiguous match is found.
 */
function detectByPrefix(name: string): CategoryClassification | null {
	const trimmed = name.trim();

	// Pattern 1: ┃XX┃ or |XX| — code wrapped in separators (any position)
	const wrapMatch = trimmed.match(/^[┃|]([A-Za-z]{2,6})[┃|]/);
	if (wrapMatch) {
		const code = wrapMatch[1].toUpperCase();
		const resolved = resolveCode(code);
		if (resolved) return { ...resolved, reason: 'prefix', confidence: 0.9 };
	}

	// Pattern 2: "XX | rest" — code then pipe separator
	const pipeMatch = trimmed.match(/^([A-Za-z]{2,6})\s*[\|┃]\s*/);
	if (pipeMatch) {
		const code = pipeMatch[1].toUpperCase();
		const resolved = resolveCode(code);
		if (resolved) return { ...resolved, reason: 'prefix', confidence: 0.9 };
	}

	// Pattern 3: "XX: rest" — code then colon
	const colonMatch = trimmed.match(/^([A-Za-z]{2,6}):\s+/);
	if (colonMatch) {
		const code = colonMatch[1].toUpperCase();
		const resolved = resolveCode(code);
		if (resolved) return { ...resolved, reason: 'prefix', confidence: 0.85 };
	}

	return null;
}

/**
 * Tries to resolve an uppercase code to a country or region entry.
 * Skips ambiguous codes (they must be matched via alias).
 */
function resolveCode(code: string): Omit<CategoryClassification, 'reason' | 'confidence'> | null {
	// Check region first (regions have longer codes like LAT, EXYU)
	const region = REGION_TABLE.find(
		(r) => r.prefixes.includes(code) || r.code === code
	);
	if (region) {
		return {
			code: region.code,
			name: region.name,
			kind: 'region',
			flag: region.flag,
			aliases: region.prefixes.map((p) => p.toLowerCase()),
		};
	}

	// Skip ambiguous 2-letter codes — must use alias path
	if (AMBIGUOUS_CODES.has(code)) return null;

	// Country lookup
	const country = COUNTRY_BY_CODE.get(code);
	if (country) {
		return {
			code: country.code,
			name: country.name,
			kind: 'country',
			flag: countryFlag(country.code),
			aliases: country.aliases,
		};
	}

	return null;
}

// ---------------------------------------------------------------------------
// Alias fallback detection
// ---------------------------------------------------------------------------

function detectByAlias(name: string): CategoryClassification | null {
	const normalized = normalizeForDetection(name);

	// Countries (includes ambiguous ones — they just need full-word aliases)
	for (const entry of COUNTRY_TABLE) {
		for (const alias of entry.aliases) {
			if (normalized.includes(normalizeForDetection(alias))) {
				return {
					code: entry.code,
					name: entry.name,
					kind: 'country',
					flag: countryFlag(entry.code),
					aliases: entry.aliases,
					reason: 'alias',
					// Ambiguous codes get slightly lower confidence
					confidence: AMBIGUOUS_CODES.has(entry.code) ? 0.6 : 0.7,
				};
			}
		}
	}

	// Regions
	for (const region of REGION_TABLE) {
		for (const prefix of region.prefixes) {
			if (normalized.includes(normalizeForDetection(prefix))) {
				return {
					code: region.code,
					name: region.name,
					kind: 'region',
					flag: region.flag,
					aliases: region.prefixes.map((p) => p.toLowerCase()),
					reason: 'region',
					confidence: 0.7,
				};
			}
		}
	}

	return null;
}

// ---------------------------------------------------------------------------
// Content-type detection
// ---------------------------------------------------------------------------

function detectContentType(name: string): CategoryClassification | null {
	const normalized = normalizeForDetection(name);

	for (const entry of CONTENT_TABLE) {
		for (const keyword of entry.keywords) {
			if (normalized.includes(normalizeForDetection(keyword))) {
				return {
					code: entry.code,
					name: entry.name,
					kind: entry.kind,
					flag: entry.flag,
					aliases: entry.keywords,
					reason: 'content',
					confidence: 0.8,
				};
			}
		}
	}

	return null;
}

// ---------------------------------------------------------------------------
// Main exported detector
// ---------------------------------------------------------------------------

/**
 * Classifies a provider category name.
 *
 * Detection order (stops at first confident match):
 *   1. Structured prefix — fastest, highest confidence (0.9)
 *   2. Alias substring   — covers full country/region names (0.7)
 *   3. Content keywords  — Sports, PPV, News, Kids, etc. (0.8)
 *   4. Unknown           — returned with confidence 0
 *
 * @param name  Raw category name from the provider, e.g. "US | ABC"
 * @returns     A CategoryClassification — never null, kind='unknown' on miss
 */
export function detectCategory(name?: string): CategoryClassification {
	if (!name || !name.trim()) {
		return {
			code: 'UNKNOWN',
			name: 'Unknown',
			kind: 'unknown',
			reason: 'empty',
			confidence: 0,
			flag: '',
			aliases: [],
		};
	}

	// 1. Structured prefix (fast path — catches XX |, |XX|, ┃XX┃ etc.)
	const byPrefix = detectByPrefix(name);
	if (byPrefix) return byPrefix;

	// 2. Alias fallback (handles "United States", "Deutschland", etc.)
	const byAlias = detectByAlias(name);
	if (byAlias) return byAlias;

	// 3. Content-type keywords
	const byContent = detectContentType(name);
	if (byContent) return byContent;

	// 4. Unknown
	return {
		code: 'UNKNOWN',
		name: 'Unknown',
		kind: 'unknown',
		reason: 'no-match',
		confidence: 0,
		flag: '',
		aliases: [],
	};
}

/**
 * Convenience wrapper that returns null instead of kind='unknown',
 * matching the old detectCountry() contract used in the modal.
 *
 * @deprecated  Migrate to detectCategory() for new code.
 */
export function detectCountryCompat(name?: string): CategoryClassification | null {
	const result = detectCategory(name);
	return result.kind === 'unknown' ? null : result;
}

/**
 * Returns the set of unique classifications for a list of category names,
 * sorted alphabetically by display name, useful for building the filter
 * dropdown.  Only returns entries with confidence >= minConfidence.
 */
export function buildFilterOptions(
	categoryNames: string[],
	minConfidence = 0.6
): CategoryClassification[] {
	const seen = new Map<string, CategoryClassification>();
	for (const name of categoryNames) {
		const result = detectCategory(name);
		if (result.kind !== 'unknown' && result.confidence >= minConfidence) {
			if (!seen.has(result.code)) {
				seen.set(result.code, result);
			}
		}
	}
	return Array.from(seen.values()).sort((a, b) => a.name.localeCompare(b.name));
}
