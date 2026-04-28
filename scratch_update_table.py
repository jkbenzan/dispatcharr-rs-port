import re

path = 'Dispatcharr-main/frontend/src/components/tables/ChannelsTable.jsx'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

# Add streamCheckerMode to props
content = content.replace(
    'const ChannelsTable = ({ hideLinks = false }) => {',
    'const ChannelsTable = ({ hideLinks = false, streamCheckerMode = false }) => {'
)

# Insert activeColumns before const table = useTable({
active_cols_code = """
  const activeColumns = useMemo(() => {
    if (streamCheckerMode) {
      let filtered = columns.filter(c => c.id !== 'epg' && c.id !== 'logo' && c.id !== 'actions');
      filtered.push({
        id: 'stream_aggregations',
        header: 'Stream Aggregations',
        size: 350,
        enableResizing: true,
        cell: ({ row }) => {
          const streams = row.original.streams || [];
          const total = streams.length;
          let unreachable = 0;
          let maxRes = 0;
          const providers = new Set();
          
          streams.forEach(cs => {
             if (cs.stream) {
                 if (cs.stream.m3u_playlist_id) providers.add(cs.stream.m3u_playlist_id);
                 const stats = cs.stream.custom_properties?.stream_stats;
                 if (stats) {
                     if (stats.video_resolution) {
                         const res = parseInt(stats.video_resolution, 10);
                         if (!isNaN(res) && res > maxRes) maxRes = res;
                     }
                 } else {
                     unreachable++;
                 }
             }
          });
          
          return (
            <Group gap="xs" style={{ fontSize: '12px' }}>
              <Badge size="sm" color="blue">Total: {total}</Badge>
              <Badge size="sm" color={unreachable > 0 ? "red" : "green"}>Unreachable: {unreachable}</Badge>
              <Badge size="sm" color="grape">Providers: {providers.size}</Badge>
              {maxRes > 0 && <Badge size="sm" color="teal">Max Res: {maxRes}p</Badge>}
            </Group>
          );
        }
      });
      return filtered;
    }
    return columns;
  }, [columns, streamCheckerMode]);

  const table = useTable({
    data,
    columns: activeColumns,
"""

content = content.replace(
    '  const table = useTable({\n    data,\n    columns,',
    active_cols_code
)

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print("Updated ChannelsTable.jsx successfully.")
