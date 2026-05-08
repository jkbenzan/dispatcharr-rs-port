<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { Activity, Tv, AlertCircle, TrendingUp } from 'lucide-svelte';

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

	onMount(async () => {
		try {
			const data = await api.getDashboardStats();
			Object.assign(stats, data);
		} catch (e) {
			console.error('Failed to load dashboard stats');
		}
	});
</script>

<div class="dashboard fade-in">
	<div class="welcome">
		<h1>Dashboard</h1>
		<p>Overview of your streaming engine</p>
	</div>

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
</div>

<style lang="less">
	.dashboard {
		display: flex;
		flex-direction: column;
		gap: 32px;
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
		&.red-dim { background: rgba(155, 18, 24, 0.1); color: var(--accent-dim); }
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

	.recent-activity {
		margin-top: 16px;
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
</style>
