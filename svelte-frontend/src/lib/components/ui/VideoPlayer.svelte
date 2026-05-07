<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Hls from 'hls.js';
  import mpegts from 'mpegts.js';
  import { Play, Pause, Volume2, VolumeX, Maximize, AlertCircle, Loader2 } from 'lucide-svelte';

  let { 
    src = '', 
    title = 'Live Stream', 
    autoplay = true,
    muted = $bindable(false)
  } = $props();

  let videoElement: HTMLVideoElement;
  let hls: Hls | null = null;
  let mpegPlayer: any = null;
  
  let loading = $state(true);
  let error = $state<string | null>(null);
  let playing = $state(false);
  let volume = $state(1);
  let showControls = $state(true);
  let controlsTimeout: any;

  function initPlayer() {
    if (!videoElement || !src) return;
    
    // Reset state
    destroyPlayer();
    loading = true;
    error = null;

    const isHls = src.includes('.m3u8') || src.includes('format=hls');
    const isTs = src.includes('.ts') || src.includes('/stream/') || src.includes('format=ts');

    if (isHls) {
      if (Hls.isSupported()) {
        hls = new Hls({
          enableWorker: true,
          lowLatencyMode: true,
          backBufferLength: 60
        });
        hls.loadSource(src);
        hls.attachMedia(videoElement);
        hls.on(Hls.Events.MANIFEST_PARSED, () => {
          loading = false;
          if (autoplay) videoElement.play().catch(() => {
            playing = false;
          });
        });
        hls.on(Hls.Events.ERROR, (_, data) => {
          if (data.fatal) {
            error = `HLS Error: ${data.details}`;
            loading = false;
          }
        });
      } else if (videoElement.canPlayType('application/vnd.apple.mpegurl')) {
        videoElement.src = src;
        videoElement.addEventListener('loadedmetadata', () => {
          loading = false;
          if (autoplay) videoElement.play();
        });
      } else {
        error = "HLS playback is not supported in this browser.";
        loading = false;
      }
    } else if (isTs) {
      // Use mpegts.isSupported() as it's more comprehensive than just checking .mse
      if (mpegts.isSupported()) {
        mpegPlayer = mpegts.createPlayer({
          type: 'mse',
          isLive: true,
          url: src,
          cors: true
        });
        mpegPlayer.attachMediaElement(videoElement);
        mpegPlayer.load();
        
        mpegPlayer.on(mpegts.Events.ERROR, (type: any, detail: any) => {
          console.error('MPEG-TS Error:', type, detail);
          error = `Stream Error: ${type} (${detail})`;
          loading = false;
        });
        
        mpegPlayer.on(mpegts.Events.METADATA_ARRIVED, () => {
          loading = false;
          if (autoplay) videoElement.play().catch(() => {});
        });

        // Fallback for metadata delay
        setTimeout(() => {
          if (loading && !error) loading = false;
        }, 3000);

      } else {
        // Fallback: try native playback anyway, some browsers might handle it or mpegts check might be too strict
        console.warn('mpegts.isSupported() returned false, attempting native fallback...');
        videoElement.src = src;
        videoElement.addEventListener('canplay', () => {
          loading = false;
          if (autoplay) videoElement.play();
        });
        videoElement.addEventListener('error', () => {
          error = "MPEG-TS playback is not supported in this browser. Try using a Chromium-based browser or open in an external player.";
          loading = false;
        });
      }
    } else {
      videoElement.src = src;
      videoElement.addEventListener('canplay', () => {
        loading = false;
        if (autoplay) videoElement.play();
      });
      videoElement.addEventListener('error', () => {
        error = "Failed to load video stream.";
        loading = false;
      });
    }
  }

  function destroyPlayer() {
    if (hls) {
      hls.destroy();
      hls = null;
    }
    if (mpegPlayer) {
      mpegPlayer.unload();
      mpegPlayer.detachMediaElement();
      mpegPlayer.destroy();
      mpegPlayer = null;
    }
    if (videoElement) {
      videoElement.src = '';
      videoElement.load();
    }
  }

  function togglePlay() {
    if (videoElement.paused) videoElement.play();
    else videoElement.pause();
  }

  function toggleMute() {
    muted = !muted;
  }

  function toggleFullscreen() {
    if (document.fullscreenElement) {
      document.exitFullscreen();
    } else {
      videoElement.parentElement?.requestFullscreen();
    }
  }

  function handleMouseMove() {
    showControls = true;
    clearTimeout(controlsTimeout);
    controlsTimeout = setTimeout(() => {
      if (playing) showControls = false;
    }, 3000);
  }

  $effect(() => {
    if (src) initPlayer();
    return () => destroyPlayer();
  });

  onMount(() => {
    handleMouseMove();
  });

  onDestroy(() => {
    destroyPlayer();
    clearTimeout(controlsTimeout);
  });
</script>

<div class="video-wrapper" onmousemove={handleMouseMove} role="presentation">
  <!-- svelte-ignore a11y_media_has_caption -->
  <video
    bind:this={videoElement}
    bind:paused={playing}
    bind:muted={muted}
    bind:volume={volume}
    playsinline
    class="video-element"
    onclick={togglePlay}
  ></video>

  {#if loading}
    <div class="overlay loading-overlay">
      <Loader2 class="spinning" size={48} />
      <span>Connecting to stream...</span>
    </div>
  {/if}

  {#if error}
    <div class="overlay error-overlay">
      <AlertCircle size={48} color="var(--accent)" />
      <span class="error-text">{error}</span>
      <div class="error-actions">
        <button class="btn btn-secondary" onclick={initPlayer}>Try Again</button>
        <a href={src} class="btn btn-primary" target="_blank" rel="noopener noreferrer">
          Open in External Player
        </a>
      </div>
    </div>
  {/if}

  <!-- Custom Controls Overlay -->
  <div class="controls-overlay" class:visible={showControls || !playing || error}>
    <div class="controls-top">
      <span class="stream-title">{title}</span>
    </div>

    <div class="controls-bottom">
      <div class="controls-left">
        <button class="control-btn" onclick={togglePlay}>
          {#if playing}
            <Pause size={20} fill="currentColor" />
          {:else}
            <Play size={20} fill="currentColor" />
          {/if}
        </button>

        <button class="control-btn" onclick={toggleMute}>
          {#if muted || volume === 0}
            <VolumeX size={20} />
          {:else}
            <Volume2 size={20} />
          {/if}
        </button>
        
        <input 
          type="range" 
          min="0" 
          max="1" 
          step="0.05" 
          bind:value={volume} 
          class="volume-slider" 
        />
      </div>

      <div class="controls-right">
        <div class="live-indicator">
          <span class="dot"></span>
          LIVE
        </div>
        <button class="control-btn" onclick={toggleFullscreen}>
          <Maximize size={20} />
        </button>
      </div>
    </div>
  </div>
</div>

<style lang="less">
  .video-wrapper {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    background: #000;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: inherit;

    &:fullscreen {
      aspect-ratio: auto;
      width: 100vw;
      height: 100vh;
      border-radius: 0;
    }
  }

  .video-element {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    z-index: 10;
    color: white;
    text-align: center;
    padding: 24px;
  }

  .error-text {
    color: #fca5a5;
    font-size: 14px;
    max-width: 300px;
    margin-bottom: 8px;
  }

  .error-actions {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .controls-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 20px;
    background: linear-gradient(to bottom, rgba(0,0,0,0.6) 0%, transparent 20%, transparent 80%, rgba(0,0,0,0.8) 100%);
    opacity: 0;
    transition: opacity 0.3s ease;
    z-index: 5;
    pointer-events: none;

    &.visible {
      opacity: 1;
      pointer-events: auto;
    }
  }

  .controls-top {
    .stream-title {
      font-size: 16px;
      font-weight: 700;
      color: white;
      text-shadow: 0 2px 4px rgba(0,0,0,0.5);
    }
  }

  .controls-bottom {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .controls-left, .controls-right {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .control-btn {
    background: none;
    border: none;
    color: white;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 8px;
    border-radius: 50%;
    transition: background 0.2s;

    &:hover {
      background: rgba(255, 255, 255, 0.1);
    }
  }

  .volume-slider {
    width: 80px;
    height: 4px;
    accent-color: var(--accent);
    cursor: pointer;
  }

  .live-indicator {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
    font-size: 11px;
    font-weight: 800;
    padding: 4px 10px;
    border-radius: 4px;
    border: 1px solid rgba(239, 68, 68, 0.3);

    .dot {
      width: 6px;
      height: 6px;
      background: #ef4444;
      border-radius: 50%;
      box-shadow: 0 0 4px #ef4444;
      animation: pulse 1.5s infinite;
    }
  }

  @keyframes pulse {
    0% { opacity: 1; }
    50% { opacity: 0.4; }
    100% { opacity: 1; }
  }

  .spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
