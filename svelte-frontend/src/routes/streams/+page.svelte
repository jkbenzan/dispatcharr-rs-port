<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { Plus, RefreshCw, Edit2, Trash2, Link, Server, Clock, AlertCircle, FileText } from 'lucide-svelte';
	import { api } from '$lib/api';
	import { toast } from '$lib/toast.svelte';
	import { connectWS, wsStore } from '$lib/ws.svelte';
	import M3UProviderModal from '$lib/components/m3u/M3UProviderModal.svelte';
	import EpgProviderModal from '$lib/components/epg/EpgProviderModal.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import ProviderRefreshProgress from '$lib/components/ui/ProviderRefreshProgress.svelte';

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
	let providerPollTimer: ReturnType<typeof setInterval> | null = null;
	let elapsedTimer: ReturnType<typeof setInterval> | null = null;
	let nowMs = $state(Date.now());
	let providerProgress = $state<Record<string, {
		startedAt: number;
		progress: number | null;
		status: string;
		message: string;
		kind: 'm3u' | 'epg';
	}>>({});

	type ConfirmVariant = 'default' | 'danger';
	type ConfirmationState = {
		title: string;
		message: string;
		confirmLabel: string;
		variant: ConfirmVariant;
		onConfirm: () => Promise<void>;
	};

	let showConfirmDialog = $state(false);
	let confirmBusy = $state(false);
	let confirmation: ConfirmationState | null = $state(null);

	function requestConfirmation(config: ConfirmationState) {
		confirmation = config;
		showConfirmDialog = true;
	}

	function cancelConfirmation() {
		if (confirmBusy) return;
		showConfirmDialog = false;
		confirmation = null;
	}

	async function runConfirmedAction() {
		if (!confirmation || confirmBusy) return;

		// Copy the callback before awaiting so closing the dialog cannot clear the action mid-flight.
		const action = confirmation.onConfirm;
		confirmBusy = true;
		try {
			await action();
			showConfirmDialog = false;
			confirmation = null;
		} finally {
			confirmBusy = false;
		}
	}

	function normalizeAccountType(value?: string) {
		return (value || '').toLowerCase();
	}

	function isXcProvider(provider: any) {
		const accountType = normalizeAccountType(provider?.account_type);
		return accountType === 'xc' || accountType === 'xtream';
	}

	const queuedStatuses = new Set(['fetching', 'refreshing']);
	const DEFAULT_REFRESH_ESTIMATE_MS = 120_000;

	function isCustomM3uProvider(provider: any) {
		return provider?.name?.toLowerCase() === 'custom';
	}

	function sortByName<T extends { name?: string }>(items: T[]) {
		return [...items].sort((a, b) => (a.name || '').localeCompare(b.name || '', undefined, { sensitivity: 'base' }));
	}

	let visibleM3uProviders = $derived(
		sortByName(m3uProviders.filter((provider) => !isCustomM3uProvider(provider)))
	);

	let visibleEpgSources = $derived(sortByName(epgSources));

	let m3uRefreshSummary = $derived(() => {
		const active = visibleM3uProviders.filter((provider) => provider.is_active && !provider.locked);
		const refreshing = active.filter(isProviderRefreshing).length;
		const queued = active.filter((provider) => String(provider.last_message || '').toLowerCase().includes('queued')).length;
		return { active: active.length, refreshing, queued };
	});

	let epgRefreshSummary = $derived(() => {
		const active = visibleEpgSources.filter((source) => source.is_active);
		const refreshing = active.filter((source) => queuedStatuses.has(String(source.status || '').toLowerCase())).length;
		const queued = active.filter((source) => String(source.last_message || '').toLowerCase().includes('queued')).length;
		return { active: active.length, refreshing, queued };
	});

	function hasRefreshActivity() {
		return m3uRefreshSummary().refreshing > 0 || m3uRefreshSummary().queued > 0 || epgRefreshSummary().refreshing > 0 || epgRefreshSummary().queued > 0;
	}

	async function refreshProviderState({ silent = true } = {}) {
		await Promise.all([
			loadM3uProviders({ silent }),
			loadEpgSources({ silent })
		]);
	}

	function startProviderPolling() {
		if (providerPollTimer) return;
		providerPollTimer = setInterval(() => {
			refreshProviderState({ silent: true }).catch((err) => {
				console.error('Failed to refresh provider status:', err);
			});
		}, 5000);
	}

	function stopProviderPolling() {
		if (!providerPollTimer) return;
		clearInterval(providerPollTimer);
		providerPollTimer = null;
	}

	function startElapsedTimer() {
		if (elapsedTimer) return;
		elapsedTimer = setInterval(() => {
			nowMs = Date.now();
		}, 1000);
	}

	function stopElapsedTimer() {
		if (!elapsedTimer) return;
		clearInterval(elapsedTimer);
		elapsedTimer = null;
	}

	function progressKey(kind: 'm3u' | 'epg', id: number | string) {
		return `${kind}:${id}`;
	}

	function isActiveRefreshStatus(status?: string) {
		const normalized = String(status || '').toLowerCase();
		return normalized === 'queued' || normalized === 'fetching' || normalized === 'refreshing' || normalized === 'parsing';
	}

	function rememberProgress(kind: 'm3u' | 'epg', id: number | string, patch: Partial<Record<string, any>>) {
		const key = progressKey(kind, id);
		const existing = providerProgress[key];
		providerProgress = {
			...providerProgress,
			[key]: {
				startedAt: existing?.startedAt || Date.now(),
				progress: typeof patch.progress === 'number' ? patch.progress : (existing?.progress ?? null),
				status: String(patch.status || existing?.status || 'queued'),
				message: String(patch.message || existing?.message || 'Refresh queued'),
				kind
			}
		};
	}

	function rememberQueuedProgress(kind: 'm3u' | 'epg', id: number | string, message = 'Refresh queued') {
		rememberProgress(kind, id, {
			status: 'queued',
			message,
			progress: 4
		});
	}

	function forgetProgress(kind: 'm3u' | 'epg', id: number | string) {
		const key = progressKey(kind, id);
		if (!providerProgress[key]) return;
		const next = { ...providerProgress };
		delete next[key];
		providerProgress = next;
	}

	function syncProgressFromProvider(kind: 'm3u' | 'epg', item: any) {
		const status = kind === 'm3u' ? getProviderStatus(item) : String(item.status || '').toLowerCase();
		const queued = String(item.last_message || '').toLowerCase().includes('queued');
		if (queued || isActiveRefreshStatus(status)) {
			rememberProgress(kind, item.id, {
				status: queued ? 'queued' : status,
				message: item.last_message || (queued ? 'Refresh queued' : 'Refresh in progress')
			});
			return;
		}

		if (['success', 'healthy', 'active', 'idle', 'inactive', 'disabled', 'failed', 'error', 'pending_setup'].includes(status)) {
			forgetProgress(kind, item.id);
		}
	}

	function handleRefreshMessage(message: any) {
		if (!message || typeof message !== 'object') return;

		if (message.type === 'm3u_refresh' && message.account != null) {
			const status = String(message.status || '').toLowerCase();
			const progress = Number(message.progress);
			if (['success', 'failed', 'error', 'pending_setup'].includes(status) || progress >= 100) {
				forgetProgress('m3u', message.account);
				return;
			}
			rememberProgress('m3u', message.account, {
				status: message.status,
				message: message.message,
				progress: Number.isFinite(progress) ? progress : undefined
			});
		}

		if (message.type === 'epg_refresh' && message.source != null) {
			if (['success', 'failed', 'error'].includes(String(message.status || '').toLowerCase())) {
				forgetProgress('epg', message.source);
				return;
			}
			rememberProgress('epg', message.source, {
				status: message.status,
				message: message.message,
				progress: estimatedProgressForStatus(message.status)
			});
		}
	}

	function estimatedProgressForStatus(status?: string) {
		const normalized = String(status || '').toLowerCase();
		if (normalized === 'queued') return 4;
		if (normalized === 'fetching') return 18;
		if (normalized === 'parsing') return 62;
		if (normalized === 'refreshing') return null;
		return null;
	}

	function getProgressInfo(kind: 'm3u' | 'epg', item: any) {
		const key = progressKey(kind, item.id);
		const tracked = providerProgress[key];
		const status = kind === 'm3u' ? getProviderStatus(item) : String(item.status || '').toLowerCase();
		const queued = String(item.last_message || '').toLowerCase().includes('queued');
		const active = queued || isActiveRefreshStatus(status) || !!tracked;
		if (!active) return null;

		const startedAt = tracked?.startedAt || Date.now();
		const elapsedMs = Math.max(0, nowMs - startedAt);
		const statusProgress = estimatedProgressForStatus(queued ? 'queued' : status);
		const elapsedEstimate = Math.min(95, Math.max(8, Math.round((elapsedMs / DEFAULT_REFRESH_ESTIMATE_MS) * 100)));
		const progress = Math.min(100, Math.max(0, tracked?.progress ?? statusProgress ?? elapsedEstimate));
		const etaMs = progress > 5 && progress < 100 ? Math.max(0, Math.round((elapsedMs / progress) * (100 - progress))) : null;

		return {
			progress,
			elapsedMs,
			etaMs,
			status: tracked?.status || (queued ? 'queued' : status),
			message: tracked?.message || item.last_message || 'Refresh in progress'
		};
	}

	function formatDuration(ms: number | null) {
		if (ms == null) return '--';
		const totalSeconds = Math.max(0, Math.round(ms / 1000));
		const minutes = Math.floor(totalSeconds / 60);
		const seconds = totalSeconds % 60;
		if (minutes >= 60) {
			const hours = Math.floor(minutes / 60);
			const remainder = minutes % 60;
			return `${hours}h ${remainder}m`;
		}
		return `${minutes}:${seconds.toString().padStart(2, '0')}`;
	}

	$effect(() => {
		if (hasRefreshActivity()) {
			startProviderPolling();
		} else {
			stopProviderPolling();
		}
	});

	$effect(() => {
		if (Object.keys(providerProgress).length > 0) {
			startElapsedTimer();
		} else {
			stopElapsedTimer();
		}
	});

	$effect(() => {
		// Provider progress and system-event websocket messages are a lightweight signal to refresh card state.
		handleRefreshMessage(wsStore.lastMessage);
		if (wsStore.lastMessage?.event === 'progress' || wsStore.lastMessage?.type === 'system_event' || wsStore.lastMessage?.type === 'm3u_refresh' || wsStore.lastMessage?.type === 'epg_refresh') {
			refreshProviderState({ silent: true }).catch((err) => {
				console.error('Failed to refresh provider state after websocket event:', err);
			});
		}
	});

	onMount(() => {
		connectWS();
		loadM3uProviders();
		loadEpgSources();
	});

	onDestroy(() => {
		stopProviderPolling();
		stopElapsedTimer();
	});

	// --- M3U Logic ---
	async function loadM3uProviders({ silent = false } = {}) {
		if (!silent) {
			m3uLoading = true;
			m3uError = '';
		}
		try {
			m3uProviders = await api.getPlaylists();
			for (const provider of m3uProviders) {
				syncProgressFromProvider('m3u', provider);
			}
		} catch (err: any) {
			if (!silent) {
				m3uError = err.message || 'Failed to load M3U providers';
			}
			console.error(err);
		} finally {
			if (!silent) {
				m3uLoading = false;
			}
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
		let result;
		if (id) {
			result = await api.updateM3UAccount(id, payload);
		} else {
			result = await api.addM3UAccount(payload);
		}
		await loadM3uProviders();
		return result;
	}

	async function handleDeleteM3u(id: number) {
		requestConfirmation({
			title: 'Delete M3U Provider',
			message: 'Delete this M3U provider? This removes the provider record and cannot be undone.',
			confirmLabel: 'Delete',
			variant: 'danger',
			onConfirm: async () => {
				try {
					await api.deleteM3UAccount(id);
					await loadM3uProviders();
				} catch (err: any) {
					toast.error(err.message || 'Failed to delete provider');
				}
			}
		});
	}

	async function handleRefreshM3u(id: number) {
		try {
			rememberQueuedProgress('m3u', id);
			await api.refreshM3UAccount(id);
			await loadM3uProviders({ silent: true });
			startProviderPolling();
			toast.success('Provider refresh queued. The card status will update as work progresses.');
		} catch (err: any) {
			toast.error(err.message || 'Failed to refresh provider');
		}
	}

	async function handleRefreshAllM3u() {
		requestConfirmation({
			title: 'Refresh All M3U Providers',
			message: 'Queue refreshes for all active M3U providers? The backend provider queue controls concurrency and will run them in order.',
			confirmLabel: 'Queue Refreshes',
			variant: 'default',
			onConfirm: async () => {
				try {
					const result = await api.refreshAllM3uAccounts();
					const queuedIds = Array.isArray(result?.queued_account_ids) ? new Set(result.queued_account_ids.map(Number)) : null;
					for (const provider of visibleM3uProviders.filter((provider) => canRefreshM3u(provider) && (!queuedIds || queuedIds.has(Number(provider.id))))) {
						rememberQueuedProgress('m3u', provider.id);
					}
					await loadM3uProviders({ silent: true });
					startProviderPolling();
					toast.info('M3U provider refreshes queued.');
				} catch (err: any) {
					toast.error(err.message || 'Failed to start bulk refresh');
				}
			}
		});
	}

	function getProviderStatus(provider: any) {
		if (String(provider?.last_message || '').toLowerCase().includes('queued')) return 'queued';
		return provider.normalized_status || provider.status || (provider.is_active ? 'active' : 'inactive');
	}

	function formatProviderStatus(provider: any) {
		const status = getProviderStatus(provider);
		const labels: Record<string, string> = {
			healthy: 'Healthy',
			refreshing: 'Refreshing',
			queued: 'Queued',
			failed: 'Failed',
			pending_setup: 'Pending Setup',
			pending: 'Pending',
			active: 'Active',
			inactive: 'Inactive',
			success: 'Healthy',
			fetching: 'Refreshing',
			error: 'Failed'
		};
		return labels[status] || status;
	}

	function isProviderFailed(provider: any) {
		return provider.is_failed || getProviderStatus(provider) === 'failed' || provider.status === 'error' || provider.status === 'failed';
	}

	function isProviderRefreshing(provider: any) {
		return getProviderStatus(provider) === 'refreshing' || provider.status === 'fetching';
	}

	function isProviderQueued(provider: any) {
		return getProviderStatus(provider) === 'queued';
	}

	function canRefreshM3u(provider: any) {
		return provider.is_active && !provider.locked && !isCustomM3uProvider(provider);
	}

	// --- EPG Logic ---
	async function loadEpgSources({ silent = false } = {}) {
		if (!silent) {
			epgLoading = true;
			epgError = '';
		}
		try {
			epgSources = await api.getEpgSources();
			for (const source of epgSources) {
				syncProgressFromProvider('epg', source);
			}
		} catch (err: any) {
			if (!silent) {
				epgError = err.message || 'Failed to load EPG sources';
			}
			console.error(err);
		} finally {
			if (!silent) {
				epgLoading = false;
			}
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
		requestConfirmation({
			title: 'Delete EPG Source',
			message: 'Delete this EPG source? This removes the source record and cannot be undone.',
			confirmLabel: 'Delete',
			variant: 'danger',
			onConfirm: async () => {
				try {
					await api.deleteEpgSource(id);
					await loadEpgSources();
				} catch (err: any) {
					toast.error(err.message || 'Failed to delete EPG source');
				}
			}
		});
	}

	async function handleRefreshEpg(id: number) {
		try {
			rememberQueuedProgress('epg', id);
			await api.refreshEpgSource(id);
			await loadEpgSources({ silent: true });
			startProviderPolling();
			toast.success('EPG refresh queued. The card status will update as work progresses.');
		} catch (err: any) {
			toast.error(err.message || 'Failed to refresh EPG source');
		}
	}

	async function handleRefreshAllEpg() {
		requestConfirmation({
			title: 'Refresh All EPG Sources',
			message: 'Queue refreshes for all active EPG sources? The backend provider queue controls concurrency and will run them in order.',
			confirmLabel: 'Queue Refreshes',
			variant: 'default',
			onConfirm: async () => {
				try {
					await api.refreshAllEpgSources();
					for (const source of visibleEpgSources.filter((source) => source.is_active)) {
						rememberQueuedProgress('epg', source.id);
					}
					await loadEpgSources({ silent: true });
					startProviderPolling();
					toast.info('EPG source refreshes queued.');
				} catch (err: any) {
					toast.error(err.message || 'Failed to start bulk refresh');
				}
			}
		});
	}

	import { formatDateTime } from '$lib/settings.svelte';

	function formatDate(isoStr?: string) {
		if (!isoStr) return 'Never';
		return formatDateTime(isoStr);
	}

	function formatRefreshSummary(summary: { active: number; refreshing: number; queued: number }, label: string) {
		if (summary.refreshing === 0 && summary.queued === 0) return `${label}: idle`;
		return `${label}: ${summary.refreshing} refreshing, ${summary.queued} queued`;
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
			
			<div class="header-btns">
				{#if activeTab === 'm3u'}
					<button class="btn-secondary" onclick={handleRefreshAllM3u}>
						<RefreshCw size={18} />
						<span>Refresh All</span>
					</button>
					<button class="btn-primary" onclick={handleAddM3u}>
						<Plus size={18} />
						<span>Add Playlist</span>
					</button>
				{:else}
					<button class="btn-secondary" onclick={handleRefreshAllEpg}>
						<RefreshCw size={18} />
						<span>Refresh All</span>
					</button>
					<button class="btn-primary" onclick={handleAddEpg}>
						<Plus size={18} />
						<span>Add EPG Source</span>
					</button>
				{/if}
			</div>
		</div>
	</header>

	<div class="tab-content">
		{#if hasRefreshActivity()}
			<div class="refresh-monitor">
				<RefreshCw size={18} class="spin" />
				<div>
					<strong>Provider refresh queue active</strong>
					<span>{formatRefreshSummary(m3uRefreshSummary(), 'M3U')} / {formatRefreshSummary(epgRefreshSummary(), 'EPG')}</span>
				</div>
			</div>
		{/if}

		{#if activeTab === 'm3u'}
			<!-- M3U VIEW -->
			{#if m3uError}
				<div class="error-banner"><AlertCircle size={20} /><span>{m3uError}</span></div>
			{/if}

			{#if m3uLoading}
				<div class="loading-state"><RefreshCw size={24} class="spin" /><p>Loading providers...</p></div>
			{:else if visibleM3uProviders.length === 0}
				<div class="empty-state">
					<Server size={48} />
					<h3>No playlists found</h3>
					<p>Get started by adding your first M3U or XTREAM Codes playlist.</p>
					<button class="btn-primary" onclick={handleAddM3u}>Add Playlist</button>
				</div>
			{:else}
				<div class="providers-grid">
					{#each visibleM3uProviders as provider (provider.id)}
						<div class="provider-card">
							<div class="card-header">
								<div class="title-row">
									<div class="type-badge" class:xc={isXcProvider(provider)}>
										{isXcProvider(provider) ? 'XTREAM' : 'M3U'}
									</div>
									<h3>{provider.name}</h3>
								</div>
								<div class="actions">
									<button
										class="icon-btn"
										title={canRefreshM3u(provider) ? 'Refresh Streams' : 'Refresh unavailable'}
										onclick={() => handleRefreshM3u(provider.id)}
										disabled={!canRefreshM3u(provider)}
									><RefreshCw size={16} /></button>
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
									<div class="stat-item"><span class="label">Streams</span><span class="value">{provider.stream_count ?? 0}</span></div>
									<div class="stat-item"><span class="label">Refresh</span><span class="value">{provider.refresh_interval > 0 ? `${provider.refresh_interval}h` : 'Manual'}</span></div>
									<div class="stat-item">
										<span class="label">Status</span>
										<span
											class="value status-badge"
											class:active={getProviderStatus(provider) === 'healthy' || getProviderStatus(provider) === 'active'}
											class:refreshing={isProviderRefreshing(provider)}
											class:queued={isProviderQueued(provider)}
											class:error={isProviderFailed(provider)}
										>{formatProviderStatus(provider)}</span>
									</div>
								</div>

								{#if provider.last_message}
									<div class="status-message" class:error={isProviderFailed(provider)} title={provider.last_message}>
										{provider.last_message}
									</div>
								{/if}
							</div>

							<div class="card-footer">
								<ProviderRefreshProgress info={getProgressInfo('m3u', provider)} {formatDuration} />
								<div class="last-updated"><Clock size={12} /><span>Last refresh: {formatDate(provider.last_refresh_at || provider.updated_at)}</span></div>
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
			{:else if visibleEpgSources.length === 0}
				<div class="empty-state">
					<FileText size={48} />
					<h3>No EPG sources found</h3>
					<p>Add an XMLTV URL or HDHomeRun IP to provide guide data to your channels.</p>
					<button class="btn-primary" onclick={handleAddEpg}>Add EPG Source</button>
				</div>
			{:else}
				<div class="providers-grid">
					{#each visibleEpgSources as source (source.id)}
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
								<ProviderRefreshProgress info={getProgressInfo('epg', source)} {formatDuration} />
								<div class="last-updated"><Clock size={12} /><span>Updated: {formatDate(source.updated_at)}</span></div>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		{/if}
	</div>
</div>

<M3UProviderModal
	bind:show={showM3uModal}
	provider={selectedM3uProvider}
	onSave={handleSaveM3u}
	onRefreshQueued={() => {
		loadM3uProviders({ silent: true });
		startProviderPolling();
	}}
/>
<EpgProviderModal bind:show={showEpgModal} provider={selectedEpgSource} onSave={handleSaveEpg} />
{#if confirmation}
	<ConfirmDialog
		bind:show={showConfirmDialog}
		title={confirmation.title}
		message={confirmation.message}
		confirmLabel={confirmation.confirmLabel}
		variant={confirmation.variant}
		busy={confirmBusy}
		onConfirm={runConfirmedAction}
		onCancel={cancelConfirmation}
	/>
{/if}

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

	.header-btns {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.btn-primary, .btn-secondary {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 40px;
		padding: 0 16px;
		border-radius: var(--radius);
		font-size: 14px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
		
		span {
			@media (max-width: 768px) {
				display: none;
			}
		}
	}

	.btn-primary {
		background: var(--accent);
		color: white;
		border: none;
		&:hover { 
			background: var(--accent-dim); 
			transform: translateY(-1px);
		}
	}

	.btn-secondary {
		background: var(--surface-bright);
		color: var(--text-bright);
		border: 1px solid var(--border-bright);
		&:hover { 
			background: var(--border);
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

	.refresh-monitor {
		background: rgba(96, 165, 250, 0.08);
		border: 1px solid rgba(96, 165, 250, 0.35);
		color: var(--text-bright);
		padding: 12px 16px;
		border-radius: var(--radius);
		display: flex;
		align-items: center;
		gap: 12px;

		div {
			display: flex;
			flex-direction: column;
			gap: 2px;
		}

		span {
			color: var(--text-dim);
			font-size: 13px;
		}
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
		
		:global(.spin) {
			animation: spin 1s linear infinite;
		}

		p {
			max-width: 400px;
			line-height: 1.5;
			margin: 0 0 16px 0;
		}
	}

	.empty-state h3 {
		font-size: 18px;
		color: var(--text-bright);
		margin: 0;
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

		&:disabled {
			cursor: not-allowed;
			opacity: 0.35;

			&:hover {
				background: transparent;
				color: var(--text-dim);
			}
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
		grid-template-columns: repeat(4, 1fr);
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
		&.refreshing { color: #38bdf8 !important; }
		&.queued { color: #facc15 !important; }
		&.error { color: var(--accent) !important; }
	}

	.status-message {
		font-size: 12px;
		line-height: 1.35;
		color: var(--text-dim);
		background: rgba(255,255,255,0.04);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 8px 10px;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;

		&.error {
			color: #fca5a5;
			border-color: rgba(237, 28, 36, 0.35);
			background: rgba(237, 28, 36, 0.08);
		}
	}

	.card-footer {
		padding: 12px 16px;
		border-top: 1px solid var(--border);
		background: rgba(0,0,0,0.1);
		display: flex;
		flex-direction: column;
		gap: 10px;

		.last-updated {
			display: flex;
			align-items: center;
			gap: 6px;
			font-size: 11px;
			color: var(--text-dim);
		}
	}
</style>
