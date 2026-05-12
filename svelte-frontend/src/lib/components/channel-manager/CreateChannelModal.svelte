<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import Modal from '../ui/Modal.svelte';
	import { Pencil, Folder, Database, Tv, Image, Upload, ChevronDown, ChevronUp, PlusCircle, Search, TriangleAlert, List } from 'lucide-svelte';

	/**
	 * Props:
	 *   show        — controls modal visibility (bindable)
	 *   onCreated   — callback after create OR edit save
	 *   channelId   — when set, modal operates in EDIT mode
	 *   initialData — pre-populated form when editing an existing channel
	 */
	let { show = $bindable(false), onCreated, channelId = null as number | null, initialData = null as any } = $props();

	// true when editing an existing channel, false when creating a new one
	let isEditMode = $derived(channelId !== null);

	// --- Form State ---
	let form = $state({
		name: '',
		channel_number: null as number | null,
		channel_group_id: null as number | null,
		stream_profile_id: null as number | null,
		user_level: 3,
		logo_id: null as number | null,
		is_adult: false,
		tvg_id: '',
		tvc_guide_stationid: '',
		epg_data_id: null as number | null
	});

	// --- Data Collections ---
	let groups = $state<any[]>([]);
	let profiles = $state<any[]>([]);
	let logos = $state<any[]>([]);
	let epgSources = $state<any[]>([]);
	let epgData = $state<any[]>([]);
	let existingChannels = $state<any[]>([]);

	let userLevels = [
		{ id: 1, name: 'Admin' },
		{ id: 2, name: 'Standard User' },
		{ id: 3, name: 'Streamer' }
	];

	// --- UI State ---
	let submitting = $state(false);
	
	// Group selection
	let groupSearch = $state('');
	let showOnlyCustomGroups = $state(false);
	let addingGroup = $state(false);
	let newGroupName = $state('');

	// Sidebar
	let channelSearch = $state('');

	// DB Browser
	let dbBrowserExpanded = $state(false);
	let epgSearch = $state('');
	let dbSearchResults = $state<any[]>([]);
	let searchingEPG = $state(false);
	let dbSearchError = $state<string | null>(null);
	let selectedMatch = $state<any>(null);
	let selectedStation = $state<any>(null); // To power shortcuts

	// EPG Assignment
	let selectedEpgSourceId = $state<number | null>(null);
	let epgFilter = $state('');

	// Logo
	let newLogoUrl = $state('');
	let addingLogo = $state(false);
	let uploadingLogo = $state(false);
	let selectedLogoPreview = $state<string | null>(null);
	let fileInput: HTMLInputElement;

	// Suggestions (Channel Name)
	let suggestions = $state<any[]>([]);
	let nameTimeout: any;

	// --- Derived State ---
	let filteredGroups = $derived(
		groups.filter(g => {
			if (showOnlyCustomGroups && !g.is_custom) return false;
			if (!groupSearch) return true;
			const search = groupSearch.toLowerCase();
			if (g.name.toLowerCase().includes(search)) return true;
			if (g.m3u_accounts && Array.isArray(g.m3u_accounts)) {
				return g.m3u_accounts.some((acc: string) => acc.toLowerCase().includes(search));
			}
			return false;
		})
	);

	let filteredExistingChannels = $derived(
		existingChannels.filter(ch => {
			if (!channelSearch) return true;
			const lower = channelSearch.toLowerCase();
			return ch.name.toLowerCase().includes(lower) || String(ch.channel_number).includes(lower);
		})
	);

	let filteredEpgData = $derived(
		epgData.filter(e => {
			if (selectedEpgSourceId === null) return false;
			if (e.epg_source_id !== selectedEpgSourceId && e.epg_source !== selectedEpgSourceId) return false;
			if (!epgFilter) return true;
			const search = epgFilter.toLowerCase();
			return (e.name && e.name.toLowerCase().includes(search)) || (e.tvg_id && e.tvg_id.toLowerCase().includes(search));
		}).slice(0, 100)
	);

	// --- Lifecycle ---
	$effect(() => {
		if (show) {
			loadData();
			resetForm();
			// In edit mode, overlay the provided channel data on top of the cleared form
			if (initialData) {
				form = {
					name:                initialData.name ?? '',
					channel_number:      initialData.channel_number ?? null,
					channel_group_id:    initialData.channel_group_id ?? initialData.channel_group ?? null,
					stream_profile_id:   initialData.stream_profile_id ?? initialData.stream_profile ?? null,
					user_level:          initialData.user_level ?? 3,
					logo_id:             initialData.logo_id ?? initialData.logo ?? null,
					is_adult:            initialData.is_adult ?? false,
					tvg_id:              initialData.tvg_id ?? '',
					tvc_guide_stationid: initialData.tvc_guide_stationid ?? '',
					epg_data_id:         initialData.epg_data_id ?? initialData.epg_data ?? null
				};
			}
		}
	});

	$effect(() => {
		// Watch logo_id to update preview
		if (form.logo_id) {
			const logo = logos.find(l => l.id === form.logo_id);
			if (logo && !uploadingLogo) {
				selectedLogoPreview = logo.cache_url || logo.url;
			}
		} else {
			selectedLogoPreview = null;
		}
	});

	$effect(() => {
		// Watch name for auto-suggest
		const val = form.name;
		if (val && val.length > 2) {
			clearTimeout(nameTimeout);
			nameTimeout = setTimeout(async () => {
				try {
					const res = await api.suggestMatches(val);
					if (res && res.matches) {
						suggestions = res.matches.map((m: any) => m.station);
					}
				} catch (e) {
					suggestions = [];
				}
			}, 500);
		} else {
			suggestions = [];
		}
	});

	// --- Methods ---
	async function loadData() {
		try {
			const [groupsRes, channelsRes, profilesRes, logosRes, sourcesRes, epgRes] = await Promise.all([
				api.getChannelGroups().catch(() => ({ results: [] })),
				api.getChannelsSummary().catch(() => ({ results: [] })),
				api.getStreamProfiles().catch(() => ({ results: [] })),
				api.getLogos().catch(() => ({ results: [] })),
				api.getEpgSources().catch(() => ({ results: [] })),
				api.getEpgData().catch(() => ({ results: [] }))
			]);

			groups = (groupsRes.results || groupsRes).sort((a: any, b: any) => a.name.localeCompare(b.name));
			existingChannels = (channelsRes.results || channelsRes).sort((a: any, b: any) => (a.channel_number || 0) - (b.channel_number || 0));
			profiles = profilesRes.results || profilesRes;
			logos = logosRes.results || logosRes;
			epgSources = sourcesRes.results || sourcesRes;
			epgData = epgRes.results || epgRes;
		} catch (e) {
			console.error("Failed to load modal data", e);
		}
	}

	function resetForm() {
		form = {
			name: '',
			channel_number: null,
			channel_group_id: null,
			stream_profile_id: null,
			user_level: 3,
			logo_id: null,
			is_adult: false,
			tvg_id: '',
			tvc_guide_stationid: '',
			epg_data_id: null
		};
		dbBrowserExpanded = false;
		selectedMatch = null;
		selectedStation = null;
		selectedEpgSourceId = null;
		epgSearch = '';
		dbSearchResults = [];
		suggestions = [];
	}

	// Group Logic
	async function confirmAddGroup() {
		const name = newGroupName.trim();
		if (!name) return;
		try {
			const res = await api.createChannelGroup(name);
			groups = [...groups, res].sort((a: any, b: any) => a.name.localeCompare(b.name));
			form.channel_group_id = res.id;
			addingGroup = false;
			newGroupName = '';
		} catch (e) {
			console.error("Failed to create group", e);
		}
	}

	// Shortcuts
	function useFirstAvailable() {
		const used = new Set(existingChannels.map(ch => ch.channel_number));
		let num = 1;
		while (used.has(num)) num++;
		form.channel_number = num;
	}

	function useHighestPlusOne() {
		if (existingChannels.length === 0) {
			form.channel_number = 1;
		} else {
			const max = Math.max(...existingChannels.map(ch => ch.channel_number || 0));
			form.channel_number = max + 1;
		}
	}

	function useSmartRange() {
		const current = form.channel_number || 1;
		const used = new Set(existingChannels.map(ch => ch.channel_number));
		let num = current;
		while (used.has(num)) num++;
		form.channel_number = num;
	}

	// DB Browser Logic
	function selectSuggestion(station: any) {
		selectedStation = station;
		form.name = station.name;
		form.tvg_id = station.call_sign || station.name;
		form.tvc_guide_stationid = station.station_id || '';
		suggestions = [];
	}

	async function performEpgSearch(e?: Event) {
		if (e) e.preventDefault();
		if (!epgSearch) return;
		
		searchingEPG = true;
		dbSearchError = null;
		
		try {
			const res = await api.suggestMatches(epgSearch);
			dbSearchResults = (res.matches || []).map((m: any) => m.station);
			if (dbSearchResults.length > 0) {
				selectedMatch = dbSearchResults[0];
			} else {
				dbSearchError = `No matches found for "${epgSearch}". The channel data database may not contain this station.`;
			}
		} catch (err: any) {
			const status = err?.message?.includes('503') ? 503 : 0;
			if (status === 503) {
				dbSearchError = 'Channel data database is not available. Place channel_data.db in the data/ directory.';
			} else {
				dbSearchError = `Search failed: ${err.message}`;
			}
			dbSearchResults = [];
		} finally {
			searchingEPG = false;
		}
	}

	function applyMatchField(field: 'name' | 'tvg_id' | 'tvc_guide_stationid' | 'logo' | 'all') {
		if (!selectedMatch) return;

		if (field === 'name' || field === 'all') form.name = selectedMatch.name;
		if (field === 'tvg_id' || field === 'all') {
			const targetTvgId = selectedMatch.call_sign || selectedMatch.name;
			const targetStationId = selectedMatch.station_id;
			form.tvg_id = targetTvgId;

			// Auto-match EPG
			const match = epgData.find(e => 
				(e.tvg_id && e.tvg_id === targetTvgId) || 
				(targetStationId && e.tvg_id === targetStationId)
			);
			if (match) {
				selectedEpgSourceId = match.epg_source_id || match.epg_source;
				form.epg_data_id = match.id;
			}
		}
		if (field === 'tvc_guide_stationid' || field === 'all') form.tvc_guide_stationid = selectedMatch.station_id;
		if (field === 'logo' || field === 'all') {
			if (selectedMatch.logo_uri) {
				newLogoUrl = selectedMatch.logo_uri;
				addLogoFromUrl();
			}
		}

		selectedStation = selectedMatch;
	}

	function useDummyEpg() {
		form.tvg_id = 'dummy';
		form.tvc_guide_stationid = 'dummy';
		form.epg_data_id = null;
		selectedStation = { name: 'Dummy EPG', station_id: 'dummy' };
	}

	// Logo Logic
	async function addLogoFromUrl() {
		const url = newLogoUrl.trim();
		if (!url) return;
		addingLogo = true;
		try {
			const res = await api.createLogo({ name: url.split('/').pop() || 'Imported Logo', url });
			logos = [res, ...logos];
			form.logo_id = res.id;
			newLogoUrl = '';
		} finally {
			addingLogo = false;
		}
	}

	async function uploadLogoFile(event: Event) {
		const file = (event.target as HTMLInputElement).files?.[0];
		if (!file) return;

		if (!file.type.startsWith('image/')) return;

		// Local preview
		const reader = new FileReader();
		reader.onload = (e) => {
			selectedLogoPreview = e.target?.result as string;
		};
		reader.readAsDataURL(file);

		uploadingLogo = true;
		try {
			const res = await api.uploadLogo(file);
			logos = [res, ...logos];
			form.logo_id = res.id;
		} catch (e) {
			selectedLogoPreview = null;
		} finally {
			uploadingLogo = false;
			(event.target as HTMLInputElement).value = '';
		}
	}

	// Submit — routes to createChannel (POST) or updateChannel (PATCH) based on mode
	async function submit(e: Event) {
		e.preventDefault();
		if (!form.name || form.channel_group_id === null) return;

		submitting = true;
		try {
			if (isEditMode && channelId !== null) {
				await api.updateChannel(channelId, form);
			} else {
				await api.createChannel(form);
			}
			show = false;
			if (onCreated) onCreated();
		} catch (err) {
			console.error(isEditMode ? 'Failed to update channel' : 'Failed to create channel', err);
		} finally {
			submitting = false;
		}
	}
</script>

<Modal bind:show title={isEditMode ? 'Edit Channel' : 'Create New Channel'} width="900px" height="85vh">
	<div class="split-layout">
		<!-- LEFT PANEL: Form -->
		<div class="form-panel">
			<form onsubmit={submit}>
				
				<!-- Configuration -->
				<div class="section-card">
					<div class="section-title">
						<Pencil size={16} /> Channel Configuration
					</div>
					
					<div class="form-group">
						<label for="channel-name">Channel Name <span class="required">*</span></label>
						<div class="input-wrapper">
							<input id="channel-name" type="text" bind:value={form.name} class="form-input" placeholder="Enter channel name" autocomplete="off" required>
							{#if suggestions.length > 0}
								<div class="suggestions-dropdown">
									{#each suggestions as s}
										<button type="button" class="suggestion-item" onclick={() => selectSuggestion(s)}>
											<span class="s-name">{s.name}</span>
											{#if s.call_sign}<span class="s-callsign">{s.call_sign}</span>{/if}
										</button>
									{/each}
								</div>
							{/if}
						</div>
					</div>

					<div class="form-group mt-3">
						<label for="channel-number">Channel Number</label>
						<div class="channel-number-row">
							<input id="channel-number" type="number" bind:value={form.channel_number} class="form-input ch-num-input" placeholder="e.g. 101">
							<div class="ch-num-buttons">
								<button type="button" class="btn-pill" onclick={useSmartRange} title="Find gap near your input number">Smart Range</button>
								<button type="button" class="btn-pill" onclick={useFirstAvailable}>First Available</button>
								<button type="button" class="btn-pill" onclick={useHighestPlusOne}>Highest + 1</button>
							</div>
						</div>
					</div>
				</div>

				<!-- Group -->
				<div class="section-card">
					<div class="section-title">
						<Folder size={16} /> Group <span class="required">*</span>
					</div>

					<input type="text" bind:value={groupSearch} class="form-input mb-2" placeholder="Type to filter groups...">
					
					{#if addingGroup}
						<div class="new-group-inline mb-2">
							<input type="text" bind:value={newGroupName} class="form-input" placeholder="New group name..." onkeyup={(e) => e.key === 'Enter' && confirmAddGroup()}>
							<button type="button" class="btn-icon confirm" onclick={confirmAddGroup}>✓</button>
							<button type="button" class="btn-icon cancel" onclick={() => addingGroup = false}>✕</button>
						</div>
					{:else}
						<button type="button" class="btn-create mb-2" onclick={() => addingGroup = true}>
							<PlusCircle size={14} /> Create New Group
						</button>
					{/if}

					<label class="custom-checkbox mb-2">
						<input type="checkbox" bind:checked={showOnlyCustomGroups}>
						<span class="checkmark"></span>
						Show only custom groups (no M3U associations)
					</label>

					<div class="group-list">
						{#each filteredGroups as g}
							<button type="button" class="group-item" class:selected={form.channel_group_id === g.id} onclick={() => form.channel_group_id = g.id}>
								<span class="group-name">{g.name}</span>
								<div class="group-badges">
									{#if g.is_custom}<span class="badge custom">custom</span>{/if}
									{#each (g.m3u_accounts || []) as acc}
										<span class="badge m3u" title={acc}>{acc}</span>
									{/each}
								</div>
							</button>
						{/each}
						{#if filteredGroups.length === 0}
							<div class="empty-text">No groups match filter.</div>
						{/if}
					</div>
				</div>

				<!-- DB Browser -->
				<div class="section-card">
					<button type="button" class="section-title db-toggle" onclick={() => dbBrowserExpanded = !dbBrowserExpanded}>
						<span class="flex-align"><Database size={16} /> Channel Data Lookup</span>
						{#if dbBrowserExpanded}<ChevronUp size={16} />{:else}<ChevronDown size={16} />{/if}
					</button>

					{#if dbBrowserExpanded}
						<div class="db-browser">
							<div class="db-search-row">
								<input type="text" bind:value={epgSearch} class="form-input" placeholder="Search channel data..." onkeydown={(e) => e.key === 'Enter' && performEpgSearch(e)}>
								<button type="button" class="btn-search" onclick={() => performEpgSearch()}>
									<Search size={16} />
								</button>
							</div>

							{#if searchingEPG}
								<div class="loading-text">Searching channel data...</div>
							{:else if dbSearchError}
								<div class="error-box">
									<TriangleAlert size={16} />
									<span>{dbSearchError}</span>
								</div>
							{:else if dbSearchResults.length > 0}
								<div class="db-results">
									{#each dbSearchResults as m}
										<button type="button" class="db-result-item" class:active={selectedMatch?.station_id === m.station_id} onclick={() => selectedMatch = m}>
											<div class="db-result-logo">
												{#if m.logo_uri}
													<img src={m.logo_uri} alt="" onerror={(e) => m.logo_uri = null}>
												{:else}
													<Tv size={16} />
												{/if}
											</div>
											<div class="db-result-info">
												<span class="name">{m.name}</span>
												{#if m.call_sign}<span class="call">{m.call_sign}</span>{/if}
											</div>
										</button>
									{/each}
								</div>

								{#if selectedMatch}
									<div class="db-match-apply">
										<div class="apply-header">
											<span>{selectedMatch.name}</span>
											<button type="button" class="btn-pill active" onclick={() => applyMatchField('all')}>Apply All</button>
										</div>
										<div class="apply-buttons">
											<button type="button" class="btn-pill" onclick={() => applyMatchField('name')}>Name</button>
											<button type="button" class="btn-pill" onclick={() => applyMatchField('tvg_id')}>TVG-ID</button>
											<button type="button" class="btn-pill" onclick={() => applyMatchField('tvc_guide_stationid')}>Station ID</button>
											<button type="button" class="btn-pill" disabled={!selectedMatch.logo_uri} onclick={() => applyMatchField('logo')}>Logo</button>
										</div>
									</div>
								{/if}
							{:else if epgSearch}
								<div class="empty-text">No matches found.</div>
							{/if}
						</div>
					{/if}
				</div>

				<!-- EPG & Metadata -->
				<div class="section-card">
					<div class="section-title">
						<Tv size={16} /> EPG & Metadata
					</div>

					<div class="two-col">
						<div class="sub-col">
							<div class="form-group">
								<div class="label-row">
									<label for="tvg-id">TVG-ID</label>
									{#if selectedStation}
										<button type="button" class="shortcut-link" onclick={() => form.tvg_id = selectedStation.call_sign || selectedStation.name}>Use EPG</button>
									{/if}
								</div>
								<input id="tvg-id" type="text" bind:value={form.tvg_id} class="form-input" placeholder="e.g. NFLHD">
							</div>

							<div class="form-group mt-3">
								<label for="station-id">Station ID</label>
								<input id="station-id" type="text" bind:value={form.tvc_guide_stationid} class="form-input" placeholder="e.g. 45399">
							</div>

							<div class="form-group mt-3">
								<label for="stream-profile">Stream Profile</label>
								<select id="stream-profile" bind:value={form.stream_profile_id} class="form-select">
									<option value={null}>(use default)</option>
									{#each profiles as p}
										<option value={p.id}>{p.name}</option>
									{/each}
								</select>
							</div>

							<div class="form-group mt-3">
								<label for="user-level">User Level</label>
								<select id="user-level" bind:value={form.user_level} class="form-select">
									{#each userLevels as level}
										<option value={level.id}>{level.name}</option>
									{/each}
								</select>
							</div>
						</div>

						<div class="sub-col">
							<div class="form-group">
								<div class="label-row">
									<label for="epg-source">EPG Assignment</label>
									<div class="actions">
										<button type="button" class="shortcut-link" onclick={useDummyEpg}>Use Dummy</button>
										{#if form.epg_data_id}
											<button type="button" class="shortcut-link danger" onclick={() => form.epg_data_id = null}>Clear</button>
										{/if}
									</div>
								</div>
								
								<div class="epg-selector">
									<div class="epg-controls">
										<select id="epg-source" bind:value={selectedEpgSourceId} class="form-select mb-2">
											<option value={null}>Select Source...</option>
											{#each epgSources as source}
												<option value={source.id}>{source.name}</option>
											{/each}
										</select>
										<input type="text" bind:value={epgFilter} class="form-input" placeholder="Filter EPG...">
									</div>
									
									<div class="epg-list">
										{#if selectedEpgSourceId !== null}
											{#each filteredEpgData as epg}
												<button type="button" class="epg-item" class:selected={form.epg_data_id === epg.id} onclick={() => form.epg_data_id = epg.id}>
													<span class="name">{epg.name}</span>
													{#if epg.tvg_id}<span class="tvgid">({epg.tvg_id})</span>{/if}
												</button>
											{/each}
											{#if filteredEpgData.length === 0}
												<div class="empty-text">No EPG entries match.</div>
											{/if}
										{:else}
											<div class="empty-text">Select a source to view channels.</div>
										{/if}
									</div>
								</div>
							</div>

							<div class="form-group mt-3">
								<div class="label-row">
									<span class="field-label">Logo</span>
									{#if selectedStation?.logo_uri}
										<button type="button" class="shortcut-link" onclick={() => { newLogoUrl = selectedStation.logo_uri; addLogoFromUrl(); }}>Use DB Logo</button>
									{/if}
								</div>
								<div class="logo-preview">
									{#if selectedLogoPreview}
										<img src={selectedLogoPreview} alt="Logo" onerror={() => selectedLogoPreview = null}>
									{:else}
										<div class="placeholder"><Image size={24} /></div>
									{/if}
								</div>
								<button type="button" class="btn-upload mt-2" onclick={() => fileInput.click()}>
									<Upload size={14} /> Upload Logo
								</button>
								<input type="file" bind:this={fileInput} hidden accept="image/*" onchange={uploadLogoFile}>
							</div>

							<div class="form-group mt-3">
								<label class="custom-checkbox">
									<input type="checkbox" bind:checked={form.is_adult}>
									<span class="checkmark"></span>
									Mature Content
								</label>
							</div>
						</div>
					</div>
				</div>

				<div class="form-actions">
					<button type="button" class="btn-cancel" onclick={() => show = false}>Cancel</button>
					<button type="submit" class="btn-submit" disabled={!form.name || form.channel_group_id === null || submitting}>
						{#if submitting}
							{isEditMode ? 'Saving...' : 'Creating...'}
						{:else}
							{isEditMode ? '✓ Save Changes' : '+ Create Channel'}
						{/if}
					</button>
				</div>
			</form>
		</div>

		<!-- RIGHT PANEL: Sidebar -->
		<div class="sidebar-panel">
			<div class="sidebar-header">
				<List size={16} /> Existing Channels
			</div>
			<div class="sidebar-search">
				<input type="text" bind:value={channelSearch} class="form-input" placeholder="Filter by number or name...">
			</div>
			<div class="channel-list">
				{#each filteredExistingChannels as ch}
					<div class="channel-row">
						<span class="num">{ch.channel_number}</span>
						<span class="name">{ch.name}</span>
					</div>
				{/each}
				{#if filteredExistingChannels.length === 0}
					<div class="empty-text">No channels found.</div>
				{/if}
			</div>
		</div>
	</div>
</Modal>

<style>
	.split-layout {
		display: flex;
		height: 100%;
		overflow: hidden;
	}

	.form-panel {
		flex: 1;
		padding: 24px;
		overflow-y: auto;
		background: var(--surface);
	}

	.sidebar-panel {
		width: 280px;
		background: var(--surface-bright);
		border-left: 1px solid var(--border);
		display: flex;
		flex-direction: column;
	}

	.sidebar-header {
		padding: 16px;
		display: flex;
		align-items: center;
		gap: 8px;
		font-weight: 600;
		color: var(--text-bright);
		border-bottom: 1px solid var(--border);
	}

	.sidebar-search {
		padding: 12px;
		border-bottom: 1px solid var(--border);
	}

	.channel-list {
		flex: 1;
		overflow-y: auto;
		padding: 8px;
	}

	.channel-row {
		display: flex;
		align-items: center;
		padding: 8px 12px;
		border-radius: 4px;
		gap: 12px;
		font-size: 13px;

		&:hover {
			background: rgba(255, 255, 255, 0.05);
		}

		.num {
			color: var(--accent);
			font-weight: 600;
			min-width: 32px;
		}

		.name {
			color: var(--text-bright);
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
		}
	}

	/* Form Elements */
	.section-card {
		background: var(--surface-bright);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 16px;
		margin-bottom: 16px;
	}

	.section-title {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 14px;
		font-weight: 600;
		color: var(--text-bright);
		margin-bottom: 16px;
		
		&.db-toggle {
			margin-bottom: 0;
			width: 100%;
			justify-content: space-between;
			background: transparent;
			border: none;
			padding: 0;
			cursor: pointer;

			&:hover {
				color: var(--accent);
			}
		}
	}

	.flex-align {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 6px;

		label, .field-label {
			font-size: 12px;
			font-weight: 500;
			color: var(--text-dim);
			display: flex;
			align-items: center;
			gap: 4px;
		}
	}

	.label-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.required {
		color: var(--accent);
	}

	.form-input, .form-select {
		width: 100%;
		background: var(--surface);
		border: 1px solid var(--border-bright);
		color: var(--text-bright);
		padding: 8px 12px;
		border-radius: 4px;
		font-family: inherit;
		font-size: 14px;

		&:focus {
			outline: none;
			border-color: var(--accent);
		}
	}

	.input-wrapper {
		position: relative;
	}

	.suggestions-dropdown {
		position: absolute;
		top: 100%;
		left: 0;
		right: 0;
		background: var(--surface-bright);
		border: 1px solid var(--border-bright);
		border-radius: 4px;
		margin-top: 4px;
		max-height: 200px;
		overflow-y: auto;
		z-index: 10;
		box-shadow: var(--shadow-lg);
	}

	.suggestion-item {
		width: 100%;
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px 12px;
		background: transparent;
		border: none;
		border-bottom: 1px solid var(--border);
		cursor: pointer;
		text-align: left;
		color: var(--text);

		&:hover {
			background: var(--accent-transparent);
		}

		.s-name { font-weight: 500; }
		.s-callsign { font-size: 12px; color: var(--text-dim); }
	}

	.channel-number-row {
		display: flex;
		gap: 12px;
		align-items: center;
	}

	.ch-num-input {
		width: 100px;
	}

	.ch-num-buttons {
		display: flex;
		gap: 8px;
	}

	.btn-pill {
		background: var(--surface);
		border: 1px solid var(--border-bright);
		color: var(--text-dim);
		padding: 4px 12px;
		border-radius: 12px;
		font-size: 11px;
		cursor: pointer;

		&:hover {
			background: rgba(255,255,255,0.1);
			color: var(--text-bright);
		}

		&.active {
			background: var(--accent);
			border-color: var(--accent);
			color: white;
		}
	}

	/* Groups */
	.btn-create {
		background: transparent;
		border: 1px dashed var(--border-bright);
		color: var(--text-dim);
		width: 100%;
		padding: 8px;
		border-radius: 4px;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		cursor: pointer;

		&:hover {
			color: var(--accent);
			border-color: var(--accent);
		}
	}

	.new-group-inline {
		display: flex;
		gap: 8px;
	}

	.btn-icon {
		background: var(--surface);
		border: 1px solid var(--border);
		color: var(--text);
		width: 36px;
		border-radius: 4px;
		cursor: pointer;

		&.confirm:hover { color: #4ade80; border-color: #4ade80; }
		&.cancel:hover { color: var(--accent); border-color: var(--accent); }
	}

	.group-list {
		max-height: 200px;
		overflow-y: auto;
		border: 1px solid var(--border-bright);
		border-radius: 4px;
		background: var(--surface);
	}

	.group-item {
		width: 100%;
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px 12px;
		background: transparent;
		border: none;
		border-bottom: 1px solid var(--border);
		cursor: pointer;
		text-align: left;
		color: var(--text);

		&:hover { background: rgba(255,255,255,0.05); }
		&.selected { background: var(--accent-transparent); border-left: 2px solid var(--accent); }

		.group-name { font-weight: 500; font-size: 13px; }
		.group-badges { display: flex; gap: 4px; }
		.badge {
			font-size: 10px;
			padding: 2px 6px;
			border-radius: 10px;
			&.custom { background: rgba(74, 222, 128, 0.1); color: #4ade80; }
			&.m3u { background: rgba(56, 189, 248, 0.1); color: #38bdf8; }
		}
	}

	/* Two Col */
	.two-col {
		display: flex;
		gap: 24px;
	}

	.sub-col {
		flex: 1;
		min-width: 0;
	}

	/* Shortcuts */
	.shortcut-link {
		background: transparent;
		border: none;
		color: var(--accent);
		font-size: 11px;
		cursor: pointer;

		&:hover { text-decoration: underline; }
		&.danger { color: #f87171; }
	}

	.actions {
		display: flex;
		gap: 8px;
	}

	/* Checkbox */
	.custom-checkbox {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 12px;
		color: var(--text-dim);
		cursor: pointer;

		input { display: none; }
		.checkmark {
			width: 16px;
			height: 16px;
			border: 1px solid var(--border-bright);
			border-radius: 4px;
			position: relative;
		}

		input:checked ~ .checkmark {
			background: var(--accent);
			border-color: var(--accent);
			&::after {
				content: '';
				position: absolute;
				left: 4px;
				top: 1px;
				width: 4px;
				height: 8px;
				border: solid white;
				border-width: 0 2px 2px 0;
				transform: rotate(45deg);
			}
		}
	}

	/* DB Browser */
	.db-browser {
		margin-top: 16px;
		padding-top: 16px;
		border-top: 1px solid var(--border);
	}

	.db-search-row {
		display: flex;
		gap: 8px;
		margin-bottom: 12px;
	}

	.btn-search {
		background: var(--surface);
		border: 1px solid var(--border-bright);
		color: var(--text);
		padding: 0 16px;
		border-radius: 4px;
		cursor: pointer;

		&:hover { background: var(--accent); color: white; border-color: var(--accent); }
	}

	.db-results {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: 8px;
		max-height: 200px;
		overflow-y: auto;
		margin-bottom: 12px;
	}

	.db-result-item {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 8px;
		background: var(--surface);
		border: 1px solid var(--border-bright);
		border-radius: 4px;
		cursor: pointer;
		text-align: left;

		&:hover { border-color: var(--text-dim); }
		&.active { border-color: var(--accent); background: var(--accent-transparent); }

		.db-result-logo {
			width: 32px;
			height: 32px;
			background: var(--surface-bright);
			border-radius: 4px;
			display: flex;
			align-items: center;
			justify-content: center;
			color: var(--text-dim);
			img { width: 100%; height: 100%; object-fit: contain; }
		}

		.db-result-info {
			display: flex;
			flex-direction: column;
			overflow: hidden;
			.name { font-size: 12px; font-weight: 500; color: var(--text-bright); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
			.call { font-size: 10px; color: var(--text-dim); }
		}
	}

	.db-match-apply {
		background: var(--surface);
		border: 1px solid var(--accent-transparent);
		border-radius: 4px;
		padding: 12px;

		.apply-header {
			display: flex;
			justify-content: space-between;
			align-items: center;
			margin-bottom: 8px;
			font-weight: 500;
			font-size: 13px;
			color: var(--accent);
		}

		.apply-buttons {
			display: flex;
			gap: 8px;
		}
	}

	.error-box {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 12px;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.2);
		color: #fca5a5;
		border-radius: 4px;
		font-size: 12px;
	}

	/* EPG Selector */
	.epg-selector {
		border: 1px solid var(--border-bright);
		border-radius: 4px;
		background: var(--surface);
		display: flex;
		flex-direction: column;
		height: 200px;
	}

	.epg-controls {
		padding: 8px;
		border-bottom: 1px solid var(--border);
	}

	.epg-list {
		flex: 1;
		overflow-y: auto;
	}

	.epg-item {
		width: 100%;
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 6px 12px;
		background: transparent;
		border: none;
		border-bottom: 1px solid var(--border);
		cursor: pointer;
		text-align: left;
		color: var(--text);
		font-size: 12px;

		&:hover { background: rgba(255,255,255,0.05); }
		&.selected { background: var(--accent-transparent); border-left: 2px solid var(--accent); }

		.tvgid { color: var(--text-dim); font-size: 10px; }
	}

	/* Logo Preview */
	.logo-preview {
		height: 80px;
		background: var(--surface);
		border: 1px dashed var(--border-bright);
		border-radius: 4px;
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;

		img { max-width: 100%; max-height: 100%; object-fit: contain; }
		.placeholder { color: var(--text-dim); display: flex; flex-direction: column; align-items: center; gap: 4px; font-size: 11px; }
	}

	.btn-upload {
		background: var(--surface);
		border: 1px solid var(--border-bright);
		color: var(--text);
		padding: 6px 12px;
		border-radius: 4px;
		font-size: 12px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;

		&:hover { border-color: var(--text-bright); }
	}

	/* Footer */
	.form-actions {
		display: flex;
		justify-content: flex-end;
		gap: 12px;
		padding-top: 16px;
		border-top: 1px solid var(--border);
	}

	.btn-cancel {
		background: transparent;
		border: none;
		color: var(--text-dim);
		cursor: pointer;
		padding: 8px 16px;

		&:hover { color: var(--text-bright); }
	}

	.btn-submit {
		background: var(--accent);
		border: none;
		color: white;
		padding: 8px 24px;
		border-radius: 4px;
		font-weight: 500;
		cursor: pointer;

		&:hover:not(:disabled) { background: var(--accent-dim); }
		&:disabled { opacity: 0.5; cursor: not-allowed; }
	}

	.empty-text {
		padding: 12px;
		text-align: center;
		color: var(--text-dim);
		font-size: 12px;
	}

	.mt-3 { margin-top: 16px; }
	.mb-2 { margin-bottom: 8px; }
</style>
