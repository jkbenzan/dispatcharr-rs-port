<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { api } from '$lib/api';
  import Modal from '$lib/components/ui/Modal.svelte';
  import VideoPlayer from '$lib/components/ui/VideoPlayer.svelte';
  import { Calendar, Clock, ArrowLeft, ArrowRight, RefreshCw, Info, Play } from 'lucide-svelte';
  
  // State
  let channels: any[] = $state([]);
  let programsByChannel: Record<string, any[]> = $state({});
  let limit = 50;
  let offset = $state(0);
  let totalChannels = $state(0);
  let loading = $state(true);
  let loadingMore = $state(false);
  let error = $state<string | null>(null);
  
  let observer: IntersectionObserver;
  let sentinel = $state<HTMLElement>();
  let gridViewport = $state<HTMLElement>();
  let channelsColumn = $state<HTMLElement>();

  // Time / Date Selection
  let now = new Date();
  let selectedDate = $state(now.toISOString().split('T')[0]); // YYYY-MM-DD
  let selectedTime = $state(now.getHours().toString().padStart(2, '0') + ':00'); // HH:00
  
  // Computed Time Window
  let timeStart = $derived.by(() => {
    const d = new Date(`${selectedDate}T${selectedTime}`);
    return new Date(d.getTime() - 1 * 60 * 60 * 1000); // Start 1 hour before selected
  });
  
  let timeEnd = $derived.by(() => {
    return new Date(timeStart.getTime() + 6 * 60 * 60 * 1000); // 6 hour window
  });

  // Real-time marker
  let currentTimestamp = $state(Date.now());
  let timeInterval: any;

  // Constants
  const PIXELS_PER_HOUR = 400; // Increased for better readability
  const MS_PER_HOUR = 3600000;
  
  $effect(() => {
    timeInterval = setInterval(() => {
      currentTimestamp = Date.now();
    }, 60000); // update every minute
    return () => clearInterval(timeInterval);
  });

  // Reactive positions
  let markerPosition = $derived(
    ((currentTimestamp - timeStart.getTime()) / MS_PER_HOUR) * PIXELS_PER_HOUR
  );

  let gridWidth = $derived(
    ((timeEnd.getTime() - timeStart.getTime()) / MS_PER_HOUR) * PIXELS_PER_HOUR
  );

  // Detail Modal State
  let showDetailsModal = $state(false);
  let selectedProgram = $state<any>(null);

  // Playback State
  let showPlayerModal = $state(false);
  let playingChannel = $state<any>(null);
  let streamUrl = $derived(playingChannel ? `/api/proxy/stream/${playingChannel.uuid}` : '');

  async function loadChannelsAndEpg(isLoadMore = false) {
    if (isLoadMore) loadingMore = true;
    else {
      loading = true;
      error = null;
      if (!isLoadMore) {
        offset = 0;
        channels = [];
      }
    }

    try {
      // Fix: Call getChannels with the correct object signature
      const channelRes = await api.getChannels({ 
        limit, 
        offset, 
        is_active: true,
        sort_by: 'channel_number',
        order: 'asc'
      });

      if (channelRes && channelRes.results) {
        if (!isLoadMore) {
          channels = channelRes.results;
          totalChannels = channelRes.count;
        } else {
          channels = [...channels, ...channelRes.results];
        }

        const uuids = channelRes.results.map((c: any) => c.uuid);
        if (uuids.length > 0) {
          // Fetch EPG grid for the selected time window
          const epgRes = await api.getEpgGrid(timeStart.toISOString(), timeEnd.toISOString(), uuids);
          const programs = epgRes.data || [];
          
          let newProgramsByChannel = isLoadMore ? { ...programsByChannel } : {};
          
          programs.forEach((p: any) => {
            const uuid = p.channel_uuid;
            if (!newProgramsByChannel[uuid]) newProgramsByChannel[uuid] = [];
            
            // Calculate pixel dimensions for the grid
            const pStart = new Date(p.start_time).getTime();
            const pEnd = new Date(p.end_time).getTime();
            
            // Constrain positions to the visible grid window
            const startPx = ((pStart - timeStart.getTime()) / MS_PER_HOUR) * PIXELS_PER_HOUR;
            const endPx = ((pEnd - timeStart.getTime()) / MS_PER_HOUR) * PIXELS_PER_HOUR;
            const widthPx = endPx - startPx;
            
            // Only add if it overlaps the visible window
            if (endPx > 0 && startPx < gridWidth) {
              newProgramsByChannel[uuid].push({
                ...p,
                startPx,
                widthPx: Math.max(1, widthPx) // ensure at least 1px width
              });
            }
          });
          
          programsByChannel = newProgramsByChannel;
        }
      }
    } catch (e: any) {
      console.error("Failed to load EPG grid", e);
      error = e.message || "Failed to load TV Guide. Please try again.";
    } finally {
      loading = false;
      loadingMore = false;
    }
  }

  // Scroll Synchronization
  function handleGridScroll(e: Event) {
    if (!gridViewport || !channelsColumn) return;
    channelsColumn.scrollTop = gridViewport.scrollTop;
  }

  function jumpToNow() {
    if (!gridViewport) return;
    
    // Reset date/time to now if we are looking at a different day
    const currentDayStr = new Date().toISOString().split('T')[0];
    if (selectedDate !== currentDayStr) {
      selectedDate = currentDayStr;
      selectedTime = new Date().getHours().toString().padStart(2, '0') + ':00';
      // loadChannelsAndEpg will be triggered by $effect
    }

    // Scroll horizontal to marker position
    const viewportWidth = gridViewport.clientWidth;
    gridViewport.scrollTo({
      left: markerPosition - (viewportWidth / 3), // Center marker roughly at 1/3 of view
      behavior: 'smooth'
    });
  }

  function openProgramDetails(program: any, channel: any) {
    selectedProgram = { ...program, channelName: channel.name, channelLogo: channel.logo_url };
    showDetailsModal = true;
  }

  function playChannel(channel: any) {
    playingChannel = channel;
    showPlayerModal = true;
  }

  // Effect to reload when date/time changes
  $effect(() => {
    if (selectedDate && selectedTime) {
      loadChannelsAndEpg();
    }
  });

  onMount(() => {
    // Initial load handled by $effect
    
    observer = new IntersectionObserver(entries => {
      if (entries[0].isIntersecting && !loading && !loadingMore && channels.length < totalChannels) {
        offset += limit;
        loadChannelsAndEpg(true);
      }
    }, { rootMargin: '400px' });
    
    if (sentinel) observer.observe(sentinel);
  });
  
  onDestroy(() => {
    if (observer) observer.disconnect();
  });

  // Time header generation
  let timeHeaders = $derived.by(() => {
    const headers = [];
    let t = new Date(timeStart.getTime());
    // Align to nearest half hour
    t.setMinutes(t.getMinutes() >= 30 ? 30 : 0, 0, 0);
    
    while (t.getTime() < timeEnd.getTime()) {
      const pos = ((t.getTime() - timeStart.getTime()) / MS_PER_HOUR) * PIXELS_PER_HOUR;
      headers.push({
        time: t.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }),
        pos
      });
      t = new Date(t.getTime() + 30 * 60000); // +30 mins
    }
    return headers;
  });

  function formatProgramTime(iso: string) {
    return new Date(iso).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
  }
</script>

<div class="page-container">
	<header class="page-header">
		<div>
			<h1>TV Guide</h1>
			<p class="subtitle">Live schedule and programming blocks.</p>
		</div>

    <div class="controls">
      <div class="date-selector">
        <Calendar size={18} class="text-dim" />
        <input type="date" bind:value={selectedDate} />
      </div>

      <div class="time-selector">
        <Clock size={18} class="text-dim" />
        <select bind:value={selectedTime}>
          {#each Array(24) as _, i}
            <option value="{i.toString().padStart(2, '0')}:00">
              {i === 0 ? '12 AM' : i < 12 ? `${i} AM` : i === 12 ? '12 PM' : `${i - 12} PM`}
            </option>
          {/each}
        </select>
      </div>

      <button class="btn btn-secondary" onclick={jumpToNow}>
        Jump to Now
      </button>

      <button class="btn btn-icon" onclick={() => loadChannelsAndEpg()} title="Refresh">
        <RefreshCw size={18} class={loading ? 'spinning' : ''} />
      </button>
    </div>
	</header>

	<main class="content-area" class:has-error={!!error}>
    {#if error}
      <div class="error-state">
        <Info size={48} />
        <h2>Guide Unavailable</h2>
        <p>{error}</p>
        <button class="btn btn-primary" onclick={() => loadChannelsAndEpg()}>Retry</button>
      </div>
		{:else if loading && channels.length === 0}
			<div class="loading-state">
        <div class="spinner"></div>
        <span>Loading TV Guide...</span>
      </div>
		{:else}
      <div class="epg-container">
        <!-- Left Axis (Channels) -->
        <div class="channels-column" bind:this={channelsColumn}>
          <div class="channel-header-spacer"></div>
          {#each channels as channel}
            <div class="channel-cell" role="button" tabindex="0" onclick={() => playChannel(channel)} onkeydown={(e) => e.key === 'Enter' && playChannel(channel)}>
              <div class="logo-wrapper">
                {#if channel.logo_url}
                  <img src={channel.logo_url} alt={channel.name} class="channel-logo" onerror={(e) => (e.currentTarget as HTMLImageElement).style.display='none'} />
                {:else}
                  <div class="logo-placeholder">
                    <Play size={20} fill="currentColor" />
                  </div>
                {/if}
              </div>
              <div class="channel-info">
                <span class="channel-number">{channel.channel_number || '--'}</span>
                <span class="channel-name">{channel.name}</span>
              </div>
            </div>
          {/each}
          <div bind:this={sentinel} class="sentinel"></div>
        </div>
        
        <!-- Grid Area (Scrollable) -->
        <div class="grid-viewport" bind:this={gridViewport} onscroll={handleGridScroll}>
          <div class="grid-inner" style:width="{gridWidth}px">
            
            <!-- Time Headers -->
            <div class="time-header-row">
              {#each timeHeaders as header}
                <div class="time-header-tick" style:left="{header.pos}px">
                  {header.time}
                </div>
              {/each}
            </div>
            
            <!-- Current Time Marker (Only visible if within window) -->
            {#if markerPosition >= 0 && markerPosition <= gridWidth}
              <div class="time-marker" style:left="{markerPosition}px"></div>
            {/if}

            <!-- Program Rows -->
            <div class="programs-layer">
              {#each channels as channel}
                <div class="program-row">
                  {#if programsByChannel[channel.uuid] && programsByChannel[channel.uuid].length > 0}
                    {#each programsByChannel[channel.uuid] as program}
                      <button class="program-block" 
                           class:is-live={program.is_live}
                           class:is-new={program.is_new}
                           style:left="{program.startPx}px" 
                           style:width="{program.widthPx}px"
                           onclick={() => openProgramDetails(program, channel)}>
                        <div class="program-content">
                          <span class="program-title">{program.title}</span>
                          {#if program.sub_title}
                            <span class="program-subtitle">{program.sub_title}</span>
                          {/if}
                        </div>
                      </button>
                    {/each}
                  {:else}
                    <div class="no-data">No EPG Data</div>
                  {/if}
                </div>
              {/each}
            </div>

            {#if loadingMore}
              <div class="loading-more-overlay">
                <div class="spinner small"></div>
                <span>Loading channels...</span>
              </div>
            {/if}
            
          </div>
        </div>
      </div>
		{/if}
	</main>
</div>

<!-- Details Modal -->
<Modal bind:show={showDetailsModal} title="Program Details" width="500px">
  {#if selectedProgram}
    <div class="program-details">
      <div class="details-header">
        <div class="channel-mini">
          {#if selectedProgram.channelLogo}
            <img src={selectedProgram.channelLogo} alt={selectedProgram.channelName} />
          {/if}
          <span>{selectedProgram.channelName}</span>
        </div>
        <div class="time-range">
          {formatProgramTime(selectedProgram.start_time)} - {formatProgramTime(selectedProgram.end_time)}
        </div>
      </div>

      <h1 class="details-title">{selectedProgram.title}</h1>
      {#if selectedProgram.sub_title}
        <h2 class="details-subtitle">{selectedProgram.sub_title}</h2>
      {/if}

      <div class="badges">
        {#if selectedProgram.is_new}
          <span class="badge new">NEW</span>
        {/if}
        {#if selectedProgram.is_live}
          <span class="badge live">LIVE</span>
        {/if}
        {#if selectedProgram.rating}
          <span class="badge rating">{selectedProgram.rating}</span>
        {/if}
        {#if selectedProgram.category}
          <span class="badge category">{selectedProgram.category}</span>
        {/if}
      </div>

      <p class="details-description">
        {selectedProgram.description || 'No description available for this program.'}
      </p>

      <div class="details-footer">
        <button class="btn btn-primary" onclick={() => showDetailsModal = false}>Close</button>
      </div>
    </div>
  {/if}
</Modal>

<!-- Player Modal -->
<Modal bind:show={showPlayerModal} title={playingChannel?.name || 'Live Stream'} width="900px">
  <div class="player-container">
    {#if showPlayerModal && streamUrl}
      <VideoPlayer 
        src={streamUrl} 
        title={playingChannel?.name}
        autoplay={true}
      />
    {/if}
  </div>
</Modal>

<style lang="less">
	.page-container {
		display: flex;
		flex-direction: column;
		gap: 16px;
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
			margin: 0;
		}

		.subtitle {
			color: var(--text-dim);
			margin: 0;
			font-size: 13px;
		}
	}

  .controls {
    display: flex;
    align-items: center;
    gap: 12px;

    .date-selector, .time-selector {
      display: flex;
      align-items: center;
      gap: 8px;
      background: var(--surface-bright);
      border: 1px solid var(--border);
      padding: 6px 12px;
      border-radius: var(--radius);

    }

    .date-selector input,
    .time-selector select {
      background: transparent;
      border: none;
      color: var(--text-bright);
      font-size: 13px;
      font-weight: 500;
      outline: none;
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
    position: relative;

    &.has-error {
      display: flex;
      align-items: center;
      justify-content: center;
    }
	}

  .error-state {
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    color: var(--text-dim);

    h2 {
      color: var(--text-bright);
      margin: 0;
    }
  }

	.loading-state {
		margin: auto;
		color: var(--text-dim);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
	}

  .epg-container {
    display: flex;
    flex: 1;
    overflow: hidden;
    position: relative;
    background: rgba(0, 0, 0, 0.1);
  }

  .channels-column {
    width: 240px;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    background: var(--surface);
    z-index: 10;
    overflow-y: hidden; /* Synced with grid */
    box-shadow: 4px 0 12px rgba(0,0,0,0.3);
  }

  .channel-header-spacer {
    height: 48px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-bright);
  }

  .channel-cell {
    height: 64px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    padding: 0 16px;
    gap: 12px;
    background: var(--surface);
    cursor: pointer;
    transition: background 0.2s;

    &:hover {
      background: var(--surface-bright);
      
      .channel-name {
        color: var(--accent);
      }
    }

    .logo-wrapper {
      width: 44px;
      height: 44px;
      display: flex;
      align-items: center;
      justify-content: center;
      background: rgba(255,255,255,0.03);
      border-radius: 6px;
      overflow: hidden;
      flex-shrink: 0;
    }

    .logo-placeholder {
      color: rgba(255, 255, 255, 0.1);
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .channel-logo {
      max-width: 100%;
      max-height: 100%;
      object-fit: contain;
    }

    .channel-info {
      display: flex;
      flex-direction: column;
      overflow: hidden;

      .channel-number {
        font-size: 10px;
        color: var(--accent);
        font-weight: 800;
        letter-spacing: 0.5px;
      }

      .channel-name {
        font-size: 14px;
        color: var(--text-bright);
        font-weight: 600;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }
    }
  }

  .grid-viewport {
    flex: 1;
    overflow: auto;
    position: relative;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }

  .grid-inner {
    position: relative;
    min-height: 100%;
  }

  .time-header-row {
    height: 48px;
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
    background: var(--surface-bright);
    z-index: 5;
    box-shadow: 0 4px 8px rgba(0,0,0,0.1);
  }

  .time-header-tick {
    position: absolute;
    top: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    padding-left: 12px;
    font-size: 12px;
    color: var(--text-dim);
    font-weight: 700;
    border-left: 1px solid var(--border);
    background: linear-gradient(to bottom, transparent, rgba(255,255,255,0.01));
  }

  .time-marker {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--accent);
    z-index: 6;
    box-shadow: 0 0 12px var(--accent);

    &::after {
      content: '';
      position: absolute;
      top: 48px;
      left: 50%;
      transform: translateX(-50%);
      width: 10px;
      height: 10px;
      border-radius: 50%;
      background: var(--accent);
      border: 2px solid var(--surface);
    }
  }

  .program-row {
    height: 64px;
    border-bottom: 1px solid var(--border);
    position: relative;
    background: repeating-linear-gradient(90deg, transparent, transparent 199px, rgba(255,255,255,0.02) 200px);
  }

  .program-block {
    position: absolute;
    top: 6px;
    bottom: 6px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    padding: 4px 12px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    overflow: hidden;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease;

    &:hover {
      background: rgba(255, 255, 255, 0.1);
      border-color: var(--accent-transparent);
      transform: translateY(-1px);
      box-shadow: 0 4px 12px rgba(0,0,0,0.2);
      z-index: 2;
    }

    &.is-live {
      background: rgba(var(--accent-rgb, 21, 143, 118), 0.15);
      border-left: 3px solid var(--accent);
    }

    &.is-new {
      border-left: 3px solid #10b981;
    }

    .program-content {
      display: flex;
      flex-direction: column;
      gap: 1px;
      overflow: hidden;
    }

    .program-title {
      font-size: 13px;
      color: var(--text-bright);
      font-weight: 700;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .program-subtitle {
      font-size: 11px;
      color: var(--text-dim);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
  }

  .no-data {
    height: 100%;
    display: flex;
    align-items: center;
    padding-left: 16px;
    font-size: 12px;
    color: var(--text-dim);
    font-style: italic;
    opacity: 0.3;
  }

  .sentinel {
    height: 100px;
  }

  .loading-more-overlay {
    position: sticky;
    left: 0;
    bottom: 0;
    right: 0;
    padding: 16px;
    background: linear-gradient(to top, var(--surface) 50%, transparent);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--text-dim);
    font-size: 13px;
    z-index: 10;
  }

  /* Spinner */
  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 1s linear infinite;

    &.small {
      width: 16px;
      height: 16px;
      border-width: 2px;
    }
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  :global(.spinning) {
    animation: spin 1s linear infinite;
  }

  /* Program Details Modal Content */
  .program-details {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;

    .details-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      
      .channel-mini {
        display: flex;
        align-items: center;
        gap: 8px;
        color: var(--text-dim);
        font-weight: 600;
        font-size: 14px;

        img {
          height: 24px;
          object-fit: contain;
        }
      }

      .time-range {
        font-size: 14px;
        color: var(--accent);
        font-weight: 700;
      }
    }

    .details-title {
      font-size: 24px;
      font-weight: 800;
      color: var(--text-bright);
      margin: 0;
      line-height: 1.2;
    }

    .details-subtitle {
      font-size: 16px;
      font-weight: 600;
      color: var(--text-dim);
      margin: -8px 0 0 0;
    }

    .badges {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;

      .badge {
        padding: 4px 10px;
        border-radius: 4px;
        font-size: 11px;
        font-weight: 800;
        letter-spacing: 0.5px;

        &.new { background: #10b981; color: white; }
        &.live { background: #ef4444; color: white; }
        &.rating { background: rgba(255,255,255,0.1); color: var(--text-bright); }
        &.category { background: var(--accent-transparent); color: var(--accent); }
      }
    }

    .details-description {
      font-size: 15px;
      line-height: 1.6;
      color: var(--text-dim);
      margin: 8px 0;
    }

    .details-footer {
      margin-top: 16px;
      display: flex;
      justify-content: flex-end;
    }
  }

  .player-container {
    background: #000;
    border-radius: 8px;
    overflow: hidden;
    box-shadow: 0 10px 30px rgba(0,0,0,0.5);
    aspect-ratio: 16 / 9;
  }
</style>
