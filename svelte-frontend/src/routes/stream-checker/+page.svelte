<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Activity, Play, Square, Settings, CheckCircle2, XCircle, Clock, AlertCircle, RefreshCw, Plus, Pencil, Trash2, ListOrdered, ChevronDown, ChevronRight } from 'lucide-svelte';
	import { api } from '$lib/api';
	import { toast } from '$lib/toast.svelte';

	// =================== TAB STATE ===================
	// Controls which view is shown: 'testing' (bulk stream checker) or 'rules' (sorting rules CRUD)
	let activeTab: 'testing' | 'rules' = $state('testing');

	// State
	let channelGroups: any[] = $state([]);
	let loadingGroups = $state(false);
	
	let channelsByGroup: Record<number, any[]> = $state({});
	let loadingChannels: Record<number, boolean> = $state({});
	let expandedGroups: Set<number> = $state(new Set());
	let expandedChannels: Set<number> = $state(new Set());
	let channelsPage: Record<number, number> = $state({});
	let channelsHasNext: Record<number, boolean> = $state({});

	let selectedStreamIds: Set<number> = $state(new Set());

	// Status State
	let status: any = $state(null);
	let pollingInterval: any = null;
	let sortingStatus: any = $state(null);

	// Settings State
	let streamSettingsId: number | null = $state(null);
	let parallelProviders = $state(1);
	let loadingSettings = $state(false);

	// =================== SORTING RULES STATE ===================
	interface SortingRule {
		id?: number;
		name: string;
		priority: number;
		property: string;
		operator: string;
		value: string;
		score_modifier: number;
	}

	// Known stream stat properties saved by the backend stream checker.
	// Keep the legacy resolution aliases available so older rules remain easy to edit.
	const RULE_PROPERTIES = [
		'status',
		'reachable',
		'height',
		'width',
		'resolution',
		'resolution_height',
		'resolution_width',
		'fps',
		'bitrate',
		'video_codec',
		'audio_codec',
		'audio_channels',
		'consecutive_failures'
	];
	const RULE_OPERATORS = ['==', '!=', '>=', '<=', 'contains'];

	let rules: SortingRule[] = $state([]);
	let loadingRules = $state(false);
	let editingRuleId: number | null = $state(null);
	// Form state for adding/editing a rule
	let ruleForm: SortingRule = $state({ name: '', priority: 0, property: 'resolution', operator: '==', value: '', score_modifier: 10 });
	let showAddRow = $state(false);
	let savingRule = $state(false);
	let deletingRuleId: number | null = $state(null);

	onMount(async () => {
		loadSettings();
		loadGroups();
		checkStatus();
		loadRules();
	});

	onDestroy(() => {
		if (pollingInterval) clearInterval(pollingInterval);
	});

	async function loadSettings() {
		try {
			loadingSettings = true;
			const settings = await api.getSettings();
			const streamSettings = settings.find((s: any) => s.key === 'stream_settings');
			if (streamSettings) {
				streamSettingsId = streamSettings.id;
				parallelProviders = streamSettings.value.stream_checker_parallel_providers || 1;
			}
		} catch (err) {
			console.error('Failed to load settings', err);
		} finally {
			loadingSettings = false;
		}
	}

	async function updateParallelProviders() {
		if (!streamSettingsId) return;
		try {
			// Fetch fresh settings first to merge
			const settingsList = await api.getSettings();
			const current = settingsList.find((s: any) => s.id === streamSettingsId);
			if (current) {
				const newValue = {
					...current.value,
					stream_checker_parallel_providers: parallelProviders
				};
				await api.updateSetting(streamSettingsId, { key: 'stream_settings', value: newValue });
			}
		} catch (err) {
			console.error('Failed to update concurrency', err);
		}
	}

	async function checkStatus() {
		try {
			const res = await api.getBulkCheckStatus();
			status = res;

			if (status?.last_results?.length > 0) {
				let mutated = false;
				for (const groupChannels of Object.values(channelsByGroup)) {
					for (const channel of groupChannels) {
						if (channel.streams) {
							for (const stream of channel.streams) {
								const result = status.last_results.find((r: any) => r.id === stream.id);
								if (result && result.stream_stats) {
									// only mutate if different
									if (JSON.stringify(stream.stream_stats) !== JSON.stringify(result.stream_stats)) {
										stream.stream_stats = result.stream_stats;
										stream.stream_stats_updated_at = new Date().toISOString();
										mutated = true;
									}
								}
							}
						}
					}
				}
				if (mutated) {
					channelsByGroup = { ...channelsByGroup };
				}
			}

			if (status.is_running && !pollingInterval) {
				pollingInterval = setInterval(checkStatus, 2000);
			} else if (!status.is_running && pollingInterval) {
				clearInterval(pollingInterval);
				pollingInterval = null;
			}
		} catch (err) {
			console.error(err);
		}
	}


	async function loadGroups() {
		loadingGroups = true;
		try {
			const groupsRes = await api.getChannelGroups();
			const rawGroups = Array.isArray(groupsRes) ? groupsRes : groupsRes.results || [];
			channelGroups = rawGroups.filter((g: any) => g.is_custom === true);
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
		const allSelected = streams.length > 0 && streams.every((s: any) => newSet.has(s.id));
		
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
		
		const allSelected = allStreams.length > 0 && allStreams.every((s: any) => newSet.has(s.id));
		
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
	}

	async function startBulkCheck() {
		if (selectedStreamIds.size === 0) return;
		try {
			await api.bulkCheckStreams(Array.from(selectedStreamIds));
			// Give it a moment to initialize before polling
			setTimeout(checkStatus, 500);
		} catch (err: any) {
			toast.error(err.message || 'Failed to start bulk check');
		}
	}

	async function cancelBulkCheck() {
		try {
			await api.cancelBulkCheck();
		} catch (err: any) {
			toast.error(err.message || 'Failed to cancel');
		}
	}

	async function sortSelectedChannels() {
		if (selectedStreamIds.size === 0) return;
		
		// Find which channels have any of the selected streams
		const channelIdsToSort = new Set<number>();
		for (const groupChannels of Object.values(channelsByGroup)) {
			for (const channel of groupChannels) {
				if (channel.streams) {
					for (const stream of channel.streams) {
						if (selectedStreamIds.has(stream.id)) {
							channelIdsToSort.add(channel.id);
							break;
						}
					}
				}
			}
		}

		if (channelIdsToSort.size === 0) return;

		try {
			toast.info(`Sorting streams in ${channelIdsToSort.size} channels...`);
			await api.bulkSortStreams(Array.from(channelIdsToSort));
			
			// Refresh those groups so the UI reflects the new sort order
			const affectedGroups = new Set<number>();
			for (const groupChannels of Object.values(channelsByGroup)) {
				for (const channel of groupChannels) {
					if (channelIdsToSort.has(channel.id)) {
						const groupId = channel.channel_group || -1;
						affectedGroups.add(groupId);
					}
				}
			}
			for (const groupId of affectedGroups) {
				await loadChannelsForGroup(groupId, 1);
			}
			toast.success('Sorting complete!');
		} catch (err: any) {
			toast.error(err.message || 'Failed to sort channels');
		}
	}

	function getProgressPercentage() {
		if (!status || status.total === 0) return 0;
		return Math.round((status.completed / status.total) * 100);
	}

	// =================== SORTING RULES CRUD ===================

	/** Load all sorting rules from the backend */
	async function loadRules() {
		loadingRules = true;
		try {
			const data = await api.getSortingRules();
			// Sort by priority ascending for display
			rules = (Array.isArray(data) ? data : []).sort((a: SortingRule, b: SortingRule) => a.priority - b.priority);
		} catch (err) {
			console.error('Failed to load sorting rules:', err);
			toast.error('Failed to load sorting rules');
		} finally {
			loadingRules = false;
		}
	}

	/** Reset the form to default empty state */
	function resetRuleForm() {
		ruleForm = { name: '', priority: 0, property: 'resolution', operator: '==', value: '', score_modifier: 10 };
		showAddRow = false;
		editingRuleId = null;
	}

	/** Start editing an existing rule — populate form with its values */
	function startEditRule(rule: SortingRule) {
		editingRuleId = rule.id!;
		ruleForm = { ...rule };
		showAddRow = false;
	}

	/** Save a new or updated rule to the backend */
	async function saveRule() {
		if (!ruleForm.name.trim()) {
			toast.error('Rule name is required');
			return;
		}
		savingRule = true;
		try {
			if (editingRuleId) {
				await api.updateSortingRule(editingRuleId, ruleForm);
				toast.success('Rule updated');
			} else {
				await api.createSortingRule(ruleForm);
				toast.success('Rule created');
			}
			resetRuleForm();
			await loadRules();
		} catch (err: any) {
			toast.error(err.message || 'Failed to save rule');
		} finally {
			savingRule = false;
		}
	}

	/** Delete a sorting rule with confirmation (two-click) */
	async function deleteRule(id: number) {
		if (deletingRuleId !== id) {
			// First click: enter confirmation state
			deletingRuleId = id;
			return;
		}
		// Second click: actually delete
		try {
			await api.deleteSortingRule(id);
			toast.success('Rule deleted');
			if (editingRuleId === id) resetRuleForm();
			await loadRules();
		} catch (err: any) {
			toast.error(err.message || 'Failed to delete rule');
		} finally {
			deletingRuleId = null;
		}
	}
</script>

<div class="page-container">
	<header class="page-header">
		<div>
			<h1>Stream Checker</h1>
			<p class="subtitle">Bulk health testing & stream sorting rules</p>
		</div>
		
		<!-- Tab bar: Testing (bulk checker) vs Sorting Rules (CRUD) -->
		<div class="tab-bar">
			<button class="tab-btn" class:active={activeTab === 'testing'} onclick={() => activeTab = 'testing'}>
				<Activity size={15} />
				<span>Testing</span>
			</button>
			<button class="tab-btn" class:active={activeTab === 'rules'} onclick={() => activeTab = 'rules'}>
				<ListOrdered size={15} />
				<span>Sorting Rules</span>
				{#if rules.length > 0}
					<span class="tab-badge">{rules.length}</span>
				{/if}
			</button>
		</div>

		{#if activeTab === 'testing'}
		<div class="header-settings">
			<div class="setting-item" title="Parallel Providers">
				<Settings size={16} class="icon-dim" />
				<label for="parallel">Concurrency:</label>
				<input 
					type="number" 
					id="parallel" 
					bind:value={parallelProviders} 
					min="1" 
					max="10"
					onchange={updateParallelProviders}
					disabled={loadingSettings || status?.is_running}
				/>
			</div>
		</div>
		{/if}
	</header>

	<div class="main-content">
		{#if activeTab === 'testing'}
		<!-- Left Panel: Selection -->
		<div class="selection-panel pane">
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
																	{@const isTesting = status?.workers?.some((w: any) => w.current_stream_id === stream.id)}
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
																				{#if isTesting}
																					<span class="badge testing"><RefreshCw class="spin" size={10} style="margin-right:4px;" />Testing...</span>
																				{:else if stream.stream_stats?.status === 'online'}
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
																				{#if stream.stream_stats?.source_fps}
																					<span class="stat-text">{Number(stream.stream_stats.source_fps).toFixed(0)} FPS</span>
																				{/if}
																				{#if stream.stream_stats?.video_bitrate}
																					<span class="stat-text">{Math.round(stream.stream_stats.video_bitrate)} kbps</span>
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
				<div style="display: flex; gap: 8px;">
					<button 
						class="btn-secondary" 
						disabled={selectedStreamIds.size === 0 || status?.is_running}
						onclick={sortSelectedChannels}
					>
						<ListOrdered size={16} />
						<span>Sort Checked</span>
					</button>
					<button 
						class="btn-primary" 
						disabled={selectedStreamIds.size === 0 || status?.is_running}
						onclick={startBulkCheck}
					>
						<Play size={16} />
						<span>Start Test</span>
					</button>
				</div>
			</div>
		</div>

		<!-- Right Panel: Status Dashboard -->
		<div class="status-panel pane">
			<div class="pane-header">
				<h3>Live Status</h3>
				{#if status?.is_running}
					<button class="btn-cancel" onclick={cancelBulkCheck}>
						<Square size={14} fill="currentColor" />
						<span>Cancel</span>
					</button>
				{/if}
			</div>

			<div class="pane-content status-content">
				{#if !status || (!status.is_running && status.total === 0)}
					<div class="empty-state">
						<Activity size={48} class="icon-dim" style="opacity: 0.5;" />
						<p>No active tests.</p>
					</div>
				{:else}
					<div class="progress-section">
						<div class="progress-stats">
							<div class="stat">
								<span class="val">{status.completed} / {status.total}</span>
								<span class="lbl">Completed</span>
							</div>
							<div class="stat success">
								<span class="val">{status.successful}</span>
								<span class="lbl">Online</span>
							</div>
							<div class="stat error">
								<span class="val">{status.failed}</span>
								<span class="lbl">Failed</span>
							</div>
						</div>
						
						<div class="progress-bar-container">
							<div class="progress-bar" style="width: {getProgressPercentage()}%"></div>
						</div>
					</div>

					{#if status.workers?.length > 0}
						<div class="workers-section">
							<h4>Active Workers</h4>
							<div class="workers-grid">
								{#each status.workers as worker}
									<div class="worker-card">
										<div class="worker-header">
											<span class="provider-name">{worker.m3u_account_name}</span>
											<span class="worker-progress">{worker.completed}/{worker.total}</span>
										</div>
										<div class="worker-current">
											<span class="truncate">Testing: {worker.current_stream_name || '...'}</span>
										</div>
									</div>
								{/each}
							</div>
						</div>
					{/if}

					<div class="results-section">
						<h4>Recent Results</h4>
						<div class="results-list">
							{#each status.last_results.slice().reverse() as res}
								<div class="result-card">
									<div class="res-icon">
										{#if res.reachable && res.status === 'online'}
											<CheckCircle2 size={18} class="text-success" />
										{:else}
											<XCircle size={18} class="text-error" />
										{/if}
									</div>
									<div class="res-info">
										<div class="res-name truncate">{res.name || 'Unknown Stream'}</div>
										<div class="res-meta">
											{#if res.reachable}
												<span>{res.resolution || 'Unknown res'}</span>
												<span>•</span>
												<span>{res.video_codec || 'N/A'}</span>
												<span>•</span>
												<span class={res.status === 'online' ? 'text-success' : 'text-error'}>{res.status}</span>
											{:else}
												<span class="text-error">Unreachable / Timeout</span>
											{/if}
										</div>
									</div>
								</div>
							{/each}
							{#if status.last_results.length === 0}
								<div class="empty-results">Waiting for results...</div>
							{/if}
						</div>
					</div>
				{/if}
			</div>
		</div>
		<!-- End of Testing Tab -->

		{:else}
		<!-- =================== SORTING RULES TAB =================== -->
		<div class="rules-panel pane" style="max-width: 100%;">
			<div class="pane-header">
				<h3>Sorting Rules</h3>
				<p class="rules-desc">Define rules that assign score modifiers to streams based on health properties.</p>
			</div>

			<div class="pane-content">
				{#if loadingRules}
					<div class="empty-state">
						<RefreshCw class="spin" size={24} />
						<p>Loading rules...</p>
					</div>
				{:else if rules.length === 0 && !showAddRow}
					<div class="empty-state">
						<ListOrdered size={48} class="icon-dim" style="opacity: 0.5;" />
						<p>No sorting rules defined yet.</p>
						<p class="text-dim">Rules assign score modifiers to streams based on their health properties.</p>
						<button class="btn-primary" onclick={() => showAddRow = true}>
							<Plus size={16} />
							<span>Add First Rule</span>
						</button>
					</div>
				{:else}
					<div class="table-container">
						<table class="data-table rules-table">
							<thead>
								<tr>
									<th class="col-priority">Priority</th>
									<th class="col-name">Name</th>
									<th class="col-prop">Property</th>
									<th class="col-op">Operator</th>
									<th class="col-val">Value</th>
									<th class="col-score">Score Mod.</th>
									<th class="col-actions">Actions</th>
								</tr>
							</thead>
							<tbody>
								{#each rules as rule (rule.id)}
									{#if editingRuleId === rule.id}
										<!-- Inline edit mode for this rule -->
										<tr class="editing-row">
											<td><input type="number" bind:value={ruleForm.priority} min="0" class="input-sm" /></td>
											<td><input type="text" bind:value={ruleForm.name} class="input-sm" placeholder="Rule name" /></td>
											<td>
												<select bind:value={ruleForm.property} class="input-sm">
													{#each RULE_PROPERTIES as prop}
														<option value={prop}>{prop}</option>
													{/each}
												</select>
											</td>
											<td>
												<select bind:value={ruleForm.operator} class="input-sm">
													{#each RULE_OPERATORS as op}
														<option value={op}>{op}</option>
													{/each}
												</select>
											</td>
											<td><input type="text" bind:value={ruleForm.value} class="input-sm" placeholder="e.g. 1080p" /></td>
											<td><input type="number" bind:value={ruleForm.score_modifier} class="input-sm" /></td>
											<td class="actions-cell">
												<button class="btn-icon btn-save" onclick={saveRule} disabled={savingRule} title="Save">
													<CheckCircle2 size={16} />
												</button>
												<button class="btn-icon btn-dim" onclick={resetRuleForm} title="Cancel">
													<XCircle size={16} />
												</button>
											</td>
										</tr>
									{:else}
										<!-- Display row -->
										<tr>
											<td class="col-priority">{rule.priority}</td>
											<td class="col-name">{rule.name}</td>
											<td><span class="badge prop">{rule.property}</span></td>
											<td><code>{rule.operator}</code></td>
											<td>{rule.value}</td>
											<td class:positive={rule.score_modifier > 0} class:negative={rule.score_modifier < 0}>
												{rule.score_modifier > 0 ? '+' : ''}{rule.score_modifier}
											</td>
											<td class="actions-cell">
												<button class="btn-icon" onclick={() => startEditRule(rule)} title="Edit">
													<Pencil size={14} />
												</button>
												<button
													class="btn-icon btn-danger"
													class:confirming={deletingRuleId === rule.id}
													onclick={() => deleteRule(rule.id!)}
													title={deletingRuleId === rule.id ? 'Click again to confirm' : 'Delete'}
												>
													<Trash2 size={14} />
												</button>
											</td>
										</tr>
									{/if}
								{/each}

								<!-- Inline add row (shown when user clicks "Add Rule") -->
								{#if showAddRow}
									<tr class="editing-row add-row">
										<td><input type="number" bind:value={ruleForm.priority} min="0" class="input-sm" /></td>
										<td><input type="text" bind:value={ruleForm.name} class="input-sm" placeholder="Rule name" /></td>
										<td>
											<select bind:value={ruleForm.property} class="input-sm">
												{#each RULE_PROPERTIES as prop}
													<option value={prop}>{prop}</option>
												{/each}
											</select>
										</td>
										<td>
											<select bind:value={ruleForm.operator} class="input-sm">
												{#each RULE_OPERATORS as op}
													<option value={op}>{op}</option>
												{/each}
											</select>
										</td>
										<td><input type="text" bind:value={ruleForm.value} class="input-sm" placeholder="e.g. 1080p" /></td>
										<td><input type="number" bind:value={ruleForm.score_modifier} class="input-sm" /></td>
										<td class="actions-cell">
											<button class="btn-icon btn-save" onclick={saveRule} disabled={savingRule} title="Create Rule">
												<CheckCircle2 size={16} />
											</button>
											<button class="btn-icon btn-dim" onclick={resetRuleForm} title="Cancel">
												<XCircle size={16} />
											</button>
										</td>
									</tr>
								{/if}
							</tbody>
						</table>
					</div>

					{#if !showAddRow && !editingRuleId}
						<div class="rules-footer">
							<button class="btn-primary" onclick={() => { resetRuleForm(); showAddRow = true; }}>
								<Plus size={16} />
								<span>Add Rule</span>
							</button>
						</div>
					{/if}
				{/if}
			</div>
		</div>
		{/if}
	</div>
</div>

<style lang="less">
	.page-container {
		display: flex;
		flex-direction: column;
		gap: 20px;
		height: 100%;
		overflow: hidden;
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-end;
		flex-shrink: 0;

		h1 {
			font-size: 24px;
			font-weight: 700;
			color: var(--text-bright);
			margin: 0 0 4px 0;
		}

		.subtitle {
			color: var(--text-dim);
			margin: 0;
			font-size: 14px;
		}
	}

	.header-settings {
		.setting-item {
			display: flex;
			align-items: center;
			gap: 8px;
			background: var(--surface);
			border: 1px solid var(--border);
			padding: 6px 12px;
			border-radius: var(--radius);
			font-size: 13px;
			
			label {
				color: var(--text-dim);
				font-weight: 500;
			}

			input {
				background: rgba(0,0,0,0.2);
				border: 1px solid var(--border);
				color: var(--text-bright);
				width: 50px;
				padding: 4px;
				border-radius: 4px;
				text-align: center;
				
				&:focus {
					outline: none;
					border-color: var(--accent);
				}
				
				&:disabled {
					opacity: 0.5;
				}
			}
		}
	}

	.main-content {
		display: flex;
		gap: 20px;
		flex: 1;
		min-height: 0;
	}

	.pane {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.selection-panel {
		flex: 1;
		max-width: 60%;
	}

	.status-panel {
		flex: 1;
		background: rgba(0,0,0,0.2);
	}

	.pane-header {
		padding: 16px;
		border-bottom: 1px solid var(--border);
		background: var(--surface-bright);
		display: flex;
		justify-content: space-between;
		align-items: center;

		h3 {
			margin: 0;
			font-size: 16px;
			font-weight: 600;
			color: var(--text-bright);
		}

		select {
			background: rgba(0,0,0,0.2);
			border: 1px solid var(--border);
			color: var(--text-bright);
			padding: 6px 12px;
			border-radius: 4px;
			font-size: 13px;
			outline: none;

			&:focus { border-color: var(--accent); }
		}
	}

	.pane-content {
		flex: 1;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
	}

	.empty-state {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		color: var(--text-dim);
		gap: 12px;
		padding: 32px;
		text-align: center;
		
		p { margin: 0; font-size: 14px; }
	}

	.table-container {
		flex: 1;
	}

	.data-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 13px;

		th {
			position: sticky;
			top: 0;
			background: var(--surface-bright);
			text-align: left;
			padding: 10px 16px;
			font-weight: 600;
			color: var(--text-dim);
			border-bottom: 1px solid var(--border);
			z-index: 10;
		}

		td {
			padding: 10px 16px;
			border-bottom: 1px solid var(--border);
			color: var(--text-bright);
		}

		tr {
			transition: background 0.1s;
			cursor: pointer;

			&:hover {
				background: rgba(255, 255, 255, 0.02);
			}

			&.selected {
				background: rgba(255, 255, 255, 0.05);
			}
		}

		.col-checkbox {
			width: 40px;
			text-align: center;
			padding: 10px 8px;
		}

		.col-group {
			color: var(--text-dim);
			white-space: nowrap;
		}

		.col-name {
			max-width: 200px;
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
		}
	}

	.badge {
		font-size: 11px;
		padding: 2px 6px;
		border-radius: 4px;
		font-weight: 600;
		text-transform: capitalize;

		&.success { background: rgba(74, 222, 128, 0.1); color: #4ade80; }
		&.error { background: rgba(237, 28, 36, 0.1); color: var(--accent); }
		&.dim { background: rgba(255, 255, 255, 0.1); color: var(--text-dim); }
	}

	.text-success { color: #4ade80; }
	.text-error { color: var(--accent); }

	.pane-footer {
		padding: 16px;
		border-top: 1px solid var(--border);
		background: var(--surface-bright);
		display: flex;
		justify-content: space-between;
		align-items: center;

		.selection-info {
			font-size: 13px;
			color: var(--text-dim);
			font-weight: 500;
		}
	}

	.btn-primary {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--accent);
		color: white;
		border: none;
		padding: 8px 16px;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 13px;
		cursor: pointer;
		transition: all 0.2s;

		&:hover:not(:disabled) {
			background: var(--accent-dim);
		}

		&:disabled {
			opacity: 0.5;
			cursor: not-allowed;
		}
	}

	.btn-cancel {
		display: flex;
		align-items: center;
		gap: 6px;
		background: rgba(237, 28, 36, 0.1);
		border: 1px solid var(--accent);
		color: var(--accent);
		padding: 6px 12px;
		border-radius: var(--radius);
		font-size: 12px;
		font-weight: 600;
		cursor: pointer;

		&:hover {
			background: var(--accent);
			color: white;
		}
	}

	.status-content {
		padding: 24px;
		gap: 24px;
	}

	.progress-section {
		display: flex;
		flex-direction: column;
		gap: 12px;
		background: var(--surface);
		padding: 16px;
		border-radius: var(--radius);
		border: 1px solid var(--border);

		.progress-stats {
			display: flex;
			justify-content: space-between;
			
			.stat {
				display: flex;
				flex-direction: column;
				align-items: center;
				
				.val { font-size: 20px; font-weight: 700; color: var(--text-bright); }
				.lbl { font-size: 11px; text-transform: uppercase; color: var(--text-dim); letter-spacing: 0.5px; }
				
				&.success .val { color: #4ade80; }
				&.error .val { color: var(--accent); }
			}
		}

		.progress-bar-container {
			height: 8px;
			background: rgba(255, 255, 255, 0.1);
			border-radius: 4px;
			overflow: hidden;

			.progress-bar {
				height: 100%;
				background: var(--accent);
				transition: width 0.3s ease;
			}
		}
	}

	.workers-section {
		h4 {
			font-size: 13px;
			text-transform: uppercase;
			color: var(--text-dim);
			margin: 0 0 12px 0;
			letter-spacing: 0.5px;
		}

		.workers-grid {
			display: grid;
			grid-template-columns: 1fr;
			gap: 8px;
		}

		.worker-card {
			background: var(--surface);
			border: 1px solid var(--border);
			padding: 12px;
			border-radius: var(--radius);
			display: flex;
			flex-direction: column;
			gap: 4px;

			.worker-header {
				display: flex;
				justify-content: space-between;
				font-size: 12px;
				font-weight: 600;
				color: var(--text-bright);
			}

			.worker-current {
				font-size: 11px;
				color: var(--text-dim);
				
				.truncate {
					display: block;
					white-space: nowrap;
					overflow: hidden;
					text-overflow: ellipsis;
				}
			}
		}
	}

	.results-section {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;

		h4 {
			font-size: 13px;
			text-transform: uppercase;
			color: var(--text-dim);
			margin: 0 0 12px 0;
			letter-spacing: 0.5px;
		}

		.results-list {
			display: flex;
			flex-direction: column;
			gap: 8px;
			overflow-y: auto;
			padding-right: 4px;

			.empty-results {
				text-align: center;
				padding: 20px;
				color: var(--text-dim);
				font-size: 13px;
				font-style: italic;
			}
		}

		.result-card {
			display: flex;
			gap: 12px;
			background: var(--surface);
			border: 1px solid var(--border);
			padding: 12px;
			border-radius: var(--radius);
			align-items: center;

			.res-icon {
				display: flex;
				align-items: center;
				justify-content: center;
			}

			.res-info {
				flex: 1;
				min-width: 0;

				.res-name {
					font-size: 13px;
					font-weight: 600;
					color: var(--text-bright);
					margin-bottom: 2px;
				}

				.res-meta {
					display: flex;
					gap: 6px;
					font-size: 11px;
					color: var(--text-dim);
					align-items: center;
				}

				.truncate {
					white-space: nowrap;
					overflow: hidden;
					text-overflow: ellipsis;
				}
			}
		}
	}
	:global(.spin) {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		100% { transform: rotate(360deg); }
	}

	/* =================== TAB BAR =================== */
	.tab-bar {
		display: flex;
		gap: 4px;
		background: rgba(0,0,0,0.15);
		border-radius: var(--radius);
		padding: 3px;
	}
	.tab-btn {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 7px 16px;
		border: none;
		border-radius: calc(var(--radius) - 2px);
		background: transparent;
		color: var(--text-dim);
		font-size: 13px;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s ease;

		&:hover { color: var(--text-bright); background: rgba(255,255,255,0.04); }
		&.active {
			background: var(--accent);
			color: #fff;
			box-shadow: 0 1px 4px rgba(0,0,0,0.2);
		}
	}
	.tab-badge {
		background: rgba(255,255,255,0.15);
		padding: 1px 7px;
		border-radius: 10px;
		font-size: 11px;
		font-weight: 600;
		line-height: 1.4;
	}
	.tab-btn.active .tab-badge { background: rgba(255,255,255,0.25); }

	/* =================== SORTING RULES TABLE =================== */
	.rules-panel {
		flex: 1;
	}
	.rules-desc {
		color: var(--text-dim);
		font-size: 12px;
		margin: 0;
	}
	.rules-table {
		th { font-size: 12px; text-transform: uppercase; letter-spacing: 0.5px; }
		.col-priority { width: 80px; text-align: center; }
		.col-name { min-width: 140px; }
		.col-prop { width: 120px; }
		.col-op { width: 90px; text-align: center; }
		.col-val { min-width: 100px; }
		.col-score { width: 100px; text-align: center; }
		.col-actions { width: 90px; text-align: right; }

		code {
			background: rgba(0,0,0,0.2);
			padding: 2px 6px;
			border-radius: 4px;
			font-size: 12px;
			color: var(--text-bright);
		}
	}
	.badge.prop {
		background: rgba(99,102,241,0.15);
		color: #a5b4fc;
		font-size: 11px;
		padding: 2px 8px;
		border-radius: 4px;
	}
	.positive { color: #4ade80; font-weight: 600; }
	.negative { color: #f87171; font-weight: 600; }

	/* Inline form inputs for add/edit rows */
	.input-sm {
		background: rgba(0,0,0,0.25);
		border: 1px solid var(--border);
		color: var(--text-bright);
		padding: 5px 8px;
		border-radius: 4px;
		font-size: 12px;
		width: 100%;
		box-sizing: border-box;

		&:focus { outline: none; border-color: var(--accent); }
	}
	select.input-sm { cursor: pointer; }
	.editing-row {
		background: rgba(99,102,241,0.06) !important;
		td { padding-top: 6px; padding-bottom: 6px; }
	}
	.add-row { border-top: 1px dashed var(--border); }

	/* Action buttons inside table rows */
	.actions-cell {
		display: flex;
		gap: 4px;
		justify-content: flex-end;
		align-items: center;
	}
	.btn-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border: none;
		border-radius: 4px;
		background: transparent;
		color: var(--text-dim);
		cursor: pointer;
		transition: all 0.15s ease;

		&:hover { background: rgba(255,255,255,0.08); color: var(--text-bright); }
		&:disabled { opacity: 0.4; cursor: not-allowed; }
	}
	.btn-save { color: #4ade80; &:hover { background: rgba(74,222,128,0.12); } }
	.btn-dim { color: var(--text-dim); }
	.btn-danger {
		&:hover { color: #f87171; background: rgba(248,113,113,0.12); }
		&.confirming {
			color: #fff;
			background: #dc2626;
			animation: pulse-danger 0.8s ease infinite;
		}
	}
	@keyframes pulse-danger {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.7; }
	}

	.rules-footer {
		padding: 12px 16px;
		border-top: 1px solid var(--border);
	}
	/* =================== TREE VIEW (Stream Checker Left Pane) =================== */
	.tree-content {
		padding: 12px;
		background: var(--surface-bright);
	}
	.tree-container {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.tree-group {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		overflow: hidden;
	}
	.tree-row {
		display: flex;
		align-items: center;
		padding: 8px 12px;
		gap: 8px;
		cursor: pointer;
		user-select: none;
		transition: background 0.1s;

		&:hover {
			background: rgba(255,255,255,0.02);
		}
	}
	.group-row {
		background: rgba(0,0,0,0.1);
		border-bottom: 1px solid var(--border);
		font-weight: 600;
	}
	.channel-row {
		border-bottom: 1px solid var(--border);
		padding-left: 24px;
	}
	.stream-row {
		padding-left: 48px;
		padding-top: 10px;
		padding-bottom: 10px;
		border-bottom: 1px solid var(--border);
		
		&:last-child {
			border-bottom: none;
		}

		&.selected {
			background: rgba(255,255,255,0.05);
		}
	}
	.expand-btn {
		background: none;
		border: none;
		color: var(--text-dim);
		padding: 2px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		border-radius: 4px;

		&:hover {
			background: rgba(255,255,255,0.1);
			color: var(--text-bright);
		}
	}
	.checkbox-wrapper {
		display: flex;
		align-items: center;
	}
	.row-number {
		color: var(--text-dim);
		font-size: 13px;
	}
	.row-name {
		color: var(--text-bright);
		font-size: 14px;
	}
	.stream-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
		flex: 1;
		min-width: 0;
	}
	.stream-main {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 13px;
	}
	.stream-name {
		color: var(--text-bright);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.stream-provider {
		background: rgba(255,255,255,0.1);
		padding: 2px 6px;
		border-radius: 4px;
		font-size: 10px;
		color: var(--text-dim);
		white-space: nowrap;
	}
	.stream-stats {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.stat-text {
		font-size: 11px;
		color: var(--text-dim);
	}
	.loading-row, .empty-row {
		padding: 12px 24px;
		color: var(--text-dim);
		font-size: 13px;
		display: flex;
		align-items: center;
		gap: 8px;
		font-style: italic;
	}
	.load-more-btn {
		width: 100%;
		padding: 10px;
		background: rgba(0,0,0,0.2);
		border: none;
		color: var(--text-dim);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		font-size: 13px;
		transition: all 0.2s;

		&:hover:not(:disabled) {
			background: rgba(255,255,255,0.05);
			color: var(--text-bright);
		}

		&:disabled {
			opacity: 0.5;
			cursor: not-allowed;
		}
	}
</style>
