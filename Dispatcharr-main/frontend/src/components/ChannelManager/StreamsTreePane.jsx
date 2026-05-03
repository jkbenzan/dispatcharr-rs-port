import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { Box, Accordion, Loader, Text, Group, Select, TextInput, ScrollArea, Checkbox, ActionIcon, Button, Flex } from '@mantine/core';
import { Search, Eye, GripVertical } from 'lucide-react';
import { useDraggable } from '@dnd-kit/core';
import { CSS } from '@dnd-kit/utilities';
import { FixedSizeList as List } from 'react-window';
import AutoSizer from 'react-virtualized-auto-sizer';
import API from '../../api';

const DraggableStream = ({ stream, isSelected, onToggle }) => {
  const { attributes, listeners, setNodeRef, transform } = useDraggable({
    id: stream.id,
    data: { stream },
  });

  const style = {
    transform: CSS.Translate.toString(transform),
    borderBottom: '1px solid rgba(255, 255, 255, 0.05)',
    backgroundColor: isSelected ? 'rgba(51, 154, 240, 0.2)' : 'transparent',
  };

  return (
    <Box ref={setNodeRef} style={style} p="xs">
      <Flex align="center" gap="sm">
        <ActionIcon size="sm" variant="subtle" {...listeners} {...attributes} style={{ cursor: 'grab' }}>
          <GripVertical size={14} />
        </ActionIcon>
        <Checkbox 
          checked={isSelected}
          onChange={() => onToggle(stream.id)}
        />
        <Box style={{ flex: 1, overflow: 'hidden' }}>
          <Text size="sm" truncate>{stream.name}</Text>
        </Box>
        <ActionIcon size="sm" variant="subtle">
          <Eye size={14} />
        </ActionIcon>
      </Flex>
    </Box>
  );
};

const StreamsTreePane = ({ selectedStreamIds, onToggleStreamSelection, onSelectAllInGroup, onAssign }) => {
  const [m3uAccounts, setM3uAccounts] = useState([]);
  const [streamGroups, setStreamGroups] = useState([]);
  const [selectedM3u, setSelectedM3u] = useState('all');
  const [selectedGroup, setSelectedGroup] = useState('all');
  const [searchQuery, setSearchQuery] = useState('');
  
  const [loading, setLoading] = useState(true);
  const [loadedGroups, setLoadedGroups] = useState({});
  const [streamsData, setStreamsData] = useState({});
  const [accordionValue, setAccordionValue] = useState([]);

  useEffect(() => {
    const fetchFilters = async () => {
      try {
        const [m3uRes, filterRes] = await Promise.all([
          API.getPlaylists(),
          API.getStreamFilterOptions(new URLSearchParams())
        ]);
        
        const m3us = Array.isArray(m3uRes) ? m3uRes : m3uRes?.results || [];
        setM3uAccounts(m3us.filter(m => m.id).map(m => ({ value: String(m.id), label: String(m.name || m.id) })));
        
        const groups = filterRes?.groups || [];
        setStreamGroups(groups.map(g => {
          const name = typeof g === 'string' ? g : g.name;
          return name ? { value: String(name), label: String(name) } : null;
        }).filter(Boolean));
      } catch (e) {
        console.error(e);
      } finally {
        setLoading(false);
      }
    };
    fetchFilters();
  }, []);

  // Update groups when M3U provider changes
  useEffect(() => {
    const fetchGroupsForM3u = async () => {
      if (selectedM3u !== 'all') {
        const params = new URLSearchParams();
        params.set('m3u_account', selectedM3u);
        const filterRes = await API.getStreamFilterOptions(params);
        const groups = filterRes?.groups || [];
        setStreamGroups(groups.map(g => {
          const name = typeof g === 'string' ? g : g.name;
          return name ? { value: String(name), label: String(name) } : null;
        }).filter(Boolean));
      } else {
        const filterRes = await API.getStreamFilterOptions(new URLSearchParams());
        const groups = filterRes?.groups || [];
        setStreamGroups(groups.map(g => {
          const name = typeof g === 'string' ? g : g.name;
          return name ? { value: String(name), label: String(name) } : null;
        }).filter(Boolean));
      }
    };
    fetchGroupsForM3u();
  }, [selectedM3u]);

  const loadGroupStreams = async (groupName) => {
    if (!loadedGroups[groupName]) {
      setLoadedGroups(prev => ({ ...prev, [groupName]: true }));
      try {
        const params = new URLSearchParams();
        if (selectedM3u !== 'all') params.set('m3u_account', selectedM3u);
        params.set('channel_group', groupName);
        params.set('page_size', '1000');
        const response = await API.queryStreams(params);
        const streams = Array.isArray(response) ? response : response?.results || [];
        setStreamsData(prev => ({ ...prev, [groupName]: streams }));
      } catch (e) {
        console.error(e);
      }
    }
  };

  const handleAccordionChange = (values) => {
    setAccordionValue(values);
    values.forEach(groupName => {
      loadGroupStreams(groupName);
    });
  };

  const handleSearch = async (e) => {
    if (e.key === 'Enter' && searchQuery) {
      // Find streams matching the search
      const params = new URLSearchParams();
      params.set('search', searchQuery);
      if (selectedM3u !== 'all') params.set('m3u_account', selectedM3u);
      
      const response = await API.queryStreams(params);
      const streams = Array.isArray(response) ? response : response?.results || [];
      
      // Auto-expand groups containing these streams
      const groupsToExpand = [...new Set(streams.map(s => s.channel_group))].filter(Boolean);
      
      const newData = { ...streamsData };
      const newLoaded = { ...loadedGroups };
      
      groupsToExpand.forEach(groupName => {
        newLoaded[groupName] = true;
        newData[groupName] = streams.filter(s => s.channel_group === groupName);
      });
      
      setLoadedGroups(newLoaded);
      setStreamsData(newData);
      setAccordionValue(prev => [...new Set([...prev, ...groupsToExpand])]);
    }
  };

  if (loading) return <Loader size="sm" />;

  const displayedGroups = selectedGroup === 'all' 
    ? streamGroups 
    : streamGroups.filter(g => g.value === selectedGroup);

  return (
    <Box style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <Box p="sm" style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.1)' }}>
        <Group justify="space-between" mb="xs">
          <Text fw={700}>STREAMS</Text>
          <ActionIcon variant="subtle"><Search size={16} /></ActionIcon>
        </Group>
        
        <TextInput 
          placeholder="Search streams (press Enter)..." 
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onKeyDown={handleSearch}
          mb="sm"
          size="sm"
        />
        
        <Group grow gap="xs">
          <Select 
            placeholder="All Providers" 
            data={[{ value: 'all', label: 'All Providers' }, ...m3uAccounts]}
            value={selectedM3u}
            onChange={setSelectedM3u}
            size="sm"
          />
          <Select 
            placeholder="All Groups" 
            data={[{ value: 'all', label: 'All Groups' }, ...streamGroups]}
            value={selectedGroup}
            onChange={setSelectedGroup}
            size="sm"
          />
        </Group>
      </Box>

      <ScrollArea style={{ flex: 1 }}>
        <Accordion multiple value={accordionValue} onChange={handleAccordionChange}>
          {displayedGroups.map((group) => {
            const streams = streamsData[group.value] || [];
            const isLoaded = loadedGroups[group.value];
            
            // Check if all are selected
            const allSelected = streams.length > 0 && streams.every(s => selectedStreamIds.includes(s.id));
            const someSelected = streams.some(s => selectedStreamIds.includes(s.id));

            return (
              <Accordion.Item key={group.value} value={group.value}>
                <Accordion.Control>
                  <Text fw={600}>{group.label}</Text>
                </Accordion.Control>
                <Accordion.Panel>
                  {isLoaded ? (
                    <Box>
                      <Group justify="space-between" mb="xs" px="xs">
                        <Checkbox 
                          label="Select All" 
                          size="xs" 
                          checked={allSelected}
                          indeterminate={someSelected && !allSelected}
                          onChange={() => onSelectAllInGroup(streams, !allSelected)}
                        />
                        <Button size="compact-xs" onClick={onAssign} disabled={selectedStreamIds.length === 0}>
                          Assign Selected ({selectedStreamIds.length})
                        </Button>
                      </Group>
                      {streams.length === 0 ? (
                        <Text size="sm" c="dimmed" px="xs">No streams found.</Text>
                      ) : (
                        <Box style={{ height: Math.min(streams.length * 40, 400), width: '100%' }}>
                          <AutoSizer>
                            {({ height, width }) => (
                              <List
                                height={height}
                                itemCount={streams.length}
                                itemSize={40}
                                width={width}
                                itemData={{
                                  streams,
                                  selectedStreamIds,
                                  onToggleStreamSelection
                                }}
                              >
                                {({ index, style, data }) => {
                                  const stream = data.streams[index];
                                  return (
                                    <div style={style}>
                                      <DraggableStream 
                                        stream={stream} 
                                        isSelected={data.selectedStreamIds.includes(stream.id)}
                                        onToggle={data.onToggleStreamSelection}
                                      />
                                    </div>
                                  );
                                }}
                              </List>
                            )}
                          </AutoSizer>
                        </Box>
                      )}
                    </Box>
                  ) : (
                    <Loader size="xs" />
                  )}
                </Accordion.Panel>
              </Accordion.Item>
            );
          })}
        </Accordion>
      </ScrollArea>
    </Box>
  );
};

export default StreamsTreePane;
