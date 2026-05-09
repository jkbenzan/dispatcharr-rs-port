<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { ChevronDown, ChevronRight, Play, RefreshCw, Layers, Database } from 'lucide-svelte';

	interface Stream {
		id: number;
		name: string;
		url: string | null;
		channel_group: any;
		m3u_account_id: number;
	}

	interface M3UGroupView {
		name: string;
		expanded: boolean;
		streams: Stream[];
	}

	interface M3UView {
		id: number;
		name: string;
		expanded: boolean;
		groups: M3UGroupView[];
		total_streams: number;
	}

	let m3uViews = $state<M3UView[]>([]);
	let loading = $state(true);
	let searchQuery = $state('');
	let assignedStreamIds = $state(new Set<number>());
	let channelGroupNames = new Map<number, string>();

	onMount(async () => {
		await loadData();
	});

	async function loadData() {
		loading = true;
		try {
			const [groups, channelsRes, m3us, streamsRes] = await Promise.all([
				api.getChannelGroups(),
				api.getChannels({ page_size: 5000 }),
				api.getPlaylists(),
				api.getStreams({ page_size: 10000 })
			]);

			// Process Groups
			const groupList = Array.isArray(groups) ? groups : groups.results || [];
			groupList.forEach((g: any) => channelGroupNames.set(g.id, g.name));

			// Process Assigned Streams
			const channelList = Array.isArray(channelsRes) ? channelsRes : channelsRes.results || [];
			const assigned = new Set<number>();
			channelList.forEach((ch: any) => {
				(ch.streams || []).forEach((s: any) => assigned.add(s.id));
			});
			assignedStreamIds = assigned;

			// Process Tree
			const playlistList = Array.isArray(m3us) ? m3us : m3us.results || [];
			const streamList = Array.isArray(streamsRes) ? streamsRes : streamsRes.results || [];
			buildTree(playlistList, streamList);
		} catch (e) {
			console.error('Failed to load streams:', e);
		} finally {
			loading = false;
		}
	}

	function buildTree(m3us: any[], streams: any[]) {
		const streamsByM3U = new Map<number, Map<string, any[]>>();

		streams.forEach(s => {
			const m3uId = s.m3u_account_id ?? s.m3u_account;
			if (m3uId == null) return;

			if (!streamsByM3U.has(m3uId)) {
				streamsByM3U.set(m3uId, new Map<string, any[]>());
			}

			const groupName = resolveGroupName(s.channel_group) || 'Ungrouped';
			const m3uGroups = streamsByM3U.get(m3uId)!;
			if (!m3uGroups.has(groupName)) m3uGroups.set(groupName, []);
			m3uGroups.get(groupName)!.push(s);
		});

		m3uViews = m3us.map(m => {
			const m3uGroups = streamsByM3U.get(m.id) || new Map<string, any[]>();
			const groups = Array.from(m3uGroups.keys()).sort().map(name => ({
				name,
				expanded: false,
				streams: m3uGroups.get(name)!
			}));

			return {
				id: m.id,
				name: m.name,
				expanded: false,
				groups,
				total_streams: groups.reduce((acc, g) => acc + g.streams.length, 0)
			};
		}).filter(m => m.total_streams > 0);
	}

	function resolveGroupName(channelGroup: any): string {
		if (channelGroup == null) return '';
		if (typeof channelGroup === 'number') {
			return channelGroupNames.get(channelGroup) || `Group #${channelGroup}`;
		}
		return String(channelGroup);
	}

	let filteredM3Us = $derived(
		m3uViews.map(m => {
			if (!searchQuery.trim()) return m;
			const q = searchQuery.toLowerCase();
			const matchedGroups = m.groups.map(g => {
				const matchedStreams = g.streams.filter(s => s.name.toLowerCase().includes(q));
				return matchedStreams.length > 0 ? { ...g, streams: matchedStreams, expanded: true } : null;
			}).filter((g): g is M3UGroupView => g !== null);

			return matchedGroups.length > 0 ? { ...m, groups: matchedGroups, expanded: true } : null;
		}).filter((m): m is M3UView => m !== null)
	);

	function handleDragStart(e: DragEvent, stream: any) {
		if (e.dataTransfer) {
			e.dataTransfer.setData('application/json', JSON.stringify([stream.id]));
			e.dataTransfer.effectAllowed = 'copy';
		}
	}
</script>

<div class="streams-pane">
	<div class="pane-header">
		<div class="search-box">
			<input type="text" bind:value={searchQuery} placeholder="Search streams..." />
		</div>
	</div>

	<div class="pane-content">
		{#if loading}
			<div class="loading">Loading streams...</div>
		{:else if filteredM3Us.length === 0}
			<div class="empty">No streams found.</div>
		{:else}
			{#each filteredM3Us as m3u}
				<div class="m3u-row">
					<button class="expand-btn" onclick={() => m3u.expanded = !m3u.expanded}>
						{#if m3u.expanded}
							<ChevronDown size={16} />
						{:else}
							<ChevronRight size={16} />
						{/if}
						<Database size={16} class="icon-dim" />
						<span class="m3u-name">{m3u.name}</span>
						<span class="badge">{m3u.total_streams}</span>
					</button>
				</div>

				{#if m3u.expanded}
					<div class="groups-list">
						{#each m3u.groups as group}
							<div class="group-row">
								<button class="expand-btn-group" onclick={() => group.expanded = !group.expanded}>
									{#if group.expanded}
										<ChevronDown size={14} />
									{:else}
										<ChevronRight size={14} />
									{/if}
									<Layers size={14} class="icon-dim" />
									<span class="group-name">{group.name}</span>
									<span class="badge-small">{group.streams.length}</span>
								</button>
							</div>

							{#if group.expanded}
								<div class="streams-list">
									{#each group.streams as stream}
										<div 
											class="stream-row" 
											class:assigned={assignedStreamIds.has(stream.id)}
											draggable="true"
											role="listitem"
											ondragstart={(e) => handleDragStart(e, stream)}
										>
											<div class="stream-info">
												<span class="stream-name">{stream.name}</span>
												{#if assignedStreamIds.has(stream.id)}
													<span class="assigned-badge">Assigned</span>
												{/if}
											</div>
											<div class="stream-actions">
												<button class="icon-btn"><Play size={14} /></button>
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
	.streams-pane {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--surface);
		border-radius: var(--radius);
		overflow: hidden;
	}

	.pane-header {
		padding: 16px;
		border-bottom: 1px solid var(--border);
	}

	.search-box {
		input {
			width: 100%;
			background: var(--bg);
			border: 1px solid var(--border);
			padding: 8px 12px;
			border-radius: var(--radius);
			color: var(--text-bright);
			font-size: 13px;
			&:focus { outline: 1px solid var(--accent); }
		}
	}

	.pane-content {
		flex: 1;
		overflow-y: auto;
		padding: 8px 0;
	}

	.expand-btn, .expand-btn-group {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 8px;
		background: transparent;
		border: none;
		color: var(--text-bright);
		cursor: pointer;
		text-align: left;
		transition: background 0.2s;
		&:hover { background: rgba(255, 255, 255, 0.05); }
	}

	.expand-btn {
		padding: 8px 16px;
		font-size: 13px;
		font-weight: 700;
	}

	.expand-btn-group {
		padding: 6px 16px 6px 32px;
		font-size: 12px;
		font-weight: 600;
		color: var(--text-dim);
	}

	.m3u-name, .group-name { flex: 1; }
	
	.badge, .badge-small {
		font-size: 10px;
		background: var(--surface-bright);
		padding: 1px 6px;
		border-radius: 10px;
		color: var(--text-dim);
	}

	:global(.icon-dim) { color: var(--text-dim); }

	.stream-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 6px 16px 6px 56px;
		cursor: grab;
		transition: background 0.2s;

		&:hover { 
			background: rgba(255, 255, 255, 0.03); 
			.stream-actions { opacity: 1; }
		}

		&.assigned {
			.stream-name { color: var(--text-dim); }
		}
	}

	.stream-info {
		display: flex;
		align-items: center;
		gap: 8px;
		.stream-name { font-size: 12px; color: var(--text-bright); }
		.assigned-badge {
			font-size: 9px;
			text-transform: uppercase;
			color: #4ade80;
			font-weight: 700;
			letter-spacing: 0.5px;
		}
	}

	.stream-actions {
		display: flex;
		gap: 4px;
		opacity: 0;
		transition: opacity 0.2s;
	}

	.icon-btn {
		background: transparent;
		border: none;
		color: var(--text-dim);
		padding: 3px;
		border-radius: 4px;
		cursor: pointer;
		&:hover { color: var(--accent); background: rgba(237, 28, 36, 0.1); }
	}

	.loading, .empty {
		padding: 32px;
		text-align: center;
		color: var(--text-dim);
		font-style: italic;
		font-size: 13px;
	}
</style>
