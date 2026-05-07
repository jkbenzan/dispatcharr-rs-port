<script lang="ts">
	import ChannelsPane from '$lib/components/channel-manager/ChannelsPane.svelte';
	import { Plus, LayoutGrid, ListFilter } from 'lucide-svelte';

	let leftWidth = $state(40); // %
	let isResizing = $state(false);

	function startResizing(e: MouseEvent) {
		isResizing = true;
		document.body.style.cursor = 'col-resize';
		document.body.style.userSelect = 'none';
	}

	function stopResizing() {
		if (isResizing) {
			isResizing = false;
			document.body.style.cursor = '';
			document.body.style.userSelect = '';
		}
	}

	function handleMouseMove(e: MouseEvent) {
		if (!isResizing) return;
		const pct = (e.clientX / window.innerWidth) * 100;
		leftWidth = Math.max(20, Math.min(80, pct));
	}
</script>

<svelte:window onmousemove={handleMouseMove} onmouseup={stopResizing} />

<div class="page-header">
	<div class="title-section">
		<h1>Channel Manager</h1>
		<div class="badges">
			<span class="badge">Live</span>
			<span class="badge accent">Svelte 5</span>
		</div>
	</div>
	
	<div class="actions">
		<button class="btn btn-secondary"><ListFilter size={18} /> Filters</button>
		<button class="btn btn-primary"><Plus size={18} /> Create Channel</button>
	</div>
</div>

<div class="manager-layout">
	<div class="pane left" style:width="{leftWidth}%">
		<ChannelsPane />
	</div>
	
	<div class="resizer" onmousedown={startResizing}></div>
	
	<div class="pane right" style:width="{100 - leftWidth}%">
		<div class="streams-placeholder">
			<div class="empty-state">
				<LayoutGrid size={48} />
				<h3>Streams Pane</h3>
				<p>Coming soon in the Svelte migration.</p>
			</div>
		</div>
	</div>
</div>

<style lang="less">
	.page-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 24px;
	}

	.title-section {
		display: flex;
		align-items: center;
		gap: 16px;
		h1 { font-size: 24px; font-weight: 700; color: var(--text-bright); }
	}

	.badges {
		display: flex;
		gap: 8px;
		.badge {
			font-size: 11px;
			font-weight: 700;
			padding: 2px 8px;
			border-radius: 4px;
			background: var(--surface-bright);
			color: var(--text-dim);
			text-transform: uppercase;
			&.accent { background: rgba(237, 28, 36, 0.1); color: var(--accent); }
		}
	}

	.actions {
		display: flex;
		gap: 12px;
	}

	.btn {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 16px;
		border-radius: var(--radius);
		font-size: 14px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
		border: 1px solid transparent;

		&.btn-primary {
			background: var(--accent);
			color: white;
			&:hover { background: var(--accent-dim); }
		}

		&.btn-secondary {
			background: var(--surface);
			border-color: var(--border);
			color: var(--text-bright);
			&:hover { border-color: var(--border-bright); background: var(--surface-bright); }
		}
	}

	.manager-layout {
		display: flex;
		flex: 1;
		height: calc(100vh - 200px);
		gap: 0;
	}

	.pane {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.resizer {
		width: 8px;
		cursor: col-resize;
		background: transparent;
		transition: background 0.2s;
		&:hover { background: var(--accent); }
	}

	.streams-placeholder {
		height: 100%;
		background: var(--surface);
		border-radius: var(--radius);
		display: flex;
		align-items: center;
		justify-content: center;
		border: 1px dashed var(--border);
	}

	.empty-state {
		text-align: center;
		color: var(--text-dim);
		h3 { color: var(--text-bright); margin: 16px 0 8px; }
		p { font-size: 14px; }
	}
</style>
