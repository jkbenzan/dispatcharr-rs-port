<script lang="ts">
	import { fade, scale } from 'svelte/transition';
	import { X } from 'lucide-svelte';

	let {
		show = $bindable(false),
		title = '',
		width = '600px',
		height = 'auto',
		children
	} = $props();

	function close() {
		show = false;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && show) {
			close();
		}
	}
</script>

<svelte:window on:keydown={handleKeydown} />

{#if show}
	<div class="modal-backdrop" transition:fade={{ duration: 200 }} onmousedown={close} role="presentation">
		<div
			class="modal-container"
			style:width
			style:height
			transition:scale={{ duration: 200, start: 0.95 }}
			onmousedown={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
		>
			{#if title}
				<div class="modal-header">
					<h2>{title}</h2>
					<button class="close-btn" onclick={close} aria-label="Close modal">
						<X size={20} />
					</button>
				</div>
			{/if}
			<div class="modal-body">
				{@render children?.()}
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		width: 100vw;
		height: 100vh;
		background: rgba(0, 0, 0, 0.6);
		backdrop-filter: var(--glass);
		-webkit-backdrop-filter: var(--glass);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal-container {
		background: var(--surface);
		border: 1px solid var(--border-bright);
		border-radius: var(--radius);
		box-shadow: var(--shadow-lg);
		display: flex;
		flex-direction: column;
		max-height: 90vh;
		overflow: hidden;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 16px 24px;
		border-bottom: 1px solid var(--border);
		background: var(--surface-bright);

		h2 {
			font-size: 18px;
			font-weight: 600;
			color: var(--text-bright);
			margin: 0;
		}
	}

	.close-btn {
		background: transparent;
		border: none;
		color: var(--text-dim);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 4px;
		border-radius: 4px;
		transition: all 0.2s;

		&:hover {
			background: rgba(255, 255, 255, 0.1);
			color: var(--text-bright);
		}
	}

	.modal-body {
		padding: 0;
		overflow-y: auto;
		flex: 1;
		display: flex;
		flex-direction: column;
	}
</style>
