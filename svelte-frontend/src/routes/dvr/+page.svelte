<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { Calendar, CheckCircle2, Clock3, FileText, HardDrive, Settings, Upload } from 'lucide-svelte';

	let loading = $state(true);
	let error = $state('');
	let dvrSettings: any = $state({});
	let comskipConfig: any = $state({ path: '', exists: false });

	onMount(async () => {
		try {
			const [settings, comskip] = await Promise.all([api.getSettings(), api.getComskipConfig()]);
			const dvr = settings.find((setting: any) => setting.key === 'dvr_settings');
			dvrSettings = dvr?.value || {};
			comskipConfig = comskip || { path: '', exists: false };
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load DVR settings';
		} finally {
			loading = false;
		}
	});

	const templateRows = $derived([
		{ label: 'TV episodes', value: dvrSettings.tv_template },
		{ label: 'TV fallback', value: dvrSettings.tv_fallback_template },
		{ label: 'Movies', value: dvrSettings.movie_template },
		{ label: 'Movie fallback', value: dvrSettings.movie_fallback_template }
	]);
</script>

<div class="page-container">
	<header class="page-header">
		<div>
			<h1>DVR Management</h1>
			<p class="subtitle">Review recording defaults, comskip status, and output naming rules.</p>
		</div>
		<a class="btn-primary" href="/settings">
			<Settings size={16} />
			<span>DVR Settings</span>
		</a>
	</header>

	<main class="content-area">
		{#if loading}
			<div class="loading-state">Loading DVR configuration...</div>
		{:else if error}
			<div class="empty-state">
				<FileText size={42} />
				<p>{error}</p>
			</div>
		{:else}
			<section class="summary-grid">
				<div class="status-card">
					<div class="card-icon">
						<HardDrive size={22} />
					</div>
					<div>
						<span class="label">Recording Padding</span>
						<strong>{dvrSettings.pre_offset_minutes || 0}m before / {dvrSettings.post_offset_minutes || 0}m after</strong>
					</div>
				</div>

				<div class="status-card">
					<div class="card-icon">
						<CheckCircle2 size={22} />
					</div>
					<div>
						<span class="label">Comskip</span>
						<strong>{dvrSettings.comskip_enabled ? 'Enabled' : 'Disabled'}</strong>
					</div>
				</div>

				<div class="status-card">
					<div class="card-icon">
						<Upload size={22} />
					</div>
					<div>
						<span class="label">Comskip Config</span>
						<strong>{comskipConfig.exists ? comskipConfig.path : 'Not uploaded'}</strong>
					</div>
				</div>
			</section>

			<section class="panel">
				<div class="panel-header">
					<div>
						<h2>File Naming Templates</h2>
						<p>Templates used when recordings are written to disk.</p>
					</div>
					<Calendar size={20} />
				</div>

				<div class="template-list">
					{#each templateRows as row}
						<div class="template-row">
							<span>{row.label}</span>
							<code>{row.value || 'Not configured'}</code>
						</div>
					{/each}
				</div>
			</section>

			<section class="panel compact">
				<div class="panel-header">
					<div>
						<h2>Scheduler Status</h2>
						<p>Recording creation is not exposed yet; current controls are configuration-only.</p>
					</div>
					<Clock3 size={20} />
				</div>
			</section>
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
		padding: 10px 18px;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 14px;
		text-decoration: none;
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
	}

	.summary-grid {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 16px;
	}

	.status-card,
	.panel {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
	}

	.status-card {
		padding: 18px;
		display: flex;
		align-items: center;
		gap: 14px;

		.label {
			display: block;
			color: var(--text-dim);
			font-size: 12px;
			margin-bottom: 4px;
		}

		strong {
			color: var(--text-bright);
			font-size: 15px;
			overflow-wrap: anywhere;
		}
	}

	.card-icon {
		width: 42px;
		height: 42px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius);
		background: var(--accent-transparent);
		color: var(--accent);
		flex: 0 0 auto;
	}

	.panel {
		padding: 20px;

		&.compact {
			padding-bottom: 18px;
		}
	}

	.panel-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		color: var(--text-dim);
		margin-bottom: 16px;

		h2 {
			margin: 0 0 4px;
			color: var(--text-bright);
			font-size: 17px;
		}

		p {
			margin: 0;
			font-size: 13px;
		}
	}

	.template-list {
		display: flex;
		flex-direction: column;
		border-top: 1px solid var(--border);
	}

	.template-row {
		display: grid;
		grid-template-columns: 160px minmax(0, 1fr);
		gap: 16px;
		padding: 14px 0;
		border-bottom: 1px solid var(--border);
		align-items: center;

		span {
			color: var(--text-dim);
			font-size: 13px;
		}

		code {
			color: var(--text-bright);
			background: rgba(0, 0, 0, 0.25);
			border: 1px solid var(--border);
			border-radius: var(--radius);
			padding: 8px 10px;
			overflow-wrap: anywhere;
		}
	}

	@media (max-width: 900px) {
		.page-header {
			align-items: flex-start;
			flex-direction: column;
			gap: 14px;
		}

		.summary-grid {
			grid-template-columns: 1fr;
		}

		.template-row {
			grid-template-columns: 1fr;
			gap: 8px;
		}
	}
</style>
