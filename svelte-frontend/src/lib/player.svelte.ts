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

// Module-level reactive state — singleton across the entire app lifetime.
let _stream = $state<StreamInfo | null>(null);

export const playerStore = {
	/** The currently active stream (null = player closed) */
	get stream(): StreamInfo | null {
		return _stream;
	},

	/**
	 * Open the floating player with the given stream info.
	 * If a stream is already playing, it is replaced immediately.
	 */
	open(info: StreamInfo) {
		_stream = info;
	},

	/** Close the floating player and release the stream. */
	close() {
		_stream = null;
	}
};
