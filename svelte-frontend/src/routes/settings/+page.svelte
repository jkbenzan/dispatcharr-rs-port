<script lang="ts">
	import { onMount } from 'svelte';
	import { Settings, Server, Globe, HardDrive, Shield, Users, Database, MonitorPlay, Save, CheckCircle2 } from 'lucide-svelte';
	import { api } from '$lib/api';
	import { toast } from '$lib/toast.svelte';
	import { loadUiSettings } from '$lib/settings.svelte';

	let activeTab = $state('ui_settings');
	let settingsList: any[] = $state([]);
	let saving = $state(false);
	let savedMessage = $state('');

	const tabs = [
		{ id: 'ui_settings', label: 'UI Settings', icon: MonitorPlay },
		{ id: 'system_settings', label: 'System', icon: Settings },
		{ id: 'stream_settings', label: 'Stream Engine', icon: Server },
		{ id: 'dvr_settings', label: 'DVR & Comskip', icon: HardDrive },
		{ id: 'proxy_settings', label: 'Proxy & Cache', icon: Shield },
		{ id: 'network_access', label: 'Network Access', icon: Globe },
		{ id: 'user_limit_settings', label: 'User Limits', icon: Users },
		{ id: 'channel_db_settings', label: 'Channel DB', icon: Database },
	];

	let formData: Record<string, any> = $state({});
	let timeZones: string[] = $state([]);

	onMount(async () => {
		try {
			if (typeof Intl !== 'undefined' && Intl.supportedValuesOf) {
				timeZones = Intl.supportedValuesOf('timeZone');
			} else {
				timeZones = ['UTC', 'America/New_York', 'America/Chicago', 'America/Denver', 'America/Los_Angeles', 'Europe/London', 'Asia/Tokyo', 'Australia/Sydney'];
			}
		} catch (e) {
			timeZones = ['UTC', 'America/New_York'];
		}

		try {
			const res = await api.getSettings();
			settingsList = res;
			res.forEach(s => {
				const val = JSON.parse(JSON.stringify(s.value));
				if (s.key === 'system_settings' && !val.time_zone) {
					val.time_zone = 'America/New_York';
				}
				formData[s.key] = val;
			});
		} catch (e) {
			console.error(e);
		}
	});

	async function saveCategory(key: string) {
		saving = true;
		try {
			const original = settingsList.find(s => s.key === key);
			if (original) {
				await api.updateSetting(original.id, {
					key: original.key,
					name: original.name,
					value: formData[key]
				});
				if (key === 'ui_settings') {
					await loadUiSettings();
				}
				savedMessage = 'Saved ' + tabs.find(t => t.id === key)?.label;
				setTimeout(() => savedMessage = '', 3000);
			}
		} catch (e) {
			console.error(e);
			toast.error('Failed to save settings');
		} finally {
			saving = false;
		}
	}
</script>

<div class="page-container">
	<header class="page-header">
		<div>
			<h1>Global Settings</h1>
			<p class="subtitle">Configure application behavior, engine timeouts, and interface preferences</p>
		</div>
		{#if savedMessage}
			<div class="toast-success">
				<CheckCircle2 size={16} />
				<span>{savedMessage}</span>
			</div>
		{/if}
	</header>

	<div class="settings-layout">
		<!-- Sidebar Navigation -->
		<aside class="settings-nav">
			{#each tabs as tab}
				<button 
					class="nav-item {activeTab === tab.id ? 'active' : ''}" 
					onclick={() => activeTab = tab.id}
				>
					<tab.icon size={18} />
					<span>{tab.label}</span>
				</button>
			{/each}
		</aside>

		<!-- Content Area -->
		<main class="settings-content">
			{#if Object.keys(formData).length === 0}
				<div class="loading-state">Loading settings...</div>
			{:else}
				<div class="form-card">
					<!-- UI SETTINGS -->
					{#if activeTab === 'ui_settings' && formData['ui_settings']}
						<div class="form-header">
							<h2>UI Settings</h2>
							<p>Customize the visual interface and localization preferences.</p>
						</div>
						<div class="form-grid">
							<div class="form-group">
								<label>Time Format</label>
								<select bind:value={formData['ui_settings'].time_format}>
									<option value="12h">12-Hour (AM/PM)</option>
									<option value="24h">24-Hour (Military)</option>
								</select>
							</div>
							<div class="form-group">
								<label>Date Format</label>
								<select bind:value={formData['ui_settings'].date_format}>
									<option value="mdy">MM/DD/YYYY</option>
									<option value="dmy">DD/MM/YYYY</option>
									<option value="ymd">YYYY/MM/DD</option>
								</select>
							</div>
							<div class="form-group">
								<label>Table Size</label>
								<select bind:value={formData['ui_settings'].table_size}>
									<option value="compact">Compact</option>
									<option value="default">Default</option>
									<option value="comfortable">Comfortable</option>
								</select>
							</div>
						</div>

					<!-- SYSTEM SETTINGS -->
					{:else if activeTab === 'system_settings' && formData['system_settings']}
						<div class="form-header">
							<h2>System Settings</h2>
							<p>Core application behavior and logging configurations.</p>
						</div>
						<div class="form-grid">
							<div class="form-group">
								<label>System Time Zone</label>
								<select bind:value={formData['system_settings'].time_zone}>
									{#each timeZones as tz}
										<option value={tz}>{tz}</option>
									{/each}
								</select>
								<span class="help-text">Standard IANA timezone string for server-side scheduling.</span>
							</div>
							<div class="form-group">
								<label>Max System Events</label>
								<input type="number" bind:value={formData['system_settings'].max_system_events} min="10" max="1000" />
								<span class="help-text">Number of events to keep in the system log before pruning.</span>
							</div>
						</div>

					<!-- STREAM ENGINE -->
					{:else if activeTab === 'stream_settings' && formData['stream_settings']}
						<div class="form-header">
							<h2>Stream Engine</h2>
							<p>Defaults for the multiplexing engine and connection retries.</p>
						</div>
						<div class="form-grid">
							<div class="form-group">
								<label>Internal Buffer Size (KB)</label>
								<input type="number" bind:value={formData['stream_settings'].buffer_size} />
								<span class="help-text">Size of the chunks piped from FFmpeg/Direct stream.</span>
							</div>
							<div class="form-group">
								<label>Retry Count</label>
								<input type="number" bind:value={formData['stream_settings'].retry_count} />
								<span class="help-text">Number of connection attempts before failing the stream.</span>
							</div>
							<div class="form-group">
								<label>Stream Checker Concurrency</label>
								<input type="number" bind:value={formData['stream_settings'].stream_checker_parallel_providers} min="1" max="10" />
								<span class="help-text">Number of concurrent streams to test during background maintenance.</span>
							</div>
							<div class="form-group full-width">
								<label>Default User Agent</label>
								<input type="text" bind:value={formData['stream_settings'].default_user_agent} />
								<span class="help-text">The default HTTP User-Agent sent to IPTV providers if not specifically overridden on the playlist.</span>
							</div>
						</div>

					<!-- DVR & COMSKIP -->
					{:else if activeTab === 'dvr_settings' && formData['dvr_settings']}
						<div class="form-header">
							<h2>DVR & Comskip</h2>
							<p>Recording templates, padding, and commercial skipping behavior.</p>
						</div>
						<div class="form-grid">
							<div class="form-group switch-group">
								<label>
									<span>Enable Comskip</span>
									<input type="checkbox" bind:checked={formData['dvr_settings'].comskip_enabled} />
									<div class="switch"></div>
								</label>
								<span class="help-text">Automatically process recordings to flag or remove commercials.</span>
							</div>
							<div class="form-group">
								<label>Comskip Custom Path</label>
								<input type="text" bind:value={formData['dvr_settings'].comskip_custom_path} placeholder="/usr/bin/comskip" disabled={!formData['dvr_settings'].comskip_enabled} />
							</div>
							<div class="form-group">
								<label>Pre-Padding (Minutes)</label>
								<input type="number" bind:value={formData['dvr_settings'].pre_offset_minutes} min="0" />
								<span class="help-text">Start recordings early.</span>
							</div>
							<div class="form-group">
								<label>Post-Padding (Minutes)</label>
								<input type="number" bind:value={formData['dvr_settings'].post_offset_minutes} min="0" />
								<span class="help-text">End recordings late.</span>
							</div>
							<div class="form-group full-width">
								<label>TV Show Template</label>
								<input type="text" bind:value={formData['dvr_settings'].tv_template} />
								<span class="help-text">Available tokens: {'{show}, {season}, {episode}, {title}, {start}'}</span>
							</div>
							<div class="form-group full-width">
								<label>TV Show Fallback Template</label>
								<input type="text" bind:value={formData['dvr_settings'].tv_fallback_template} />
								<span class="help-text">Used when season/episode info is missing. Available tokens: {'{show}, {start}'}</span>
							</div>
							<div class="form-group full-width">
								<label>Movie Template</label>
								<input type="text" bind:value={formData['dvr_settings'].movie_template} />
								<span class="help-text">Available tokens: {'{title}, {year}, {start}'}</span>
							</div>
							<div class="form-group full-width">
								<label>Movie Fallback Template</label>
								<input type="text" bind:value={formData['dvr_settings'].movie_fallback_template} />
								<span class="help-text">Used when year info is missing. Available tokens: {'{title}, {start}'}</span>
							</div>
						</div>

					<!-- PROXY SETTINGS -->
					{:else if activeTab === 'proxy_settings' && formData['proxy_settings']}
						<div class="form-header">
							<h2>Proxy & Cache Engine</h2>
							<p>Advanced tuning for the backend streaming multiplexer and HTTP client.</p>
						</div>
						<div class="form-grid">
							<div class="form-group">
								<label>Buffering Timeout (s)</label>
								<input type="number" bind:value={formData['proxy_settings'].buffering_timeout} min="1" />
								<span class="help-text">Time to wait for first byte before failing over.</span>
							</div>
							<div class="form-group">
								<label>Buffering Speed</label>
								<input type="number" step="0.1" bind:value={formData['proxy_settings'].buffering_speed} min="0.1" />
								<span class="help-text">Multiplier for internal read speed.</span>
							</div>
							<div class="form-group">
								<label>Chunk TTL (s)</label>
								<input type="number" bind:value={formData['proxy_settings'].redis_chunk_ttl} min="1" />
								<span class="help-text">Time to keep stream chunks in memory.</span>
							</div>
							<div class="form-group">
								<label>Channel Shutdown Delay (s)</label>
								<input type="number" bind:value={formData['proxy_settings'].channel_shutdown_delay} min="0" />
								<span class="help-text">Keep the provider connection alive after the last client disconnects (fast switching).</span>
							</div>
							<div class="form-group">
								<label>Init Grace Period (s)</label>
								<input type="number" bind:value={formData['proxy_settings'].channel_init_grace_period} min="1" />
							</div>
							<div class="form-group">
								<label>New Client Offset (s)</label>
								<input type="number" bind:value={formData['proxy_settings'].new_client_behind_seconds} min="0" />
								<span class="help-text">How far behind live to start new clients (improves stability).</span>
							</div>
							<div class="form-group switch-group full-width mt">
								<label>
									<span>Route Through HTTP Proxy</span>
									<input type="checkbox" bind:checked={formData['proxy_settings'].http_proxy_enabled} />
									<div class="switch"></div>
								</label>
							</div>
							<div class="form-group full-width">
								<label>HTTP Proxy URL</label>
								<input type="text" bind:value={formData['proxy_settings'].http_proxy_url} placeholder="http://proxy:8080" disabled={!formData['proxy_settings'].http_proxy_enabled} />
							</div>
						</div>

					<!-- NETWORK ACCESS -->
					{:else if activeTab === 'network_access' && formData['network_access']}
						<div class="form-header">
							<h2>Network Access (CIDR)</h2>
							<p>IP whitelisting for different endpoints of the application.</p>
						</div>
						<div class="form-grid">
							<div class="form-group full-width">
								<label>Admin UI Interface</label>
								<input type="text" bind:value={formData['network_access'].UI} />
							</div>
							<div class="form-group full-width">
								<label>M3U & EPG Retrieval</label>
								<input type="text" bind:value={formData['network_access'].M3U_EPG} />
								<span class="help-text">Default permits local network playback.</span>
							</div>
							<div class="form-group full-width">
								<label>Direct Stream Playback</label>
								<input type="text" bind:value={formData['network_access'].STREAMS} />
							</div>
							<div class="form-group full-width">
								<label>Xtream API</label>
								<input type="text" bind:value={formData['network_access'].XC_API} />
							</div>
						</div>

					<!-- USER LIMITS -->
					{:else if activeTab === 'user_limit_settings' && formData['user_limit_settings']}
						<div class="form-header">
							<h2>User Limits & Concurrency</h2>
							<p>Control what happens when clients exceed their allotted stream count.</p>
						</div>
						<div class="form-grid">
							<div class="form-group">
								<label>Default Max Streams</label>
								<input type="number" bind:value={formData['user_limit_settings'].max_streams} min="1" />
							</div>
							<div class="form-group switch-group mt">
								<label>
									<span>Terminate on Limit Exceeded</span>
									<input type="checkbox" bind:checked={formData['user_limit_settings'].terminate_on_limit_exceeded} />
									<div class="switch"></div>
								</label>
							</div>
							<div class="form-group switch-group mt">
								<label>
									<span>Prioritize Single Client Channels</span>
									<input type="checkbox" bind:checked={formData['user_limit_settings'].prioritize_single_client_channels} />
									<div class="switch"></div>
								</label>
							</div>
							<div class="form-group switch-group mt">
								<label>
									<span>Terminate Oldest Connection</span>
									<input type="checkbox" bind:checked={formData['user_limit_settings'].terminate_oldest} disabled={!formData['user_limit_settings'].terminate_on_limit_exceeded} />
									<div class="switch"></div>
								</label>
							</div>
							<div class="form-group switch-group mt">
								<label>
									<span>Ignore Same Channel</span>
									<input type="checkbox" bind:checked={formData['user_limit_settings'].ignore_same_channel_connections} />
									<div class="switch"></div>
								</label>
								<span class="help-text">Multiple connections to the identical stream count as 1.</span>
							</div>
						</div>

					<!-- CHANNEL DB -->
					{:else if activeTab === 'channel_db_settings' && formData['channel_db_settings']}
						<div class="form-header">
							<h2>Channel Database</h2>
							<p>Automated retrieval of the community ChannelIdentifier mappings.</p>
						</div>
						<div class="form-grid">
							<div class="form-group switch-group full-width">
								<label>
									<span>Auto-Check for Updates</span>
									<input type="checkbox" bind:checked={formData['channel_db_settings'].auto_check} />
									<div class="switch"></div>
								</label>
								<span class="help-text">Automatically download new channel mappings when available.</span>
							</div>
							<div class="form-group full-width mt">
								<label>Download URL</label>
								<input type="text" bind:value={formData['channel_db_settings'].download_url} />
							</div>
						</div>
					{/if}

					<div class="form-actions">
						<button class="btn-primary" onclick={() => saveCategory(activeTab)} disabled={saving}>
							<Save size={16} />
							<span>{saving ? 'Saving...' : 'Save Changes'}</span>
						</button>
					</div>
				</div>
			{/if}
		</main>
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

	.toast-success {
		display: flex;
		align-items: center;
		gap: 8px;
		background: rgba(74, 222, 128, 0.1);
		color: #4ade80;
		border: 1px solid #4ade80;
		padding: 8px 16px;
		border-radius: var(--radius);
		font-size: 13px;
		font-weight: 600;
		animation: fadein 0.2s ease-out;
	}

	@keyframes fadein {
		from { opacity: 0; transform: translateY(-10px); }
		to { opacity: 1; transform: translateY(0); }
	}

	.settings-layout {
		display: flex;
		gap: 24px;
		flex: 1;
		min-height: 0;
	}

	.settings-nav {
		width: 240px;
		display: flex;
		flex-direction: column;
		gap: 4px;
		overflow-y: auto;
		padding-right: 8px;
		flex-shrink: 0;

		.nav-item {
			display: flex;
			align-items: center;
			gap: 12px;
			background: transparent;
			border: none;
			color: var(--text-dim);
			padding: 12px 16px;
			border-radius: var(--radius);
			font-size: 14px;
			font-weight: 500;
			cursor: pointer;
			text-align: left;
			transition: all 0.2s;

			&:hover {
				background: rgba(255, 255, 255, 0.05);
				color: var(--text-bright);
			}

			&.active {
				background: var(--surface);
				border: 1px solid var(--border);
				color: var(--accent);
			}
		}
	}

	.settings-content {
		flex: 1;
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		overflow-y: auto;
	}

	.loading-state {
		padding: 40px;
		text-align: center;
		color: var(--text-dim);
		font-style: italic;
	}

	.form-card {
		padding: 32px;
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.form-header {
		margin-bottom: 32px;
		padding-bottom: 16px;
		border-bottom: 1px solid var(--border);

		h2 {
			margin: 0 0 8px 0;
			font-size: 20px;
			color: var(--text-bright);
		}

		p {
			margin: 0;
			color: var(--text-dim);
			font-size: 14px;
		}
	}

	.form-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 24px;
		flex: 1;
		align-content: start;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 6px;

		&.full-width {
			grid-column: 1 / -1;
		}

		&.mt {
			margin-top: 8px;
		}

		label {
			font-size: 13px;
			font-weight: 600;
			color: var(--text-bright);
		}

		input[type="text"], input[type="number"], select {
			background: rgba(0,0,0,0.2);
			border: 1px solid var(--border);
			color: var(--text-bright);
			padding: 10px 12px;
			border-radius: 4px;
			font-size: 14px;
			outline: none;
			transition: border-color 0.2s;

			&:focus { border-color: var(--accent); }
			&:disabled { opacity: 0.5; cursor: not-allowed; }
		}

		.help-text {
			font-size: 12px;
			color: var(--text-dim);
			margin-top: 2px;
		}
	}

	.switch-group {
		label {
			display: flex;
			justify-content: space-between;
			align-items: center;
			cursor: pointer;
			padding: 8px 0;
			font-size: 14px;
		}

		input[type="checkbox"] {
			display: none;
		}

		.switch {
			width: 40px;
			height: 22px;
			background: var(--surface-bright);
			border-radius: 11px;
			position: relative;
			transition: background 0.2s;

			&::after {
				content: '';
				position: absolute;
				top: 2px;
				left: 2px;
				width: 18px;
				height: 18px;
				background: var(--text-dim);
				border-radius: 50%;
				transition: all 0.2s;
			}
		}

		input[type="checkbox"]:checked + .switch {
			background: var(--accent);
			&::after {
				left: 20px;
				background: white;
			}
		}

		input[type="checkbox"]:disabled + .switch {
			opacity: 0.5;
			cursor: not-allowed;
		}
	}

	.form-actions {
		margin-top: auto;
		padding-top: 32px;
		border-top: 1px solid var(--border);
		display: flex;
		justify-content: flex-end;
	}

	.btn-primary {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--accent);
		color: white;
		border: none;
		padding: 10px 24px;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 14px;
		cursor: pointer;
		transition: all 0.2s;

		&:hover:not(:disabled) {
			background: var(--accent-dim);
		}

		&:disabled {
			opacity: 0.7;
			cursor: not-allowed;
		}
	}
</style>
