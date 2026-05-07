import { api } from '$lib/api';

export const uiSettings = $state({
	time_format: '12h',
	date_format: 'mdy',
	table_size: 'default',
	time_zone: 'UTC'
});

// Since Svelte 5 $effect must be inside component initialization or root,
// we'll expose a function to call from layout, or just set it in loadUiSettings and handle reactive updates inside loadUiSettings using $effect.root

let effectRoot: any;

export async function loadUiSettings() {
	try {
		const res = await api.getSettings();
		const ui = res.find((s: any) => s.key === 'ui_settings');
		if (ui && ui.value) {
			Object.assign(uiSettings, ui.value);
		}
		
		if (typeof document !== 'undefined') {
			if (!effectRoot) {
				effectRoot = $effect.root(() => {
					$effect(() => {
						document.documentElement.setAttribute('data-table-size', uiSettings.table_size);
					});
				});
			}
		}
	} catch (e) {
		console.warn('Could not load UI settings', e);
	}
}

export function formatDateTime(isoStr: string | null | undefined): string {
	if (!isoStr) return '';
	const date = new Date(isoStr);
	if (isNaN(date.getTime())) return '';

	// Date parts
	const y = date.getFullYear();
	const m = (date.getMonth() + 1).toString().padStart(2, '0');
	const d = date.getDate().toString().padStart(2, '0');

	let datePart = '';
	if (uiSettings.date_format === 'dmy') {
		datePart = `${d}/${m}/${y}`;
	} else if (uiSettings.date_format === 'ymd') {
		datePart = `${y}-${m}-${d}`;
	} else {
		// default mdy
		datePart = `${m}/${d}/${y}`;
	}

	// Time parts
	let timePart = date.toLocaleTimeString('en-US', {
		hour: 'numeric',
		minute: '2-digit',
		second: '2-digit',
		hour12: uiSettings.time_format === '12h',
		timeZone: uiSettings.time_zone !== 'UTC' ? uiSettings.time_zone : undefined // Fallback to local if undefined or invalid, wait UTC is safe, let's pass it if it's a valid timezone
	});

	// A basic check to see if the timezone string is a valid IANA timezone. If it fails, fallback to local.
	try {
		timePart = date.toLocaleTimeString('en-US', {
			hour: 'numeric',
			minute: '2-digit',
			second: '2-digit',
			hour12: uiSettings.time_format === '12h',
			timeZone: uiSettings.time_zone
		});
	} catch (e) {
		// Timezone invalid, fallback to local timezone
		timePart = date.toLocaleTimeString('en-US', {
			hour: 'numeric',
			minute: '2-digit',
			second: '2-digit',
			hour12: uiSettings.time_format === '12h'
		});
	}

	return `${datePart} ${timePart}`;
}
