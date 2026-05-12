<script lang="ts">
	import { Wifi, WifiOff, Square, Clock, HardDrive, User, X } from 'lucide-svelte';

	// Props: a single channel connection object from /proxy/ts/status
	let {
		connection,
		onStopChannel,
		onStopClient
	}: {
		connection: any;
		onStopChannel: (channelId: string) => void;
		onStopClient: (channelId: string, clientId: string) => void;
	} = $props();

	// Track confirmation state for stop-channel button (two-click pattern)
	let confirmStop = $state(false);

	/**
	 * Format seconds into a human-readable HH:MM:SS string.
	 * Handles edge cases: NaN, negative values, and very large uptimes.
	 */
	function formatUptime(seconds: number): string {
		if (!seconds || seconds < 0) return '00:00:00';
		const h = Math.floor(seconds / 3600);
		const m = Math.floor((seconds % 3600) / 60);
		const s = Math.floor(seconds % 60);
		return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
	}

	/**
	 * Format bytes into a human-readable string (KB, MB, GB).
	 * Uses 1024-based units for accuracy.
	 */
	function formatBytes(bytes: number): string {
		if (!bytes || bytes <= 0) return '0 B';
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
		return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
	}

	/**
	 * Truncate user-agent strings that are too long for the compact table.
	 */
	function truncateUA(ua: string, max: number = 40): string {
		if (!ua) return 'Unknown';
		return ua.length > max ? ua.substring(0, max) + '…' : ua;
	}

	function handleStopChannel() {
		if (!confirmStop) {
			confirmStop = true;
			// Auto-reset confirmation after 3 seconds if not clicked again
			setTimeout(() => { confirmStop = false; }, 3000);
			return;
		}
		onStopChannel(connection.channel_id);
		confirmStop = false;
	}
</script>

<div class="connection-card">
	<!-- Card Header: Channel identity -->
	<div class="card-header">
		<div class="channel-identity">
			{#if connection.logo_url}
				<img src={connection.logo_url} alt="" class="channel-logo" />
			{:else}
				<div class="channel-logo-placeholder">
					<Wifi size={20} />
				</div>
			{/if}
			<div class="channel-info">
				<span class="channel-name">{connection.channel_name || connection.channel_id}</span>
				{#if connection.channel_number}
					<span class="channel-number">CH {connection.channel_number}</span>
				{/if}
			</div>
		</div>
		<div class="live-indicator">
			<span class="live-dot"></span>
			LIVE
		</div>
	</div>

	<!-- Stats Bar: Uptime + Data transferred -->
	<div class="stats-bar">
		<div class="stat-item">
			<Clock size={14} />
			<span class="stat-label">Uptime</span>
			<span class="stat-value">{formatUptime(connection.uptime)}</span>
		</div>
		<div class="stat-item">
			<HardDrive size={14} />
			<span class="stat-label">Data</span>
			<span class="stat-value">{formatBytes(connection.total_bytes)}</span>
		</div>
		<div class="stat-item">
			<User size={14} />
			<span class="stat-label">Clients</span>
			<span class="stat-value">{connection.clients?.length || 0}</span>
		</div>
	</div>

	<!-- Profile Badges -->
	<div class="badge-row">
		{#if connection.stream_profile}
			<span class="badge profile">Profile: {connection.stream_profile}</span>
		{/if}
		{#if connection.m3u_profile?.name}
			<span class="badge provider">M3U: {connection.m3u_profile.name}</span>
		{/if}
	</div>

	<!-- Client Table -->
	{#if connection.clients && connection.clients.length > 0}
		<div class="clients-section">
			<table class="clients-table">
				<thead>
					<tr>
						<th>IP</th>
						<th>User Agent</th>
						<th>Duration</th>
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each connection.clients as client}
						<tr>
							<td class="ip-cell">{client.ip_address}</td>
							<td class="ua-cell" title={client.user_agent}>{truncateUA(client.user_agent)}</td>
							<td class="duration-cell">{formatUptime(client.connection_duration)}</td>
							<td class="action-cell">
								<button
									class="stop-client-btn"
									title="Disconnect this client"
									onclick={() => onStopClient(connection.channel_id, client.client_id)}
								>
									<X size={14} />
								</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	<!-- Card Footer: Stop Channel -->
	<div class="card-footer">
		<button
			class="stop-channel-btn"
			class:confirm={confirmStop}
			onclick={handleStopChannel}
		>
			<Square size={14} />
			{confirmStop ? 'Confirm Stop' : 'Stop Channel'}
		</button>
	</div>
</div>

<style lang="less">
	.connection-card {
		background: var(--surface);
		border: 1px solid var(--border);
		border-left: 3px solid #4ade80;
		border-radius: var(--radius);
		overflow: hidden;
		transition: border-color 0.2s, box-shadow 0.2s;

		&:hover {
			border-color: var(--border-bright);
			box-shadow: 0 4px 16px rgba(0,0,0,0.2);
		}
	}

	.card-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px;
		background: linear-gradient(135deg, rgba(255,255,255,0.02) 0%, rgba(255,255,255,0.05) 100%);
		border-bottom: 1px solid var(--border);
	}

	.channel-identity {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}

	.channel-logo {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		object-fit: contain;
		background: rgba(0,0,0,0.2);
	}

	.channel-logo-placeholder {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		background: rgba(74, 222, 128, 0.1);
		display: flex;
		align-items: center;
		justify-content: center;
		color: #4ade80;
	}

	.channel-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.channel-name {
		font-weight: 600;
		color: var(--text-bright);
		font-size: 15px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.channel-number {
		font-size: 12px;
		color: var(--text-dim);
		font-weight: 500;
	}

	.live-indicator {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 11px;
		font-weight: 700;
		color: #4ade80;
		letter-spacing: 0.05em;
	}

	.live-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #4ade80;
		animation: pulse 1.5s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; box-shadow: 0 0 0 0 rgba(74, 222, 128, 0.4); }
		50% { opacity: 0.7; box-shadow: 0 0 0 6px rgba(74, 222, 128, 0); }
	}

	.stats-bar {
		display: flex;
		gap: 16px;
		padding: 12px 16px;
		border-bottom: 1px solid var(--border);
	}

	.stat-item {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 13px;
		color: var(--text-dim);

		.stat-value {
			color: var(--text-bright);
			font-weight: 600;
			font-variant-numeric: tabular-nums;
		}
	}

	.badge-row {
		display: flex;
		gap: 8px;
		padding: 8px 16px;
		flex-wrap: wrap;
	}

	.badge {
		font-size: 11px;
		padding: 3px 8px;
		border-radius: 4px;
		font-weight: 600;

		&.profile {
			background: rgba(96, 165, 250, 0.1);
			color: #60a5fa;
		}
		&.provider {
			background: rgba(250, 204, 21, 0.1);
			color: #facc15;
		}
	}

	.clients-section {
		padding: 0 16px 8px;
	}

	.clients-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 12px;

		th {
			text-align: left;
			color: var(--text-dim);
			font-weight: 500;
			padding: 6px 8px;
			border-bottom: 1px solid var(--border);
		}

		td {
			padding: 6px 8px;
			color: var(--text);
			border-bottom: 1px solid rgba(255,255,255,0.03);
		}

		.ip-cell { font-family: monospace; font-size: 11px; }
		.ua-cell { max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
		.duration-cell { font-variant-numeric: tabular-nums; }
		.action-cell { text-align: right; }
	}

	.stop-client-btn {
		background: none;
		border: none;
		color: var(--text-dim);
		cursor: pointer;
		padding: 4px;
		border-radius: 4px;
		transition: color 0.15s, background 0.15s;

		&:hover {
			color: #ef4444;
			background: rgba(239, 68, 68, 0.1);
		}
	}

	.card-footer {
		padding: 12px 16px;
		border-top: 1px solid var(--border);
		display: flex;
		justify-content: flex-end;
	}

	.stop-channel-btn {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 14px;
		border: 1px solid var(--border);
		background: var(--surface);
		color: var(--text-dim);
		border-radius: 6px;
		font-size: 12px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s;

		&:hover {
			border-color: #ef4444;
			color: #ef4444;
			background: rgba(239, 68, 68, 0.05);
		}

		&.confirm {
			border-color: #ef4444;
			background: rgba(239, 68, 68, 0.15);
			color: #ef4444;
			animation: shake 0.3s ease-in-out;
		}
	}

	@keyframes shake {
		0%, 100% { transform: translateX(0); }
		25% { transform: translateX(-3px); }
		75% { transform: translateX(3px); }
	}
</style>
