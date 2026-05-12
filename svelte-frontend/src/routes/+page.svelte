<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { api } from '$lib/api';
	import { Activity, Tv, AlertCircle, TrendingUp, Radio, RefreshCw } from 'lucide-svelte';
	import ConnectionCard from '$lib/components/stats/ConnectionCard.svelte';

	// Dashboard summary stats
	let stats = $state({
		channels: 0,
		streams: 0,
		m3u_accounts: 0,
		failed_m3u_accounts: 0,
		epg_sources: 0,
		failed_epg_sources: 0,
		active_users: 0,
		system_health: 'Healthy',
		bandwidth: '0 Mbps'
	});

	// Tab state: 'overview' or 'connections'
	let activeTab = $state('overview');

	// Active connections state
	let connections = $state<any[]>([]);
	let connectionsLoading = $state(false);
	let connectionsError = $state('');
	let pollInterval: ReturnType<typeof setInterval> | null = null;

	// Configurable refresh interval (seconds), persisted in localStorage
	let refreshSeconds = $state(5);

	onMount(async () => {
		// Load dashboard summary
		try {
			const data = await api.getDashboardStats();
			Object.assign(stats, data);
		} catch (e) {
			console.error('Failed to load dashboard stats');
		}

		// Load persisted refresh interval
		const stored = localStorage.getItem('stats-refresh-interval');
		if (stored) refreshSeconds = parseInt(stored, 10) || 5;

		// Check URL hash for direct navigation to connections tab
		if (window.location.hash === '#connections') {
			activeTab = 'connections';
		}
	});

	onDestroy(() => {
		// Clean up polling when leaving the page
		if (pollInterval) clearInterval(pollInterval);
	});

	/**
	 * Fetch active connections from the proxy status endpoint.
	 * Handles errors gracefully — shows error state without crashing.
	 */
	async function fetchConnections() {
		connectionsError = '';
		try {
			const data = await api.getActiveConnections();
			connections = data?.channels || [];
		} catch (e: any) {
			connectionsError = e?.message || 'Failed to fetch connections';
			console.error('Failed to fetch active connections:', e);
		}
	}

	/**
	 * Start/restart the polling loop when switching to the connections tab.
	 * Clears any existing interval to prevent duplicates.
	 */
	function startPolling() {
		if (pollInterval) clearInterval(pollInterval);
		fetchConnections();
		pollInterval = setInterval(fetchConnections, refreshSeconds * 1000);
	}

	/**
	 * Stop polling when switching away from the connections tab.
	 */
	function stopPolling() {
		if (pollInterval) {
			clearInterval(pollInterval);
			pollInterval = null;
		}
	}

	/**
	 * Switch active tab and manage polling lifecycle.
	 */
	function setTab(tab: string) {
		activeTab = tab;
		if (tab === 'connections') {
			startPolling();
		} else {
			stopPolling();
		}
	}

	/**
	 * Update refresh interval and restart the polling loop.
	 */
	function updateRefreshInterval(e: Event) {
		const target = e.target as HTMLSelectElement;
		refreshSeconds = parseInt(target.value, 10);
		localStorage.setItem('stats-refresh-interval', String(refreshSeconds));
		if (activeTab === 'connections') startPolling();
	}

	/**
	 * Stop a channel — remove it from the connections list optimistically,
	 * then confirm with a re-fetch.
	 */
	async function handleStopChannel(channelId: string) {
		// Optimistic removal for instant feedback
		connections = connections.filter(c => c.channel_id !== channelId);
		try {
			await api.stopChannel(channelId);
		} catch (e) {
			console.error('Failed to stop channel:', e);
		}
		// Re-fetch to confirm server state
		await fetchConnections();
	}

	/**
	 * Stop a single client — remove from the specific connection optimistically.
	 */
	async function handleStopClient(channelId: string, clientId: string) {
		// Optimistic removal
		connections = connections.map(c => {
			if (c.channel_id === channelId) {
				return { ...c, clients: c.clients.filter((cl: any) => cl.client_id !== clientId) };
			}
			return c;
		}).filter(c => c.clients.length > 0);
		try {
			await api.stopClient(channelId, clientId);
		} catch (e) {
			console.error('Failed to stop client:', e);
		}
		await fetchConnections();
	}

	// Count total clients across all connections for the tab badge
	let totalClients = $derived(
		connections.reduce((sum, c) => sum + (c.clients?.length || 0), 0)
	);
</script>

<div class="dashboard fade-in">
	<div class="welcome">
		<h1>Dashboard</h1>
		<p>Overview of your streaming engine</p>
	</div>

	<!-- Tab Bar -->
	<div class="tab-bar">
		<button class="tab" class:active={activeTab === 'overview'} onclick={() => setTab('overview')}>
			Overview
		</button>
		<button class="tab" class:active={activeTab === 'connections'} onclick={() => setTab('connections')}>
			<Radio size={14} />
			Active Connections
			{#if connections.length > 0}
				<span class="tab-badge">{connections.length}</span>
			{/if}
		</button>
	</div>

	<!-- Overview Tab -->
	{#if activeTab === 'overview'}
		<div class="stats-grid">
			<div class="stat-card">
				<div class="stat-icon red"><Tv size={24} /></div>
				<div class="stat-content">
					<span class="stat-label">Total Channels</span>
					<span class="stat-value">{stats.channels || 0}</span>
				</div>
			</div>
			<div class="stat-card">
				<div class="stat-icon green"><Activity size={24} /></div>
				<div class="stat-content">
					<span class="stat-label">Active Broadcasters</span>
					<span class="stat-value">{stats.active_users || 0}</span>
				</div>
			</div>
			<div class="stat-card">
				<div class="stat-icon {stats.failed_m3u_accounts > 0 ? 'yellow' : 'blue'}"><TrendingUp size={24} /></div>
				<div class="stat-content">
					<span class="stat-label">M3U Providers</span>
					<span class="stat-value">
						{stats.m3u_accounts}
						{#if stats.failed_m3u_accounts > 0}
							<span class="failed-badge">({stats.failed_m3u_accounts} failed)</span>
						{/if}
					</span>
				</div>
			</div>
			<div class="stat-card">
				<div class="stat-icon {stats.failed_epg_sources > 0 ? 'red-dim' : 'green-dim'}"><AlertCircle size={24} /></div>
				<div class="stat-content">
					<span class="stat-label">EPG Sources</span>
					<span class="stat-value">
						{stats.epg_sources}
						{#if stats.failed_epg_sources > 0}
							<span class="failed-badge">({stats.failed_epg_sources} failed)</span>
						{/if}
					</span>
				</div>
			</div>
		</div>

		{#if stats.failed_m3u_accounts > 0 || stats.failed_epg_sources > 0}
			<div class="health-alert warning">
				<AlertCircle size={20} />
				<div class="alert-content">
					<strong>System Attention Required</strong>
					<p>Some data providers are currently failing to refresh. Check your provider settings.</p>
				</div>
				<a href="/streams" class="action-btn">Manage Providers</a>
			</div>
		{/if}

		<section class="recent-activity">
			<div class="section-header">
				<h2>System Events</h2>
				<a href="/activity" class="view-all">View All</a>
			</div>
			<div class="activity-list">
				<div class="empty-activity">
					<p>No recent events to display.</p>
				</div>
			</div>
		</section>

	<!-- Connections Tab -->
	{:else if activeTab === 'connections'}
		<div class="connections-header">
			<div class="connections-summary">
				<span class="conn-count">{connections.length} active channel{connections.length !== 1 ? 's' : ''}</span>
				<span class="client-count">{totalClients} connected client{totalClients !== 1 ? 's' : ''}</span>
			</div>
			<div class="connections-controls">
				<select class="refresh-select" value={refreshSeconds} onchange={updateRefreshInterval}>
					<option value="3">3s</option>
					<option value="5">5s</option>
					<option value="10">10s</option>
					<option value="30">30s</option>
					<option value="60">60s</option>
				</select>
				<button class="refresh-btn" onclick={fetchConnections} title="Refresh now">
					<RefreshCw size={14} />
					Refresh
				</button>
			</div>
		</div>

		{#if connectionsError}
			<div class="connections-error">
				<AlertCircle size={16} />
				<span>{connectionsError}</span>
			</div>
		{/if}

		{#if connections.length === 0}
			<div class="empty-connections">
				<Radio size={48} />
				<p>No active connections</p>
				<span>Channels will appear here when clients connect to the stream proxy.</span>
			</div>
		{:else}
			<div class="connections-grid">
				{#each connections as conn (conn.channel_id)}
					<ConnectionCard
						connection={conn}
						onStopChannel={handleStopChannel}
						onStopClient={handleStopClient}
					/>
				{/each}
			</div>
		{/if}
	{/if}
</div>

<style lang="less">
	.dashboard {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	.welcome {
		h1 {
			font-size: 32px;
			font-weight: 700;
			color: var(--text-bright);
			margin-bottom: 8px;
		}
		p {
			color: var(--text-dim);
			font-size: 16px;
		}
	}

	/* =================== Tab Bar =================== */
	.tab-bar {
		display: flex;
		gap: 4px;
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 4px;
	}

	.tab {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 8px 16px;
		background: none;
		border: none;
		border-radius: 6px;
		color: var(--text-dim);
		font-size: 14px;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.15s;

		&:hover {
			color: var(--text-bright);
			background: rgba(255,255,255,0.03);
		}

		&.active {
			color: var(--text-bright);
			background: var(--border);
			font-weight: 600;
		}
	}

	.tab-badge {
		background: var(--accent);
		color: white;
		font-size: 11px;
		font-weight: 700;
		padding: 1px 7px;
		border-radius: 10px;
		min-width: 18px;
		text-align: center;
	}

	/* =================== Stats Grid =================== */
	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
		gap: 24px;
	}

	.stat-card {
		background: var(--surface);
		padding: 24px;
		border-radius: var(--radius);
		border: 1px solid var(--border);
		display: flex;
		align-items: center;
		gap: 20px;
		transition: transform 0.2s, border-color 0.2s;

		&:hover {
			transform: translateY(-4px);
			border-color: var(--border-bright);
		}
	}

	.stat-icon {
		width: 52px;
		height: 52px;
		border-radius: 12px;
		display: flex;
		align-items: center;
		justify-content: center;
		
		&.red { background: rgba(237, 28, 36, 0.1); color: var(--accent); }
		&.green { background: rgba(74, 222, 128, 0.1); color: #4ade80; }
		&.yellow { background: rgba(250, 204, 21, 0.1); color: #facc15; }
		&.blue { background: rgba(96, 165, 250, 0.1); color: #60a5fa; }
		&.red-dim { background: rgba(155, 18, 24, 0.1); color: var(--accent-dim); }
		&.green-dim { background: rgba(74, 222, 128, 0.05); color: #4ade80; }
	}

	.stat-content {
		display: flex;
		flex-direction: column;
	}

	.stat-label {
		font-size: 13px;
		color: var(--text-dim);
		font-weight: 500;
	}

	.stat-value {
		font-size: 24px;
		font-weight: 700;
		color: var(--text-bright);
		display: flex;
		align-items: baseline;
		gap: 8px;

		.failed-badge {
			font-size: 14px;
			color: var(--accent);
			font-weight: 500;
		}
	}

	/* =================== Health Alert =================== */
	.health-alert {
		background: rgba(237, 28, 36, 0.05);
		border: 1px solid rgba(237, 28, 36, 0.2);
		border-radius: var(--radius);
		padding: 20px 24px;
		display: flex;
		align-items: center;
		gap: 20px;
		margin-bottom: 8px;

		&.warning {
			background: rgba(250, 204, 21, 0.05);
			border-color: rgba(250, 204, 21, 0.2);
			color: #facc15;
		}

		.alert-content {
			flex: 1;
			strong { display: block; color: var(--text-bright); margin-bottom: 2px; }
			p { color: var(--text-dim); font-size: 14px; margin: 0; }
		}

		.action-btn {
			background: var(--surface-bright);
			border: 1px solid var(--border-bright);
			color: var(--text-bright);
			padding: 8px 16px;
			border-radius: 6px;
			text-decoration: none;
			font-size: 14px;
			font-weight: 600;
			transition: background 0.2s;

			&:hover { background: var(--border); }
		}
	}

	/* =================== Recent Activity =================== */
	.recent-activity {
		margin-top: 8px;
	}

	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 24px;

		h2 { font-size: 20px; font-weight: 600; color: var(--text-bright); }
		.view-all { color: var(--accent); text-decoration: none; font-size: 14px; font-weight: 600; }
	}

	.activity-list {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		min-height: 200px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.empty-activity {
		color: var(--text-dim);
		font-style: italic;
		font-size: 14px;
	}

	/* =================== Connections Tab =================== */
	.connections-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		flex-wrap: wrap;
	}

	.connections-summary {
		display: flex;
		gap: 16px;
		font-size: 14px;

		.conn-count {
			color: var(--text-bright);
			font-weight: 600;
		}
		.client-count {
			color: var(--text-dim);
		}
	}

	.connections-controls {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.refresh-select {
		background: var(--surface);
		border: 1px solid var(--border);
		color: var(--text);
		padding: 6px 10px;
		border-radius: 6px;
		font-size: 13px;
		cursor: pointer;

		&:focus {
			outline: 1px solid var(--accent);
		}
	}

	.refresh-btn {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 12px;
		background: var(--surface);
		border: 1px solid var(--border);
		color: var(--text);
		border-radius: 6px;
		font-size: 13px;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.15s;

		&:hover {
			border-color: var(--accent);
			color: var(--accent);
		}
	}

	.connections-error {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 12px 16px;
		background: rgba(239, 68, 68, 0.05);
		border: 1px solid rgba(239, 68, 68, 0.2);
		border-radius: var(--radius);
		color: #ef4444;
		font-size: 13px;
	}

	.empty-connections {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 12px;
		padding: 60px 20px;
		color: var(--text-dim);
		text-align: center;
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);

		p {
			font-size: 18px;
			font-weight: 600;
			color: var(--text);
			margin: 0;
		}
		span {
			font-size: 13px;
			max-width: 400px;
		}
	}

	.connections-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(420px, 1fr));
		gap: 16px;
	}
</style>
