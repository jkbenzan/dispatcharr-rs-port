import sys

file_path = r'c:\Users\jbenz\dispatcharr-rs-port\Dispatcharr-main\frontend\src\components\tables\ChannelTableStreams.jsx'

with open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Update DraggableRow to check is_stale on nested stream
old_row_stale = "row.original.is_stale ? ' stale-stream-row' : ''"
new_row_stale = "(row.original.is_stale || row.original.stream?.is_stale) ? ' stale-stream-row' : ''"
content = content.replace(old_row_stale, new_row_stale)

# 2. Update removeStream to send stream_ids
old_remove_stream = """  const removeStream = async (stream) => {
    const newStreamList = data.filter((s) => s.id !== stream.id);
    await API.updateChannel({
      ...channel,
      streams: newStreamList.map((s) => s.id),
    });"""
new_remove_stream = """  const removeStream = async (stream) => {
    const newStreamList = data.filter((s) => s.id !== stream.id);
    await API.updateChannel({
      ...channel,
      streams: newStreamList.map((s) => s.stream_id || s.stream?.id || s.id),
    });"""
content = content.replace(old_remove_stream, new_remove_stream)

# 3. Update name column cell renderer
old_cell_renderer = """          cell: ({ row }) => {
            const stream = row.original;
            const playlistName =
              playlists[stream.m3u_account]?.name || 'Unknown';
            const accountName =
              m3uAccountsMap[stream.m3u_account] || playlistName;

            // Categorize stream stats
            const categorizedStats = categorizeStreamStats(stream.stream_stats);"""
new_cell_renderer = """          cell: ({ row }) => {
            const streamData = row.original.stream || row.original;
            const accountId = streamData.m3u_account_id || streamData.m3u_account;
            const playlistName =
              playlists[accountId]?.name || 'Unknown';
            const accountName =
              m3uAccountsMap[accountId] || playlistName;

            // Categorize stream stats
            const stats = streamData.stream_stats || streamData.custom_properties?.stream_stats;
            const categorizedStats = categorizeStreamStats(stats);"""
content = content.replace(old_cell_renderer, new_cell_renderer)

# 4. Update name and handleWatchStream usage in cell
content = content.replace("{stream.name}", "{streamData.name || 'Unknown Stream'}")
content = content.replace("stream.stream_hash || stream.id", "streamData.stream_hash || streamData.id")
content = content.replace("stream.name", "streamData.name") # For handleWatchStream second arg
content = content.replace("stream.url", "streamData.url")
content = content.replace("stream.quality", "streamData.quality")

# 5. Update stats display blocks
content = content.replace("stream.stream_stats", "stats")
content = content.replace("stream.stream_stats_updated_at", "streamData.stream_stats_updated_at || streamData.custom_properties?.stream_stats_updated_at")

# 6. Update handleDragEnd to send stream_ids
old_drag_end_update = """        API.updateChannel({
          ...channelUpdate,
          streams: retval.map((row) => row.id),
        }).then(() => {"""
new_drag_end_update = """        API.updateChannel({
          ...channelUpdate,
          streams: retval.map((row) => row.stream_id || row.stream?.id || row.id),
        }).then(() => {"""
content = content.replace(old_drag_end_update, new_drag_end_update)

with open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)
print("Updated ChannelTableStreams.jsx")
