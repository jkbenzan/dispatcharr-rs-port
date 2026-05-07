<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { Activity, Tv, AlertCircle, TrendingUp } from 'lucide-svelte';

	let stats = $state({
		total_channels: 0,
		active_streams: 0,
		errors: 0,
		bandwidth: '0 Mbps'
	});

	onMount(async () => {
		try {
			const res = await api.getSettings(); // Placeholder for actual stats endpoint
			// Populate stats here once we have the actual endpoint mapped
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
				<span class="stat-value">{stats.total_channels || 0}</span>
			</div>
		</div>
		<div class="stat-card">
			<div class="stat-icon green"><Activity size={24} /></div>
			<div class="stat-content">
				<span class="stat-label">Active Streams</span>
				<span class="stat-value">{stats.active_streams || 0}</span>
			</div>
		</div>
		<div class="stat-card">
			<div class="stat-icon yellow"><TrendingUp size={24} /></div>
			<div class="stat-content">
				<span class="stat-label">Bandwidth</span>
				<span class="stat-value">{stats.bandwidth}</span>
			</div>
		</div>
		<div class="stat-card">
			<div class="stat-icon red-dim"><AlertCircle size={24} /></div>
			<div class="stat-content">
				<span class="stat-label">Recent Errors</span>
				<span class="stat-value">{stats.errors}</span>
			</div>
		</div>
	</div>

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
