import { describe, it, expect } from 'vitest';
import { detectCategory } from './categoryDetection';

describe('categoryDetection', () => {
	it('detects countries with standard prefixes', () => {
		const us = detectCategory('US | Movies');
		expect(us.code).toBe('US');
		expect(us.flag).toBe('🇺🇸');

		const uk = detectCategory('UK: Entertainment');
		expect(uk.code).toBe('GB');
		expect(uk.flag).toBe('🇬🇧');
	});

	it('detects streaming services in VOD names', () => {
		const netflix = detectCategory('VOD: Netflix');
		expect(netflix.kind).toBe('movies');
		expect(netflix.flag).toBe('🎬');

		const disney = detectCategory('Movies: Disney+');
		expect(disney.kind).toBe('movies');
	});

	it('prioritizes country over streaming service when both present', () => {
		// Currently detectCategory stops at the first prefix match (country)
		const usNetflix = detectCategory('US | Netflix');
		expect(usNetflix.code).toBe('US');
		expect(usNetflix.flag).toBe('🇺🇸');
	});

	it('detects non-country content types', () => {
		const sports = detectCategory('Live Sports');
		expect(sports.kind).toBe('sports');
		expect(sports.flag).toBe('🏆');

		const kids = detectCategory('Kids Channels');
		expect(kids.kind).toBe('kids');
		expect(kids.flag).toBe('🧒');
	});

	it('handles unknown categories gracefully', () => {
		const unknown = detectCategory('Generic Category');
		expect(unknown.kind).toBe('unknown');
		expect(unknown.flag).toBe('');
	});
});
