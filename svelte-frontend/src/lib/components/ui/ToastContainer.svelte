<script lang="ts">
	import { toast } from '$lib/toast.svelte';
	import { fade, fly, slide } from 'svelte/transition';
	import { flip } from 'svelte/animate';
	import { Info, CheckCircle2, AlertTriangle, XCircle, X } from 'lucide-svelte';

	const icons = {
		info: Info,
		success: CheckCircle2,
		warning: AlertTriangle,
		error: XCircle
	};
</script>

<div class="toast-container">
	{#each toast.toasts as t (t.id)}
		<div
			class="toast {t.type}"
			in:fly={{ y: 20, duration: 400, opacity: 0 }}
			out:fade={{ duration: 200 }}
			animate:flip={{ duration: 400 }}
		>
			<div class="icon-container">
				<svelte:component this={icons[t.type]} size={20} />
			</div>
			<div class="message">{t.message}</div>
			<button class="close-btn" onclick={() => toast.remove(t.id)}>
				<X size={16} />
			</button>
			<div class="progress-bar">
				<div 
					class="progress-fill" 
					style="animation-duration: {t.duration}ms"
				></div>
			</div>
		</div>
	{/each}
</div>

<style lang="less">
	.toast-container {
		position: fixed;
		bottom: 24px;
		right: 24px;
		display: flex;
		flex-direction: column;
		gap: 12px;
		z-index: 9999;
		pointer-events: none;
		max-width: 400px;
	}

	.toast {
		pointer-events: auto;
		display: flex;
		align-items: flex-start;
		gap: 12px;
		padding: 16px;
		background: var(--glass-bg);
		backdrop-filter: blur(12px);
		-webkit-backdrop-filter: blur(12px);
		border: 1px solid var(--border);
		border-radius: 12px;
		box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
		color: var(--text-bright);
		position: relative;
		overflow: hidden;
		min-width: 300px;

		&.info {
			.icon-container { color: #3b82f6; }
			.progress-fill { background: #3b82f6; }
		}
		&.success {
			.icon-container { color: #10b981; }
			.progress-fill { background: #10b981; }
		}
		&.warning {
			.icon-container { color: #f59e0b; }
			.progress-fill { background: #f59e0b; }
		}
		&.error {
			.icon-container { color: #ef4444; }
			.progress-fill { background: #ef4444; }
		}
	}

	.icon-container {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		margin-top: 2px;
	}

	.message {
		flex: 1;
		font-size: 14px;
		font-weight: 500;
		line-height: 1.5;
		color: var(--text-bright);
	}

	.close-btn {
		background: transparent;
		border: none;
		color: var(--text-dim);
		cursor: pointer;
		padding: 2px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 4px;
		transition: all 0.2s;

		&:hover {
			background: rgba(255, 255, 255, 0.1);
			color: var(--text-bright);
		}
	}

	.progress-bar {
		position: absolute;
		bottom: 0;
		left: 0;
		width: 100%;
		height: 3px;
		background: rgba(255, 255, 255, 0.05);
	}

	.progress-fill {
		height: 100%;
		width: 0%;
		animation: progress-drain linear forwards;
	}

	@keyframes progress-drain {
		from { width: 100%; }
		to { width: 0%; }
	}
</style>
