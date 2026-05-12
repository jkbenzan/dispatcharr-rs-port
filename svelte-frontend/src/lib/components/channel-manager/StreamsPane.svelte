<script lang="ts">
	import { api } from '$lib/api';
	import { ChevronDown, ChevronRight, Play, Database, Layers, Eye, EyeOff, CheckSquare, Square } from 'lucide-svelte';
	import { onMount } from 'svelte';

	// --- Types ---
	interface Stream { id: number; name: string; url: string | null; channel_group: any; group_title?: string; m3u_account_id: number; m3u_account?: number; }
	interface CategoryGroupView { name: string; expanded: boolean; streams: Stream[]; }
	interface M3UView { id: number; name: string; expanded: boolean; groups: CategoryGroupView[]; total_streams: number; }

	// --- Props ---
	let { onPlayStream }: { onPlayStream?: (info: { url: string; title: string }) => void } = $props();

	// --- State ---
	let m3uViews = $state<M3UView[]>([]);
	let allM3Us = $state<any[]>([]); // Raw provider list for the filter dropdown
	let loading = $state(true);
	let searchQuery = $state('');
	let assignedStreamIds = $state(new Set<number>());
	let showUnassignedOnly = $state(false);

	// Provider filter: '' = show all, number = filter to that provider
	let selectedProviderId = $state<number | ''>('');

	// Multi-select
	let selectedIds = $state(new Set<number>());

	// Expose reload for parent
	export function reload() { loadData(); }

	// --- Load ---
	async function loadData() {
		loading = true;
		try {
			const [channelsRes, m3us, streamsRes] = await Promise.all([
				api.getChannels({ page_size: 5000 }),
				api.getPlaylists(),
				api.getStreams({ page_size: 10000 })
			]);
			const channelList= Array.isArray(channelsRes) ? channelsRes : channelsRes.results || [];
			const playlistList= Array.isArray(m3us)       ? m3us        : m3us.results        || [];
			const streamList = Array.isArray(streamsRes)  ? streamsRes  : streamsRes.results  || [];

			allM3Us = playlistList;

			// Build assigned set from channel→stream join data
			const assigned = new Set<number>();
			channelList.forEach((ch: any) => (ch.streams || []).forEach((s: any) => assigned.add(s.id)));
			assignedStreamIds = assigned;

			buildTree(playlistList, streamList);
		} catch (e) {
			console.error('Failed to load streams:', e);
		} finally {
			loading = false;
		}
	}

	/**
	 * Build the M3U → Category → Stream tree.
	 * Uses `group_title` (the M3U provider's category name) for grouping,
	 * NOT the curated channel group FK. This distinguishes "Category"
	 * (provider-side) from "Channel Group" (user-created in Channel Manager).
	 */
	function buildTree(m3us: any[], streams: any[]) {
		const byM3U = new Map<number, Map<string, any[]>>();
		streams.forEach(s => {
			const m3uId = s.m3u_account_id ?? s.m3u_account;
			if (m3uId == null) return;
			if (!byM3U.has(m3uId)) byM3U.set(m3uId, new Map());
			// Use group_title (M3U category) for grouping; fall back to 'Uncategorized'
			const categoryName = s.group_title || 'Uncategorized';
			const grp = byM3U.get(m3uId)!;
			if (!grp.has(categoryName)) grp.set(categoryName, []);
			grp.get(categoryName)!.push(s);
		});
		m3uViews = m3us.map(m => {
			const grps = byM3U.get(m.id) || new Map();
			const groups = Array.from(grps.keys()).sort().map(name => ({ name, expanded: false, streams: grps.get(name)! }));
			return { id: m.id, name: m.name, expanded: false, groups, total_streams: groups.reduce((a, g) => a + g.streams.length, 0) };
		}).filter(m => m.total_streams > 0);
	}

	// --- Filtered view (search + unassigned-only + provider filter) ---
	let filteredM3Us = $derived(
		m3uViews
			// Provider filter: if a provider is selected, only show that provider
			.filter(m => selectedProviderId === '' || m.id === selectedProviderId)
			.map(m => {
				if (!searchQuery.trim() && !showUnassignedOnly) return m;
				const q = searchQuery.toLowerCase();
				const matchedGroups = m.groups.map(g => {
					let streams = g.streams;
					if (showUnassignedOnly) streams = streams.filter(s => !assignedStreamIds.has(s.id));
					if (q) streams = streams.filter(s => s.name.toLowerCase().includes(q));
					return streams.length > 0 ? { ...g, streams, expanded: true } : null;
				}).filter((g): g is CategoryGroupView => g !== null);
				return matchedGroups.length > 0 ? { ...m, groups: matchedGroups, expanded: !!(searchQuery || showUnassignedOnly) } : null;
			}).filter((m): m is M3UView => m !== null)
	);

	// --- All visible stream IDs (for global select all) ---
	let allVisibleStreamIds = $derived(
		filteredM3Us.flatMap(m => m.groups.flatMap(g => g.streams.map(s => s.id)))
	);

	// --- Selection ---
	function toggleSelect(streamId: number, e: MouseEvent) {
		const next = new Set(selectedIds);
		if (next.has(streamId)) next.delete(streamId); else next.add(streamId);
		selectedIds = next;
	}

	function toggleGroupSelect(streams: Stream[]) {
		const ids = streams.map(s => s.id);
		const allSelected = ids.every(id => selectedIds.has(id));
		const next = new Set(selectedIds);
		if (allSelected) ids.forEach(id => next.delete(id)); else ids.forEach(id => next.add(id));
		selectedIds = next;
	}

	/** Select or deselect ALL currently visible streams */
	function toggleSelectAll() {
		const visibleIds = allVisibleStreamIds;
		const allSelected = visibleIds.length > 0 && visibleIds.every(id => selectedIds.has(id));
		const next = new Set(selectedIds);
		if (allSelected) {
			// Deselect all visible
			visibleIds.forEach(id => next.delete(id));
		} else {
			// Select all visible
			visibleIds.forEach(id => next.add(id));
		}
		selectedIds = next;
	}

	// --- Drag (single or multi-select) ---
	function handleDragStart(e: DragEvent, stream: Stream) {
		// If this stream is part of a selection, drag all selected; otherwise drag just this one
		const ids = selectedIds.has(stream.id) && selectedIds.size > 1
			? Array.from(selectedIds)
			: [stream.id];
		if (e.dataTransfer) {
			e.dataTransfer.setData('application/json', JSON.stringify(ids));
			e.dataTransfer.effectAllowed = 'copy';
		}
	}

	// --- Play ---
	function playStream(stream: Stream) {
		if (onPlayStream && stream.url) {
			onPlayStream({ url: stream.url, title: stream.name });
		}
	}

	onMount(() => { loadData(); });
</script>

<div class="streams-pane">
	<!-- Header -->
	<div class="pane-header">
		<input type="text" bind:value={searchQuery} placeholder="Search streams…" class="search-input" />

		<!-- Provider filter dropdown -->
		<select class="provider-filter" bind:value={selectedProviderId} title="Filter by provider">
			<option value="">All Providers</option>
			{#each allM3Us as m3u (m3u.id)}
				<option value={m3u.id}>{m3u.name}</option>
			{/each}
		</select>

		<button
			class="filter-btn"
			class:active={showUnassignedOnly}
			title={showUnassignedOnly ? 'Showing unassigned only' : 'Show all streams'}
			onclick={() => showUnassignedOnly = !showUnassignedOnly}
		>
			{#if showUnassignedOnly}<Eye size={15} />{:else}<EyeOff size={15} />{/if}
			{showUnassignedOnly ? 'Unassigned' : 'All'}
		</button>
	</div>

	<!-- Selection bar -->
	<div class="selection-bar">
		<!-- Global select all / deselect all -->
		<button class="select-all-global" title="Select / deselect all visible streams" onclick={toggleSelectAll}>
			{#if allVisibleStreamIds.length > 0 && allVisibleStreamIds.every(id => selectedIds.has(id))}
				<CheckSquare size={14} />
			{:else}
				<Square size={14} />
			{/if}
			{selectedIds.size > 0 ? `${selectedIds.size} selected` : 'Select All'}
		</button>
		{#if selectedIds.size > 0}
			<button class="clear-btn" onclick={() => selectedIds = new Set()}>Clear</button>
		{/if}
	</div>

	<div class="pane-content">
		{#if loading}
			<div class="state-msg">Loading streams…</div>
		{:else if filteredM3Us.length === 0}
			<div class="state-msg">No streams found.</div>
		{:else}
			{#each filteredM3Us as m3u (m3u.id)}
				<!-- M3U ACCOUNT ROW -->
				<div class="m3u-row">
					<button class="expand-btn" onclick={() => m3u.expanded = !m3u.expanded}>
						{#if m3u.expanded}<ChevronDown size={15}/>{:else}<ChevronRight size={15}/>{/if}
						<Database size={14} class="icon-dim" />
						<span class="m3u-name">{m3u.name}</span>
						<span class="badge">{m3u.total_streams}</span>
					</button>
				</div>

				{#if m3u.expanded}
					<div class="groups-list">
						{#each m3u.groups as group (group.name)}
							<!-- CATEGORY ROW -->
							<div class="group-row">
								<button class="expand-btn-group" onclick={() => group.expanded = !group.expanded}>
									{#if group.expanded}<ChevronDown size={13}/>{:else}<ChevronRight size={13}/>{/if}
									<Layers size={13} class="icon-dim" />
									<span class="group-name">{group.name}</span>
									<span class="badge-small">{group.streams.length}</span>
								</button>
								<!-- Select all in category -->
								{#if group.expanded}
									<button class="select-all-btn" title="Select/deselect all in category" onclick={() => toggleGroupSelect(group.streams)}>
										{group.streams.every(s => selectedIds.has(s.id)) ? '✓' : '○'}
									</button>
								{/if}
							</div>

							{#if group.expanded}
								<div class="streams-list">
									{#each group.streams as stream (stream.id)}
										<div
											class="stream-row"
											class:assigned={assignedStreamIds.has(stream.id)}
											class:selected={selectedIds.has(stream.id)}
											draggable="true"
											role="option"
											tabindex="0"
											aria-selected={selectedIds.has(stream.id)}
											ondragstart={(e) => handleDragStart(e, stream)}
											onclick={(e) => toggleSelect(stream.id, e)}
											onkeydown={(e) => e.key === ' ' && toggleSelect(stream.id, e as any)}
										>
											<!-- Checkbox -->
											<div class="stream-check" class:checked={selectedIds.has(stream.id)}>
												{#if selectedIds.has(stream.id)}✓{/if}
											</div>

											<div class="stream-info">
												<span class="stream-name">{stream.name}</span>
												{#if assignedStreamIds.has(stream.id)}
													<span class="assigned-badge">Assigned</span>
												{/if}
											</div>

											<div class="stream-actions">
												<!-- Play button -->
												{#if stream.url}
													<button class="icon-btn" title="Preview stream" onclick={(e) => { e.stopPropagation(); playStream(stream); }}>
														<Play size={12} fill="currentColor" />
													</button>
												{/if}
											</div>
										</div>
									{/each}
								</div>
							{/if}
						{/each}
					</div>
				{/if}
			{/each}
		{/if}
	</div>
</div>

<style lang="less">
.streams-pane { display: flex; flex-direction: column; height: 100%; background: var(--surface); border-radius: var(--radius); overflow: hidden; }

.pane-header {
	padding: 12px 16px; border-bottom: 1px solid var(--border);
	display: flex; gap: 8px; align-items: center;
	.search-input { flex: 1; background: var(--bg); border: 1px solid var(--border); padding: 8px 12px; border-radius: var(--radius); color: var(--text-bright); font-size: 13px; &:focus { outline: 1px solid var(--accent); } }
}

.provider-filter {
	background: var(--surface-bright); border: 1px solid var(--border); border-radius: var(--radius);
	color: var(--text-bright); padding: 7px 10px; font-size: 12px; cursor: pointer;
	max-width: 160px; overflow: hidden; text-overflow: ellipsis;
	&:focus { outline: 1px solid var(--accent); border-color: var(--accent); }
}

.filter-btn {
	display: flex; align-items: center; gap: 5px; padding: 7px 10px;
	background: var(--surface-bright); border: 1px solid var(--border); border-radius: var(--radius);
	color: var(--text-dim); font-size: 12px; cursor: pointer; white-space: nowrap;
	transition: all 0.15s;
	&.active { border-color: var(--accent); color: var(--accent); background: rgba(237,28,36,0.1); }
	&:hover:not(.active) { border-color: var(--border-bright); color: var(--text-bright); }
}

.selection-bar {
	display: flex; align-items: center; justify-content: space-between;
	padding: 5px 16px; background: var(--surface-bright); border-bottom: 1px solid var(--border);
	font-size: 12px; color: var(--text-dim); min-height: 30px;
}
.select-all-global {
	display: flex; align-items: center; gap: 5px;
	background: transparent; border: none; color: var(--text-dim); cursor: pointer; font-size: 12px; padding: 2px 4px; border-radius: 3px;
	&:hover { color: var(--accent); }
}
.clear-btn { background: transparent; border: none; color: var(--text-dim); cursor: pointer; font-size: 11px; &:hover { color: var(--text-bright); } }

.pane-content { flex: 1; overflow-y: auto; padding: 8px 0; }
.state-msg { padding: 32px; text-align: center; color: var(--text-dim); font-style: italic; font-size: 13px; }

.expand-btn, .expand-btn-group {
	width: 100%; display: flex; align-items: center; gap: 7px; background: transparent; border: none;
	color: var(--text-bright); cursor: pointer; text-align: left; transition: background 0.15s;
	&:hover { background: rgba(255,255,255,0.05); }
}
.expand-btn { padding: 8px 16px; font-size: 13px; font-weight: 700; }
.expand-btn-group { padding: 5px 16px 5px 28px; font-size: 12px; font-weight: 600; color: var(--text-dim); flex: 1; }
.group-row { display: flex; align-items: center; }
.select-all-btn { background: transparent; border: none; color: var(--text-dim); font-size: 13px; padding: 4px 12px; cursor: pointer; &:hover { color: var(--accent); } }
.m3u-name, .group-name { flex: 1; }
.badge, .badge-small { font-size: 10px; background: var(--surface-bright); padding: 1px 6px; border-radius: 10px; color: var(--text-dim); }
:global(.icon-dim) { color: var(--text-dim); }

.stream-row {
	display: flex; align-items: center; gap: 6px;
	padding: 5px 16px 5px 48px; cursor: grab; transition: background 0.15s;
	&:hover { background: rgba(255,255,255,0.03); .stream-actions { opacity: 1; } }
	&.assigned .stream-name { color: var(--text-dim); }
	&.selected { background: rgba(237,28,36,0.08); }
}
.stream-check {
	width: 16px; height: 16px; border: 1px solid var(--border-bright); border-radius: 3px;
	display: flex; align-items: center; justify-content: center; font-size: 11px;
	color: var(--accent); flex-shrink: 0; transition: all 0.15s;
	&.checked { background: rgba(237,28,36,0.2); border-color: var(--accent); }
}
.stream-info { display: flex; align-items: center; gap: 7px; flex: 1; min-width: 0; .stream-name { font-size: 12px; color: var(--text-bright); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } }
.assigned-badge { font-size: 9px; text-transform: uppercase; color: #4ade80; font-weight: 700; letter-spacing: 0.5px; flex-shrink: 0; }
.stream-actions { display: flex; gap: 3px; opacity: 0; transition: opacity 0.15s; flex-shrink: 0; }
.icon-btn { background: transparent; border: none; color: var(--text-dim); padding: 3px; border-radius: 4px; cursor: pointer; display: flex; align-items: center; &:hover { color: #4ade80; background: rgba(74,222,128,0.1); } }
</style>
