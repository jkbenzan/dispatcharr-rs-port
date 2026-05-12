import re

with open('svelte-frontend/src/routes/stream-checker/+page.svelte', 'r', encoding='utf-8') as f:
    content = f.read()

imports_repl = "import { Activity, Play, Square, Settings, CheckCircle2, XCircle, Clock, AlertCircle, RefreshCw, Plus, Pencil, Trash2, ListOrdered, ChevronDown, ChevronRight } from 'lucide-svelte';"
content = re.sub(r"import \{ Activity.*lucide-svelte';", imports_repl, content, count=1)

state_repl = """// State
	let channelGroups: any[] = $state([]);
	let loadingGroups = $state(false);
	
	let channelsByGroup: Record<number, any[]> = $state({});
	let loadingChannels: Record<number, boolean> = $state({});
	let expandedGroups: Set<number> = $state(new Set());
	let expandedChannels: Set<number> = $state(new Set());
	let channelsPage: Record<number, number> = $state({});
	let channelsHasNext: Record<number, boolean> = $state({});

	let selectedStreamIds: Set<number> = $state(new Set());"""
content = re.sub(r"// State.*?let selectAll = \$state\(false\);", state_repl, content, flags=re.DOTALL)

onmount_old = """onMount(async () => {
		loadSettings();
		providers = await api.getPlaylists().catch(() => []);
		checkStatus();
		loadRules();
	});"""
onmount_new = """onMount(async () => {
		loadSettings();
		loadGroups();
		checkStatus();
		loadRules();
	});"""
content = content.replace(onmount_old, onmount_new)

loadstreams_old = r"async function loadStreams\(\).*?selectedStreamIds = newSet;\n\t\}"
loadstreams_new = """async function loadGroups() {
		loadingGroups = true;
		try {
			const groupsRes = await api.getChannelGroups();
			channelGroups = Array.isArray(groupsRes) ? groupsRes : groupsRes.results || [];
			channelGroups.push({ id: -1, name: 'Ungrouped' });
		} catch (err) {
			console.error('Failed to load channel groups', err);
		} finally {
			loadingGroups = false;
		}
	}

	async function toggleGroup(groupId: number) {
		if (expandedGroups.has(groupId)) {
			expandedGroups.delete(groupId);
			expandedGroups = new Set(expandedGroups);
		} else {
			expandedGroups.add(groupId);
			expandedGroups = new Set(expandedGroups);
			if (!channelsByGroup[groupId]) {
				await loadChannelsForGroup(groupId, 1);
			}
		}
	}

	async function loadChannelsForGroup(groupId: number, page: number) {
		loadingChannels[groupId] = true;
		loadingChannels = { ...loadingChannels };
		try {
			const res = await api.getChannels({ channel_group: groupId === -1 ? '' : groupId, page, page_size: 50 });
			const newChannels = Array.isArray(res) ? res : res.results || [];
			
			if (page === 1) {
				channelsByGroup[groupId] = newChannels;
			} else {
				channelsByGroup[groupId] = [...(channelsByGroup[groupId] || []), ...newChannels];
			}
			channelsPage[groupId] = page;
			channelsHasNext[groupId] = !!res.next;
			
			channelsByGroup = { ...channelsByGroup };
			channelsPage = { ...channelsPage };
			channelsHasNext = { ...channelsHasNext };
		} catch (err) {
			console.error('Failed to load channels for group', groupId, err);
		} finally {
			loadingChannels[groupId] = false;
			loadingChannels = { ...loadingChannels };
		}
	}

	function toggleChannelExpansion(channelId: number, e?: Event) {
		if (e) e.stopPropagation();
		if (expandedChannels.has(channelId)) {
			expandedChannels.delete(channelId);
		} else {
			expandedChannels.add(channelId);
		}
		expandedChannels = new Set(expandedChannels);
	}

	function isStreamSelected(id: number) {
		return selectedStreamIds.has(id);
	}

	function toggleStreamSelection(id: number, e?: Event) {
		if (e) e.stopPropagation();
		const newSet = new Set(selectedStreamIds);
		if (newSet.has(id)) newSet.delete(id);
		else newSet.add(id);
		selectedStreamIds = newSet;
	}

	function toggleChannelSelection(channel: any, e?: Event) {
		if (e) e.stopPropagation();
		const newSet = new Set(selectedStreamIds);
		const streams = channel.streams || [];
		const allSelected = streams.length > 0 && streams.every(s => newSet.has(s.id));
		
		for (const s of streams) {
			if (allSelected) newSet.delete(s.id);
			else newSet.add(s.id);
		}
		selectedStreamIds = newSet;
	}

	function toggleGroupSelection(groupId: number, e?: Event) {
		if (e) e.stopPropagation();
		const newSet = new Set(selectedStreamIds);
		const channels = channelsByGroup[groupId] || [];
		const allStreams = channels.flatMap(ch => ch.streams || []);
		
		const allSelected = allStreams.length > 0 && allStreams.every(s => newSet.has(s.id));
		
		for (const s of allStreams) {
			if (allSelected) newSet.delete(s.id);
			else newSet.add(s.id);
		}
		selectedStreamIds = newSet;
	}

	function getChannelSelectionState(channel: any) {
		const streams = channel.streams || [];
		if (streams.length === 0) return { checked: false, indeterminate: false };
		let selectedCount = 0;
		for (const s of streams) {
			if (selectedStreamIds.has(s.id)) selectedCount++;
		}
		if (selectedCount === 0) return { checked: false, indeterminate: false };
		if (selectedCount === streams.length) return { checked: true, indeterminate: false };
		return { checked: false, indeterminate: true };
	}

	function getGroupSelectionState(groupId: number) {
		const channels = channelsByGroup[groupId] || [];
		const allStreams = channels.flatMap(ch => ch.streams || []);
		if (allStreams.length === 0) return { checked: false, indeterminate: false };
		
		let selectedCount = 0;
		for (const s of allStreams) {
			if (selectedStreamIds.has(s.id)) selectedCount++;
		}
		
		if (selectedCount === 0) return { checked: false, indeterminate: false };
		if (selectedCount === allStreams.length) return { checked: true, indeterminate: false };
		return { checked: false, indeterminate: true };
	}

	function indeterminate(node: HTMLInputElement, value: boolean) {
		node.indeterminate = value;
		return {
			update(newValue: boolean) {
				node.indeterminate = newValue;
			}
		};
	}"""
content = re.sub(loadstreams_old, loadstreams_new, content, flags=re.DOTALL)

# DOM Replacement
selection_panel_old = """<div class="selection-panel pane">
			<div class="pane-header">
				<h3>Select Streams</h3>
				<select bind:value={selectedProviderId} onchange={loadStreams} disabled={status?.is_running}>
					<option value="">-- Choose Provider --</option>
					{#each providers as provider}
						<option value={provider.id}>{provider.name}</option>
					{/each}
				</select>
			</div>

			<div class="pane-content">
				{#if loadingStreams}
					<div class="empty-state">
						<RefreshCw class="spin" size={24} />
						<p>Loading streams...</p>
					</div>
				{:else if !selectedProviderId}
					<div class="empty-state">
						<Activity size={32} class="icon-dim" />
						<p>Select a provider to load streams.</p>
					</div>
				{:else if streams.length === 0}
					<div class="empty-state">
						<AlertCircle size={32} class="icon-dim" />
						<p>No streams found for this provider.</p>
					</div>
				{:else}
					<div class="table-container">
						<table class="data-table">
							<thead>
								<tr>
									<th class="col-checkbox">
										<input type="checkbox" checked={selectAll} onchange={toggleSelectAll} disabled={status?.is_running} />
									</th>
									<th>Name</th>
									<th>Group</th>
									<th>Status</th>
								</tr>
							</thead>
							<tbody>
								{#each streams as stream}
									<tr class:selected={selectedStreamIds.has(stream.id)} onclick={() => !status?.is_running && toggleStream(stream.id)}>
										<td class="col-checkbox" onclick={(e) => e.stopPropagation()}>
											<input type="checkbox" checked={selectedStreamIds.has(stream.id)} onchange={() => toggleStream(stream.id)} disabled={status?.is_running} />
										</td>
										<td class="col-name">{stream.name}</td>
										<td class="col-group">{stream.group_title || 'Uncategorized'}</td>
										<td class="col-status">
											{#if stream.stream_stats?.status === 'online'}
												<span class="badge success">Online</span>
											{:else if stream.stream_stats?.status}
												<span class="badge error">{stream.stream_stats.status}</span>
											{:else}
												<span class="badge dim">Untested</span>
											{/if}
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</div>

			<div class="pane-footer">
				<div class="selection-info">
					{selectedStreamIds.size} streams selected
				</div>
				<button 
					class="btn-primary" 
					disabled={selectedStreamIds.size === 0 || status?.is_running}
					onclick={startBulkCheck}
				>
					<Play size={16} />
					<span>Start Test</span>
				</button>
			</div>
		</div>"""
selection_panel_new = """<div class="selection-panel pane">
			<div class="pane-header">
				<h3>Select Curated Streams</h3>
			</div>

			<div class="pane-content tree-content">
				{#if loadingGroups}
					<div class="empty-state">
						<RefreshCw class="spin" size={24} />
						<p>Loading channel groups...</p>
					</div>
				{:else if channelGroups.length === 0}
					<div class="empty-state">
						<AlertCircle size={32} class="icon-dim" />
						<p>No channel groups found.</p>
					</div>
				{:else}
					<div class="tree-container">
						{#each channelGroups as group}
							{@const groupState = getGroupSelectionState(group.id)}
							<div class="tree-group">
								<!-- Group Header -->
								<div class="tree-row group-row" onclick={() => toggleGroup(group.id)}>
									<button class="expand-btn">
										{#if expandedGroups.has(group.id)}
											<ChevronDown size={16} />
										{:else}
											<ChevronRight size={16} />
										{/if}
									</button>
									<div class="checkbox-wrapper" onclick={(e) => toggleGroupSelection(group.id, e)}>
										<input type="checkbox" checked={groupState.checked} use:indeterminate={groupState.indeterminate} disabled={status?.is_running} />
									</div>
									<span class="row-name">{group.name}</span>
								</div>

								<!-- Group Content (Channels) -->
								{#if expandedGroups.has(group.id)}
									<div class="tree-children">
										{#if loadingChannels[group.id] && !channelsByGroup[group.id]}
											<div class="loading-row">
												<RefreshCw class="spin" size={14} /> Loading channels...
											</div>
										{:else if (channelsByGroup[group.id] || []).length === 0}
											<div class="empty-row">No channels in this group.</div>
										{:else}
											{#each channelsByGroup[group.id] as channel}
												{@const channelState = getChannelSelectionState(channel)}
												<div class="tree-channel">
													<!-- Channel Header -->
													<div class="tree-row channel-row" onclick={() => toggleChannelExpansion(channel.id)}>
														<button class="expand-btn">
															{#if expandedChannels.has(channel.id)}
																<ChevronDown size={14} />
															{:else}
																<ChevronRight size={14} />
															{/if}
														</button>
														<div class="checkbox-wrapper" onclick={(e) => toggleChannelSelection(channel, e)}>
															<input type="checkbox" checked={channelState.checked} use:indeterminate={channelState.indeterminate} disabled={status?.is_running} />
														</div>
														<span class="row-number">{channel.channel_number ? channel.channel_number + ' - ' : ''}</span>
														<span class="row-name">{channel.name}</span>
													</div>

													<!-- Channel Content (Streams) -->
													{#if expandedChannels.has(channel.id)}
														<div class="tree-children streams-list">
															{#if !channel.streams || channel.streams.length === 0}
																<div class="empty-row">No streams assigned.</div>
															{:else}
																{#each channel.streams as stream}
																	{@const isSelected = isStreamSelected(stream.id)}
																	<div class="tree-row stream-row" class:selected={isSelected} onclick={(e) => !status?.is_running && toggleStreamSelection(stream.id, e)}>
																		<div class="stream-drag-spacer"></div>
																		<div class="checkbox-wrapper">
																			<input type="checkbox" checked={isSelected} onchange={(e) => toggleStreamSelection(stream.id, e)} disabled={status?.is_running} />
																		</div>
																		<div class="stream-info">
																			<div class="stream-main">
																				<span class="stream-name" title={stream.name}>{stream.name}</span>
																				<span class="stream-provider">{stream.m3u_account_name || 'Custom'}</span>
																			</div>
																			<div class="stream-stats">
																				{#if stream.stream_stats?.status === 'online'}
																					<span class="badge success">Online</span>
																				{:else if stream.stream_stats?.status}
																					<span class="badge error">{stream.stream_stats.status}</span>
																				{:else}
																					<span class="badge dim">Untested</span>
																				{/if}
																				{#if stream.stream_stats?.resolution}
																					<span class="stat-text">{stream.stream_stats.resolution}</span>
																				{/if}
																				{#if stream.stream_stats?.video_codec}
																					<span class="stat-text">{stream.stream_stats.video_codec}</span>
																				{/if}
																			</div>
																		</div>
																	</div>
																{/each}
															{/if}
														</div>
													{/if}
												</div>
											{/each}
											{#if channelsHasNext[group.id]}
												<button class="load-more-btn" onclick={() => loadChannelsForGroup(group.id, channelsPage[group.id] + 1)} disabled={loadingChannels[group.id]}>
													{#if loadingChannels[group.id]}
														<RefreshCw class="spin" size={14} /> Loading...
													{:else}
														Load More Channels
													{/if}
												</button>
											{/if}
										{/if}
									</div>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			</div>

			<div class="pane-footer">
				<div class="selection-info">
					{selectedStreamIds.size} streams selected
				</div>
				<button 
					class="btn-primary" 
					disabled={selectedStreamIds.size === 0 || status?.is_running}
					onclick={startBulkCheck}
				>
					<Play size={16} />
					<span>Start Test</span>
				</button>
			</div>
		</div>"""
content = content.replace(selection_panel_old, selection_panel_new)

with open('svelte-frontend/src/routes/stream-checker/+page.svelte', 'w', encoding='utf-8') as f:
    f.write(content)
