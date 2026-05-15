<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { X, Play, Clock, Star, Tv, Film, Server, ChevronDown, Calendar, ExternalLink, Layers } from 'lucide-svelte';

    export let show = false;
    export let vod = null;
    export let type = "movie"; // "movie" or "series"

    const dispatch = createEventDispatcher();

    let isVisible = false;
    let selectedProvider = null;
    let customProps = {};
    let episodes = [];
    let loadingEpisodes = false;

    // React to 'show' changes for animations
    $: if (show) {
        setTimeout(() => isVisible = true, 50);
        if (vod) {
            try {
                customProps = typeof vod.custom_properties === 'string' ? JSON.parse(vod.custom_properties) : (vod.custom_properties || {});
            } catch (e) {
                customProps = {};
            }
            if (vod.providers && vod.providers.length > 0) {
                selectedProvider = vod.providers[0];
            }
            if (type === "series") {
                fetchEpisodes();
            } else {
                episodes = [];
            }
        }
    } else {
        isVisible = false;
        setTimeout(() => {
            episodes = [];
        }, 300); // Wait for transition
    }

    async function fetchEpisodes() {
        if (!vod || !vod.id) return;
        loadingEpisodes = true;
        try {
            const res = await fetch(`/api/vod/series/${vod.id}/episodes`);
            if (res.ok) {
                episodes = await res.json();
            }
        } catch (err) {
            console.error("Failed to fetch episodes", err);
        } finally {
            loadingEpisodes = false;
        }
    }

    function close() {
        show = false;
        dispatch('close');
    }

    function handlePlayMovie() {
        if (!selectedProvider) return;
        dispatch('play', {
            type: 'movie',
            id: vod.id,
            stream_id: selectedProvider.stream_id,
            m3u_account_id: selectedProvider.account_id
        });
    }

    function handlePlayEpisode(episode) {
        // Episode objects should have their own providers
        let provider = null;
        if (episode.providers && episode.providers.length > 0) {
            // Try to match the currently selected provider for the series if possible
            provider = episode.providers.find(p => p.account_id === selectedProvider?.account_id) || episode.providers[0];
        }

        dispatch('play', {
            type: 'episode',
            id: episode.id,
            stream_id: provider ? provider.stream_id : null,
            m3u_account_id: provider ? provider.account_id : null
        });
    }
    
    function formatDuration(minutes) {
        if (!minutes) return '';
        const h = Math.floor(minutes / 60);
        const m = minutes % 60;
        return h > 0 ? `${h}h ${m}m` : `${m}m`;
    }
</script>

{#if show}
<div class="modal-backdrop {isVisible ? 'visible' : ''}" on:click={close}>
    <div class="modal-container {isVisible ? 'visible' : ''}" on:click|stopPropagation>
        <button class="close-button" on:click={close}>
            <X size={24} />
        </button>

        <!-- Header / Hero Section -->
        <div class="hero-section" style="background-image: url('{customProps.local_poster ? `/api/vod/images/${customProps.local_poster}` : (vod?.poster_url || '')}');">
            <div class="hero-overlay"></div>
            
            <div class="hero-content">
                <div class="poster-container">
                    {#if customProps.local_poster}
                        <img src={`/api/vod/images/${customProps.local_poster}`} alt={vod?.name} class="poster" />
                    {:else if vod?.poster_url}
                        <img src={vod.poster_url} alt={vod.name} class="poster" />
                    {:else}
                        <div class="poster-placeholder">
                            {#if type === "movie"}
                                <Film size={48} />
                            {:else}
                                <Tv size={48} />
                            {/if}
                        </div>
                    {/if}
                </div>

                <div class="metadata-container">
                    <h1 class="title">{vod?.name}</h1>
                    
                    <div class="tags">
                        {#if vod?.year}
                            <span class="tag"><Calendar size={14} /> {vod.year}</span>
                        {/if}
                        {#if customProps.duration}
                            <span class="tag"><Clock size={14} /> {formatDuration(customProps.duration)}</span>
                        {/if}
                        {#if customProps.rating}
                            <span class="tag highlight"><Star size={14} /> {customProps.rating}</span>
                        {/if}
                        {#if customProps.genre}
                            <span class="tag">{customProps.genre}</span>
                        {/if}
                    </div>

                    {#if customProps.description || customProps.plot}
                        <p class="description">
                            {customProps.description || customProps.plot}
                        </p>
                    {/if}

                    <div class="actions">
                        {#if type === "movie"}
                            <button class="btn-primary play-btn" on:click={handlePlayMovie}>
                                <Play fill="currentColor" size={20} />
                                <span>Play Movie</span>
                            </button>
                        {/if}
                        
                        {#if customProps.trailer_url || customProps.youtube_trailer}
                            <a href={customProps.trailer_url || `https://youtube.com/watch?v=${customProps.youtube_trailer}`} target="_blank" class="btn-secondary trailer-btn">
                                <ExternalLink size={20} />
                                <span>Trailer</span>
                            </a>
                        {/if}
                    </div>

                    <!-- Provider Selection -->
                    {#if vod?.providers && vod.providers.length > 0}
                        <div class="provider-section">
                            <label for="provider-select" class="provider-label">
                                <Server size={14} /> Source
                            </label>
                            <div class="select-wrapper">
                                <select id="provider-select" bind:value={selectedProvider} class="provider-select">
                                    {#each vod.providers as provider}
                                        <option value={provider}>{provider.account_name}</option>
                                    {/each}
                                </select>
                                <ChevronDown size={16} class="select-icon" />
                            </div>
                        </div>
                    {/if}
                </div>
            </div>
        </div>

        <!-- Details & Episodes Section -->
        <div class="content-section">
            {#if type === "series"}
                <div class="episodes-container">
                    <h2 class="section-title"><Layers size={20} /> Episodes</h2>
                    
                    {#if loadingEpisodes}
                        <div class="loading">Loading episodes...</div>
                    {:else if episodes.length === 0}
                        <div class="empty-state">No episodes available.</div>
                    {:else}
                        <div class="episodes-list">
                            {#each episodes as episode}
                                <div class="episode-card" on:click={() => handlePlayEpisode(episode)}>
                                    <div class="episode-image-wrapper">
                                        {#if episode.custom_properties}
                                            {@const epProps = JSON.parse(episode.custom_properties)}
                                            {#if epProps.poster_url || epProps.image}
                                                <img src={epProps.poster_url || epProps.image} alt={episode.name} class="episode-image" />
                                            {:else}
                                                <div class="episode-placeholder"><Tv size={24} /></div>
                                            {/if}
                                        {:else}
                                            <div class="episode-placeholder"><Tv size={24} /></div>
                                        {/if}
                                        <div class="episode-play-overlay">
                                            <Play fill="currentColor" size={24} />
                                        </div>
                                    </div>
                                    <div class="episode-info">
                                        <div class="episode-number">S{episode.season_number} E{episode.episode_number}</div>
                                        <h3 class="episode-name">{episode.name}</h3>
                                        {#if episode.custom_properties}
                                            {@const epProps = JSON.parse(episode.custom_properties)}
                                            {#if epProps.duration}
                                                <div class="episode-duration">{formatDuration(epProps.duration)}</div>
                                            {/if}
                                        {/if}
                                    </div>
                                    {#if episode.providers && episode.providers.length > 1}
                                        <div class="provider-count" title="Available from {episode.providers.length} sources">
                                            {episode.providers.length} sources
                                        </div>
                                    {/if}
                                </div>
                            {/each}
                        </div>
                    {/if}
                </div>
            {/if}

            {#if customProps.cast || customProps.director}
                <div class="credits-section">
                    <h2 class="section-title">Credits</h2>
                    {#if customProps.director}
                        <div class="credit-item">
                            <span class="credit-role">Director</span>
                            <span class="credit-name">{customProps.director}</span>
                        </div>
                    {/if}
                    {#if customProps.cast}
                        <div class="credit-item">
                            <span class="credit-role">Cast</span>
                            <span class="credit-name">{customProps.cast}</span>
                        </div>
                    {/if}
                </div>
            {/if}
        </div>
    </div>
</div>
{/if}

<style lang="less">
    .modal-backdrop {
        position: fixed;
        top: 0;
        left: 0;
        width: 100vw;
        height: 100vh;
        background: rgba(0, 0, 0, 0);
        backdrop-filter: blur(0px);
        z-index: 1000;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
        padding: 2rem;
        box-sizing: border-box;
        
        &.visible {
            background: rgba(0, 0, 0, 0.6);
            backdrop-filter: blur(10px);
        }
    }

    .modal-container {
        width: 100%;
        max-width: 1000px;
        max-height: 90vh;
        background: rgba(20, 20, 25, 0.85);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 16px;
        overflow: hidden;
        box-shadow: 0 24px 48px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255, 255, 255, 0.05);
        transform: translateY(20px) scale(0.98);
        opacity: 0;
        transition: all 0.4s cubic-bezier(0.16, 1, 0.3, 1);
        display: flex;
        flex-direction: column;
        position: relative;
        
        &.visible {
            transform: translateY(0) scale(1);
            opacity: 1;
        }
    }

    .close-button {
        position: absolute;
        top: 1rem;
        right: 1rem;
        width: 36px;
        height: 36px;
        border-radius: 50%;
        background: rgba(0, 0, 0, 0.5);
        color: white;
        border: 1px solid rgba(255, 255, 255, 0.1);
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        z-index: 10;
        transition: all 0.2s ease;
        
        &:hover {
            background: rgba(255, 255, 255, 0.2);
            transform: scale(1.05);
        }
    }

    .hero-section {
        position: relative;
        padding: 3rem;
        background-size: cover;
        background-position: center 20%;
        min-height: 350px;
        display: flex;
        align-items: flex-end;
    }

    .hero-overlay {
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: linear-gradient(to top, rgba(20, 20, 25, 1) 0%, rgba(20, 20, 25, 0.8) 50%, rgba(20, 20, 25, 0.4) 100%);
    }

    .hero-content {
        position: relative;
        z-index: 2;
        display: flex;
        gap: 2.5rem;
        width: 100%;
        align-items: flex-end;
    }

    .poster-container {
        flex-shrink: 0;
        width: 200px;
        height: 300px;
        border-radius: 12px;
        overflow: hidden;
        box-shadow: 0 16px 32px rgba(0, 0, 0, 0.5);
        border: 1px solid rgba(255, 255, 255, 0.15);
        background: #1a1a24;
    }

    .poster {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }

    .poster-placeholder {
        width: 100%;
        height: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
        color: rgba(255, 255, 255, 0.2);
    }

    .metadata-container {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 1rem;
        padding-bottom: 0.5rem;
    }

    .title {
        font-size: 2.5rem;
        font-weight: 700;
        color: #fff;
        margin: 0;
        line-height: 1.1;
        text-shadow: 0 2px 10px rgba(0,0,0,0.5);
    }

    .tags {
        display: flex;
        flex-wrap: wrap;
        gap: 0.75rem;
    }

    .tag {
        display: inline-flex;
        align-items: center;
        gap: 0.35rem;
        padding: 0.25rem 0.75rem;
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.05);
        border-radius: 20px;
        font-size: 0.85rem;
        color: rgba(255, 255, 255, 0.9);
        backdrop-filter: blur(4px);
        
        &.highlight {
            background: rgba(234, 179, 8, 0.15);
            color: #facc15;
            border-color: rgba(234, 179, 8, 0.3);
        }
    }

    .description {
        font-size: 1rem;
        line-height: 1.6;
        color: rgba(255, 255, 255, 0.75);
        margin: 0;
        max-width: 800px;
        display: -webkit-box;
        -webkit-line-clamp: 3;
        -webkit-box-orient: vertical;
        overflow: hidden;
    }

    .actions {
        display: flex;
        gap: 1rem;
        margin-top: 0.5rem;
    }

    .btn-primary {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.75rem 1.5rem;
        background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
        color: white;
        border: none;
        border-radius: 8px;
        font-weight: 600;
        font-size: 1rem;
        cursor: pointer;
        transition: all 0.2s ease;
        box-shadow: 0 4px 12px rgba(37, 99, 235, 0.3);
        
        &:hover {
            transform: translateY(-2px);
            box-shadow: 0 6px 16px rgba(37, 99, 235, 0.4);
            background: linear-gradient(135deg, #60a5fa 0%, #3b82f6 100%);
        }
        
        &:active {
            transform: translateY(0);
        }
    }

    .btn-secondary {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.75rem 1.5rem;
        background: rgba(255, 255, 255, 0.1);
        color: white;
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 8px;
        font-weight: 600;
        font-size: 1rem;
        cursor: pointer;
        transition: all 0.2s ease;
        text-decoration: none;
        backdrop-filter: blur(4px);
        
        &:hover {
            background: rgba(255, 255, 255, 0.15);
            border-color: rgba(255, 255, 255, 0.3);
            transform: translateY(-2px);
        }
    }

    .provider-section {
        display: flex;
        align-items: center;
        gap: 1rem;
        margin-top: 0.5rem;
    }

    .provider-label {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        color: rgba(255, 255, 255, 0.6);
        font-size: 0.9rem;
        font-weight: 500;
    }

    .select-wrapper {
        position: relative;
        width: 250px;
    }

    .provider-select {
        width: 100%;
        appearance: none;
        background: rgba(0, 0, 0, 0.4);
        border: 1px solid rgba(255, 255, 255, 0.15);
        color: white;
        padding: 0.6rem 2.5rem 0.6rem 1rem;
        border-radius: 6px;
        font-size: 0.95rem;
        cursor: pointer;
        transition: all 0.2s;
        
        &:hover {
            border-color: rgba(255, 255, 255, 0.3);
        }
        
        &:focus {
            outline: none;
            border-color: #3b82f6;
            box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.25);
        }
        
        option {
            background: #1a1a24;
            color: white;
        }
    }

    .select-icon {
        position: absolute;
        right: 1rem;
        top: 50%;
        transform: translateY(-50%);
        color: rgba(255, 255, 255, 0.5);
        pointer-events: none;
    }

    .content-section {
        padding: 2rem 3rem;
        overflow-y: auto;
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 2rem;
        
        &::-webkit-scrollbar {
            width: 8px;
        }
        &::-webkit-scrollbar-track {
            background: rgba(0, 0, 0, 0.1);
        }
        &::-webkit-scrollbar-thumb {
            background: rgba(255, 255, 255, 0.1);
            border-radius: 4px;
        }
        &::-webkit-scrollbar-thumb:hover {
            background: rgba(255, 255, 255, 0.2);
        }
    }

    .section-title {
        font-size: 1.25rem;
        font-weight: 600;
        color: white;
        margin: 0 0 1rem 0;
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .credits-section {
        display: flex;
        flex-direction: column;
        gap: 1rem;
        background: rgba(0, 0, 0, 0.2);
        padding: 1.5rem;
        border-radius: 12px;
        border: 1px solid rgba(255, 255, 255, 0.05);
    }

    .credit-item {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }

    .credit-role {
        font-size: 0.85rem;
        color: rgba(255, 255, 255, 0.5);
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    .credit-name {
        font-size: 1rem;
        color: rgba(255, 255, 255, 0.9);
    }

    .episodes-list {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
        gap: 1rem;
    }

    .episode-card {
        display: flex;
        gap: 1rem;
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid rgba(255, 255, 255, 0.05);
        border-radius: 8px;
        padding: 0.75rem;
        cursor: pointer;
        transition: all 0.2s ease;
        position: relative;
        
        &:hover {
            background: rgba(255, 255, 255, 0.08);
            border-color: rgba(255, 255, 255, 0.15);
            transform: translateY(-2px);
            
            .episode-play-overlay {
                opacity: 1;
            }
        }
    }

    .episode-image-wrapper {
        position: relative;
        width: 120px;
        height: 68px;
        border-radius: 4px;
        overflow: hidden;
        background: #111;
        flex-shrink: 0;
    }

    .episode-image {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }

    .episode-placeholder {
        width: 100%;
        height: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
        color: rgba(255, 255, 255, 0.2);
    }

    .episode-play-overlay {
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: rgba(0, 0, 0, 0.5);
        display: flex;
        align-items: center;
        justify-content: center;
        color: white;
        opacity: 0;
        transition: opacity 0.2s ease;
    }

    .episode-info {
        flex: 1;
        display: flex;
        flex-direction: column;
        justify-content: center;
        min-width: 0;
    }

    .episode-number {
        font-size: 0.75rem;
        color: #3b82f6;
        font-weight: 600;
        margin-bottom: 0.25rem;
    }

    .episode-name {
        font-size: 0.95rem;
        color: rgba(255, 255, 255, 0.9);
        margin: 0 0 0.25rem 0;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .episode-duration {
        font-size: 0.8rem;
        color: rgba(255, 255, 255, 0.5);
    }

    .provider-count {
        position: absolute;
        top: 0.5rem;
        right: 0.5rem;
        background: rgba(0, 0, 0, 0.6);
        border: 1px solid rgba(255, 255, 255, 0.2);
        color: rgba(255, 255, 255, 0.8);
        font-size: 0.7rem;
        padding: 0.1rem 0.4rem;
        border-radius: 12px;
        backdrop-filter: blur(4px);
    }

    .loading, .empty-state {
        padding: 2rem;
        text-align: center;
        color: rgba(255, 255, 255, 0.5);
        background: rgba(255, 255, 255, 0.02);
        border-radius: 8px;
        border: 1px dashed rgba(255, 255, 255, 0.1);
    }

    @media (max-width: 768px) {
        .modal-container {
            height: 100vh;
            max-height: 100vh;
            border-radius: 0;
        }
        
        .hero-section {
            padding: 2rem;
        }
        
        .hero-content {
            flex-direction: column;
            align-items: center;
            text-align: center;
        }
        
        .tags {
            justify-content: center;
        }
        
        .actions {
            justify-content: center;
        }
        
        .provider-section {
            justify-content: center;
            flex-direction: column;
            gap: 0.5rem;
        }
        
        .content-section {
            padding: 1.5rem;
        }
        
        .episodes-list {
            grid-template-columns: 1fr;
        }
    }
</style>
