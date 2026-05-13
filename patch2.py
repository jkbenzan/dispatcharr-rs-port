import io

with io.open('svelte-frontend/src/routes/stream-checker/+page.svelte', 'r', encoding='utf-8') as f:
    content = f.read()

patch1 = '''	async function checkStatus() {
		try {
			const res = await api.getBulkCheckStatus();
			status = res;

			if (status?.last_results?.length > 0) {
				let mutated = false;
				for (const groupChannels of Object.values(channelsByGroup)) {
					for (const channel of groupChannels) {
						if (channel.streams) {
							for (const stream of channel.streams) {
								const result = status.last_results.find((r: any) => r.id === stream.id);
								if (result && result.stream_stats) {
									// only mutate if different
									if (JSON.stringify(stream.stream_stats) !== JSON.stringify(result.stream_stats)) {
										stream.stream_stats = result.stream_stats;
										stream.stream_stats_updated_at = new Date().toISOString();
										mutated = true;
									}
								}
							}
						}
					}
				}
				if (mutated) {
					channelsByGroup = { ...channelsByGroup };
				}
			}

			if (status.is_running && !pollingInterval) {'''

content = content.replace(
    '''	async function checkStatus() {\n		try {\n			const res = await api.getBulkCheckStatus();\n			status = res;\n\n			if (status.is_running && !pollingInterval) {''',
    patch1
)

patch2 = '''																{#each channel.streams as stream}
																	{@const isSelected = isStreamSelected(stream.id)}
																	{@const isTesting = status?.workers?.some(w => w.current_stream_id === stream.id)}
																	<div class="tree-row stream-row" class:selected={isSelected} onclick={(e) => !status?.is_running && toggleStreamSelection(stream.id, e)}>
																		<div class="stream-drag-spacer"></div>
																		<div class="checkbox-wrapper">
																			<input type="checkbox" checked={isSelected} onchange={(e) => toggleStreamSelection(stream.id, e)} disabled={status?.is_running} />
																		</div>
																		<div class="stream-info">
																			<div class="stream-main">
																				<span class="stream-name" title={stream.name}>{stream.name}</span>
																				<span class="stream-provider">{stream.m3u_account_name || 'Custom'}</span>
																			</div>
																			<div class="stream-stats">
																				{#if isTesting}
																					<span class="badge testing"><RefreshCw class="spin" size={10} style="margin-right:4px;" />Testing...</span>
																				{:else if stream.stream_stats?.status === 'online'}'''

content = content.replace(
    '''																{#each channel.streams as stream}
																	{@const isSelected = isStreamSelected(stream.id)}
																	<div class="tree-row stream-row" class:selected={isSelected} onclick={(e) => !status?.is_running && toggleStreamSelection(stream.id, e)}>
																		<div class="stream-drag-spacer"></div>
																		<div class="checkbox-wrapper">
																			<input type="checkbox" checked={isSelected} onchange={(e) => toggleStreamSelection(stream.id, e)} disabled={status?.is_running} />
																		</div>
																		<div class="stream-info">
																			<div class="stream-main">
																				<span class="stream-name" title={stream.name}>{stream.name}</span>
																				<span class="stream-provider">{stream.m3u_account_name || 'Custom'}</span>
																			</div>
																			<div class="stream-stats">
																				{#if stream.stream_stats?.status === 'online'}''',
    patch2
)

patch3 = '''	.badge {
		padding: 2px 6px;
		border-radius: 4px;
		font-size: 10px;
		font-weight: 500;
		text-transform: uppercase;
		display: inline-flex;
		align-items: center;

		&.testing {
			background: rgba(59, 130, 246, 0.2);
			color: #60a5fa;
			border: 1px solid rgba(59, 130, 246, 0.3);
		}
		
		&.success {'''

content = content.replace(
    '''	.badge {
		padding: 2px 6px;
		border-radius: 4px;
		font-size: 10px;
		font-weight: 500;
		text-transform: uppercase;
		
		&.success {''',
    patch3
)

with io.open('svelte-frontend/src/routes/stream-checker/+page.svelte', 'w', encoding='utf-8') as f:
    f.write(content)
