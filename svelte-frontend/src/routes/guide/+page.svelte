<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { api } from '$lib/api';
  
  // State
  let channels: any[] = $state([]);
  let programsByChannel: Record<string, any[]> = $state({});
  let limit = 50;
  let offset = $state(0);
  let totalChannels = $state(0);
  let loading = $state(true);
  let loadingMore = $state(false);
  let observer: IntersectionObserver;
  let sentinel: HTMLElement;

  let now = new Date();
  let timeStart = new Date(now.getTime() - 1 * 60 * 60 * 1000);
  let timeEnd = new Date(now.getTime() + 4 * 60 * 60 * 1000);
  
  // Real-time marker
  let currentTimestamp = $state(Date.now());
  let timeInterval: any;

  // Constants
  const PIXELS_PER_HOUR = 300;
  const MS_PER_HOUR = 3600000;
  
  $effect(() => {
    timeInterval = setInterval(() => {
      currentTimestamp = Date.now();
    }, 60000); // update every minute
    return () => clearInterval(timeInterval);
  });

  let markerPosition = $derived(
    Math.max(0, ((currentTimestamp - timeStart.getTime()) / MS_PER_HOUR) * PIXELS_PER_HOUR)
  );

  let gridWidth = $derived(
    ((timeEnd.getTime() - timeStart.getTime()) / MS_PER_HOUR) * PIXELS_PER_HOUR
  );

  async function loadChannelsAndEpg(isLoadMore = false) {
    if (isLoadMore) loadingMore = true;
    else loading = true;

    try {
      const channelRes = await api.getChannels(limit, offset, '', '', '', true); // assuming true for is_active, check if signature matches
      if (channelRes && channelRes.results) {
        if (!isLoadMore) {
          channels = channelRes.results;
          totalChannels = channelRes.count;
        } else {
          channels = [...channels, ...channelRes.results];
        }

        const uuids = channelRes.results.map((c: any) => c.uuid);
        if (uuids.length > 0) {
          const epgRes = await api.getEpgGrid(timeStart.toISOString(), timeEnd.toISOString(), uuids);
          const programs = epgRes.data || [];
          
          let newProgramsByChannel = isLoadMore ? { ...programsByChannel } : {};
          
          programs.forEach((p: any) => {
            const uuid = p.channel_uuid;
            if (!newProgramsByChannel[uuid]) newProgramsByChannel[uuid] = [];
            
            // Calculate pixel dimensions
            const pStart = new Date(p.start_time).getTime();
            const pEnd = new Date(p.end_time).getTime();
            
            const startPx = Math.max(0, ((pStart - timeStart.getTime()) / MS_PER_HOUR) * PIXELS_PER_HOUR);
            const endPx = Math.min(gridWidth, ((pEnd - timeStart.getTime()) / MS_PER_HOUR) * PIXELS_PER_HOUR);
            const widthPx = Math.max(0, endPx - startPx);
            
            newProgramsByChannel[uuid].push({
              ...p,
              startPx,
              widthPx
            });
          });
          
          programsByChannel = newProgramsByChannel;
        }
      }
    } catch (e) {
      console.error("Failed to load EPG grid", e);
    } finally {
      loading = false;
      loadingMore = false;
    }
  }

  onMount(() => {
    loadChannelsAndEpg();
    
    observer = new IntersectionObserver(entries => {
      if (entries[0].isIntersecting && !loading && !loadingMore && channels.length < totalChannels) {
        offset += limit;
        loadChannelsAndEpg(true);
      }
    }, { rootMargin: '200px' });
    
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
      if (pos >= 0) {
        headers.push({
          time: t.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }),
          pos
        });
      }
      t = new Date(t.getTime() + 30 * 60000); // +30 mins
    }
    return headers;
  });
</script>

<div class="page-container">
	<header class="page-header">
		<div>
			<h1>TV Guide</h1>
			<p class="subtitle">Live schedule and programming blocks.</p>
		</div>
	</header>

	<main class="content-area">
		{#if loading && channels.length === 0}
			<div class="loading-state">Loading TV Guide...</div>
		{:else}
      <div class="epg-container">
        <!-- Left Axis (Channels) -->
        <div class="channels-column">
          <div class="channel-header-spacer"></div>
          {#each channels as channel}
            <div class="channel-cell">
              {#if channel.logo_url}
                <img src={channel.logo_url} alt={channel.name} class="channel-logo" onerror={(e) => (e.currentTarget as HTMLImageElement).style.display='none'} />
              {/if}
              <div class="channel-info">
                <span class="channel-number">{channel.channel_number}</span>
                <span class="channel-name">{channel.name}</span>
              </div>
            </div>
          {/each}
          <div bind:this={sentinel} class="sentinel">
            {#if loadingMore}
              <span class="loading-more">Loading...</span>
            {/if}
          </div>
        </div>
        
        <!-- Grid Area (Scrollable) -->
        <div class="grid-viewport">
          <div class="grid-inner" style:width="{gridWidth}px">
            
            <!-- Time Headers -->
            <div class="time-header-row">
              {#each timeHeaders as header}
                <div class="time-header-tick" style:left="{header.pos}px">
                  {header.time}
                </div>
              {/each}
            </div>
            
            <!-- Current Time Marker -->
            <div class="time-marker" style:left="{markerPosition}px"></div>

            <!-- Program Rows -->
            <div class="programs-layer">
              {#each channels as channel}
                <div class="program-row">
                  {#if programsByChannel[channel.uuid] && programsByChannel[channel.uuid].length > 0}
                    {#each programsByChannel[channel.uuid] as program}
                      <div class="program-block" 
                           class:is-live={program.is_live}
                           class:is-new={program.is_new}
                           style:left="{program.startPx}px" 
                           style:width="{program.widthPx}px"
                           title="{program.title} - {program.description}">
                        <div class="program-content">
                          <span class="program-title">{program.title}</span>
                          {#if program.sub_title}
                            <span class="program-subtitle">{program.sub_title}</span>
                          {/if}
                        </div>
                      </div>
                    {/each}
                  {:else}
                    <div class="no-data">No EPG Data</div>
                  {/if}
                </div>
              {/each}
            </div>
            
          </div>
        </div>
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

  .epg-container {
    display: flex;
    flex: 1;
    overflow: hidden;
    position: relative;
    background: rgba(0, 0, 0, 0.2);
  }

  .channels-column {
    width: 240px;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    background: var(--surface);
    z-index: 10;
    overflow-y: hidden; /* Syncs via scroll event or kept hidden if synced by grid */
    box-shadow: 2px 0 8px rgba(0,0,0,0.2);
  }

  .channel-header-spacer {
    height: 40px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  .channel-cell {
    height: 60px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    padding: 0 16px;
    gap: 12px;
    background: var(--surface);

    .channel-logo {
      width: 40px;
      height: 40px;
      object-fit: contain;
      background: rgba(255,255,255,0.05);
      border-radius: 4px;
    }

    .channel-info {
      display: flex;
      flex-direction: column;
      overflow: hidden;

      .channel-number {
        font-size: 11px;
        color: var(--accent);
        font-weight: 700;
      }

      .channel-name {
        font-size: 13px;
        color: var(--text-bright);
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
  }

  .grid-inner {
    position: relative;
    min-height: 100%;
  }

  .time-header-row {
    height: 40px;
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
    background: var(--surface);
    z-index: 5;
  }

  .time-header-tick {
    position: absolute;
    top: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    padding-left: 8px;
    font-size: 12px;
    color: var(--text-dim);
    font-weight: 600;
    border-left: 1px solid var(--border);
  }

  .time-marker {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--accent);
    z-index: 6;
    box-shadow: 0 0 8px var(--accent);

    &::after {
      content: '';
      position: absolute;
      top: 40px; /* Below header */
      left: 50%;
      transform: translateX(-50%);
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--accent);
    }
  }

  .programs-layer {
    position: relative;
  }

  .program-row {
    height: 60px;
    border-bottom: 1px solid var(--border);
    position: relative;
  }

  .program-block {
    position: absolute;
    top: 4px;
    bottom: 4px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    padding: 6px 12px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    overflow: hidden;
    cursor: pointer;
    transition: all 0.2s;

    &:hover {
      background: rgba(255, 255, 255, 0.1);
      border-color: rgba(255, 255, 255, 0.2);
    }

    &.is-live {
      border-left: 3px solid var(--accent);
    }

    &.is-new {
      border-left: 3px solid #10b981; /* Green for new */
    }

    .program-content {
      display: flex;
      flex-direction: column;
      gap: 2px;
      overflow: hidden;
    }

    .program-title {
      font-size: 13px;
      color: var(--text-bright);
      font-weight: 600;
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
    opacity: 0.5;
  }

  .sentinel {
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    
    .loading-more {
      font-size: 12px;
      color: var(--text-dim);
    }
  }
</style>
