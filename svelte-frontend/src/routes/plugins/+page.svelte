<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { AlertCircle, DownloadCloud, PackageOpen, Plug, Puzzle } from 'lucide-svelte';

	let plugins: any[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	onMount(async () => {
		try {
			const res = await api.getPlugins();
			plugins = res.results || [];
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load plugins';
		} finally {
			loading = false;
		}
	});
</script>

<div class="page-container">
	<header class="page-header">
		<div>
			<h1>Plugins</h1>
			<p class="subtitle">Inspect installed extension packages and parser integrations.</p>
		</div>
		<button class="btn-primary" disabled title="Plugin installation is not exposed in the UI yet">
			<DownloadCloud size={16} />
			<span>Install Plugin</span>
		</button>
	</header>

	<main class="content-area">
		{#if loading}
			<div class="loading-state">Loading plugins...</div>
		{:else if error}
			<div class="empty-state">
				<AlertCircle size={42} />
				<p>{error}</p>
			</div>
		{:else if plugins.length === 0}
			<div class="empty-state">
				<div class="icon-circle">
					<Puzzle size={42} />
				</div>
				<h2>No Plugins Installed</h2>
				<p>The backend returned an empty plugin registry. Installed plugins will appear here when the registry endpoint reports them.</p>
			</div>
		{:else}
			<div class="plugin-list">
				{#each plugins as plugin}
					<article class="plugin-card">
						<div class="plugin-icon">
							<Plug size={22} />
						</div>
						<div class="plugin-body">
							<h2>{plugin.name || plugin.key || 'Unnamed plugin'}</h2>
							<p>{plugin.description || plugin.summary || 'No description provided.'}</p>
						</div>
						<span class="status-pill">{plugin.enabled === false ? 'Disabled' : 'Enabled'}</span>
					</article>
				{/each}
			</div>
		{/if}

		<section class="registry-note">
			<PackageOpen size={18} />
			<span>Plugin data is sourced from `/api/plugins/plugins/`.</span>
		</section>
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
		padding: 10px 18px;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 14px;
		opacity: 0.55;
		cursor: not-allowed;
	}

	.content-area {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 16px;
		overflow: auto;
	}

	.loading-state,
	.empty-state {
		margin: auto;
		color: var(--text-dim);
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
		max-width: 520px;
		text-align: center;
	}

	.empty-state {
		h2 {
			color: var(--text-bright);
			margin: 8px 0 0;
			font-size: 20px;
		}

		p {
			margin: 0;
			line-height: 1.5;
		}
	}

	.icon-circle,
	.plugin-icon {
		width: 56px;
		height: 56px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius);
		background: var(--accent-transparent);
		color: var(--accent);
	}

	.plugin-list {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.plugin-card,
	.registry-note {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
	}

	.plugin-card {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		gap: 16px;
		align-items: center;
		padding: 16px;
	}

	.plugin-icon {
		width: 44px;
		height: 44px;
	}

	.plugin-body {
		min-width: 0;

		h2 {
			margin: 0 0 4px;
			color: var(--text-bright);
			font-size: 16px;
		}

		p {
			margin: 0;
			color: var(--text-dim);
			font-size: 13px;
			overflow-wrap: anywhere;
		}
	}

	.status-pill {
		border: 1px solid var(--border);
		border-radius: 999px;
		padding: 5px 10px;
		color: var(--text-bright);
		font-size: 12px;
		background: rgba(255, 255, 255, 0.04);
	}

	.registry-note {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 12px 14px;
		color: var(--text-dim);
		font-size: 13px;
	}

	@media (max-width: 760px) {
		.page-header {
			align-items: flex-start;
			flex-direction: column;
			gap: 14px;
		}

		.plugin-card {
			grid-template-columns: auto minmax(0, 1fr);
		}

		.status-pill {
			grid-column: 2;
			justify-self: start;
		}
	}
</style>
