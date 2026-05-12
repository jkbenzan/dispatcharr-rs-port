<script lang="ts">
	import ChannelsPane from '$lib/components/channel-manager/ChannelsPane.svelte';
	import StreamsPane from '$lib/components/channel-manager/StreamsPane.svelte';
	import CreateChannelModal from '$lib/components/channel-manager/CreateChannelModal.svelte';
	import FloatingPlayer from '$lib/components/ui/FloatingPlayer.svelte';
	import { Plus, ListFilter } from 'lucide-svelte';

	// --- Split-pane resize ---
	let leftWidth = $state(40); // %
	let isResizing = $state(false);

	function startResizing() {
		isResizing = true;
		document.body.style.cursor = 'col-resize';
		document.body.style.userSelect = 'none';
	}
	function stopResizing() {
		if (isResizing) { isResizing = false; document.body.style.cursor = ''; document.body.style.userSelect = ''; }
	}
	function handleMouseMove(e: MouseEvent) {
		if (!isResizing) return;
		leftWidth = Math.max(20, Math.min(80, (e.clientX / window.innerWidth) * 100));
	}

	// --- Create modal ---
	let showCreateModal = $state(false);

	// --- Pane refs for reactive reload ---
	let channelsPaneRef: any;
	let streamsPaneRef: any;

	function reloadBothPanes() {
		channelsPaneRef?.reload();
		streamsPaneRef?.reload();
	}

	// --- Floating player ---
	interface PlayerStream { url: string; title: string; uuid?: string; provider?: string; }
	let activeStream = $state<PlayerStream | null>(null);

	function openPlayer(info: { url: string; title: string; uuid?: string; provider?: string }) {
		activeStream = info;
	}
	function closePlayer() { activeStream = null; }
</script>

<svelte:window onmousemove={handleMouseMove} onmouseup={stopResizing} />

<!-- Floating video player overlay -->
<FloatingPlayer stream={activeStream} onclose={closePlayer} />

<div class="page-header">
	<div class="title-section">
		<h1>Channel Manager</h1>
	</div>
	<div class="actions">
		<button class="btn btn-secondary"><ListFilter size={18} /> Filters</button>
		<button class="btn btn-primary" onclick={() => showCreateModal = true}><Plus size={18} /> Create Channel</button>
	</div>
</div>

<div class="manager-layout">
	<div class="pane left" style:width="{leftWidth}%">
		<ChannelsPane bind:this={channelsPaneRef} onPlayStream={openPlayer} />
	</div>

	<button
		type="button"
		class="resizer"
		onmousedown={startResizing}
		aria-label="Resize panes"
	></button>

	<div class="pane right" style:width="{100 - leftWidth}%">
		<StreamsPane bind:this={streamsPaneRef} onPlayStream={openPlayer} />
	</div>
</div>

<!-- Create channel modal — on success reload both panes reactively (no page reload) -->
<CreateChannelModal
	bind:show={showCreateModal}
	onCreated={reloadBothPanes}
/>

<style lang="less">
.page-header {
	display: flex; align-items: center; justify-content: space-between; margin-bottom: 24px;
}
.title-section {
	display: flex; align-items: center; gap: 16px;
	h1 { font-size: 24px; font-weight: 700; color: var(--text-bright); }
}
.actions { display: flex; gap: 12px; }
.btn {
	display: flex; align-items: center; gap: 8px; padding: 10px 16px;
	border-radius: var(--radius); font-size: 14px; font-weight: 600; cursor: pointer;
	transition: all 0.2s; border: 1px solid transparent;
	&.btn-primary { background: var(--accent); color: white; &:hover { background: var(--accent-dim); } }
	&.btn-secondary { background: var(--surface); border-color: var(--border); color: var(--text-bright); &:hover { border-color: var(--border-bright); background: var(--surface-bright); } }
}
.manager-layout { display: flex; flex: 1; height: calc(100vh - 200px); gap: 0; }
.pane { display: flex; flex-direction: column; height: 100%; }
.resizer {
	width: 6px; cursor: col-resize; background: transparent; border: 0; padding: 0;
	transition: background 0.2s; flex-shrink: 0;
	&:hover { background: var(--accent); }
}
</style>
