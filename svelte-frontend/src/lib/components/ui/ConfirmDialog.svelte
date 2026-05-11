<script lang="ts">
	import { fade, scale } from 'svelte/transition';
	import { AlertTriangle, X } from 'lucide-svelte';

	type Variant = 'default' | 'danger';

	let {
		show = $bindable(false),
		title,
		message,
		confirmLabel = 'Confirm',
		cancelLabel = 'Cancel',
		variant = 'default' as Variant,
		busy = false,
		onConfirm,
		onCancel
	}: {
		show: boolean;
		title: string;
		message: string;
		confirmLabel?: string;
		cancelLabel?: string;
		variant?: Variant;
		busy?: boolean;
		onConfirm: () => void;
		onCancel?: () => void;
	} = $props();

	let dialogRef: HTMLDivElement | undefined = $state();

	$effect(() => {
		if (show && dialogRef) {
			// Move focus into the dialog so keyboard users do not remain behind the modal overlay.
			setTimeout(() => dialogRef?.focus(), 0);
		}
	});

	function cancel() {
		if (busy) return;
		show = false;
		onCancel?.();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && show) {
			cancel();
		}
	}
</script>

<svelte:window on:keydown={handleKeydown} />

{#if show}
	<div class="confirm-backdrop" transition:fade={{ duration: 160 }} onmousedown={cancel} role="presentation">
		<div
			bind:this={dialogRef}
			class="confirm-dialog"
			class:danger={variant === 'danger'}
			transition:scale={{ duration: 160, start: 0.96 }}
			onmousedown={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
			aria-labelledby="confirm-dialog-title"
			aria-describedby="confirm-dialog-message"
			tabindex="-1"
		>
			<div class="confirm-header">
				<div class="confirm-icon" aria-hidden="true">
					<AlertTriangle size={20} />
				</div>
				<h2 id="confirm-dialog-title">{title}</h2>
				<button class="close-btn" onclick={cancel} disabled={busy} aria-label="Close confirmation">
					<X size={18} />
				</button>
			</div>

			<p id="confirm-dialog-message">{message}</p>

			<div class="confirm-actions">
				<button type="button" class="btn-secondary" onclick={cancel} disabled={busy}>
					{cancelLabel}
				</button>
				<button
					type="button"
					class="btn-confirm"
					class:danger={variant === 'danger'}
					onclick={onConfirm}
					disabled={busy}
				>
					{busy ? 'Working...' : confirmLabel}
				</button>
			</div>
		</div>
	</div>
{/if}

<style lang="less">
	.confirm-backdrop {
		position: fixed;
		inset: 0;
		z-index: 1100;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 20px;
		background: rgba(0, 0, 0, 0.62);
		backdrop-filter: var(--glass);
		-webkit-backdrop-filter: var(--glass);
	}

	.confirm-dialog {
		width: min(420px, 100%);
		background: var(--surface);
		border: 1px solid var(--border-bright);
		border-radius: var(--radius);
		box-shadow: var(--shadow-lg);
		padding: 20px;
		color: var(--text);
	}

	.confirm-header {
		display: grid;
		grid-template-columns: auto 1fr auto;
		align-items: center;
		gap: 12px;
		margin-bottom: 12px;

		h2 {
			margin: 0;
			color: var(--text-bright);
			font-size: 18px;
			font-weight: 700;
			line-height: 1.25;
		}
	}

	.confirm-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		border-radius: var(--radius);
		background: rgba(59, 130, 246, 0.12);
		color: #60a5fa;
	}

	.confirm-dialog.danger .confirm-icon {
		background: rgba(239, 68, 68, 0.12);
		color: #f87171;
	}

	p {
		margin: 0;
		color: var(--text-dim);
		font-size: 14px;
		line-height: 1.5;
	}

	.confirm-actions {
		display: flex;
		justify-content: flex-end;
		gap: 10px;
		margin-top: 24px;
	}

	button {
		height: 38px;
		border-radius: var(--radius);
		padding: 0 14px;
		font-size: 14px;
		font-weight: 700;
		cursor: pointer;
		transition: all 0.2s;

		&:disabled {
			cursor: not-allowed;
			opacity: 0.65;
			transform: none;
		}
	}

	.close-btn {
		width: 32px;
		height: 32px;
		padding: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		border: none;
		color: var(--text-dim);

		&:hover:not(:disabled) {
			background: rgba(255, 255, 255, 0.08);
			color: var(--text-bright);
		}
	}

	.btn-secondary {
		background: var(--surface-bright);
		color: var(--text-bright);
		border: 1px solid var(--border-bright);

		&:hover:not(:disabled) {
			background: var(--border);
			transform: translateY(-1px);
		}
	}

	.btn-confirm {
		background: var(--accent);
		color: white;
		border: none;

		&:hover:not(:disabled) {
			background: var(--accent-dim);
			transform: translateY(-1px);
		}

		&.danger {
			background: #dc2626;

			&:hover:not(:disabled) {
				background: #b91c1c;
			}
		}
	}
</style>
