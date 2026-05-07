<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { Puzzle, DownloadCloud, FileCode2, PackageOpen } from 'lucide-svelte';

	let plugins: any[] = $state([]);
	let loading = $state(true);

	onMount(async () => {
		try {
			const res = await api.getPlugins();
			plugins = res.results || [];
		} catch (e) {
			console.error('Failed to load plugins', e);
		} finally {
			loading = false;
		}
	});
</script>

<div class="page-container">
	<header class="page-header">
		<div>
			<h1>Plugins</h1>
			<p class="subtitle">Extend the Dispatcharr engine with custom parsers, EPG scrapers, and metadata agents.</p>
		</div>
		<button class="btn-primary" disabled>
			<DownloadCloud size={16} />
			<span>Install Plugin</span>
		</button>
	</header>

	<main class="content-area">
		{#if loading}
			<div class="loading-state">Loading plugins...</div>
		{:else if plugins.length === 0}
			<div class="empty-state">
				<div class="icon-circle primary large">
					<Puzzle size={48} />
				</div>
				<h2>No Plugins Installed</h2>
				<p>The plugin engine is currently initializing. In the future, this is where you will manage community-built Lua and WebAssembly scripts to parse custom IPTV provider layouts or fetch localized EPG data.</p>
				
				<div class="feature-preview">
					<h3>Upcoming Capabilities</h3>
					<div class="grid">
						<div class="preview-card">
							<FileCode2 size={20} class="text-accent" />
							<h4>Custom Parsers</h4>
							<span>Inject scripts to normalize non-standard M3U tags or complex VOD hierarchies before they hit the database.</span>
						</div>
						<div class="preview-card">
							<PackageOpen size={20} class="text-accent" />
							<h4>Metadata Agents</h4>
							<span>Pull rich poster art and descriptions from TMDB/IMDB for your VOD libraries dynamically.</span>
						</div>
					</div>
				</div>
			</div>
		{:else}
			<div class="grid-list">
				{#each plugins as plugin}
					<div class="plugin-card">
						{plugin.name}
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
		max-width: 650px;
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
			background: var(--accent-transparent);
			color: var(--accent);
			border-color: rgba(237, 28, 36, 0.2);
		}

		&.large {
			width: 96px;
			height: 96px;
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

	:global(.text-accent) {
		color: var(--accent) !important;
	}
</style>
