<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { api } from '$lib/api';
  import { Search, Film, Tv, Star, Info, Play } from 'lucide-svelte';
  import VodDetailModal from '$lib/components/VodDetailModal.svelte';
  import VideoPlayer from '$lib/components/ui/VideoPlayer.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';

  // State
  let activeTab: 'movies' | 'series' = $state('movies');
  let categories: any[] = $state([]);
  let activeCategoryId: number | null = $state(null);
  
  let items: any[] = $state([]);
  let totalItems = $state(0);
  let limit = 24;
  let offset = $state(0);
  let loading = $state(true);
  let loadingMore = $state(false);
  let searchQuery = $state('');
  
  let selectedVod = $state<any>(null);
  let showVodModal = $state(false);
  
  // Video Player state
  let showPlayer = $state(false);
  let playerUrl = $state('');
  let playerTitle = $state('');
  let playerItem = $state<any>(null);
  
  let observer: IntersectionObserver;
  let sentinel = $state<HTMLElement>();

  async function loadCategories() {
    try {
      const res = await api.getVodCategories();
      if (res && res.results) {
        categories = res.results.filter((c: any) => c.category_type === activeTab || c.category_type === 'all');
      }
    } catch (e) {
      console.error("Failed to load categories", e);
    }
  }

  async function loadItems(isLoadMore = false) {
    if (isLoadMore) loadingMore = true;
    else {
      loading = true;
      if (!isLoadMore) {
        offset = 0;
        items = [];
      }
    }

    try {
      let res;
      if (activeTab === 'movies') {
        res = await api.getVodMovies(limit, offset, searchQuery, activeCategoryId);
      } else {
        res = await api.getVodSeries(limit, offset, searchQuery, activeCategoryId);
      }
      
      if (res && res.results) {
        let fetchedItems = res.results;
        
        if (!isLoadMore) {
          items = fetchedItems;
          totalItems = res.count;
        } else {
          items = [...items, ...fetchedItems];
        }
      }
    } catch (e) {
      console.error(`Failed to load ${activeTab}`, e);
    } finally {
      loading = false;
      loadingMore = false;
    }
  }

  function handleTabChange(tab: 'movies' | 'series') {
    activeTab = tab;
    activeCategoryId = null;
    loadCategories();
    loadItems();
  }

  function handleSearch() {
    offset = 0;
    loadItems();
  }

  let searchTimeout: any;
  function onSearchInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    searchQuery = val;
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => {
      handleSearch();
    }, 500);
  }

  onMount(() => {
    loadCategories();
    loadItems();
    
    observer = new IntersectionObserver(entries => {
      if (entries[0].isIntersecting && !loading && !loadingMore && items.length < totalItems) {
        offset += limit;
        loadItems(true);
      }
    }, { rootMargin: '200px' });
    
    if (sentinel) observer.observe(sentinel);
  });

  onDestroy(() => {
    if (observer) observer.disconnect();
  });

  function normalizePosterUrl(value: any) {
    if (typeof value !== 'string' || value.trim() === '') return null;
    const trimmed = value.trim();
    if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) return trimmed;
    if (trimmed.startsWith('/')) return `https://image.tmdb.org/t/p/w500${trimmed}`;
    return null;
  }

  // Helper to get a real poster URL from provider or metadata fields.
  function getImageUrl(item: any) {
    const directPoster = normalizePosterUrl(item.poster_url || item.poster_path || item.cover || item.stream_icon);
    if (directPoster) return directPoster;

    if (item.custom_properties) {
      let props;
      try {
        props = typeof item.custom_properties === 'string' ? JSON.parse(item.custom_properties) : item.custom_properties;
        const poster = normalizePosterUrl(
          props.stream_icon || props.cover || props.poster_url || props.poster_path || props.movie_image || props.image
        );
        if (poster) return poster;
      } catch (e) {}
    }

    return null; // Fallback to placeholder
  }

  function openVodDetails(item: any) {
      selectedVod = item;
      selectedVod.poster_url = getImageUrl(item);
      showVodModal = true;
  }

  function handlePlay(event: any) {
      const { type, id, stream_id, m3u_account_id } = event.detail;
      
      // Construct VOD proxy URL
      playerUrl = `/proxy/vod/${type}/${id}?stream_id=${encodeURIComponent(stream_id || '')}&m3u_account_id=${m3u_account_id || ''}`;
      playerTitle = selectedVod?.name || 'VOD Stream';
      playerItem = selectedVod;
      
      showVodModal = false;
      showPlayer = true;
  }
</script>

<div class="page-container">
	<header class="page-header">
		<div>
			<h1>Video On Demand</h1>
			<p class="subtitle">Browse movies and series from your providers.</p>
		</div>
		
		<div class="header-actions">
			<div class="search-box">
				<Search size={18} class="search-icon" />
				<input type="text" placeholder="Search..." value={searchQuery} oninput={onSearchInput} />
			</div>
		</div>
	</header>

	<main class="content-area">
    <!-- Tabs -->
    <div class="tabs-container">
      <button class="tab" class:active={activeTab === 'movies'} onclick={() => handleTabChange('movies')}>
        <Film size={18} />
        Movies
      </button>
      <button class="tab" class:active={activeTab === 'series'} onclick={() => handleTabChange('series')}>
        <Tv size={18} />
        Series
      </button>
    </div>

    <!-- Horizontal Categories (Trakt-style) -->
    <div class="categories-scroll">
      <button 
        class="category-pill" 
        class:active={activeCategoryId === null}
        onclick={() => { activeCategoryId = null; offset = 0; loadItems(); }}>
        All
      </button>
      {#each categories as cat}
        <button 
          class="category-pill" 
          class:active={activeCategoryId === cat.id}
          onclick={() => { activeCategoryId = cat.id; offset = 0; loadItems(); }}>
          {cat.name}
        </button>
      {/each}
    </div>

    <!-- Grid -->
		<div class="grid-container">
      {#if loading && items.length === 0}
        <div class="empty-state">
          <div class="spinner"></div>
          <p>Loading {activeTab}...</p>
        </div>
      {:else if items.length === 0}
        <div class="empty-state">
          <Info size={48} class="text-dim" />
          <p>No {activeTab} found.</p>
        </div>
      {:else}
        <div class="poster-grid">
          {#each items as item}
            <div class="poster-card" onclick={() => openVodDetails(item)}>
              <div class="poster-image-container">
                {#if getImageUrl(item)}
                  <img src={getImageUrl(item)} alt={item.name} class="poster-img" onerror={(e) => (e.currentTarget as HTMLImageElement).style.display='none'} />
                {:else}
                  <div class="poster-placeholder">
                    {#if activeTab === 'movies'}
                      <Film size={32} />
                    {:else}
                      <Tv size={32} />
                    {/if}
                  </div>
                {/if}
                
                <div class="poster-overlay">
                  <div class="overlay-content">
                    <span class="overlay-title">{item.name}</span>
                    <span class="overlay-desc">{item.description || 'No description available.'}</span>
                  </div>
                </div>
                
                {#if item.rating}
                  <div class="rating-badge">
                    <Star size={12} fill="currentColor" />
                    <span>{item.rating}</span>
                  </div>
                {/if}
              </div>
              <div class="poster-info">
                <h3 class="title" title={item.name}>{item.name}</h3>
                <span class="year">{item.year || 'Unknown'}</span>
              </div>
            </div>
          {/each}
        </div>
        
        <div bind:this={sentinel} class="sentinel">
          {#if loadingMore}
            <div class="spinner small"></div>
            <span>Loading more...</span>
          {/if}
        </div>
      {/if}
		</div>
	</main>
</div>

<VodDetailModal 
  show={showVodModal} 
  vod={selectedVod} 
  type={activeTab === 'movies' ? 'movie' : 'series'}
  on:close={() => showVodModal = false}
  on:play={handlePlay}
/>

<Modal bind:show={showPlayer} title={playerTitle} width="900px">
  <div class="player-container">
    {#if showPlayer && playerUrl}
      <VideoPlayer 
        src={playerUrl} 
        title={playerTitle}
        autoplay={true}
      />
    {/if}
  </div>
</Modal>

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

  .header-actions {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .search-box {
    position: relative;
    display: flex;
    align-items: center;

    :global(.search-icon) {
      position: absolute;
      left: 12px;
      color: var(--text-dim);
    }

    input {
      background: var(--surface);
      border: 1px solid var(--border);
      color: var(--text-bright);
      padding: 10px 12px 10px 40px;
      border-radius: var(--radius);
      width: 250px;
      font-size: 14px;
      transition: all 0.2s;

      &:focus {
        outline: none;
        border-color: var(--accent);
        box-shadow: 0 0 0 2px var(--accent-transparent);
      }
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

  .tabs-container {
    display: flex;
    padding: 16px 24px 0;
    gap: 24px;
    border-bottom: 1px solid var(--border);
  }

  .tab {
    background: none;
    border: none;
    padding: 12px 0;
    color: var(--text-dim);
    font-size: 15px;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    position: relative;
    transition: color 0.2s;

    &:hover {
      color: var(--text-bright);
    }

    &.active {
      color: var(--accent);

      &::after {
        content: '';
        position: absolute;
        bottom: -1px;
        left: 0;
        right: 0;
        height: 2px;
        background: var(--accent);
        border-top-left-radius: 2px;
        border-top-right-radius: 2px;
      }
    }
  }

  .categories-scroll {
    display: flex;
    gap: 12px;
    padding: 16px 24px;
    overflow-x: auto;
    border-bottom: 1px solid var(--border);
    scrollbar-width: none; // Firefox
    
    &::-webkit-scrollbar {
      display: none; // Chrome/Safari
    }
  }

  .category-pill {
    white-space: nowrap;
    padding: 6px 16px;
    border-radius: 20px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border);
    color: var(--text-dim);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;

    &:hover {
      background: rgba(255, 255, 255, 0.1);
      color: var(--text-bright);
    }

    &.active {
      background: var(--accent);
      color: white;
      border-color: var(--accent);
    }
  }

  .grid-container {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
  }

  .empty-state {
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    color: var(--text-dim);
    
    :global(.text-dim) {
      opacity: 0.5;
    }
  }

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

  .poster-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 24px;
  }

  .poster-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    cursor: pointer;
    
    &:hover .poster-overlay {
      opacity: 1;
    }

    &:hover .poster-img {
      transform: scale(1.05);
    }
  }

  .poster-image-container {
    aspect-ratio: 2 / 3;
    background: #111;
    border-radius: var(--radius);
    overflow: hidden;
    position: relative;
    border: 1px solid var(--border);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .poster-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 0.3s ease;
  }

  .poster-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.02);
    color: var(--text-dim);
    opacity: 0.3;
  }

  .poster-overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(to top, rgba(0,0,0,0.9) 0%, rgba(0,0,0,0.4) 50%, transparent 100%);
    opacity: 0;
    transition: opacity 0.2s ease;
    display: flex;
    align-items: flex-end;
    padding: 16px;
  }

  .overlay-content {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .overlay-title {
    color: white;
    font-weight: 700;
    font-size: 14px;
    line-height: 1.2;
  }

  .overlay-desc {
    color: rgba(255, 255, 255, 0.7);
    font-size: 11px;
    line-height: 1.4;
    display: -webkit-box;
    line-clamp: 3;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .rating-badge {
    position: absolute;
    top: 8px;
    right: 8px;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    color: #f59e0b;
    font-size: 11px;
    font-weight: 700;
    padding: 4px 8px;
    border-radius: 12px;
    display: flex;
    align-items: center;
    gap: 4px;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .poster-info {
    display: flex;
    flex-direction: column;
    gap: 2px;

    .title {
      margin: 0;
      font-size: 14px;
      color: var(--text-bright);
      font-weight: 600;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .year {
      font-size: 12px;
      color: var(--text-dim);
    }
  }

  .sentinel {
    padding: 24px 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-dim);
    font-size: 13px;
  }
</style>
