<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Activity, Play, Square, Settings, CheckCircle2, XCircle, Clock, AlertCircle, RefreshCw, Plus, Pencil, Trash2, ListOrdered } from 'lucide-svelte';
	import { api } from '$lib/api';
	import { toast } from '$lib/toast.svelte';

	// =================== TAB STATE ===================
	// Controls which view is shown: 'testing' (bulk stream checker) or 'rules' (sorting rules CRUD)
	let activeTab: 'testing' | 'rules' = $state('testing');

	// State
	let providers: any[] = $state([]);
	let selectedProviderId: number | '' = $state('');
	
	let streams: any[] = $state([]);
	let loadingStreams = $state(false);
	
	let selectedStreamIds: Set<number> = $state(new Set());
	let selectAll = $state(false);

	// Status State
	let status: any = $state(null);
	let pollingInterval: any = null;

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

	// Known stream stat properties for the property dropdown
	const RULE_PROPERTIES = ['resolution', 'video_codec', 'audio_codec', 'fps', 'bitrate', 'status', 'reachable'];
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
		providers = await api.getPlaylists().catch(() => []);
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

	async function loadStreams() {
		if (!selectedProviderId) {
			streams = [];
			selectedStreamIds.clear();
			selectAll = false;
			return;
		}

		loadingStreams = true;
		try {
			streams = await api.getStreams({ m3u_account_id: selectedProviderId });
			selectedStreamIds.clear();
			selectAll = false;
		} catch (err) {
			console.error(err);
		} finally {
			loadingStreams = false;
		}
	}

	function toggleSelectAll() {
		selectAll = !selectAll;
		if (selectAll) {
			selectedStreamIds = new Set(streams.map(s => s.id));
		} else {
			selectedStreamIds.clear();
		}
	}

	function toggleStream(id: number) {
		const newSet = new Set(selectedStreamIds);
		if (newSet.has(id)) {
			newSet.delete(id);
			selectAll = false;
		} else {
			newSet.add(id);
			if (newSet.size === streams.length) selectAll = true;
		}
		selectedStreamIds = newSet;
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
</style>
