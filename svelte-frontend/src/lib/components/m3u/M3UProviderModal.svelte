<script lang="ts">
	import { api } from '$lib/api';
	import { toast } from '$lib/toast.svelte';
	import Modal from '$lib/components/ui/Modal.svelte';
	import { Save, AlertCircle, Search, Server, Tv, Film, Clapperboard, RefreshCw } from 'lucide-svelte';
	import {
		detectCategory,
		countryFlag,
		normalizeForDetection,
		buildFilterOptions,
		type CategoryClassification
	} from '$lib/utils/categoryDetection';

	let {
		show = $bindable(false),
		provider = null,
		onSave = () => {},
		onRefreshQueued = () => {}
	} = $props();

	let loading = $state(false);
	let syncing = $state(false);
	let savingSelections = $state(false);
	let syncStatus = $state('');
	let error = $state('');
	let activeTab = $state('general');
	let searchQuery = $state('');
	let countrySearch = $state('');
	let countryFilter = $state('');

	// System Data
	let allChannelGroups = $state<any[]>([]);
	let allVodCategories = $state<any[]>([]);

	type ProviderAccountSummary = {
		id?: number | string;
		m3u_account?: number | string;
		stream_count?: number;
	};

	// CountryOption is now a type alias for CategoryClassification so existing
	// template references (country.code, country.name, country.aliases) continue
	// to work without further template changes.
	type CountryOption = CategoryClassification;

	type CategoryItem = {
		id: number;
		name: string;
		m3u_accounts?: ProviderAccountSummary[];
	};

	// Form state
	let name = $state('');
	let accountType = $state('');
	let m3uUrl = $state('');
	let serverUrl = $state('');
	let username = $state('');
	let password = $state('');
	let maxStreams = $state(1);
	let refreshInterval = $state(24);
	let staleStreamDays = $state(7);
	let enableVod = $state(false);

	// Provider-specific settings (mappings)
	let groupSettings = $state<Record<number, { enabled: boolean; auto_channel_sync: boolean }>>({});
	let categorySettings = $state<Record<number, { enabled: boolean }>>({});

	function normalizeAccountType(value?: string) {
		const accountType = (value || '').toLowerCase();
		return accountType === 'xtream' ? 'xc' : accountType;
	}

	function isXcAccountType(value?: string) {
		return normalizeAccountType(value) === 'xc';
	}

	function isM3uAccountType(value?: string) {
		return normalizeAccountType(value) === 'm3u';
	}

	// detectCountry is a thin wrapper over the new utility so the template
	// ({@const country = detectCountry(group.name)}) continues to work unchanged.
	function detectCountry(name?: string): CountryOption | null {
		const result = detectCategory(name);
		return result.kind === 'unknown' ? null : result;
	}

	function getAccountSummary(item: CategoryItem) {
		return item.m3u_accounts?.find((acc: ProviderAccountSummary) =>
			Number(acc.id ?? acc.m3u_account) === Number(provider?.id)
		);
	}

	function getStreamCount(item: CategoryItem) {
		return getAccountSummary(item)?.stream_count || 0;
	}

	function belongsToCurrentProvider(item: CategoryItem) {
		return item.m3u_accounts?.some((acc: ProviderAccountSummary) =>
			Number(acc.id ?? acc.m3u_account) === Number(provider?.id)
		);
	}

	function matchesSearch(item: CategoryItem) {
		return item.name.toLowerCase().includes(searchQuery.toLowerCase());
	}

	function matchesCountry(item: CategoryItem) {
		if (!countryFilter) return true;
		return detectCountry(item.name)?.code === countryFilter;
	}

	function filteredByCountrySearch(country: CountryOption) {
		const query = countrySearch.trim().toLowerCase();
		if (!query) return true;
		return country.name.toLowerCase().includes(query) || country.aliases.some((alias) => alias.includes(query));
	}

	function countryFilterPlaceholder() {
		if (countryDetectionCoverage < 0.35) return 'No strong classification pattern detected';
		if (detectedCountryOptions.length === 0) return 'No detected categories match search';
		return 'All detected categories';
	}

	function isGroupEnabled(groupId: number) {
		return groupSettings[groupId]?.enabled === true;
	}

	function setVisibleGroups(enabled: boolean) {
		// Bulk actions only touch currently visible rows, preserving hidden/filter-excluded selections.
		for (const group of filteredGroups) {
			const current = groupSettings[group.id] || { enabled: false, auto_channel_sync: false };
			groupSettings[group.id] = { ...current, enabled };
		}
	}

	function handleGroupKeydown(e: KeyboardEvent, groupId: number) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			toggleGroup(groupId, 'enabled');
		}
	}

	function configSaveLabel() {
		if (activeTab === 'movies') return 'Save Movie Selections';
		if (activeTab === 'series') return 'Save Series Selections';
		return 'Save Category Selections';
	}

	// Filtered lists
	const filteredGroups = $derived(
		allChannelGroups.filter(g => 
			belongsToCurrentProvider(g) &&
			matchesSearch(g) &&
			matchesCountry(g)
		)
	);

	const detectedCountryOptions = $derived.by(() => {
		const countryMap = new Map<string, CountryOption>();
		for (const group of allChannelGroups) {
			if (!belongsToCurrentProvider(group) || !matchesSearch(group)) continue;
			const country = detectCountry(group.name);
			if (country) countryMap.set(country.code, country);
		}
		return Array.from(countryMap.values())
			.sort((a, b) => a.name.localeCompare(b.name));
	});

	const filteredCountryOptions = $derived.by(() => {
		const selectedCountry = detectedCountryOptions.find((country) => country.code === countryFilter);
		const filtered = detectedCountryOptions.filter(filteredByCountrySearch);
		if (selectedCountry && !filtered.some((country) => country.code === selectedCountry.code)) {
			return [selectedCountry, ...filtered];
		}
		return filtered;
	});

	const countryDetectionCoverage = $derived.by(() => {
		const providerGroups = allChannelGroups.filter((group) => belongsToCurrentProvider(group));
		if (providerGroups.length === 0) return 0;
		const detected = providerGroups.filter((group) => detectCountry(group.name)).length;
		return detected / providerGroups.length;
	});

	const shouldShowCountryFilter = $derived(
		activeTab === 'categories' && allChannelGroups.some((group) => belongsToCurrentProvider(group))
	);

	const filteredMovies = $derived(
		allVodCategories.filter(c => 
			c.category_type === 'movie' && 
			c.m3u_accounts?.some((acc: any) => Number(acc.m3u_account) === Number(provider?.id)) &&
			c.name.toLowerCase().includes(searchQuery.toLowerCase())
		)
	);

	const filteredSeries = $derived(
		allVodCategories.filter(c => 
			c.category_type === 'series' && 
			c.m3u_accounts?.some((acc: any) => Number(acc.m3u_account) === Number(provider?.id)) &&
			c.name.toLowerCase().includes(searchQuery.toLowerCase())
		)
	);

	// Effect to populate form when provider changes or modal opens
	$effect(() => {
		if (show) {
			if (provider) {
				name = provider.name || '';
				accountType = normalizeAccountType(provider.account_type) || 'm3u';
				m3uUrl = provider.server_url || '';
				serverUrl = provider.server_url || '';
				username = '';
				password = '';
				maxStreams = provider.max_streams || 1;
				refreshInterval = provider.refresh_interval || 24;
				staleStreamDays = provider.stale_stream_days || 7;
				enableVod = provider.enable_vod === true;
				countrySearch = '';
				countryFilter = '';
				searchQuery = '';

				// Initialize mappings from provider data
				const gSettings: Record<number, any> = {};
				if (provider.channel_groups) {
					provider.channel_groups.forEach((g: any) => {
						gSettings[g.channel_group] = {
							enabled: g.enabled,
							auto_channel_sync: g.auto_channel_sync
						};
					});
				}
				groupSettings = gSettings;

				const cSettings: Record<number, any> = {};
				if (provider.vod_categories) {
					provider.vod_categories.forEach((c: any) => {
						cSettings[c.id] = { enabled: c.enabled };
					});
				}
				categorySettings = cSettings;
			} else {
				// Reset form
				name = '';
				accountType = '';
				m3uUrl = '';
				serverUrl = '';
				username = '';
				password = '';
				maxStreams = 1;
				refreshInterval = 24;
				staleStreamDays = 7;
				enableVod = false;
				groupSettings = {};
				categorySettings = {};
				countrySearch = '';
				countryFilter = '';
				searchQuery = '';
				activeTab = 'general';
			}
			error = '';
			loadSystemData();
		}
	});

	async function loadSystemData() {
		try {
			const [groups, categories] = await Promise.all([
				api.getStreamGroups(),
				api.getVodCategories()
			]);
			allChannelGroups = groups || [];
			// Categories backend returns { results: [] }
			allVodCategories = categories?.results || [];
		} catch (err) {
			console.error('Failed to load system groups:', err);
		}
	}

	function buildGroupSettingsPayload() {
		return {
			group_settings: Object.entries(groupSettings).map(([id, settings]) => ({
				channel_group: parseInt(id),
				...settings
			})),
			category_settings: Object.entries(categorySettings).map(([id, settings]) => ({
				id: parseInt(id),
				...settings
			}))
		};
	}

	async function saveProviderDetails(e?: Event) {
		e?.preventDefault();
		error = '';
		loading = true;
		
		if (!accountType) {
			error = 'Please select an account type';
			loading = false;
			return;
		}

		if (isXcAccountType(accountType) && !provider?.id && (!username || !password)) {
			error = 'Username and password are required for new XTREAM Codes providers';
			loading = false;
			return;
		}

		try {
			const payload: Record<string, any> = {
				name,
				account_type: normalizeAccountType(accountType),
				server_url: isXcAccountType(accountType) ? serverUrl : m3uUrl,
				max_streams: maxStreams,
				refresh_interval: refreshInterval,
				stale_stream_days: staleStreamDays,
				is_active: true,
				enable_vod: enableVod
			};

			if (isXcAccountType(accountType)) {
				if (username) payload.username = username;
				if (password) payload.password = password;
			}

			// Save basic info
			const result = await onSave(payload, provider?.id);

			// If it was a new provider, we need to wait for the initial sync to discover groups
			if (!provider?.id && result?.id) {
				provider = result; // Update local reference for subsequent tabs
				syncing = true;
				syncStatus = 'Synchronizing with provider...';
				
				// Poll for discovered groups
				let attempts = 0;
				const maxAttempts = 30; // 30 seconds max
				
				while (attempts < maxAttempts) {
					syncStatus = `Synchronizing... attempt ${attempts + 1}/${maxAttempts}`;
					await new Promise(r => setTimeout(r, 1000));
					await loadSystemData();
					
					// If we have any data at all, we can stop waiting
					const hasCategories = filteredGroups.length > 0;
					const hasVod = filteredMovies.length > 0 || filteredSeries.length > 0;
					
					if (hasCategories || (enableVod && hasVod)) {
						break;
					}
					attempts++;
				}
				
				syncing = false;
				activeTab = 'categories';
				toast.success('Provider synced! You can now configure categories.');
				return;
			}

			toast.success('Provider saved successfully');
			show = false;
		} catch (err: any) {
			error = err.message || 'Failed to save provider';
			toast.error(error);
		} finally {
			loading = false;
		}
	}

	function toggleGroup(groupId: number, field: 'enabled' | 'auto_channel_sync') {
		const current = groupSettings[groupId] || { enabled: false, auto_channel_sync: false };
		const next = { ...current, [field]: !current[field] };
		
		// If we enable auto-sync, we must also enable the group
		if (field === 'auto_channel_sync' && next.auto_channel_sync) {
			next.enabled = true;
		}
		
		groupSettings[groupId] = next;
	}

	function toggleCategory(catId: number) {
		const current = categorySettings[catId] || { enabled: false };
		categorySettings[catId] = { ...current, enabled: !current.enabled };
	}

	async function saveImportSelections() {
		if (!provider?.id || savingSelections) return;

		error = '';
		savingSelections = true;
		try {
			// Mapping changes affect local stream inventory, so persist them before queueing a refresh.
			await api.updateM3UGroupSettings(provider.id, buildGroupSettingsPayload());
			await api.refreshM3UAccount(provider.id);
			onRefreshQueued();
			await loadSystemData();
			toast.success('Import selections saved. Provider refresh queued.');
		} catch (err: any) {
			error = err.message || 'Failed to save import selections';
			toast.error(error);
		} finally {
			savingSelections = false;
		}
	}
</script>

<Modal bind:show title={provider ? `Edit Provider: ${name}` : 'Add Provider'} width="800px">
	<div class="modal-layout">
		<aside class="modal-sidebar">
			<button 
				class="sidebar-item" 
				class:active={activeTab === 'general'} 
				onclick={() => activeTab = 'general'}
			>
				<Server size={18} />
				<span>General</span>
			</button>
			<button 
				class="sidebar-item" 
				class:active={activeTab === 'categories'} 
				onclick={() => activeTab = 'categories'}
				disabled={!provider}
			>
				<Tv size={18} />
				<span>Channel Categories</span>
			</button>
			<button 
				class="sidebar-item" 
				class:active={activeTab === 'movies'} 
				onclick={() => activeTab = 'movies'}
				disabled={!provider}
			>
				<Film size={18} />
				<span>VOD Movies</span>
			</button>
			<button 
				class="sidebar-item" 
				class:active={activeTab === 'series'} 
				onclick={() => activeTab = 'series'}
				disabled={!provider}
			>
				<Clapperboard size={18} />
				<span>VOD Series</span>
			</button>

			{#if !provider}
				<div class="sidebar-hint">
					<AlertCircle size={14} />
					<p>Groups can be configured after initial sync.</p>
				</div>
			{/if}
		</aside>

		<div class="modal-content">
			{#if syncing}
				<div class="sync-overlay">
					<RefreshCw class="animate-spin" size={48} />
					<h3>{syncStatus}</h3>
					<p>Fetching groups and categories from the provider...</p>
				</div>
			{/if}

			{#if activeTab === 'general'}
				<form id="provider-form" onsubmit={saveProviderDetails} class="tab-pane">
					<div class="form-group">
						<label for="name">Provider Name</label>
						<input type="text" id="name" bind:value={name} placeholder="e.g. My Premium IPTV" required />
					</div>

					<div class="form-row">
						<div class="form-group">
							<label for="accountType">Account Type</label>
							<select id="accountType" bind:value={accountType} required>
								<option value="" disabled selected>-- Please Select --</option>
								<option value="m3u">Standard M3U Playlist</option>
								<option value="xc">XTREAM Codes</option>
							</select>
						</div>
						
						<div class="form-group">
							<label for="maxStreams">Max Connections</label>
							<input type="number" id="maxStreams" bind:value={maxStreams} min="1" required />
						</div>
					</div>

					{#if isM3uAccountType(accountType)}
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
								<input
									type="text"
									id="username"
									bind:value={username}
									required={!provider}
									placeholder={provider?.has_username ? 'Saved - leave blank to keep' : 'Username'}
								/>
								{#if provider?.has_username}
									<span class="helper-text">A username is saved. Enter a new one only to replace it.</span>
								{/if}
							</div>
							<div class="form-group">
								<label for="password">Password</label>
								<input
									type="password"
									id="password"
									bind:value={password}
									required={!provider}
									placeholder={provider?.has_password ? 'Saved - leave blank to keep' : 'Password'}
									autocomplete="new-password"
								/>
								{#if provider?.has_password}
									<span class="helper-text">A password is saved. Enter a new one only to replace it.</span>
								{/if}
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

					<div class="form-group checkbox-group">
						<label class="checkbox-label">
							<input type="checkbox" bind:checked={enableVod} />
							<div class="checkbox-info">
								<span class="label-text">Enable VOD Ingestion</span>
								<span class="helper-text">If enabled, movies and series will be imported from this provider.</span>
							</div>
						</label>
					</div>
				</form>
			{:else}
				<div class="tab-pane">
					<div class="pane-header">
						<div class="search-box">
							<Search size={16} />
							<input type="text" placeholder="Search categories..." bind:value={searchQuery} />
						</div>
						{#if activeTab === 'categories'}
							<div class="category-toolbar">
								{#if shouldShowCountryFilter}
									<div class="category-filter-panel">
										<div class="filter-field">
											<label for="countrySearch">Category search</label>
											<input id="countrySearch" type="search" placeholder="Type to narrow categories..." bind:value={countrySearch} />
										</div>
										<div class="filter-field">
											<label for="countryFilter">Detected category</label>
											<select id="countryFilter" bind:value={countryFilter} aria-label="Filter categories by detected classification">
												<option value="">{countryFilterPlaceholder()}</option>
												{#each filteredCountryOptions as country}
													<option value={country.code}>{country.flag} {country.name}</option>
												{/each}
											</select>
										</div>
									</div>
								{/if}
								<div class="bulk-actions">
									<button type="button" onclick={() => setVisibleGroups(true)}>Select visible</button>
									<button type="button" onclick={() => setVisibleGroups(false)}>Deselect visible</button>
								</div>
							</div>
						{/if}
					</div>

					<div class="settings-list">
						{#if activeTab === 'categories'}
							{#each filteredGroups as group}
								{@const country = detectCountry(group.name)}
								<div
									class="setting-item selectable"
									class:selected={isGroupEnabled(group.id)}
									onclick={() => toggleGroup(group.id, 'enabled')}
									onkeydown={(e) => handleGroupKeydown(e, group.id)}
									role="button"
									tabindex="0"
									aria-pressed={isGroupEnabled(group.id)}
								>
									<div class="setting-info">
										<span class="setting-name">
											{#if country}
												<span class="flag" title={country.name}>{country.flag}</span>
											{/if}
											{group.name}
										</span>
										<span class="setting-sub">
											{getStreamCount(group)} streams found
											{#if country}
												- {country.name}
											{/if}
										</span>
									</div>
									<div class="setting-controls">
										<span class="selection-pill" class:active={isGroupEnabled(group.id)}>
											{isGroupEnabled(group.id) ? 'Selected' : 'Not selected'}
										</span>
										<button 
											class="toggle-btn" 
											class:active={groupSettings[group.id]?.auto_channel_sync}
											onclick={(e) => {
												e.stopPropagation();
												toggleGroup(group.id, 'auto_channel_sync');
											}}
										>
											<RefreshCw size={14} />
											<span>Auto-Sync</span>
										</button>
									</div>
								</div>
							{/each}
							{#if filteredGroups.length === 0}
								<div class="empty-settings">
									No channel categories match the current filters.
								</div>
							{/if}
						{:else if activeTab === 'movies'}
							{#each filteredMovies as cat}
								<div class="setting-item">
									<div class="setting-info">
										<span class="setting-name">{cat.name}</span>
										<span class="setting-sub">
											{(cat.m3u_accounts?.find((a: ProviderAccountSummary) => Number(a.m3u_account) === Number(provider?.id))?.stream_count || 0)} streams found
										</span>
									</div>
									<div class="setting-controls">
										<button 
											class="toggle-btn" 
											class:active={categorySettings[cat.id]?.enabled}
											onclick={() => toggleCategory(cat.id)}
										>
											{categorySettings[cat.id]?.enabled ? 'Enabled' : 'Disabled'}
										</button>
									</div>
								</div>
							{/each}
						{:else if activeTab === 'series'}
							{#each filteredSeries as cat}
								<div class="setting-item">
									<div class="setting-info">
										<span class="setting-name">{cat.name}</span>
										<span class="setting-sub">
											{(cat.m3u_accounts?.find((a: ProviderAccountSummary) => Number(a.m3u_account) === Number(provider?.id))?.stream_count || 0)} streams found
										</span>
									</div>
									<div class="setting-controls">
										<button 
											class="toggle-btn" 
											class:active={categorySettings[cat.id]?.enabled}
											onclick={() => toggleCategory(cat.id)}
										>
											{categorySettings[cat.id]?.enabled ? 'Enabled' : 'Disabled'}
										</button>
									</div>
								</div>
							{/each}
						{/if}
					</div>
				</div>
			{/if}

			<div class="modal-footer">
				<div class="footer-status">
					{#if error}
						<span class="status-error"><AlertCircle size={14} /> {error}</span>
					{/if}
				</div>
				<div class="footer-actions">
					<button type="button" class="btn-cancel" onclick={() => show = false} disabled={loading || savingSelections}>
						{activeTab === 'general' ? 'Cancel' : 'Close'}
					</button>
					{#if activeTab === 'general'}
						<button type="button" class="btn-submit" onclick={saveProviderDetails} disabled={loading || syncing || !name}>
							<Save size={18} />
							<span>{loading ? 'Saving...' : syncing ? 'Syncing...' : 'Save Provider'}</span>
						</button>
					{:else}
						<button type="button" class="btn-submit" onclick={saveImportSelections} disabled={savingSelections || syncing || !provider?.id}>
							<Save size={18} />
							<span>{savingSelections ? 'Saving...' : configSaveLabel()}</span>
						</button>
					{/if}
				</div>
			</div>
		</div>
	</div>
</Modal>

<style lang="less">
	.modal-layout {
		display: flex;
		height: 550px;
		background: var(--surface);
	}

	.modal-sidebar {
		width: 220px;
		border-right: 1px solid var(--border);
		padding: 20px 12px;
		display: flex;
		flex-direction: column;
		gap: 4px;
		background: rgba(0, 0, 0, 0.1);

		.sidebar-item {
			display: flex;
			align-items: center;
			gap: 12px;
			padding: 10px 14px;
			border-radius: var(--radius);
			border: none;
			background: transparent;
			color: var(--text-dim);
			font-size: 14px;
			font-weight: 500;
			cursor: pointer;
			transition: all 0.2s;
			text-align: left;

			&:hover:not(:disabled) {
				background: rgba(255, 255, 255, 0.05);
				color: var(--text-bright);
			}

			&.active {
				background: var(--accent);
				color: white;
			}

			&:disabled {
				opacity: 0.3;
				cursor: not-allowed;
			}
		}

		.sidebar-hint {
			margin-top: auto;
			padding: 12px;
			background: rgba(255, 255, 255, 0.03);
			border-radius: var(--radius);
			display: flex;
			gap: 8px;
			color: var(--text-dim);
			
			p {
				font-size: 11px;
				line-height: 1.4;
				margin: 0;
			}
		}
	}

	.checkbox-group {
		margin-top: 10px;
		padding: 12px;
		background: rgba(255, 255, 255, 0.03);
		border-radius: var(--radius);
		border: 1px solid var(--border);

		.checkbox-label {
			display: flex;
			align-items: flex-start;
			gap: 12px;
			cursor: pointer;

			input[type="checkbox"] {
				width: 18px;
				height: 18px;
				margin-top: 2px;
			}

			.checkbox-info {
				display: flex;
				flex-direction: column;
				gap: 2px;

				.label-text {
					font-weight: 500;
					color: var(--text);
				}

				.helper-text {
					font-size: 0.8rem;
					color: var(--text-dim);
				}
			}
		}
	}

	.modal-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		position: relative;
	}

	.tab-pane {
		flex: 1;
		padding: 24px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.pane-header {
		margin-bottom: 8px;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.search-box {
		position: relative;
		display: flex;
		align-items: center;

		:global(svg) {
			position: absolute;
			left: 12px;
			color: var(--text-dim);
		}

		input {
			width: 100%;
			background: rgba(0, 0, 0, 0.2);
			border: 1px solid var(--border);
			padding: 10px 12px 10px 38px;
			border-radius: 20px;
			color: var(--text-bright);
			font-size: 14px;

			&:focus {
				outline: none;
				border-color: var(--accent);
			}
		}
	}

	.settings-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.category-toolbar {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: 12px;
		flex-wrap: wrap;
	}

	.category-filter-panel {
		display: grid;
		grid-template-columns: minmax(180px, 1fr) minmax(210px, 1.2fr);
		gap: 10px;
		flex: 1;
		min-width: min(100%, 420px);
		padding: 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: rgba(255, 255, 255, 0.03);
	}

	.filter-field {
		display: flex;
		flex-direction: column;
		gap: 5px;

		label {
			color: var(--text-dim);
			font-size: 11px;
			font-weight: 700;
			text-transform: uppercase;
			letter-spacing: 0.5px;
		}

		input, select {
			background: rgba(0, 0, 0, 0.2);
			border: 1px solid var(--border);
			color: var(--text-bright);
			padding: 8px 10px;
			border-radius: var(--radius);
			font-size: 13px;

			&:focus {
				outline: none;
				border-color: var(--accent);
			}
		}

		input {
			width: 100%;
		}

		select {
			width: 100%;
			max-width: 100%;
		}
	}

	.bulk-actions {
		display: flex;
		gap: 8px;
		margin-left: auto;

		button {
			background: var(--surface-bright);
			border: 1px solid var(--border);
			color: var(--text);
			border-radius: var(--radius);
			padding: 8px 10px;
			font-size: 12px;
			font-weight: 600;
			cursor: pointer;

			&:hover {
				border-color: var(--border-bright);
				color: var(--text-bright);
			}
		}
	}

	.setting-item {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		padding: 12px 16px;
		border-radius: var(--radius);
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;

		.setting-info {
			display: flex;
			flex-direction: column;
			gap: 2px;
			min-width: 0;

			.setting-name {
				font-size: 14px;
				font-weight: 500;
				color: var(--text-bright);
				white-space: nowrap;
				overflow: hidden;
				text-overflow: ellipsis;
			}

			.setting-sub {
				font-size: 12px;
				color: var(--text-dim);
			}
		}

		.setting-controls {
			display: flex;
			gap: 8px;
		}

		&.selectable {
			cursor: pointer;

			&:hover {
				background: rgba(255, 255, 255, 0.06);
				border-color: var(--border-bright);
			}

			&:focus {
				outline: 2px solid var(--accent);
				outline-offset: 2px;
			}

			&.selected {
				background: rgba(59, 130, 246, 0.1);
				border-color: rgba(96, 165, 250, 0.55);
			}
		}
	}

	.flag {
		margin-right: 8px;
	}

	.selection-pill {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 92px;
		padding: 6px 10px;
		border-radius: 4px;
		border: 1px solid var(--border);
		color: var(--text-dim);
		font-size: 12px;
		font-weight: 600;

		&.active {
			background: rgba(74, 222, 128, 0.14);
			border-color: rgba(74, 222, 128, 0.5);
			color: #4ade80;
		}
	}

	.empty-settings {
		border: 1px dashed var(--border);
		border-radius: var(--radius);
		color: var(--text-dim);
		padding: 20px;
		text-align: center;
	}

	.toggle-btn {
		padding: 6px 12px;
		border-radius: 4px;
		border: 1px solid var(--border);
		background: transparent;
		color: var(--text-dim);
		font-size: 12px;
		font-weight: 500;
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 6px;
		transition: all 0.2s;

		&.active {
			background: var(--accent);
			border-color: var(--accent);
			color: white;
		}

		&:hover:not(.active) {
			background: rgba(255, 255, 255, 0.05);
		}
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
		}

		.helper-text {
			font-size: 12px;
			color: var(--text-dim);
			line-height: 1.35;
		}

		select {
			appearance: none;
			background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%23a0a0a0' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
			background-repeat: no-repeat;
			background-position: right 12px center;
			padding-right: 40px;
		}
	}

	.modal-footer {
		margin-top: auto;
		padding: 20px 24px;
		border-top: 1px solid var(--border);
		display: flex;
		align-items: center;
		justify-content: space-between;
		background: rgba(0, 0, 0, 0.1);

		.footer-status {
			.status-error {
				color: var(--accent);
				font-size: 13px;
				display: flex;
				align-items: center;
				gap: 6px;
			}
		}

		.footer-actions {
			display: flex;
			gap: 12px;

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
	}

	.sync-overlay {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: var(--surface);
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 16px;
		z-index: 10;
		text-align: center;
		padding: 40px;

		h3 {
			margin: 0;
			color: var(--text-bright);
		}

		p {
			color: var(--text-dim);
			margin: 0;
		}
	}

	:global(.animate-spin) {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		from { transform: rotate(0deg); }
		to { transform: rotate(360deg); }
	}
</style>
