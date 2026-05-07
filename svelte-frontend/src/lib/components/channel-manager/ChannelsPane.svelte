<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { ChevronDown, ChevronRight, MoreVertical, Play, Info, CheckCircle2 } from 'lucide-svelte';

	interface StreamView {
		id: number;
		name: string;
		url: string | null;
		m3u_account_name: string;
	}

	interface ChannelView {
		id: number;
		uuid: string;
		name: string;
		channel_number: number | null;
		logo_url: string | null;
		expanded: boolean;
		streams: StreamView[];
	}

	interface GroupView {
		id: number;
		name: string;
		expanded: boolean;
		channels: ChannelView[];
	}

	let groupViews = $state<GroupView[]>([]);
	let loading = $state(true);
	let searchQuery = $state('');

	onMount(async () => {
		await loadData();
	});

	async function loadData() {
		loading = true;
		try {
			const [groups, channelsRes] = await Promise.all([
				api.getChannelGroups(),
				api.getChannels({ page_size: 5000 })
			]);

			const channelList = Array.isArray(channelsRes) ? channelsRes : channelsRes.results || [];
			const groupList = Array.isArray(groups) ? groups : groups.results || [];

			buildGroupViews(groupList, channelList);
		} catch (e) {
			console.error('Failed to load channels:', e);
		} finally {
			loading = false;
		}
	}

	function buildGroupViews(groups: any[], channels: any[]) {
		const channelsByGroup = new Map<number, any[]>();
		const ungrouped: any[] = [];

		channels.forEach(ch => {
			const gid = ch.channel_group_id || ch.channel_group;
			if (gid) {
				if (!channelsByGroup.has(gid)) channelsByGroup.set(gid, []);
				channelsByGroup.get(gid)!.push(ch);
			} else {
				ungrouped.push(ch);
			}
		});

		const views: GroupView[] = groups.map(g => ({
			id: g.id,
			name: g.name,
			expanded: false,
			channels: (channelsByGroup.get(g.id) || []).map(ch => ({
				...ch,
				expanded: false,
				streams: ch.streams || []
			}))
		})).filter(g => g.channels.length > 0);

		if (ungrouped.length > 0) {
			views.push({
				id: -1,
				name: 'Ungrouped',
				expanded: false,
				channels: ungrouped.map(ch => ({
					...ch,
					expanded: false,
					streams: ch.streams || []
				}))
			});
		}

		groupViews = views;
	}

	let filteredGroups = $derived(
		groupViews.map(g => {
			if (!searchQuery.trim()) return g;
			const q = searchQuery.toLowerCase();
			const matched = g.channels.filter(ch => 
				ch.name.toLowerCase().includes(q) || 
				(ch.channel_number && String(ch.channel_number).includes(q))
			);
			return matched.length > 0 ? { ...g, channels: matched, expanded: true } : null;
		}).filter((g): g is GroupView => g !== null)
	);
	let crossPaneDropTargetId = $state<number | null>(null);

	async function handleChannelDrop(e: DragEvent, channel: ChannelView) {
		e.preventDefault();
		crossPaneDropTargetId = null;

		const data = e.dataTransfer?.getData('application/json');
		if (!data) return;

		try {
			const streamIds = JSON.parse(data);
			if (!Array.isArray(streamIds)) return;

			// Add streams to channel
			const currentIds = channel.streams.map(s => s.id);
			const newIds = [...currentIds, ...streamIds.filter(id => !currentIds.includes(id))];
			
			await api.updateChannel(channel.id, { streams: newIds });
			await loadData(); // Refresh to show new streams
		} catch (err) {
			console.error('Failed to drop streams:', err);
		}
	}

	function handleChannelDragOver(e: DragEvent, channel: ChannelView) {
		if (e.dataTransfer?.types.includes('application/json')) {
			e.preventDefault();
			e.dataTransfer.dropEffect = 'copy';
			crossPaneDropTargetId = channel.id;
		}
	}
</script>

<div class="channels-pane">
	<div class="pane-header">
		<div class="search-box">
			<input type="text" bind:value={searchQuery} placeholder="Filter channels..." />
		</div>
	</div>

	<div class="pane-content">
		{#if loading}
			<div class="loading">Loading channels...</div>
		{:else if filteredGroups.length === 0}
			<div class="empty">No channels found.</div>
		{:else}
			{#each filteredGroups as group}
				<div class="group-row" class:expanded={group.expanded}>
					<button class="expand-btn" onclick={() => group.expanded = !group.expanded}>
						{#if group.expanded}
							<ChevronDown size={16} />
						{:else}
							<ChevronRight size={16} />
						{/if}
						<span class="group-name">{group.name}</span>
						<span class="badge">{group.channels.length}</span>
					</button>
				</div>

				{#if group.expanded}
					<div class="channels-list">
						{#each group.channels as channel}
							<div 
								class="channel-row" 
								class:drop-target={crossPaneDropTargetId === channel.id}
								ondragover={(e) => handleChannelDragOver(e, channel)}
								ondragleave={() => crossPaneDropTargetId = null}
								ondrop={(e) => handleChannelDrop(e, channel)}
							>
								<div class="channel-info">
									<div class="channel-logo">
										{#if channel.logo_url}
											<img src={channel.logo_url} alt="" />
										{:else}
											<Tv size={18} />
										{/if}
									</div>
									<div class="channel-text">
										<span class="ch-name">{channel.name}</span>
										<span class="ch-num">{channel.channel_number || ''}</span>
									</div>
								</div>
								
								<div class="channel-actions">
									<button class="icon-btn"><Play size={16} /></button>
									<button class="icon-btn"><MoreVertical size={16} /></button>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			{/each}
		{/if}
	</div>
</div>

<style lang="less">
	.channels-pane {
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

	.group-row {
		.expand-btn {
			width: 100%;
			display: flex;
			align-items: center;
			gap: 8px;
			padding: 8px 16px;
			background: transparent;
			border: none;
			color: var(--text-bright);
			cursor: pointer;
			font-size: 13px;
			font-weight: 600;
			text-align: left;
			
			&:hover { background: rgba(255, 255, 255, 0.05); }
		}

		.group-name { flex: 1; }
		.badge {
			font-size: 11px;
			background: var(--surface-bright);
			padding: 2px 6px;
			border-radius: 10px;
			color: var(--text-dim);
		}
	}

	.channels-list {
		padding: 4px 0 12px;
	}

	.channel-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 16px 8px 40px;
		cursor: pointer;
		transition: background 0.2s, border-color 0.2s;

		&:hover { 
			background: rgba(255, 255, 255, 0.03); 
			.channel-actions { opacity: 1; }
		}

		&.drop-target {
			background: rgba(237, 28, 36, 0.15);
			outline: 1px dashed var(--accent);
		}
	}

	.channel-info {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.channel-logo {
		width: 32px;
		height: 20px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--bg);
		border-radius: 4px;
		overflow: hidden;
		img { max-width: 100%; max-height: 100%; object-fit: contain; }
		color: var(--text-dim);
	}

	.channel-text {
		display: flex;
		flex-direction: column;
		.ch-name { font-size: 13px; color: var(--text-bright); font-weight: 500; }
		.ch-num { font-size: 11px; color: var(--text-dim); }
	}

	.channel-actions {
		display: flex;
		gap: 4px;
		opacity: 0;
		transition: opacity 0.2s;
	}

	.icon-btn {
		background: transparent;
		border: none;
		color: var(--text-dim);
		padding: 4px;
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
