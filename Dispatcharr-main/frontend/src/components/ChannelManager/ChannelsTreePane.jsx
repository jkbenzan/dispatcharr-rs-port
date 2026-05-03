import React, { useState, useEffect } from 'react';
import { Box, Accordion, Loader, Text, Group, ActionIcon, ScrollArea } from '@mantine/core';
import { Search } from 'lucide-react';
import { useDroppable } from '@dnd-kit/core';
import API from '../../api';

const DroppableChannel = ({ channel, isSelected, onSelectChannel }) => {
  const { setNodeRef, isOver } = useDroppable({ id: channel.id });

  return (
    <Box
      ref={setNodeRef}
      p="xs"
      style={{
        cursor: 'pointer',
        backgroundColor: isOver ? 'rgba(51, 154, 240, 0.4)' : isSelected ? 'rgba(51, 154, 240, 0.2)' : 'transparent',
        borderBottom: '1px solid rgba(255, 255, 255, 0.05)'
      }}
      onClick={() => onSelectChannel(channel.id)}
    >
      <Group justify="space-between">
        <Text size="sm">{channel.name}</Text>
        <Text size="xs" c="dimmed">#{channel.channel_number}</Text>
      </Group>
    </Box>
  );
};

const ChannelsTreePane = ({ selectedChannelId, onSelectChannel }) => {
  const [channelGroups, setChannelGroups] = useState([]);
  const [loading, setLoading] = useState(true);
  const [loadedGroups, setLoadedGroups] = useState({});
  const [channelsData, setChannelsData] = useState({});

  useEffect(() => {
    const fetchGroups = async () => {
      try {
        const response = await API.getChannelGroups();
        // Assume response is an array of groups, or response.results
        const groups = Array.isArray(response) ? response : response.results || [];
        setChannelGroups(groups);
      } catch (e) {
        console.error(e);
      } finally {
        setLoading(false);
      }
    };
    fetchGroups();
  }, []);

  const handleAccordionChange = async (values) => {
    // Find newly opened groups
    values.forEach(async (groupId) => {
      if (!loadedGroups[groupId]) {
        setLoadedGroups((prev) => ({ ...prev, [groupId]: true }));
        try {
          const params = new URLSearchParams();
          params.set('channel_group', groupId);
          params.set('page_size', '1000'); // Load a reasonable chunk
          const response = await API.queryChannels(params);
          const channels = Array.isArray(response) ? response : response.results || [];
          setChannelsData((prev) => ({ ...prev, [groupId]: channels }));
        } catch (e) {
          console.error(e);
        }
      }
    });
  };

  if (loading) return <Loader size="sm" />;

  return (
    <Box style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <Box p="sm" style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.1)' }}>
        <Text fw={700} mb="xs">CHANNELS</Text>
        {/* Placeholder for search */}
      </Box>
      <ScrollArea style={{ flex: 1 }}>
        <Accordion multiple onChange={handleAccordionChange}>
          {channelGroups.map((group) => (
            <Accordion.Item key={group.id} value={String(group.id)}>
              <Accordion.Control>
                <Text fw={600}>{group.name}</Text>
              </Accordion.Control>
              <Accordion.Panel>
                {loadedGroups[group.id] ? (
                  channelsData[group.id] ? (
                    channelsData[group.id].map((channel) => (
                      <DroppableChannel 
                        key={channel.id} 
                        channel={channel} 
                        isSelected={selectedChannelId === channel.id}
                        onSelectChannel={onSelectChannel} 
                      />
                    ))
                  ) : (
                    <Loader size="xs" />
                  )
                ) : null}
              </Accordion.Panel>
            </Accordion.Item>
          ))}
        </Accordion>
      </ScrollArea>
    </Box>
  );
};

export default ChannelsTreePane;
