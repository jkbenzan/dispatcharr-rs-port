<script lang="ts">
	import { onMount } from 'svelte';
	import { Plus, RefreshCw, Edit2, Trash2, Link, Server, Clock, AlertCircle, FileText } from 'lucide-svelte';
	import { api } from '$lib/api';
	import M3UProviderModal from '$lib/components/m3u/M3UProviderModal.svelte';
	import EpgProviderModal from '$lib/components/epg/EpgProviderModal.svelte';

	let activeTab = $state('m3u'); // 'm3u' or 'epg'

	// M3U State
	let m3uProviders: any[] = $state([]);
	let m3uLoading = $state(true);
	let m3uError = $state('');
	let showM3uModal = $state(false);
	let selectedM3uProvider: any = $state(null);

	// EPG State
	let epgSources: any[] = $state([]);
	let epgLoading = $state(true);
	let epgError = $state('');
	let showEpgModal = $state(false);
	let selectedEpgSource: any = $state(null);

	onMount(() => {
		loadM3uProviders();
		loadEpgSources();
	});

	// --- M3U Logic ---
	async function loadM3uProviders() {
		m3uLoading = true;
		m3uError = '';
		try {
			m3uProviders = await api.getPlaylists();
		} catch (err: any) {
			m3uError = err.message || 'Failed to load M3U providers';
			console.error(err);
		} finally {
			m3uLoading = false;
		}
	}

	function handleAddM3u() {
		selectedM3uProvider = null;
		showM3uModal = true;
	}

	function handleEditM3u(provider: any) {
		selectedM3uProvider = provider;
		showM3uModal = true;
	}

	async function handleSaveM3u(payload: any, id?: number) {
		if (id) {
			await api.updateM3UAccount(id, payload);
		} else {
			await api.addM3UAccount(payload);
		}
		await loadM3uProviders();
	}

	async function handleDeleteM3u(id: number) {
		if (confirm('Are you sure you want to delete this M3U provider?')) {
			try {
				await api.deleteM3UAccount(id);
				await loadM3uProviders();
			} catch (err: any) {
				alert(err.message || 'Failed to delete provider');
			}
		}
	}

	async function handleRefreshM3u(id: number) {
		try {
			await api.refreshM3UAccount(id);
			alert('Refresh triggered successfully. Monitor activity log for progress.');
		} catch (err: any) {
			alert(err.message || 'Failed to refresh provider');
		}
	}

	// --- EPG Logic ---
	async function loadEpgSources() {
		epgLoading = true;
		epgError = '';
		try {
			epgSources = await api.getEpgSources();
		} catch (err: any) {
			epgError = err.message || 'Failed to load EPG sources';
			console.error(err);
		} finally {
			epgLoading = false;
		}
	}

	function handleAddEpg() {
		selectedEpgSource = null;
		showEpgModal = true;
	}

	function handleEditEpg(source: any) {
		selectedEpgSource = source;
		showEpgModal = true;
	}

	async function handleSaveEpg(payload: any, id?: number) {
		if (id) {
			await api.updateEpgSource(id, payload);
		} else {
			await api.createEpgSource(payload);
		}
		await loadEpgSources();
	}

	async function handleDeleteEpg(id: number) {
		if (confirm('Are you sure you want to delete this EPG source?')) {
			try {
				await api.deleteEpgSource(id);
				await loadEpgSources();
			} catch (err: any) {
				alert(err.message || 'Failed to delete EPG source');
			}
		}
	}

	async function handleRefreshEpg(id: number) {
		try {
			await api.refreshEpgSource(id);
			alert('EPG Refresh triggered successfully.');
		} catch (err: any) {
			alert(err.message || 'Failed to refresh EPG source');
		}
	}

	// --- Shared ---

	function handleAdd() {
		selectedProvider = null;
		showModal = true;
	}

	function handleEdit(provider: any) {
		selectedProvider = provider;
		showModal = true;
	}

	async function handleSave(payload: any, id?: number) {
		if (id) {
			await api.updateM3UAccount(id, payload);
		} else {
			await api.addM3UAccount(payload);
		}
		await loadProviders();
	}

	async function handleDelete(id: number) {
		if (confirm('Are you sure you want to delete this provider? This action cannot be undone.')) {
			try {
				await api.deleteM3UAccount(id);
				await loadProviders();
			} catch (err: any) {
				alert(err.message || 'Failed to delete provider');
			}
		}
	}

	async function handleRefresh(id: number) {
		try {
			await api.refreshM3UAccount(id);
			// Show temporary success or just let WS events update status
			alert('Refresh triggered successfully. Monitor activity log for progress.');
		} catch (err: any) {
			alert(err.message || 'Failed to refresh provider');
		}
	}

	function formatDate(isoStr?: string) {
		if (!isoStr) return 'Never';
		return new Date(isoStr).toLocaleString(undefined, {
			month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'
		});
	}
</script>

<div class="page-container">
	<header class="page-header">
		<div>
			<h1>Stream Providers</h1>
			<p class="subtitle">Manage M3U, XTREAM Codes, and EPG sources</p>
		</div>
		<div class="header-actions">
			<div class="tabs">
				<button class="tab" class:active={activeTab === 'm3u'} onclick={() => activeTab = 'm3u'}>
					<Server size={16} /> M3U / XTREAM
				</button>
				<button class="tab" class:active={activeTab === 'epg'} onclick={() => activeTab = 'epg'}>
					<FileText size={16} /> EPG Sources
				</button>
			</div>
			
			{#if activeTab === 'm3u'}
				<button class="btn-primary" onclick={handleAddM3u}>
					<Plus size={18} />
					<span>Add Playlist</span>
				</button>
			{:else}
				<button class="btn-primary" onclick={handleAddEpg}>
					<Plus size={18} />
					<span>Add EPG Source</span>
				</button>
			{/if}
		</div>
	</header>

	<div class="tab-content">
		{#if activeTab === 'm3u'}
			<!-- M3U VIEW -->
			{#if m3uError}
				<div class="error-banner"><AlertCircle size={20} /><span>{m3uError}</span></div>
			{/if}

			{#if m3uLoading}
				<div class="loading-state"><RefreshCw size={24} class="spin" /><p>Loading providers...</p></div>
			{:else if m3uProviders.length === 0}
				<div class="empty-state">
					<Server size={48} />
					<h3>No playlists found</h3>
					<p>Get started by adding your first M3U or XTREAM Codes playlist.</p>
					<button class="btn-primary" onclick={handleAddM3u}>Add Playlist</button>
				</div>
			{:else}
				<div class="providers-grid">
					{#each m3uProviders as provider}
						<div class="provider-card">
							<div class="card-header">
								<div class="title-row">
									<div class="type-badge" class:xc={provider.account_type === 'xc'}>
										{provider.account_type === 'xc' ? 'XTREAM' : 'M3U'}
									</div>
									<h3>{provider.name}</h3>
								</div>
								<div class="actions">
									<button class="icon-btn" title="Refresh Streams" onclick={() => handleRefreshM3u(provider.id)}><RefreshCw size={16} /></button>
									<button class="icon-btn" title="Edit Provider" onclick={() => handleEditM3u(provider)}><Edit2 size={16} /></button>
									<button class="icon-btn danger" title="Delete Provider" onclick={() => handleDeleteM3u(provider.id)}><Trash2 size={16} /></button>
								</div>
							</div>
							
							<div class="card-body">
								<div class="info-row">
									<Link size={14} class="icon-dim" />
									<span class="truncate" title={provider.server_url || provider.file_path}>
										{provider.server_url || provider.file_path || 'No URL specified'}
									</span>
								</div>
								
								<div class="stats-grid">
									<div class="stat-item"><span class="label">Conns</span><span class="value">{provider.max_streams}</span></div>
									<div class="stat-item"><span class="label">Refresh</span><span class="value">{provider.refresh_interval > 0 ? `${provider.refresh_interval}h` : 'Manual'}</span></div>
									<div class="stat-item"><span class="label">Status</span><span class="value status-badge" class:active={provider.is_active} class:error={provider.status === 'error'}>{provider.status || (provider.is_active ? 'Active' : 'Inactive')}</span></div>
								</div>
							</div>

							<div class="card-footer">
								<div class="last-updated"><Clock size={12} /><span>Updated: {formatDate(provider.updated_at)}</span></div>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		{:else}
			<!-- EPG VIEW -->
			{#if epgError}
				<div class="error-banner"><AlertCircle size={20} /><span>{epgError}</span></div>
			{/if}

			{#if epgLoading}
				<div class="loading-state"><RefreshCw size={24} class="spin" /><p>Loading EPG sources...</p></div>
			{:else if epgSources.length === 0}
				<div class="empty-state">
					<FileText size={48} />
					<h3>No EPG sources found</h3>
					<p>Add an XMLTV URL or HDHomeRun IP to provide guide data to your channels.</p>
					<button class="btn-primary" onclick={handleAddEpg}>Add EPG Source</button>
				</div>
			{:else}
				<div class="providers-grid">
					{#each epgSources as source}
						<div class="provider-card">
							<div class="card-header">
								<div class="title-row">
									<div class="type-badge xc">{source.source_type.toUpperCase()}</div>
									<h3>{source.name}</h3>
								</div>
								<div class="actions">
									<button class="icon-btn" title="Refresh EPG" onclick={() => handleRefreshEpg(source.id)}><RefreshCw size={16} /></button>
									<button class="icon-btn" title="Edit Source" onclick={() => handleEditEpg(source)}><Edit2 size={16} /></button>
									<button class="icon-btn danger" title="Delete Source" onclick={() => handleDeleteEpg(source.id)}><Trash2 size={16} /></button>
								</div>
							</div>
							
							<div class="card-body">
								<div class="info-row">
									<Link size={14} class="icon-dim" />
									<span class="truncate" title={source.url || source.file_path}>
										{source.url || source.file_path || 'No URL specified'}
									</span>
								</div>
								
								<div class="stats-grid">
									<div class="stat-item"><span class="label">Refresh</span><span class="value">{source.refresh_interval > 0 ? `${source.refresh_interval}h` : 'Manual'}</span></div>
									<div class="stat-item"><span class="label">Status</span><span class="value status-badge" class:active={source.is_active} class:error={source.status === 'error'}>{source.status || (source.is_active ? 'Active' : 'Inactive')}</span></div>
								</div>
							</div>

							<div class="card-footer">
								<div class="last-updated"><Clock size={12} /><span>Updated: {formatDate(source.updated_at)}</span></div>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		{/if}
	</div>
</div>

<M3UProviderModal bind:show={showM3uModal} provider={selectedM3uProvider} onSave={handleSaveM3u} />
<EpgProviderModal bind:show={showEpgModal} provider={selectedEpgSource} onSave={handleSaveEpg} />

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
		align-items: center;

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

	.header-actions {
		display: flex;
		align-items: center;
		gap: 16px;
	}

	.tabs {
		display: flex;
		background: rgba(0,0,0,0.2);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 4px;

		.tab {
			background: transparent;
			border: none;
			color: var(--text-dim);
			padding: 8px 16px;
			border-radius: 4px;
			font-size: 13px;
			font-weight: 600;
			cursor: pointer;
			transition: all 0.2s;
			display: flex;
			align-items: center;
			gap: 8px;

			&:hover:not(.active) {
				color: var(--text-bright);
			}

			&.active {
				background: var(--surface-bright);
				color: var(--text-bright);
				box-shadow: 0 1px 3px rgba(0,0,0,0.2);
			}
		}
	}

	.btn-primary {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--accent);
		color: white;
		border: none;
		padding: 10px 20px;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 14px;
		cursor: pointer;
		transition: all 0.2s;

		&:hover {
			background: var(--accent-dim);
			transform: translateY(-1px);
		}
	}

	.error-banner {
		background: rgba(237, 28, 36, 0.1);
		border: 1px solid var(--accent);
		color: var(--accent);
		padding: 16px;
		border-radius: var(--radius);
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.loading-state, .empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		flex: 1;
		color: var(--text-dim);
		gap: 16px;
		text-align: center;
		
		h3 {
			font-size: 18px;
			color: var(--text-bright);
			margin: 0;
		}
		
		p {
			max-width: 400px;
			line-height: 1.5;
			margin: 0 0 16px 0;
		}
	}

	.spin {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		100% { transform: rotate(360deg); }
	}

	.providers-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
		gap: 20px;
	}

	.provider-card {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		display: flex;
		flex-direction: column;
		overflow: hidden;
		transition: all 0.2s;

		&:hover {
			border-color: var(--border-bright);
			transform: translateY(-2px);
			box-shadow: 0 4px 12px rgba(0,0,0,0.2);
		}
	}

	.card-header {
		padding: 16px;
		border-bottom: 1px solid var(--border);
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		background: var(--surface-bright);

		.title-row {
			display: flex;
			align-items: center;
			gap: 12px;

			h3 {
				margin: 0;
				font-size: 16px;
				font-weight: 600;
				color: var(--text-bright);
			}
		}

		.actions {
			display: flex;
			gap: 4px;
		}
	}

	.type-badge {
		font-size: 10px;
		font-weight: 800;
		padding: 4px 8px;
		border-radius: 4px;
		background: rgba(255, 255, 255, 0.1);
		color: var(--text-bright);
		letter-spacing: 0.5px;

		&.xc {
			background: var(--accent-transparent);
			color: var(--accent);
		}
	}

	.icon-btn {
		background: transparent;
		border: none;
		color: var(--text-dim);
		cursor: pointer;
		padding: 6px;
		border-radius: 4px;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.2s;

		&:hover {
			background: rgba(255, 255, 255, 0.1);
			color: var(--text-bright);
		}

		&.danger:hover {
			background: rgba(237, 28, 36, 0.1);
			color: var(--accent);
		}
	}

	.card-body {
		padding: 16px;
		display: flex;
		flex-direction: column;
		gap: 16px;
		flex: 1;
	}

	.info-row {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 13px;
		color: var(--text-dim);

		.truncate {
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
		}
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 12px;
		background: rgba(0,0,0,0.2);
		padding: 12px;
		border-radius: var(--radius);
	}

	.stat-item {
		display: flex;
		flex-direction: column;
		gap: 4px;

		.label {
			font-size: 11px;
			text-transform: uppercase;
			color: var(--text-dim);
			font-weight: 600;
			letter-spacing: 0.5px;
		}

		.value {
			font-size: 14px;
			font-weight: 500;
			color: var(--text-bright);
		}
	}

	.status-badge {
		font-size: 12px !important;
		font-weight: 600 !important;
		text-transform: capitalize;

		&.active { color: #4ade80 !important; }
		&.error { color: var(--accent) !important; }
	}

	.card-footer {
		padding: 12px 16px;
		border-top: 1px solid var(--border);
		background: rgba(0,0,0,0.1);

		.last-updated {
			display: flex;
			align-items: center;
			gap: 6px;
			font-size: 11px;
			color: var(--text-dim);
		}
	}
</style>
