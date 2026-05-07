<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { wsStore, connectWS } from '$lib/ws.svelte';
	import { Terminal, Play, Pause, Trash2, ChevronDown, ChevronRight, Activity } from 'lucide-svelte';

	let historicalEvents: any[] = $state([]);
	let isPaused = $state(false);
	let autoScroll = $state(true);
	let terminalRef: HTMLElement;
	
	let expandedRows = $state<Record<number, boolean>>({});

	function toggleExpand(id: number) {
		expandedRows[id] = !expandedRows[id];
	}

	onMount(async () => {
		connectWS();
		try {
			const res = await api.getSystemEvents(200, 0);
			if (res.events) {
				historicalEvents = res.events.reverse();
			}
		} catch (e) {
			console.error('Failed to load historical events', e);
		}
	});

	// Derive the merged log view
	let mergedEvents = $derived(() => {
		let events = [...historicalEvents];
		
		// Map WebSocket messages into standard format if necessary
		for (const msg of wsStore.messages) {
			// WS pushes { type: "system_event", event: { id, event_type, ... } }
			if (msg.type === 'system_event' && msg.event) {
				events.push(msg.event);
			} else if (msg.event_type && msg.timestamp) {
				// Direct push
				events.push(msg);
			}
		}
		
		events.sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());
		
		const uniqueEvents = [];
		const seen = new Set();
		for (const e of events) {
			if (!seen.has(e.id)) {
				seen.add(e.id);
				uniqueEvents.push(e);
			}
		}
		return uniqueEvents;
	});

	$effect(() => {
		// Auto scroll logic
		if (autoScroll && !isPaused && terminalRef) {
			const length = mergedEvents().length;
			setTimeout(() => {
				terminalRef.scrollTop = terminalRef.scrollHeight;
			}, 50);
		}
	});

	function clearLogs() {
		historicalEvents = [];
		wsStore.messages = [];
	}

	function getSeverityColor(type: string): string {
		const t = type.toLowerCase();
		if (t.includes('error') || t.includes('fail') || t.includes('timeout')) return 'var(--accent)';
		if (t.includes('warn')) return '#facc15';
		if (t.includes('success') || t.includes('done') || t.includes('complete')) return '#4ade80';
		return '#60a5fa'; // default info blue
	}

	import { uiSettings } from '$lib/settings.svelte';

	function formatDate(isoString: string): string {
		const d = new Date(isoString);
		
		let timeStr = '';
		try {
			timeStr = d.toLocaleTimeString([], { 
				hour12: uiSettings.time_format === '12h',
				timeZone: uiSettings.time_zone !== 'UTC' ? uiSettings.time_zone : undefined
			});
		} catch (e) {
			timeStr = d.toLocaleTimeString([], { hour12: uiSettings.time_format === '12h' });
		}
		
		return timeStr + '.' + d.getMilliseconds().toString().padStart(3, '0');
	}

	function formatDetails(details: any): string {
		try {
			return JSON.stringify(details, null, 2);
		} catch {
			return String(details);
		}
	}
</script>

<div class="page-container">
	<header class="page-header">
		<div>
			<h1>Activity & Logs</h1>
			<p class="subtitle">Real-time system events, parser logs, and background worker statuses.</p>
		</div>

		<div class="header-actions">
			<button class="btn-secondary" onclick={() => autoScroll = !autoScroll} class:active={autoScroll}>
				<Activity size={16} />
				<span>{autoScroll ? 'Auto-Scroll On' : 'Auto-Scroll Off'}</span>
			</button>
			<button class="btn-secondary" onclick={() => isPaused = !isPaused} class:paused={isPaused}>
				{#if isPaused}
					<Play size={16} />
					<span>Resume</span>
				{:else}
					<Pause size={16} />
					<span>Pause</span>
				{/if}
			</button>
			<button class="btn-danger" onclick={clearLogs}>
				<Trash2 size={16} />
				<span>Clear</span>
			</button>
		</div>
	</header>

	<div class="terminal-wrapper">
		<div class="terminal-header">
			<Terminal size={16} />
			<span>dispatcharr-core ~ /var/log/system</span>
			<div class="status-indicator" class:connected={wsStore.connected}>
				<div class="dot"></div>
				<span>{wsStore.connected ? 'Streaming live' : 'Disconnected'}</span>
			</div>
		</div>

		<div class="terminal-body" bind:this={terminalRef}>
			{#if mergedEvents().length === 0}
				<div class="empty-state">No events logged yet.</div>
			{:else}
				{#each mergedEvents() as event}
					{@const color = getSeverityColor(event.event_type)}
					<div class="log-entry">
						<div class="log-line" onclick={() => toggleExpand(event.id)}>
							<span class="timestamp">[{formatDate(event.timestamp)}]</span>
							<span class="severity" style="color: {color}">[{event.event_type.toUpperCase()}]</span>
							{#if event.channel_name}
								<span class="channel-name">[{event.channel_name}]</span>
							{/if}
							
							<span class="log-message">
								{#if event.details && event.details.message}
									{event.details.message}
								{:else if event.details && typeof event.details === 'string'}
									{event.details}
								{:else}
									Raw Event Data
								{/if}
							</span>

							<button class="expand-btn">
								{#if expandedRows[event.id]}
									<ChevronDown size={14} />
								{:else}
									<ChevronRight size={14} />
								{/if}
							</button>
						</div>

						{#if expandedRows[event.id]}
							<div class="log-details">
								<pre>{formatDetails(event.details)}</pre>
							</div>
						{/if}
					</div>
				{/each}
			{/if}
		</div>
	</div>
</div>

<style lang="less">
	.page-container {
		display: flex;
		flex-direction: column;
		gap: 20px;
		height: 100%;
		overflow: hidden;
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-end;
		flex-shrink: 0;

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

	.header-actions {
		display: flex;
		gap: 12px;

		button {
			display: flex;
			align-items: center;
			gap: 8px;
			padding: 8px 16px;
			border-radius: var(--radius);
			font-size: 13px;
			font-weight: 600;
			border: none;
			cursor: pointer;
			transition: all 0.2s;
		}

		.btn-secondary {
			background: var(--surface-bright);
			color: var(--text);
			border: 1px solid var(--border);

			&:hover {
				background: rgba(255, 255, 255, 0.1);
				color: var(--text-bright);
			}

			&.active {
				background: rgba(74, 222, 128, 0.1);
				color: #4ade80;
				border-color: #4ade80;
			}

			&.paused {
				background: rgba(250, 204, 21, 0.1);
				color: #facc15;
				border-color: #facc15;
			}
		}

		.btn-danger {
			background: rgba(237, 28, 36, 0.1);
			color: var(--accent);
			border: 1px solid rgba(237, 28, 36, 0.2);

			&:hover {
				background: var(--accent);
				color: white;
			}
		}
	}

	.terminal-wrapper {
		flex: 1;
		display: flex;
		flex-direction: column;
		background: #0f111a;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		overflow: hidden;
		box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
	}

	.terminal-header {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px 16px;
		background: #1a1d27;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
		color: var(--text-dim);
		font-family: monospace;
		font-size: 13px;

		.status-indicator {
			margin-left: auto;
			display: flex;
			align-items: center;
			gap: 8px;
			font-family: 'Inter', sans-serif;
			font-size: 12px;
			color: var(--text-dim);

			.dot {
				width: 8px;
				height: 8px;
				border-radius: 50%;
				background: var(--text-dim);
			}

			&.connected {
				color: #4ade80;
				.dot {
					background: #4ade80;
					box-shadow: 0 0 8px rgba(74, 222, 128, 0.6);
				}
			}
		}
	}

	.terminal-body {
		flex: 1;
		padding: 16px;
		overflow-y: auto;
		font-family: 'JetBrains Mono', 'Fira Code', 'Courier New', monospace;
		font-size: 13px;
		line-height: 1.5;
		color: #a6accd;

		/* Custom scrollbar for terminal */
		&::-webkit-scrollbar { width: 10px; }
		&::-webkit-scrollbar-track { background: #0f111a; }
		&::-webkit-scrollbar-thumb { background: #292d3e; border-radius: 5px; border: 2px solid #0f111a; }
		&::-webkit-scrollbar-thumb:hover { background: #3e445e; }
	}

	.empty-state {
		text-align: center;
		color: #4c566a;
		font-style: italic;
		padding: 40px;
	}

	.log-entry {
		margin-bottom: 4px;
		border-radius: 4px;
		
		&:hover .log-line {
			background: rgba(255, 255, 255, 0.03);
		}
	}

	.log-line {
		display: flex;
		align-items: flex-start;
		gap: 12px;
		padding: 4px 8px;
		cursor: pointer;
		border-radius: 4px;
		transition: background 0.1s;

		.timestamp {
			color: #4c566a;
			white-space: nowrap;
		}

		.severity {
			font-weight: 600;
			white-space: nowrap;
			min-width: 90px;
		}

		.channel-name {
			color: #c792ea;
			white-space: nowrap;
		}

		.log-message {
			flex: 1;
			color: #a6accd;
			word-break: break-word;
		}

		.expand-btn {
			background: transparent;
			border: none;
			color: #4c566a;
			cursor: pointer;
			padding: 2px;
			display: flex;
			align-items: center;
			justify-content: center;
			border-radius: 4px;

			&:hover {
				background: rgba(255, 255, 255, 0.1);
				color: #a6accd;
			}
		}
	}

	.log-details {
		margin: 4px 8px 12px 120px;
		padding: 12px;
		background: #1a1d27;
		border: 1px solid rgba(255, 255, 255, 0.05);
		border-radius: var(--radius);

		pre {
			margin: 0;
			color: #82aaff;
			white-space: pre-wrap;
			word-break: break-all;
			font-family: inherit;
		}
	}
</style>
