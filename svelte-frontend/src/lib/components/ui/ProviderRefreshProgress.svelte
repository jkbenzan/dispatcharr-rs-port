<script lang="ts">
	let {
		info,
		formatDuration
	}: {
		info: {
			progress: number;
			elapsedMs: number;
			etaMs: number | null;
			status: string;
			message: string;
		} | null;
		formatDuration: (ms: number | null) => string;
	} = $props();
</script>

{#if info}
	<div class="refresh-progress" class:queued={info.status === 'queued'}>
		<div class="progress-meta">
			<span>{Math.round(info.progress)}% {info.status === 'queued' ? 'queued' : 'refreshing'}</span>
			<span>Elapsed {formatDuration(info.elapsedMs)} / ETA {formatDuration(info.etaMs)}</span>
		</div>
		<div
			class="progress-track"
			aria-label="Refresh progress"
			aria-valuemin="0"
			aria-valuemax="100"
			aria-valuenow={Math.round(info.progress)}
			role="progressbar"
		>
			<div class="progress-fill" style:width={`${info.progress}%`}></div>
		</div>
	</div>
{/if}

<style lang="less">
	.refresh-progress {
		display: flex;
		flex-direction: column;
		gap: 6px;

		.progress-meta {
			display: flex;
			align-items: center;
			justify-content: space-between;
			gap: 12px;
			color: var(--text-dim);
			font-size: 11px;
			font-weight: 600;

			span:first-child {
				color: #38bdf8;
				text-transform: capitalize;
			}
		}

		.progress-track {
			height: 7px;
			width: 100%;
			overflow: hidden;
			border-radius: 999px;
			background: rgba(255, 255, 255, 0.09);
			border: 1px solid rgba(255, 255, 255, 0.08);
		}

		.progress-fill {
			height: 100%;
			min-width: 8px;
			border-radius: inherit;
			background: linear-gradient(90deg, #38bdf8, #4ade80);
			transition: width 0.45s ease;
		}

		&.queued {
			.progress-meta span:first-child {
				color: #facc15;
			}

			.progress-fill {
				background: linear-gradient(90deg, #facc15, #38bdf8);
			}
		}
	}
</style>
