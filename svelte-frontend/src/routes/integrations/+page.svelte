<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { Link, Plus, Webhook, MessageSquare, Key } from 'lucide-svelte';

	let integrations: any[] = $state([]);
	let loading = $state(true);

	onMount(async () => {
		try {
			const res = await api.getIntegrations();
			// Handle the stubbed backend which returns { count: 0, results: [] }
			integrations = res.results || [];
		} catch (e) {
			console.error('Failed to load integrations', e);
		} finally {
			loading = false;
		}
	});
</script>

<div class="page-container">
	<header class="page-header">
		<div>
			<h1>Integrations</h1>
			<p class="subtitle">Connect external services, configure webhooks, and manage API keys.</p>
		</div>
		<button class="btn-primary" disabled>
			<Plus size={16} />
			<span>Add Integration</span>
		</button>
	</header>

	<main class="content-area">
		{#if loading}
			<div class="loading-state">Loading integrations...</div>
		{:else if integrations.length === 0}
			<div class="empty-state">
				<div class="icon-group">
					<div class="icon-circle"><Webhook size={24} /></div>
					<div class="icon-circle primary"><Link size={32} /></div>
					<div class="icon-circle"><MessageSquare size={24} /></div>
				</div>
				<h2>No Integrations Configured</h2>
				<p>Connect Discord, Slack, or configure custom webhooks to receive real-time notifications about system events, channel mapping updates, and proxy failures.</p>
				
				<div class="feature-preview">
					<h3>Coming Soon</h3>
					<div class="grid">
						<div class="preview-card">
							<MessageSquare size={20} class="text-accent" />
							<h4>Chat Notifications</h4>
							<span>Send M3U mapping reports straight to Discord/Slack.</span>
						</div>
						<div class="preview-card">
							<Key size={20} class="text-accent" />
							<h4>API Tokens</h4>
							<span>Generate scoped access tokens for third-party apps.</span>
						</div>
					</div>
				</div>
			</div>
		{:else}
			<!-- Render actual integrations when backend is ready -->
			<div class="grid-list">
				{#each integrations as integration}
					<div class="integration-card">
						{integration.name}
					</div>
				{/each}
			</div>
		{/if}
	</main>
</div>

<style lang="less">
	.page-container {
		display: flex;
		flex-direction: column;
		gap: 24px;
		height: 100%;
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-end;

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
			opacity: 0.5;
			cursor: not-allowed;
		}
	}

	.content-area {
		flex: 1;
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.loading-state {
		margin: auto;
		color: var(--text-dim);
		font-style: italic;
	}

	.empty-state {
		margin: auto;
		max-width: 600px;
		padding: 40px;
		text-align: center;
		display: flex;
		flex-direction: column;
		align-items: center;

		h2 {
			margin: 24px 0 12px;
			color: var(--text-bright);
			font-size: 24px;
		}

		p {
			color: var(--text-dim);
			line-height: 1.6;
			margin-bottom: 40px;
		}
	}

	.icon-group {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 16px;

		.icon-circle {
			width: 64px;
			height: 64px;
			border-radius: 50%;
			background: rgba(255, 255, 255, 0.05);
			display: flex;
			align-items: center;
			justify-content: center;
			color: var(--text-dim);
			border: 1px solid var(--border);

			&.primary {
				width: 80px;
				height: 80px;
				background: var(--accent-transparent);
				color: var(--accent);
				border-color: rgba(237, 28, 36, 0.2);
			}
		}
	}

	.feature-preview {
		width: 100%;
		border-top: 1px solid var(--border);
		padding-top: 32px;

		h3 {
			font-size: 14px;
			text-transform: uppercase;
			letter-spacing: 1px;
			color: var(--text-dim);
			margin-bottom: 24px;
		}

		.grid {
			display: grid;
			grid-template-columns: 1fr 1fr;
			gap: 16px;
		}

		.preview-card {
			background: rgba(0, 0, 0, 0.2);
			border: 1px solid var(--border);
			border-radius: var(--radius);
			padding: 20px;
			text-align: left;
			display: flex;
			flex-direction: column;
			gap: 8px;

			h4 {
				margin: 0;
				color: var(--text-bright);
			}

			span {
				font-size: 13px;
				color: var(--text-dim);
				line-height: 1.5;
			}
		}
	}

	/* Utility to force accent color */
	:global(.text-accent) {
		color: var(--accent) !important;
	}
</style>
