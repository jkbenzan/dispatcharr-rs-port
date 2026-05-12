<script lang="ts">
	import { api } from '$lib/api';
	import { ChevronDown, ChevronRight, ChevronUp, MoreVertical, Play, Pencil, Trash2, Tv, GripVertical, ArrowUpDown } from 'lucide-svelte';
	import { toast } from '$lib/toast.svelte';
	import CreateChannelModal from './CreateChannelModal.svelte';

	// --- Types ---
	// StreamView now includes stream_stats for health display in the sub-list
	interface StreamStats {
		status?: string;      // 'online', 'offline', 'frozen', 'black_screen'
		reachable?: boolean;
		resolution?: string;
		video_codec?: string;
		audio_codec?: string;
		fps?: number;
		bitrate?: number;
	}
	interface StreamView {
		id: number; name: string; url: string | null; m3u_account_name: string;
		stream_stats?: StreamStats | null;
		stream_stats_updated_at?: string | null;
		// UI toggle for expanded stats view per stream
		statsExpanded?: boolean;
	}
	interface ChannelView { id: number; uuid: string; name: string; channel_number: number | null; logo_url: string | null; expanded: boolean; streams: StreamView[]; }
	interface GroupView { id: number; name: string; expanded: boolean; channels: ChannelView[]; }

	// --- Props (allows parent to trigger reload) ---
	let { onPlayStream }: { onPlayStream?: (info: { url: string; title: string; uuid: string }) => void } = $props();

	// --- State ---
	let groupViews = $state<GroupView[]>([]);
	let loading = $state(true);
	let searchQuery = $state('');
	let crossPaneDropTargetId = $state<number | null>(null);

	// Kebab menu state: which channel has the menu open
	let openMenuId = $state<number | null>(null);

	// Delete confirmation
	let deletingId = $state<number | null>(null);
	let deleteConfirmId = $state<number | null>(null);

	// Sort-by-health state
	let sortingChannelId = $state<number | null>(null);

	// Edit modal
	let showEditModal = $state(false);
	let editChannelId = $state<number | null>(null);
	let editInitialData = $state<any>(null);

	// Drag-to-reorder within channel streams
	let draggingStreamId = $state<number | null>(null);
	let dragOverStreamId = $state<number | null>(null);
	let dragSourceChannelId = $state<number | null>(null);

	// --- Expose reload() to parent via binding ---
	export function reload() { loadData(); }

	// --- Load ---
	async function loadData() {
		loading = true;
		try {
			// Snapshot expanded state before reload so we can restore it
			const expandedGroups = new Set(groupViews.filter(g => g.expanded).map(g => g.id));
			const expandedChannels = new Set(
				groupViews.flatMap(g => g.channels.filter(ch => ch.expanded).map(ch => ch.id))
			);

			const [groups, channelsRes] = await Promise.all([
				api.getChannelGroups(),
				api.getChannels({ page_size: 5000 })
			]);
			const channelList = Array.isArray(channelsRes) ? channelsRes : channelsRes.results || [];
			const groupList   = Array.isArray(groups)     ? groups     : groups.results     || [];
			buildGroupViews(groupList, channelList, expandedGroups, expandedChannels);
		} catch (e) {
			console.error('Failed to load channels:', e);
		} finally {
			loading = false;
		}
	}

	function buildGroupViews(
		groups: any[],
		channels: any[],
		expandedGroups: Set<number> = new Set(),
		expandedChannels: Set<number> = new Set()
	) {
		const byGroup = new Map<number, any[]>();
		const ungrouped: any[] = [];
		channels.forEach(ch => {
			const gid = ch.channel_group_id ?? ch.channel_group;
			if (gid) { if (!byGroup.has(gid)) byGroup.set(gid, []); byGroup.get(gid)!.push(ch); }
			else ungrouped.push(ch);
		});
		const views: GroupView[] = groups
			.map(g => ({
				id: g.id, name: g.name, expanded: expandedGroups.has(g.id),
				channels: (byGroup.get(g.id) || [])
					.sort((a: any, b: any) => (a.channel_number || 0) - (b.channel_number || 0))
					.map((ch: any) => ({ ...ch, expanded: expandedChannels.has(ch.id), streams: ch.streams || [] }))
			}))
			.filter(g => g.channels.length > 0);
		if (ungrouped.length > 0) views.push({
			id: -1, name: 'Ungrouped', expanded: expandedGroups.has(-1),
			channels: ungrouped.map(ch => ({ ...ch, expanded: expandedChannels.has(ch.id), streams: ch.streams || [] }))
		});
		groupViews = views;
	}

	// --- Filtered (search) ---
	let filteredGroups = $derived(
		groupViews.map(g => {
			if (!searchQuery.trim()) return g;
			const q = searchQuery.toLowerCase();
			const matched = g.channels.filter(ch =>
				ch.name.toLowerCase().includes(q) || String(ch.channel_number ?? '').includes(q)
			);
			return matched.length > 0 ? { ...g, channels: matched, expanded: true } : null;
		}).filter((g): g is GroupView => g !== null)
	);

	// --- Play ---
	// Play the first assigned stream for the channel.
	// If no streams are assigned, do nothing.
	function playChannel(channel: ChannelView) {
		if (!onPlayStream || channel.streams.length === 0) return;
		// Use the first stream's direct URL rather than the proxy endpoint,
		// since the proxy may not be available or configured for all channels.
		const stream = channel.streams[0];
		onPlayStream({
			url: stream.url || '',
			title: `${channel.name}`,
			uuid: channel.uuid
		});
	}

	// --- Delete ---
	async function deleteChannel(channel: ChannelView) {
		if (deleteConfirmId !== channel.id) {
			// First click: enter confirmation state, keep the menu open
			deleteConfirmId = channel.id;
			return;
		}
		// Second click: actually delete
		deletingId = channel.id;
		deleteConfirmId = null;
		openMenuId = null;
		try {
			await api.deleteChannel(channel.id);
			await loadData();
		} catch (e) {
			console.error('Failed to delete channel:', e);
		} finally {
			deletingId = null;
		}
	}

	// --- Edit ---
	function openEdit(channel: ChannelView) {
		editChannelId = channel.id;
		editInitialData = channel;
		showEditModal = true;
		openMenuId = null;
	}

	// --- Sort by Health ---
	// Reorders streams within a channel based on health score using backend sorting rules.
	async function sortByHealth(channel: ChannelView) {
		sortingChannelId = channel.id;
		openMenuId = null;
		try {
			await api.bulkSortStreams([channel.id]);
			toast.success(`Streams sorted for "${channel.name}"`);
			await loadData();
		} catch (err: any) {
			toast.error(err.message || 'Failed to sort streams');
		} finally {
			sortingChannelId = null;
		}
	}

	// --- Helper: determine health status class for row shading ---
	function getStreamHealthClass(stream: StreamView): string {
		if (!stream.stream_stats) return '';  // untested = no shading
		const status = stream.stream_stats.status;
		if (status === 'online') return '';   // online = normal display
		return 'health-offline';              // offline/frozen/black_screen = red tint
	}

	// --- Helper: safe display of stream stat values (no 'null' strings) ---
	function displayStat(val: any): string {
		if (val === null || val === undefined || val === 'null') return '';
		return String(val);
	}

	// --- Remove stream from channel ---
	async function removeStream(channel: ChannelView, streamId: number) {
		const newIds = channel.streams.filter(s => s.id !== streamId).map(s => s.id);
		try {
			await api.updateChannel(channel.id, { streams: newIds });
			await loadData();
		} catch (e) {
			console.error('Failed to remove stream:', e);
		}
	}

	// --- Reorder streams within a channel ---
	function handleStreamDragStart(e: DragEvent, channelId: number, streamId: number) {
		draggingStreamId = streamId;
		dragSourceChannelId = channelId;
		e.dataTransfer!.effectAllowed = 'move';
		// Also allow cross-pane drags to be superseded (don't set application/json here)
	}

	async function handleStreamDrop(e: DragEvent, channel: ChannelView, targetStreamId: number) {
		e.preventDefault();
		if (dragSourceChannelId !== channel.id || draggingStreamId === null || draggingStreamId === targetStreamId) {
			draggingStreamId = null; dragOverStreamId = null; dragSourceChannelId = null;
			return;
		}
		const ids = channel.streams.map(s => s.id);
		const fromIdx = ids.indexOf(draggingStreamId);
		const toIdx   = ids.indexOf(targetStreamId);
		if (fromIdx < 0 || toIdx < 0) return;
		ids.splice(fromIdx, 1); ids.splice(toIdx, 0, draggingStreamId);
		draggingStreamId = null; dragOverStreamId = null; dragSourceChannelId = null;
		try {
			await api.reorderChannelStreams(channel.id, ids);
			await loadData();
		} catch (e) {
			console.error('Failed to reorder streams:', e);
		}
	}

	// --- Cross-pane drop (streams FROM StreamsPane) ---
	async function handleChannelDrop(e: DragEvent, channel: ChannelView) {
		e.preventDefault();
		crossPaneDropTargetId = null;
		const data = e.dataTransfer?.getData('application/json');
		if (!data) return;
		try {
			const streamIds: number[] = JSON.parse(data);
			if (!Array.isArray(streamIds)) return;
			const currentIds = channel.streams.map(s => s.id);
			const merged = [...currentIds, ...streamIds.filter(id => !currentIds.includes(id))];
			await api.updateChannel(channel.id, { streams: merged });
			await loadData();
		} catch (err) {
			console.error('Failed to drop streams onto channel:', err);
		}
	}

	function handleChannelDragOver(e: DragEvent, channel: ChannelView) {
		if (e.dataTransfer?.types.includes('application/json')) {
			e.preventDefault();
			e.dataTransfer.dropEffect = 'copy';
			crossPaneDropTargetId = channel.id;
		}
	}

	// Track when a deleteConfirmId was just set, so the window click handler
	// doesn't immediately clear it on the same event.
	let deleteConfirmSetAt = 0;

	// Close menus when clicking outside
	function handleWindowClick(e: MouseEvent) {
		if (!(e.target as HTMLElement).closest('.kebab-wrapper')) {
			openMenuId = null;
			deleteConfirmId = null;
		}
	}

	import { onMount } from 'svelte';
	onMount(() => { loadData(); });
</script>

<svelte:window onclick={handleWindowClick} />

<div class="channels-pane">
	<div class="pane-header">
		<input type="text" bind:value={searchQuery} placeholder="Filter channels…" class="search-input" />
	</div>

	<div class="pane-content">
		{#if loading}
			<div class="state-msg">Loading channels…</div>
		{:else if filteredGroups.length === 0}
			<div class="state-msg">No channels found.</div>
		{:else}
			{#each filteredGroups as group (group.id)}
				<!-- GROUP ROW -->
				<div class="group-row">
					<button class="expand-btn" onclick={() => group.expanded = !group.expanded}>
						{#if group.expanded}<ChevronDown size={15} />{:else}<ChevronRight size={15} />{/if}
						<span class="group-name">{group.name}</span>
						<span class="badge">{group.channels.length}</span>
					</button>
				</div>

				{#if group.expanded}
					<div class="channels-list">
						{#each group.channels as channel (channel.id)}
							<!-- CHANNEL ROW -->
							<div
								class="channel-row"
								class:drop-target={crossPaneDropTargetId === channel.id}
								class:deleting={deletingId === channel.id}
								role="listitem"
								ondragover={(e) => handleChannelDragOver(e, channel)}
								ondragleave={() => crossPaneDropTargetId = null}
								ondrop={(e) => handleChannelDrop(e, channel)}
							>
								<!-- Expand toggle for streams sub-list -->
								<button class="ch-expand" onclick={() => channel.expanded = !channel.expanded} title="Show assigned streams">
									{#if channel.expanded}<ChevronDown size={13}/>{:else}<ChevronRight size={13}/>{/if}
								</button>

								<!-- Logo -->
								<div class="ch-logo">
									{#if channel.logo_url}
										<img src={channel.logo_url} alt="" />
									{:else}
										<Tv size={16} />
									{/if}
								</div>

								<!-- Name + number -->
								<div class="ch-text">
									<span class="ch-name">{channel.name}</span>
									{#if channel.channel_number}
										<span class="ch-num">#{channel.channel_number}</span>
									{/if}
								</div>

								<!-- Stream count badge -->
								{#if channel.streams.length > 0}
									<span class="stream-count" title="{channel.streams.length} stream(s) assigned">{channel.streams.length}</span>
								{/if}

								<!-- Actions -->
								<div class="ch-actions">
									<!-- Play -->
									<button class="icon-btn play-btn" title="Preview stream" onclick={() => playChannel(channel)}>
										<Play size={14} fill="currentColor" />
									</button>

									<!-- Kebab -->
									<div class="kebab-wrapper">
										<button class="icon-btn" title="More actions" onclick={(e) => { e.stopPropagation(); openMenuId = openMenuId === channel.id ? null : channel.id; }}>
											<MoreVertical size={14} />
										</button>

										{#if openMenuId === channel.id}
											<div class="dropdown-menu" role="menu" onclick={(e) => e.stopPropagation()}>
												<button class="menu-item" onclick={() => openEdit(channel)}>
													<Pencil size={13} /> Edit
												</button>
												<!-- Sort by Health: reorder streams using backend health scoring -->
												<button
													class="menu-item"
													disabled={channel.streams.length < 2 || sortingChannelId === channel.id}
													onclick={(e) => { e.stopPropagation(); sortByHealth(channel); }}
												>
													<ArrowUpDown size={13} /> {sortingChannelId === channel.id ? 'Sorting…' : 'Sort by Health'}
												</button>
												{#if deleteConfirmId === channel.id}
													<button class="menu-item danger confirm" onclick={(e) => { e.stopPropagation(); deleteChannel(channel); }}>
														<Trash2 size={13} /> Confirm Delete
													</button>
												{:else}
													<button class="menu-item danger" onclick={(e) => { e.stopPropagation(); deleteChannel(channel); }}>
														<Trash2 size={13} /> Delete
													</button>
												{/if}
											</div>
										{/if}
									</div>
								</div>
							</div>

							<!-- STREAM SUB-LIST (when channel is expanded) -->
							{#if channel.expanded}
								<div class="streams-sublist">
									{#if channel.streams.length === 0}
										<div class="sub-empty">No streams assigned. Drag streams here.</div>
									{:else}
										{#each channel.streams as stream, idx (stream.id)}
											<div
												class="sub-stream-row {getStreamHealthClass(stream)}"
												class:drag-over={dragOverStreamId === stream.id}
												draggable="true"
												role="listitem"
												ondragstart={(e) => handleStreamDragStart(e, channel.id, stream.id)}
												ondragover={(e) => { e.preventDefault(); dragOverStreamId = stream.id; }}
												ondragleave={() => dragOverStreamId = null}
												ondrop={(e) => handleStreamDrop(e, channel, stream.id)}
											>
												<!-- Condensed view: position, grip, name, key stats, actions -->
												<span class="sub-stream-pos">{idx + 1}.</span>
												<GripVertical size={13} class="grip-icon" />
												<!-- Health indicator dot: gray for untested -->
												{#if !stream.stream_stats}
													<span class="health-dot untested" title="Untested"></span>
												{/if}
												<span class="sub-stream-name">{stream.name}</span>
												<!-- Condensed stats: resolution + codec when available -->
												{#if stream.stream_stats}
													<span class="condensed-stats">
														{#if displayStat(stream.stream_stats.resolution)}
															<span class="stat-chip">{displayStat(stream.stream_stats.resolution)}</span>
														{/if}
														{#if displayStat(stream.stream_stats.video_codec)}
															<span class="stat-chip">{displayStat(stream.stream_stats.video_codec)}</span>
														{/if}
													</span>
												{/if}
												{#if stream.m3u_account_name}
													<span class="sub-provider">{stream.m3u_account_name}</span>
												{/if}
												<!-- Expand/collapse stats detail -->
												{#if stream.stream_stats}
													<button class="stats-toggle" title="Toggle detailed stats" onclick={() => stream.statsExpanded = !stream.statsExpanded}>
														{#if stream.statsExpanded}<ChevronUp size={11}/>{:else}<ChevronDown size={11}/>{/if}
													</button>
												{/if}
												<!-- Play individual stream -->
												<button class="sub-play-btn" title="Play this stream" onclick={() => { if (onPlayStream) onPlayStream({ url: stream.url || '', title: stream.name, uuid: '' }); }}>
													<Play size={11} fill="currentColor" />
												</button>
												<button class="remove-btn" title="Remove from channel" onclick={() => removeStream(channel, stream.id)}>✕</button>
											</div>
											<!-- Expanded stats detail (3 sub-rows) -->
											{#if stream.statsExpanded && stream.stream_stats}
												<div class="stream-stats-detail {getStreamHealthClass(stream)}">
													<div class="stats-row">
														<span class="stat-label">Status</span>
														<span class="stat-value" class:text-online={stream.stream_stats.status === 'online'} class:text-offline={stream.stream_stats.status !== 'online'}>{displayStat(stream.stream_stats.status) || '—'}</span>
														<span class="stat-label">Resolution</span>
														<span class="stat-value">{displayStat(stream.stream_stats.resolution) || '—'}</span>
													</div>
													<div class="stats-row">
														<span class="stat-label">Video</span>
														<span class="stat-value">{displayStat(stream.stream_stats.video_codec) || '—'}</span>
														<span class="stat-label">Audio</span>
														<span class="stat-value">{displayStat(stream.stream_stats.audio_codec) || '—'}</span>
													</div>
													<div class="stats-row">
														<span class="stat-label">FPS</span>
														<span class="stat-value">{displayStat(stream.stream_stats.fps) || '—'}</span>
														<span class="stat-label">Bitrate</span>
														<span class="stat-value">{stream.stream_stats.bitrate ? `${Math.round(stream.stream_stats.bitrate / 1000)}k` : '—'}</span>
													</div>
												</div>
											{/if}
										{/each}
									{/if}
								</div>
							{/if}
						{/each}
					</div>
				{/if}
			{/each}
		{/if}
	</div>
</div>

<!-- Edit channel modal (re-uses CreateChannelModal in edit mode) -->
<CreateChannelModal
	bind:show={showEditModal}
	channelId={editChannelId}
	initialData={editInitialData}
	onCreated={() => loadData()}
/>

<style lang="less">
.channels-pane {
	display: flex; flex-direction: column; height: 100%;
	background: var(--surface); border-radius: var(--radius); overflow: hidden;
}
.pane-header {
	padding: 12px 16px; border-bottom: 1px solid var(--border);
	.search-input {
		width: 100%; background: var(--bg); border: 1px solid var(--border);
		padding: 8px 12px; border-radius: var(--radius); color: var(--text-bright);
		font-size: 13px; &:focus { outline: 1px solid var(--accent); }
	}
}
.pane-content { flex: 1; overflow-y: auto; padding: 8px 0; }
.state-msg { padding: 32px; text-align: center; color: var(--text-dim); font-style: italic; font-size: 13px; }

.group-row .expand-btn {
	width: 100%; display: flex; align-items: center; gap: 8px;
	padding: 8px 16px; background: transparent; border: none;
	color: var(--text-bright); cursor: pointer; font-size: 13px; font-weight: 600; text-align: left;
	&:hover { background: rgba(255,255,255,0.05); }
	.group-name { flex: 1; }
	.badge { font-size: 11px; background: var(--surface-bright); padding: 2px 6px; border-radius: 10px; color: var(--text-dim); }
}

.channels-list { padding: 2px 0 8px; }

.channel-row {
	display: flex; align-items: center; gap: 6px;
	padding: 6px 12px 6px 16px; cursor: default;
	transition: background 0.15s, outline 0.15s;
	&:hover { background: rgba(255,255,255,0.03); .ch-actions { opacity: 1; } }
	&.drop-target { background: rgba(237,28,36,0.12); outline: 1px dashed var(--accent); }
	&.deleting { opacity: 0.4; pointer-events: none; }
}
.ch-expand { background: transparent; border: none; color: var(--text-dim); cursor: pointer; padding: 2px; border-radius: 3px; &:hover { color: var(--accent); } }
.ch-logo { width: 28px; height: 18px; display: flex; align-items: center; justify-content: center; background: var(--bg); border-radius: 3px; overflow: hidden; color: var(--text-dim); flex-shrink: 0; img { max-width: 100%; max-height: 100%; object-fit: contain; } }
.ch-text { flex: 1; display: flex; align-items: center; gap: 6px; min-width: 0; .ch-name { font-size: 13px; color: var(--text-bright); font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } .ch-num { font-size: 11px; color: var(--text-dim); flex-shrink: 0; } }
.stream-count { font-size: 10px; background: var(--surface-bright); padding: 1px 5px; border-radius: 8px; color: var(--text-dim); flex-shrink: 0; }
.ch-actions { display: flex; gap: 2px; opacity: 0; transition: opacity 0.15s; flex-shrink: 0; align-items: center; }
.icon-btn { background: transparent; border: none; color: var(--text-dim); padding: 4px; border-radius: 4px; cursor: pointer; display: flex; align-items: center; &:hover { color: var(--accent); background: rgba(237,28,36,0.1); } }
.play-btn { &:hover { color: #4ade80; background: rgba(74,222,128,0.1); } }

.kebab-wrapper { position: relative; }
.dropdown-menu {
	position: absolute; right: 0; top: calc(100% + 4px); z-index: 1000;
	background: var(--surface-bright); border: 1px solid var(--border-bright);
	border-radius: 8px; padding: 4px; min-width: 140px;
	box-shadow: 0 8px 24px rgba(0,0,0,0.4);
}
.menu-item {
	display: flex; align-items: center; gap: 8px; width: 100%; padding: 8px 12px;
	background: transparent; border: none; color: var(--text-bright); font-size: 13px;
	cursor: pointer; border-radius: 5px; text-align: left;
	&:hover { background: rgba(255,255,255,0.07); }
	&.danger { color: #f87171; &:hover { background: rgba(239,68,68,0.12); } }
	&.confirm { font-weight: 700; }
}

/* Stream sub-list — compact indentation to save horizontal space */
.streams-sublist { padding: 2px 8px 8px 28px; border-left: 2px solid rgba(255,255,255,0.05); margin: 0 8px 0 24px; }
.sub-empty { font-size: 12px; color: var(--text-dim); font-style: italic; padding: 6px 0; }
.sub-stream-row {
	display: flex; align-items: center; gap: 5px; padding: 4px 6px; border-radius: 5px;
	cursor: grab; transition: background 0.15s;
	&:hover { background: rgba(255,255,255,0.04); .remove-btn, .sub-play-btn { opacity: 1; } }
	&.drag-over { background: rgba(237,28,36,0.1); outline: 1px dashed var(--accent); }
}
.sub-stream-pos { font-size: 11px; color: var(--text-dim); min-width: 18px; text-align: right; flex-shrink: 0; font-variant-numeric: tabular-nums; }
:global(.grip-icon) { color: var(--text-dim); flex-shrink: 0; }
.sub-stream-name { flex: 1; font-size: 12px; color: var(--text-bright); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.sub-provider { font-size: 10px; color: var(--text-dim); background: var(--surface-bright); padding: 1px 5px; border-radius: 4px; flex-shrink: 0; }
.sub-play-btn { background: transparent; border: none; color: var(--text-dim); cursor: pointer; opacity: 0; padding: 2px 3px; border-radius: 3px; transition: opacity 0.15s; display: flex; align-items: center; &:hover { color: #4ade80; background: rgba(74,222,128,0.1); } }
.remove-btn { background: transparent; border: none; color: #f87171; font-size: 12px; cursor: pointer; opacity: 0; padding: 2px 4px; border-radius: 3px; transition: opacity 0.15s; &:hover { background: rgba(239,68,68,0.15); } }

/* =================== STREAM HEALTH DISPLAY =================== */
/* Offline/frozen/black_screen row shading — subtle red tint, not high contrast */
.sub-stream-row.health-offline,
.stream-stats-detail.health-offline {
	background: rgba(239, 68, 68, 0.06);
	border-left: 2px solid rgba(239, 68, 68, 0.25);
}

/* Untested indicator: gray dot */
.health-dot {
	display: inline-block;
	width: 6px; height: 6px;
	border-radius: 50%;
	flex-shrink: 0;
}
.health-dot.untested { background: var(--text-dim); opacity: 0.4; }

/* Condensed stats chips (resolution, codec) shown inline */
.condensed-stats {
	display: flex; gap: 3px; flex-shrink: 0;
}
.stat-chip {
	font-size: 9px; padding: 1px 5px; border-radius: 3px;
	background: rgba(99,102,241,0.12); color: #a5b4fc;
	text-transform: uppercase; letter-spacing: 0.3px; font-weight: 500;
}

/* Stats toggle chevron button */
.stats-toggle {
	background: transparent; border: none; color: var(--text-dim);
	cursor: pointer; padding: 2px 3px; border-radius: 3px;
	display: flex; align-items: center; flex-shrink: 0;
	&:hover { color: var(--text-bright); background: rgba(255,255,255,0.06); }
}

/* Expanded stats detail card — 3 sub-rows of label/value pairs */
.stream-stats-detail {
	display: flex; flex-direction: column; gap: 2px;
	padding: 4px 8px 6px 36px;
	margin-bottom: 2px;
	font-size: 11px;
}
.stats-row {
	display: flex; gap: 6px; align-items: center;
}
.stat-label {
	color: var(--text-dim); font-size: 10px; min-width: 52px;
	text-transform: uppercase; letter-spacing: 0.3px;
}
.stat-value {
	color: var(--text-bright); font-size: 11px; font-weight: 500;
	min-width: 60px;
}
.text-online { color: #4ade80; }
.text-offline { color: #f87171; }
</style>
