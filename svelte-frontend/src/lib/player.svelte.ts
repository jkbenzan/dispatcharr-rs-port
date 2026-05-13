/**
 * player.svelte.ts
 *
 * Global player state store using Svelte 5 runes.
 * Lives at the app-shell level so the FloatingPlayer survives SvelteKit
 * page navigations (the root +layout.svelte renders the player, not
 * individual route pages).
 *
 * Usage from any page/component:
 *   import { playerStore } from '$lib/player.svelte';
 *   playerStore.open({ url, title, uuid?, provider? });
 *   playerStore.close();
 */

export interface StreamInfo {
	/** Proxy or direct URL for the stream */
	url: string;
	/** Human-readable title shown in the player header */
	title: string;
	/** Optional channel UUID (used for Dispatcharr proxy routing) */
	uuid?: string;
	/** Optional provider / M3U account name */
	provider?: string;
}

export const playerStore = $state({
	stream: null as StreamInfo | null,
	open(info: StreamInfo) {
		this.stream = info;
	},
	close() {
		this.stream = null;
	}
});
