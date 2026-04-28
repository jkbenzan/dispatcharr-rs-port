import sys

file_path = r'c:\Users\jbenz\dispatcharr-rs-port\Dispatcharr-main\frontend\src\components\tables\ChannelTableStreams.jsx'

with open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Update DraggableRow to check is_stale (it's now top-level)
content = content.replace("(row.original.is_stale || row.original.stream?.is_stale)", "row.original.is_stale")

# 2. Update removeStream to use flat s.id (which is stream ID)
old_remove_stream = """  const removeStream = async (stream) => {
    const newStreamList = data.filter((s) => s.id !== stream.id);
    await API.updateChannel({
      ...channel,
      streams: newStreamList.map((s) => s.stream_id || s.stream?.id || s.id),
    });"""
new_remove_stream = """  const removeStream = async (stream) => {
    const newStreamList = data.filter((s) => s.id !== stream.id);
    await API.updateChannel({
      ...channel,
      streams: newStreamList.map((s) => s.id),
    });"""
content = content.replace(old_remove_stream, new_remove_stream)

# 3. Update name column cell renderer (it's now flat)
old_cell_renderer = """          cell: ({ row }) => {
            const streamData = row.original.stream || row.original;
            const accountId = streamData.m3u_account_id || streamData.m3u_account;
            const playlistName =
              playlists[accountId]?.name || 'Unknown';
            const accountName =
              m3uAccountsMap[accountId] || playlistName;

            // Categorize stream stats
            const stats = streamData.stream_stats || streamData.custom_properties?.stream_stats;
            const categorizedStats = categorizeStreamStats(stats);"""
new_cell_renderer = """          cell: ({ row }) => {
            const stream = row.original;
            const accountId = stream.m3u_account_id || stream.m3u_account;
            const playlistName =
              playlists[accountId]?.name || 'Unknown';
            const accountName =
              m3uAccountsMap[accountId] || playlistName;

            // Categorize stream stats
            const stats = stream.stream_stats;
            const categorizedStats = categorizeStreamStats(stats);"""
content = content.replace(old_cell_renderer, new_cell_renderer)

# 4. Clean up variable names in JSX
content = content.replace("streamData", "stream")

# 5. Fix handleDragEnd
old_drag_end_update = """        API.updateChannel({
          ...channelUpdate,
          streams: retval.map((row) => row.stream_id || row.stream?.id || row.id),
        }).then(() => {"""
new_drag_end_update = """        API.updateChannel({
          ...channelUpdate,
          streams: retval.map((row) => row.id),
        }).then(() => {"""
content = content.replace(old_drag_end_update, new_drag_end_update)

# 6. Use channel_stream_id for row uniqueness
content = content.replace("id: row.original.id", "id: row.original.channel_stream_id || row.original.id")
content = content.replace("getRowId: (row) => row.id", "getRowId: (row) => row.channel_stream_id || row.id")

with open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)
print("Updated ChannelTableStreams.jsx (Flattened version)")
