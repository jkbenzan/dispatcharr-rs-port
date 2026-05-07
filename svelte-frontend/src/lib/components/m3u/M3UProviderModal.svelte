<script lang="ts">
	import Modal from '$lib/components/ui/Modal.svelte';
	import { Save, AlertCircle } from 'lucide-svelte';

	let {
		show = $bindable(false),
		provider = null,
		onSave = () => {}
	} = $props();

	let loading = $state(false);
	let error = $state('');

	// Form state
	let name = $state('');
	let accountType = $state('m3u');
	let m3uUrl = $state('');
	let serverUrl = $state('');
	let username = $state('');
	let password = $state('');
	let maxStreams = $state(1);
	let refreshInterval = $state(24);
	let staleStreamDays = $state(7);

	// Effect to populate form when provider changes or modal opens
	$effect(() => {
		if (show) {
			if (provider) {
				name = provider.name || '';
				accountType = provider.account_type || 'm3u';
				m3uUrl = provider.server_url || '';
				serverUrl = provider.server_url || '';
				username = provider.username || '';
				password = provider.password || '';
				maxStreams = provider.max_streams || 1;
				refreshInterval = provider.refresh_interval || 24;
				staleStreamDays = provider.stale_stream_days || 7;
			} else {
				// Reset form
				name = '';
				accountType = 'm3u';
				m3uUrl = '';
				serverUrl = '';
				username = '';
				password = '';
				maxStreams = 1;
				refreshInterval = 24;
				staleStreamDays = 7;
			}
			error = '';
		}
	});

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		loading = true;

		try {
			const payload = {
				name,
				account_type: accountType,
				server_url: accountType === 'xc' ? serverUrl : m3uUrl,
				username: accountType === 'xc' ? username : null,
				password: accountType === 'xc' ? password : null,
				max_streams: maxStreams,
				refresh_interval: refreshInterval,
				stale_stream_days: staleStreamDays,
				is_active: true
			};

			await onSave(payload, provider?.id);
			show = false;
		} catch (err: any) {
			error = err.message || 'Failed to save provider';
		} finally {
			loading = false;
		}
	}
</script>

<Modal bind:show title={provider ? 'Edit Provider' : 'Add Provider'} width="600px">
	<form onsubmit={handleSubmit} class="provider-form">
		{#if error}
			<div class="error-banner">
				<AlertCircle size={18} />
				<span>{error}</span>
			</div>
		{/if}

		<div class="form-group">
			<label for="name">Provider Name</label>
			<input type="text" id="name" bind:value={name} placeholder="e.g. My Premium IPTV" required />
		</div>

		<div class="form-row">
			<div class="form-group">
				<label for="accountType">Account Type</label>
				<select id="accountType" bind:value={accountType}>
					<option value="m3u">Standard M3U Playlist</option>
					<option value="xc">XTREAM Codes</option>
				</select>
			</div>
			
			<div class="form-group">
				<label for="maxStreams">Max Connections</label>
				<input type="number" id="maxStreams" bind:value={maxStreams} min="1" required />
			</div>
		</div>

		{#if accountType === 'm3u'}
			<div class="form-group">
				<label for="m3uUrl">Playlist URL</label>
				<input type="url" id="m3uUrl" bind:value={m3uUrl} placeholder="http://example.com/playlist.m3u" required />
			</div>
		{:else}
			<div class="form-group">
				<label for="serverUrl">Server URL</label>
				<input type="url" id="serverUrl" bind:value={serverUrl} placeholder="http://example.com:8080" required />
			</div>
			<div class="form-row">
				<div class="form-group">
					<label for="username">Username</label>
					<input type="text" id="username" bind:value={username} required />
				</div>
				<div class="form-group">
					<label for="password">Password</label>
					<input type="password" id="password" bind:value={password} required />
				</div>
			</div>
		{/if}

		<div class="form-row">
			<div class="form-group">
				<label for="refreshInterval">Auto-Refresh Interval (Hours)</label>
				<input type="number" id="refreshInterval" bind:value={refreshInterval} min="0" placeholder="0 = Disabled" />
			</div>
			<div class="form-group">
				<label for="staleStreamDays">Stale Stream Cleanup (Days)</label>
				<input type="number" id="staleStreamDays" bind:value={staleStreamDays} min="1" />
			</div>
		</div>

		<div class="form-actions">
			<button type="button" class="btn-cancel" onclick={() => show = false} disabled={loading}>
				Cancel
			</button>
			<button type="submit" class="btn-submit" disabled={loading || !name}>
				<Save size={18} />
				<span>{loading ? 'Saving...' : 'Save Provider'}</span>
			</button>
		</div>
	</form>
</Modal>

<style lang="less">
	.provider-form {
		padding: 24px;
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.error-banner {
		background: rgba(237, 28, 36, 0.1);
		border: 1px solid var(--accent);
		color: var(--accent);
		padding: 12px 16px;
		border-radius: var(--radius);
		display: flex;
		align-items: center;
		gap: 12px;
		font-size: 14px;
	}

	.form-row {
		display: flex;
		gap: 16px;
		
		.form-group {
			flex: 1;
		}
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 8px;

		label {
			font-size: 13px;
			font-weight: 500;
			color: var(--text-dim);
			text-transform: uppercase;
			letter-spacing: 0.5px;
		}

		input, select {
			background: rgba(0, 0, 0, 0.2);
			border: 1px solid var(--border);
			color: var(--text-bright);
			padding: 10px 12px;
			border-radius: var(--radius);
			font-family: inherit;
			font-size: 14px;
			transition: all 0.2s;

			&:focus {
				outline: none;
				border-color: var(--accent);
				background: rgba(0, 0, 0, 0.4);
			}

			&[type="number"] {
				font-variant-numeric: tabular-nums;
			}
		}

		select {
			appearance: none;
			background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%23a0a0a0' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
			background-repeat: no-repeat;
			background-position: right 12px center;
			padding-right: 40px;
			
			option {
				background: var(--surface-bright);
				color: var(--text-bright);
			}
		}
	}

	.form-actions {
		display: flex;
		justify-content: flex-end;
		gap: 12px;
		margin-top: 8px;
		padding-top: 24px;
		border-top: 1px solid var(--border);

		button {
			display: flex;
			align-items: center;
			gap: 8px;
			padding: 10px 20px;
			border-radius: var(--radius);
			font-size: 14px;
			font-weight: 500;
			cursor: pointer;
			transition: all 0.2s;

			&:disabled {
				opacity: 0.5;
				cursor: not-allowed;
			}
		}

		.btn-cancel {
			background: transparent;
			border: 1px solid var(--border);
			color: var(--text-dim);

			&:hover:not(:disabled) {
				background: rgba(255, 255, 255, 0.05);
				color: var(--text-bright);
			}
		}

		.btn-submit {
			background: var(--accent);
			border: 1px solid var(--accent);
			color: white;

			&:hover:not(:disabled) {
				background: var(--accent-dim);
				border-color: var(--accent-dim);
			}
		}
	}
</style>
