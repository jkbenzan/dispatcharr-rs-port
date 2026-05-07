<script lang="ts">
	import { HardDrive, PlaySquare, Settings, Calendar } from 'lucide-svelte';
</script>

<div class="page-container">
	<header class="page-header">
		<div>
			<h1>DVR Management</h1>
			<p class="subtitle">Schedule recordings, manage storage, and configure comskip integration.</p>
		</div>
		<button class="btn-primary" disabled>
			<Calendar size={16} />
			<span>New Recording</span>
		</button>
	</header>

	<main class="content-area">
		<div class="empty-state">
			<div class="icon-circle primary large">
				<HardDrive size={48} />
			</div>
			<h2>DVR Engine Initializing</h2>
			<p>The Digital Video Recorder subsystem is currently undergoing architectural review. Once finalized, this dashboard will allow you to browse upcoming scheduled recordings, manage completed captures, and define storage retention policies.</p>
			
			<div class="feature-preview">
				<h3>Upcoming Features</h3>
				<div class="grid">
					<div class="preview-card">
						<PlaySquare size={20} class="text-accent" />
						<h4>Series Pass</h4>
						<span>Automatically schedule recordings for all new episodes of your favorite shows.</span>
					</div>
					<div class="preview-card">
						<Settings size={20} class="text-accent" />
						<h4>Comskip Integration</h4>
						<span>Automatically detect and remove commercial breaks from completed recordings.</span>
					</div>
				</div>
			</div>
		</div>
	</main>
</div>

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

	.btn-primary {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--accent);
		color: white;
		border: none;
		padding: 10px 24px;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 14px;
		cursor: pointer;
		transition: all 0.2s;

		&:hover:not(:disabled) {
			background: var(--accent-dim);
		}

		&:disabled {
			opacity: 0.5;
			cursor: not-allowed;
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

	.empty-state {
		margin: auto;
		max-width: 650px;
		padding: 40px;
		text-align: center;
		display: flex;
		flex-direction: column;
		align-items: center;

		h2 {
			margin: 24px 0 12px;
			color: var(--text-bright);
			font-size: 24px;
		}

		p {
			color: var(--text-dim);
			line-height: 1.6;
			margin-bottom: 40px;
		}
	}

	.icon-circle {
		width: 64px;
		height: 64px;
		border-radius: 50%;
		background: rgba(255, 255, 255, 0.05);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-dim);
		border: 1px solid var(--border);

		&.primary {
			background: var(--accent-transparent);
			color: var(--accent);
			border-color: rgba(237, 28, 36, 0.2);
		}

		&.large {
			width: 96px;
			height: 96px;
		}
	}

	.feature-preview {
		width: 100%;
		border-top: 1px solid var(--border);
		padding-top: 32px;

		h3 {
			font-size: 14px;
			text-transform: uppercase;
			letter-spacing: 1px;
			color: var(--text-dim);
			margin-bottom: 24px;
		}

		.grid {
			display: grid;
			grid-template-columns: 1fr 1fr;
			gap: 16px;
		}

		.preview-card {
			background: rgba(0, 0, 0, 0.2);
			border: 1px solid var(--border);
			border-radius: var(--radius);
			padding: 20px;
			text-align: left;
			display: flex;
			flex-direction: column;
			gap: 8px;

			h4 {
				margin: 0;
				color: var(--text-bright);
			}

			span {
				font-size: 13px;
				color: var(--text-dim);
				line-height: 1.5;
			}
		}
	}

	:global(.text-accent) {
		color: var(--accent) !important;
	}
</style>
