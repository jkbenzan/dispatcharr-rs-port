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
	let sourceType = $state('xmltv');
	let url = $state('');
	let refreshInterval = $state(24);

	// Effect to populate form when provider changes or modal opens
	$effect(() => {
		if (show) {
			if (provider) {
				name = provider.name || '';
				sourceType = provider.source_type || 'xmltv';
				url = provider.url || provider.file_path || '';
				refreshInterval = provider.refresh_interval || 24;
			} else {
				// Reset form
				name = '';
				sourceType = 'xmltv';
				url = '';
				refreshInterval = 24;
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
				source_type: sourceType,
				url,
				refresh_interval: refreshInterval,
				is_active: true
			};

			await onSave(payload, provider?.id);
			show = false;
		} catch (err: any) {
			error = err.message || 'Failed to save EPG provider';
		} finally {
			loading = false;
		}
	}
</script>

<Modal bind:show title={provider ? 'Edit EPG Source' : 'Add EPG Source'} width="500px">
	<form onsubmit={handleSubmit} class="provider-form">
		{#if error}
			<div class="error-banner">
				<AlertCircle size={18} />
				<span>{error}</span>
			</div>
		{/if}

		<div class="form-group">
			<label for="name">Source Name</label>
			<input type="text" id="name" bind:value={name} placeholder="e.g. NextPVR Guide" required />
		</div>

		<div class="form-group">
			<label for="sourceType">Source Type</label>
			<select id="sourceType" bind:value={sourceType}>
				<option value="xmltv">XMLTV / Standard</option>
				<option value="hdhr">HDHomeRun</option>
			</select>
		</div>

		<div class="form-group">
			<label for="url">EPG URL or Path</label>
			<input type="text" id="url" bind:value={url} placeholder="http://example.com/guide.xml" required />
		</div>

		<div class="form-group">
			<label for="refreshInterval">Auto-Refresh Interval (Hours)</label>
			<input type="number" id="refreshInterval" bind:value={refreshInterval} min="0" placeholder="0 = Disabled" />
		</div>

		<div class="form-actions">
			<button type="button" class="btn-cancel" onclick={() => show = false} disabled={loading}>
				Cancel
			</button>
			<button type="submit" class="btn-submit" disabled={loading || !name}>
				<Save size={18} />
				<span>{loading ? 'Saving...' : 'Save Source'}</span>
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

		}

		input[type="number"] {
			font-variant-numeric: tabular-nums;
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
