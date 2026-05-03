import React, { useState } from 'react';
import { Allotment } from 'allotment';
import 'allotment/dist/style.css';
import { Box, Paper } from '@mantine/core';
import { notifications } from '@mantine/notifications';
import { DndContext, pointerWithin } from '@dnd-kit/core';
import ChannelsTreePane from './ChannelsTreePane';
import StreamsTreePane from './StreamsTreePane';
import API from '../../api';

const ChannelManagerLayout = () => {
  const [selectedChannelId, setSelectedChannelId] = useState(null);
  const [selectedStreamIds, setSelectedStreamIds] = useState([]);

  const handleToggleStreamSelection = (id) => {
    setSelectedStreamIds(prev => 
      prev.includes(id) ? prev.filter(streamId => streamId !== id) : [...prev, id]
    );
  };

  const handleSelectAllInGroup = (streams, isSelected) => {
    if (isSelected) {
      const newIds = new Set(selectedStreamIds);
      streams.forEach(s => newIds.add(s.id));
      setSelectedStreamIds([...newIds]);
    } else {
      const streamIdsToDeselect = new Set(streams.map(s => s.id));
      setSelectedStreamIds(prev => prev.filter(id => !streamIdsToDeselect.has(id)));
    }
  };

  const handleAssign = async () => {
    if (!selectedChannelId) {
      notifications.show({
        title: 'Error',
        message: 'Please select a channel on the left first.',
        color: 'red',
      });
      return;
    }

    try {
      // First, get the current streams for the selected channel
      const channelStreamsRes = await API.getChannelStreams(selectedChannelId);
      const existingStreamIds = Array.isArray(channelStreamsRes) 
        ? channelStreamsRes.map(s => s.stream_id ?? s.stream?.id ?? s.id) 
        : [];

      // Add the newly selected streams
      const newStreamIds = [...new Set([...existingStreamIds, ...selectedStreamIds])];

      await API.updateChannel({
        id: selectedChannelId,
        streams: newStreamIds,
      });

      notifications.show({
        title: 'Success',
        message: `Assigned ${selectedStreamIds.length} stream(s) to the channel.`,
        color: 'green',
      });

      // Clear selection after assignment
      setSelectedStreamIds([]);
      // You may want to trigger a refresh of the ChannelsTreePane data here if it displayed stream counts
    } catch (e) {
      console.error(e);
      // errorNotification is handled by API layer
    }
  };

  const handleDragEnd = async (event) => {
    const { active, over } = event;
    if (!over) return;
    
    // active.id is the dragged stream ID
    // over.id is the target channel ID
    const streamId = active.id;
    const targetChannelId = over.id;

    try {
      const channelStreamsRes = await API.getChannelStreams(targetChannelId);
      const existingStreamIds = Array.isArray(channelStreamsRes) 
        ? channelStreamsRes.map(s => s.stream_id ?? s.stream?.id ?? s.id) 
        : [];

      if (!existingStreamIds.includes(streamId)) {
        await API.updateChannel({
          id: targetChannelId,
          streams: [...existingStreamIds, streamId],
        });

        notifications.show({
          title: 'Success',
          message: `Assigned stream to channel.`,
          color: 'green',
        });
      }
    } catch (e) {
      console.error('Drag assignment failed:', e);
    }
  };

  return (
    <DndContext collisionDetection={pointerWithin} onDragEnd={handleDragEnd}>
      <Paper style={{ height: 'calc(100vh - 120px)', width: '100%', display: 'flex' }} radius="md" withBorder>
        <Allotment>
          <Allotment.Pane minSize={300}>
          <Box style={{ height: '100%', overflow: 'hidden' }}>
            <ChannelsTreePane 
              selectedChannelId={selectedChannelId}
              onSelectChannel={setSelectedChannelId}
            />
          </Box>
        </Allotment.Pane>
        
        <Allotment.Pane minSize={400}>
          <Box style={{ height: '100%', overflow: 'hidden', borderLeft: '1px solid rgba(255, 255, 255, 0.1)' }}>
            <StreamsTreePane 
              selectedStreamIds={selectedStreamIds}
              onToggleStreamSelection={handleToggleStreamSelection}
              onSelectAllInGroup={handleSelectAllInGroup}
              onAssign={handleAssign}
            />
          </Box>
        </Allotment.Pane>
      </Allotment>
    </Paper>
    </DndContext>
  );
};

export default ChannelManagerLayout;
