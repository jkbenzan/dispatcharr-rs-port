/**
 * categoryDetection.test.ts
 *
 * Unit tests for the categoryDetection utility.
 *
 * Run with:  npx vitest run src/lib/utils/categoryDetection.test.ts
 *
 * Test cases are sourced from the May 2026 real-provider audit of 8,821
 * category mappings and cover the dominant provider formats found there.
 */

import { describe, it, expect } from 'vitest';
import {
	detectCategory,
	detectCountryCompat,
	buildFilterOptions,
	normalizeForDetection,
	countryFlag,
	type CategoryClassification,
} from './categoryDetection';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function expectCountry(result: CategoryClassification, code: string) {
	expect(result.kind).toBe('country');
	expect(result.code).toBe(code);
	expect(result.confidence).toBeGreaterThan(0);
}

function expectRegion(result: CategoryClassification, code: string) {
	expect(result.kind).toBe('region');
	expect(result.code).toBe(code);
}

function expectContent(result: CategoryClassification, code: string) {
	expect(result.kind).not.toBe('unknown');
	expect(result.code).toBe(code);
}

// ---------------------------------------------------------------------------
// Structured prefix detection (XX | format)
// ---------------------------------------------------------------------------
describe('structured prefix detection — XX | format', () => {
	it('detects US from "US | ABC"', () => expectCountry(detectCategory('US | ABC'), 'US'));
	it('detects GB from "UK | Premier Sports GB"', () => expectCountry(detectCategory('UK | Premier Sports GB'), 'GB'));
	it('detects DE from "DE | ZDF"', () => expectCountry(detectCategory('DE | ZDF'), 'DE'));
	it('detects FR from "FR | Canal+"', () => expectCountry(detectCategory('FR | Canal+'), 'FR'));
	it('detects CA from "CA | CBC"', () => expectCountry(detectCategory('CA | CBC'), 'CA'));
	it('detects MX from "MX | Televisa"', () => expectCountry(detectCategory('MX | Televisa'), 'MX'));
	it('detects BR from "BR | Globo"', () => expectCountry(detectCategory('BR | Globo'), 'BR'));
	it('detects IT from "IT | RAI 1"', () => expectCountry(detectCategory('IT | RAI 1'), 'IT'));
	it('detects PT from "PT | RTP"', () => expectCountry(detectCategory('PT | RTP'), 'PT'));
	it('detects NL from "NL | NPO1"', () => expectCountry(detectCategory('NL | NPO1'), 'NL'));
	it('detects TR from "TR | TRT 1"', () => expectCountry(detectCategory('TR | TRT 1'), 'TR'));
	it('detects SA from "SA | MBC"', () => expectCountry(detectCategory('SA | MBC'), 'SA'));
	it('detects AE from "AE | Dubai TV"', () => expectCountry(detectCategory('AE | Dubai TV'), 'AE'));
	it('detects AU from "AU | ABC Australia"', () => expectCountry(detectCategory('AU | ABC Australia'), 'AU'));
	it('detects NZ from "NZ | TVNZ"', () => expectCountry(detectCategory('NZ | TVNZ'), 'NZ'));
});

// ---------------------------------------------------------------------------
// Pipe-wrapped format (|XX| and ┃XX┃)
// ---------------------------------------------------------------------------
describe('pipe-wrapped prefix detection', () => {
	it('detects DE from "|DE|"', () => expectCountry(detectCategory('|DE|'), 'DE'));
	it('detects FR from "|FR| CINEMA"', () => expectCountry(detectCategory('|FR| CINEMA'), 'FR'));
	it('detects DE from "┃DE┃ PREMIUM HORROR"', () => expectCountry(detectCategory('┃DE┃ PREMIUM HORROR'), 'DE'));
	it('detects US from "┃US┃ NEWS"', () => expectCountry(detectCategory('┃US┃ NEWS'), 'US'));
	it('detects GB from "|UK|"', () => expectCountry(detectCategory('|UK|'), 'GB'));
});

// ---------------------------------------------------------------------------
// Alias fallback (full words — old detector behaviour preserved)
// ---------------------------------------------------------------------------
describe('alias fallback detection', () => {
	it('detects US from "United States"', () => expectCountry(detectCategory('United States'), 'US'));
	it('detects US from "USA Channels"', () => expectCountry(detectCategory('USA Channels'), 'US'));
	it('detects GB from "United Kingdom"', () => expectCountry(detectCategory('United Kingdom'), 'GB'));
	it('detects GB from "UK Channels"', () => expectCountry(detectCategory('UK Channels'), 'GB'));
	it('detects DE from "Germany Premium"', () => expectCountry(detectCategory('Germany Premium'), 'DE'));
	it('detects DE from "Deutschland HD"', () => expectCountry(detectCategory('Deutschland HD'), 'DE'));
	it('detects FR from "France 24"', () => expectCountry(detectCategory('France 24'), 'FR'));
	it('detects BR from "Brasil HD"', () => expectCountry(detectCategory('Brasil HD'), 'BR'));
	it('detects AU from "Australia: Free to Air"', () => expectCountry(detectCategory('Australia: Free to Air'), 'AU'));
	it('detects MX from "Mexico Canales"', () => expectCountry(detectCategory('Mexico Canales'), 'MX'));
});

// ---------------------------------------------------------------------------
// Region detection
// ---------------------------------------------------------------------------
describe('region detection', () => {
	it('detects LAT region from "LAT| MEXICO"', () => expectRegion(detectCategory('LAT| MEXICO'), 'LAT'));
	it('detects LAT region from "LATAM Channels"', () => expectRegion(detectCategory('LATAM Channels'), 'LAT'));
	it('detects EXYU from "EXYU Channels"', () => expectRegion(detectCategory('EXYU Channels'), 'EXYU'));
	it('detects EXYU from "EX-YU Sports"', () => expectRegion(detectCategory('EX-YU Sports'), 'EXYU'));
	it('detects EU from "EU | Eurosport"', () => expectRegion(detectCategory('EU | Eurosport'), 'EU'));
	it('detects AFR from "AFR | DSTV"', () => expectRegion(detectCategory('AFR | DSTV'), 'AFR'));
	it('detects MENA from "MENA | MBC"', () => expectRegion(detectCategory('MENA | MBC'), 'MENA'));
	it('detects MENA from "ARABIC | beIN"', () => expectRegion(detectCategory('ARABIC | beIN'), 'MENA'));
	it('detects ASIA from "ASIA | Star"', () => expectRegion(detectCategory('ASIA | Star'), 'ASIA'));
});

// ---------------------------------------------------------------------------
// Non-country content categories
// ---------------------------------------------------------------------------
describe('content-type detection', () => {
	it('detects SPORTS from "SPORTS | Tennis Channel"', () =>
		expectContent(detectCategory('SPORTS | Tennis Channel'), 'SPORTS'));
	it('detects SPORTS from "NFL RedZone"', () => expectContent(detectCategory('NFL RedZone'), 'SPORTS'));
	it('detects SPORTS from "LIVE | Formula 1"', () => expectContent(detectCategory('LIVE | Formula 1'), 'SPORTS'));
	it('detects PPV from "LIVE | PPV Events"', () => expectContent(detectCategory('LIVE | PPV Events'), 'PPV'));
	it('detects EVENTS from "LIVE | Concerts"', () => expectContent(detectCategory('LIVE | Concerts'), 'EVENTS'));
	it('detects NEWS from "BBC News"', () => expectContent(detectCategory('BBC News'), 'NEWS'));
	it('detects MOVIES from "Movies: HBO"', () => expectContent(detectCategory('Movies: HBO'), 'MOVIES'));
	it('detects KIDS from "Kids: Disney"', () => expectContent(detectCategory('Kids: Disney'), 'KIDS'));
	it('detects MUSIC from "Music Channels"', () => expectContent(detectCategory('Music Channels'), 'MUSIC'));
});

// ---------------------------------------------------------------------------
// Ambiguous prefix blacklist — these must NOT resolve via prefix
// ---------------------------------------------------------------------------
describe('ambiguous code blacklist', () => {
	// AR | ... should NOT auto-resolve to Argentina (often means Arabic)
	it('does not resolve AR prefix to Argentina', () => {
		const result = detectCategory('AR | beIN Entertainment');
		// If detected, it should NOT be country/Argentina via prefix
		// It may still be detected if 'beIN Entertainment' triggers MENA content — that's fine
		if (result.kind === 'country') {
			// If it resolved as country, it must NOT have confidence=0.9 (prefix)
			expect(result.reason).not.toBe('prefix');
		}
	});

	// AF | ... should NOT auto-resolve to Afghanistan (often means Africa)
	it('does not resolve AF prefix to Afghanistan via prefix', () => {
		const result = detectCategory('AF | DSTV Africa');
		if (result.kind === 'country' && result.code === 'AF') {
			expect(result.reason).not.toBe('prefix');
		}
	});

	// CH | ... ambiguous China vs Switzerland
	it('does not resolve CH prefix to Switzerland or China via prefix', () => {
		const result = detectCategory('CH | SRF 1');
		if (result.kind === 'country') {
			expect(result.reason).not.toBe('prefix');
		}
	});

	// IN | ... — India, but some providers use language buckets
	it('does not resolve IN prefix via prefix path', () => {
		const result = detectCategory('IN | Zee TV');
		if (result.kind === 'country' && result.code === 'IN') {
			expect(result.reason).not.toBe('prefix');
		}
	});

	// Full country name SHOULD still work via alias
	it('resolves Argentina via alias even though AR prefix is blacklisted', () =>
		expectCountry(detectCategory('Argentina HD'), 'AR'));

	it('resolves Switzerland via alias even though CH prefix is blacklisted', () =>
		expectCountry(detectCategory('Switzerland Channels'), 'CH'));
});

// ---------------------------------------------------------------------------
// Unknown / edge cases
// ---------------------------------------------------------------------------
describe('unknown and edge cases', () => {
	it('returns unknown for empty string', () => {
		expect(detectCategory('').kind).toBe('unknown');
	});
	it('returns unknown for undefined', () => {
		expect(detectCategory(undefined).kind).toBe('unknown');
	});
	it('"General" maps to entertainment (not unknown — "general" is a valid content keyword)', () => {
		const result = detectCategory('General');
		// "general" is an entertainment keyword — content detection is correct here.
		expect(result.kind).toBe('entertainment');
	});
	it('handles multi-word with no country gracefully', () => {
		expect(detectCategory('Premium HD Package').confidence).toBeLessThanOrEqual(0.8);
	});
});

// ---------------------------------------------------------------------------
// detectCountryCompat — legacy contract
// ---------------------------------------------------------------------------
describe('detectCountryCompat (legacy API)', () => {
	it('returns null for unknown categories', () => {
		expect(detectCountryCompat('Random Category')).toBeNull();
	});
	it('returns a result for known country', () => {
		expect(detectCountryCompat('US | NBC')).not.toBeNull();
		expect(detectCountryCompat('US | NBC')?.code).toBe('US');
	});
});

// ---------------------------------------------------------------------------
// buildFilterOptions
// ---------------------------------------------------------------------------
describe('buildFilterOptions', () => {
	it('deduplicates entries with the same code', () => {
		const categories = ['US | ABC', 'US | NBC', 'US | CBS', 'DE | ZDF', 'DE | ARD'];
		const options = buildFilterOptions(categories);
		expect(options.filter((o) => o.code === 'US')).toHaveLength(1);
		expect(options.filter((o) => o.code === 'DE')).toHaveLength(1);
	});

	it('sorts alphabetically by name', () => {
		const categories = ['US | ABC', 'DE | ZDF', 'FR | Canal+'];
		const options = buildFilterOptions(categories);
		const names = options.map((o) => o.name);
		expect(names).toEqual([...names].sort());
	});

	it('excludes unknown categories', () => {
		const categories = ['Random Category', 'Another Unknown'];
		const options = buildFilterOptions(categories);
		expect(options).toHaveLength(0);
	});

	it('respects minConfidence filter', () => {
		const categories = ['Argentina HD']; // alias match on ambiguous code → 0.6
		const high = buildFilterOptions(categories, 0.9);
		const low = buildFilterOptions(categories, 0.5);
		// high threshold should filter it out, low should include it
		expect(high).toHaveLength(0);
		expect(low).toHaveLength(1);
	});
});

// ---------------------------------------------------------------------------
// Helper exports
// ---------------------------------------------------------------------------
describe('helper functions', () => {
	it('countryFlag generates correct flag emoji for US', () => {
		expect(countryFlag('US')).toBe('🇺🇸');
	});
	it('countryFlag returns empty string for invalid codes', () => {
		expect(countryFlag('')).toBe('');
		expect(countryFlag(undefined)).toBe('');
		expect(countryFlag('USA')).toBe(''); // 3-letter, not 2
	});
	it('normalizeForDetection strips accents and lowercases', () => {
		expect(normalizeForDetection('Éspaña')).toContain('espana');
	});
});

// ---------------------------------------------------------------------------
// Real-world audit examples (from the May 2026 provider audit)
// ---------------------------------------------------------------------------
describe('real-world audit examples', () => {
	const cases: [string, string, string][] = [
		// [input, expectedCode, expectedKind]
		['US | ABC', 'US', 'country'],
		['┃DE┃ PREMIUM HORROR', 'DE', 'country'],
		['UK | Premier Sports GB', 'GB', 'country'],
		['LAT| MEXICO', 'LAT', 'region'],
		['LIVE | PPV Events', 'PPV', 'event'],
		['SPORTS | Tennis Channel', 'SPORTS', 'sports'],
		['EXYU | HRT 1', 'EXYU', 'region'],
		['|FR| CINEMA', 'FR', 'country'],
		['SA | MBC Max', 'SA', 'country'],
		['AU | 9Network', 'AU', 'country'],
		['NZ | Sky Sport', 'NZ', 'country'],
		['TR | TRT Spor', 'TR', 'country'],
		['United Kingdom', 'GB', 'country'],
		['Deutschland HD', 'DE', 'country'],
	];

	cases.forEach(([input, code, kind]) => {
		it(`classifies "${input}" as ${kind}/${code}`, () => {
			const result = detectCategory(input);
			expect(result.code).toBe(code);
			expect(result.kind).toBe(kind);
		});
	});
});
