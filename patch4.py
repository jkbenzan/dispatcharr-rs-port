import io

with io.open('svelte-frontend/src/routes/stream-checker/+page.svelte', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Add FPS and Bitrate
patch1 = '''																				{#if stream.stream_stats?.resolution}
																					<span class="stat-text">{stream.stream_stats.resolution}</span>
																				{/if}
																				{#if stream.stream_stats?.video_codec}
																					<span class="stat-text">{stream.stream_stats.video_codec}</span>
																				{/if}
																				{#if stream.stream_stats?.source_fps}
																					<span class="stat-text">{Number(stream.stream_stats.source_fps).toFixed(0)} FPS</span>
																				{/if}
																				{#if stream.stream_stats?.video_bitrate}
																					<span class="stat-text">{Math.round(stream.stream_stats.video_bitrate)} kbps</span>
																				{/if}'''

content = content.replace(
    '''																				{#if stream.stream_stats?.resolution}
																					<span class="stat-text">{stream.stream_stats.resolution}</span>
																				{/if}
																				{#if stream.stream_stats?.video_codec}
																					<span class="stat-text">{stream.stream_stats.video_codec}</span>
																				{/if}''',
    patch1
)

# 2. Add Sort Function
patch2 = '''	async function cancelBulkCheck() {
		try {
			await api.cancelBulkCheck();
		} catch (err: any) {
			toast.error(err.message || 'Failed to cancel');
		}
	}

	async function sortSelectedChannels() {
		if (selectedStreamIds.size === 0) return;
		
		// Find which channels have any of the selected streams
		const channelIdsToSort = new Set<number>();
		for (const groupChannels of Object.values(channelsByGroup)) {
			for (const channel of groupChannels) {
				if (channel.streams) {
					for (const stream of channel.streams) {
						if (selectedStreamIds.has(stream.id)) {
							channelIdsToSort.add(channel.id);
							break;
						}
					}
				}
			}
		}

		if (channelIdsToSort.size === 0) return;

		try {
			toast.info(Sorting streams in  channels...);
			await api.bulkSortStreams(Array.from(channelIdsToSort));
			
			// Refresh those groups so the UI reflects the new sort order
			const affectedGroups = new Set<number>();
			for (const groupChannels of Object.values(channelsByGroup)) {
				for (const channel of groupChannels) {
					if (channelIdsToSort.has(channel.id)) {
						const groupId = channel.channel_group || -1;
						affectedGroups.add(groupId);
					}
				}
			}
			for (const groupId of affectedGroups) {
				await loadChannelsForGroup(groupId, 1);
			}
			toast.success('Sorting complete!');
		} catch (err: any) {
			toast.error(err.message || 'Failed to sort channels');
		}
	}'''

content = content.replace(
    '''	async function cancelBulkCheck() {
		try {
			await api.cancelBulkCheck();
		} catch (err: any) {
			toast.error(err.message || 'Failed to cancel');
		}
	}''',
    patch2
)

# 3. Add Sort Button to HTML
patch3 = '''			<div class="pane-footer">
				<div class="selection-info">
					{selectedStreamIds.size} streams selected
				</div>
				<div style="display: flex; gap: 8px;">
					<button 
						class="btn-secondary" 
						disabled={selectedStreamIds.size === 0 || status?.is_running}
						onclick={sortSelectedChannels}
					>
						<ListOrdered size={16} />
						<span>Sort Checked</span>
					</button>
					<button 
						class="btn-primary" 
						disabled={selectedStreamIds.size === 0 || status?.is_running}
						onclick={startBulkCheck}
					>
						<Play size={16} />
						<span>Start Test</span>
					</button>
				</div>
			</div>'''

content = content.replace(
    '''			<div class="pane-footer">
				<div class="selection-info">
					{selectedStreamIds.size} streams selected
				</div>
				<button 
					class="btn-primary" 
					disabled={selectedStreamIds.size === 0 || status?.is_running}
					onclick={startBulkCheck}
				>
					<Play size={16} />
					<span>Start Test</span>
				</button>
			</div>''',
    patch3
)

with io.open('svelte-frontend/src/routes/stream-checker/+page.svelte', 'w', encoding='utf-8') as f:
    f.write(content)
