<script lang="ts">
  /**
   * FloatingPlayer.svelte
   *
   * A draggable, resizable floating video player overlay for in-app stream preview.
   * Wraps VideoPlayer.svelte and adds:
   *  - Drag-to-reposition header
   *  - Minimize / close controls
   *  - Stream title + provider info display
   *  - Keyboard shortcut: Escape to close
   */
  import { onMount, onDestroy } from 'svelte';
  import VideoPlayer from './VideoPlayer.svelte';
  import { X, Minus, Maximize2 } from 'lucide-svelte';

  interface StreamInfo {
    /** Proxy/direct URL for the stream */
    url: string;
    /** Human-readable title shown in the player header */
    title: string;
    /** Optional provider name (M3U account) */
    provider?: string;
    /** Channel UUID — used to build the Dispatcharr stream proxy URL */
    channelUuid?: string;
  }

  let {
    stream = null as StreamInfo | null,
    onclose
  }: {
    stream: StreamInfo | null;
    onclose?: () => void;
  } = $props();

  // --- Position & Drag State ---
  // Default to bottom-right corner with comfortable margin
  let posX = $state(window.innerWidth - 480 - 24);
  let posY = $state(window.innerHeight - 290 - 24);
  let isDragging = $state(false);
  let dragOffsetX = 0;
  let dragOffsetY = 0;

  // --- Minimized State ---
  let minimized = $state(false);

  // --- Player muted state (bindable through to VideoPlayer) ---
  let muted = $state(false);

  // --- Computed stream URL ---
  // Prefer the Dispatcharr proxy URL when we have a UUID (always better for load balancing)
  let effectiveUrl = $derived(
    stream?.channelUuid
      ? `/api/streams/${stream.channelUuid}`
      : (stream?.url ?? '')
  );

  // --- Drag Handlers ---
  function startDrag(e: MouseEvent) {
    // Only drag from the header bar, not child buttons
    if ((e.target as HTMLElement).closest('button')) return;
    isDragging = true;
    dragOffsetX = e.clientX - posX;
    dragOffsetY = e.clientY - posY;
    document.body.style.userSelect = 'none';
  }

  function onMouseMove(e: MouseEvent) {
    if (!isDragging) return;
    // Clamp within viewport bounds
    posX = Math.max(0, Math.min(window.innerWidth - 440, e.clientX - dragOffsetX));
    posY = Math.max(0, Math.min(window.innerHeight - 60, e.clientY - dragOffsetY));
  }

  function stopDrag() {
    if (isDragging) {
      isDragging = false;
      document.body.style.userSelect = '';
    }
  }

  // --- Keyboard: Escape closes the player ---
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  function close() {
    if (onclose) onclose();
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
    document.body.style.userSelect = '';
  });
</script>

<!-- Global mouse tracking for smooth drag across the whole window -->
<svelte:window onmousemove={onMouseMove} onmouseup={stopDrag} />

{#if stream}
  <div
    class="floating-player"
    class:minimized
    style:left="{posX}px"
    style:top="{posY}px"
    role="dialog"
    aria-label="Floating video player: {stream.title}"
  >
    <!-- ══ Header / Drag Handle ══ -->
    <div
      class="player-header"
      role="presentation"
      onmousedown={startDrag}
    >
      <div class="header-info">
        <span class="live-dot" aria-hidden="true"></span>
        <span class="stream-title" title={stream.title}>{stream.title}</span>
        {#if stream.provider}
          <span class="provider-badge">{stream.provider}</span>
        {/if}
      </div>

      <div class="header-actions">
        <!-- Minimize / restore -->
        <button
          class="hdr-btn"
          title={minimized ? 'Restore player' : 'Minimize player'}
          onclick={() => minimized = !minimized}
          aria-label={minimized ? 'Restore' : 'Minimize'}
        >
          {#if minimized}
            <Maximize2 size={14} />
          {:else}
            <Minus size={14} />
          {/if}
        </button>

        <!-- Close -->
        <button
          class="hdr-btn close-btn"
          title="Close player (Esc)"
          onclick={close}
          aria-label="Close player"
        >
          <X size={14} />
        </button>
      </div>
    </div>

    <!-- ══ Video Area (hidden when minimized) ══ -->
    {#if !minimized}
      <div class="player-body">
        <VideoPlayer
          src={effectiveUrl}
          title={stream.title}
          autoplay={true}
          bind:muted
        />
      </div>
    {/if}
  </div>
{/if}

<style lang="less">
  .floating-player {
    // Always on top of everything else
    position: fixed;
    z-index: 9999;
    width: 440px;
    background: #0d0d0f;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 12px;
    box-shadow:
      0 24px 60px rgba(0, 0, 0, 0.7),
      0 0 0 1px rgba(255, 255, 255, 0.04);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    // Smooth size transition when minimizing
    transition: box-shadow 0.2s, border-color 0.2s;

    &.minimized {
      width: 320px;
      border-color: rgba(255, 255, 255, 0.08);
    }
  }

  // ── Header ──────────────────────────────────────────────────────────────────
  .player-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    background: linear-gradient(135deg, rgba(237, 28, 36, 0.15), rgba(20, 20, 24, 0.9));
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    cursor: grab;
    gap: 8px;

    &:active { cursor: grabbing; }
  }

  .header-info {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0; // allow text truncation
    flex: 1;
  }

  .live-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #ef4444;
    box-shadow: 0 0 6px #ef4444;
    flex-shrink: 0;
    animation: pulse-dot 1.5s ease-in-out infinite;
  }

  .stream-title {
    font-size: 13px;
    font-weight: 600;
    color: #fff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .provider-badge {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: rgba(255, 255, 255, 0.45);
    background: rgba(255, 255, 255, 0.07);
    padding: 2px 6px;
    border-radius: 4px;
    flex-shrink: 0;
  }

  .header-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .hdr-btn {
    background: rgba(255, 255, 255, 0.07);
    border: none;
    color: rgba(255, 255, 255, 0.6);
    width: 24px;
    height: 24px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;

    &:hover {
      background: rgba(255, 255, 255, 0.14);
      color: #fff;
    }

    &.close-btn:hover {
      background: rgba(239, 68, 68, 0.25);
      color: #ef4444;
    }
  }

  // ── Video Body ───────────────────────────────────────────────────────────────
  .player-body {
    // VideoPlayer will fill this with 16:9 aspect ratio
    background: #000;
    border-radius: 0 0 12px 12px;
    overflow: hidden;
  }

  // ── Animations ──────────────────────────────────────────────────────────────
  @keyframes pulse-dot {
    0%   { opacity: 1; }
    50%  { opacity: 0.3; }
    100% { opacity: 1; }
  }
</style>
