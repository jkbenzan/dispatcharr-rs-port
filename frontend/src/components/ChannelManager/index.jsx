import React, { useState, useRef, useCallback } from 'react';
import { notifications } from '@mantine/notifications';
import { DndContext, pointerWithin } from '@dnd-kit/core';
import ChannelsTreePane from './ChannelsTreePane';
import StreamsTreePane from './StreamsTreePane';
import API from '../../api';
import './ChannelManager.css';

// Simple resizable divider state
const MIN_LEFT = 200;
const MAX_LEFT = 600;
const DEFAULT_LEFT = 320;

const ChannelManagerLayout = () => {
  const [selectedChannelId, setSelectedChannelId] = useState(null);
  const [selectedStreamIds, setSelectedStreamIds] = useState([]);
  const [leftWidth, setLeftWidth] = useState(DEFAULT_LEFT);
  const isDraggingDivider = useRef(false);
  const containerRef = useRef(null);

  // --- Stream selection handlers ---
  const handleToggleStreamSelection = useCallback((id) => {
    setSelectedStreamIds((prev) =>
      prev.includes(id) ? prev.filter((sid) => sid !== id) : [...prev, id]
    );
  }, []);

  const handleSelectAllInGroup = useCallback((streams, selectAll) => {
    if (selectAll) {
      setSelectedStreamIds((prev) => [...new Set([...prev, ...streams.map((s) => s.id)])]);
    } else {
      const toRemove = new Set(streams.map((s) => s.id));
      setSelectedStreamIds((prev) => prev.filter((id) => !toRemove.has(id)));
    }
  }, []);

  // --- Assign button handler ---
  const handleAssign = useCallback(async () => {
    if (!selectedChannelId) {
      notifications.show({
        title: 'No channel selected',
        message: 'Click a channel on the left panel first, then assign streams.',
        color: 'orange',
      });
      return;
    }
    if (selectedStreamIds.length === 0) return;

    try {
      const channelStreamsRes = await API.getChannelStreams(selectedChannelId);
      const existingIds = Array.isArray(channelStreamsRes)
        ? channelStreamsRes.map((s) => s.stream_id ?? s.stream?.id ?? s.id)
        : [];

      const newIds = [...new Set([...existingIds, ...selectedStreamIds])];

      await API.updateChannel({ id: selectedChannelId, streams: newIds });

      notifications.show({
        title: 'Streams assigned',
        message: `Assigned ${selectedStreamIds.length} stream(s) to the channel.`,
        color: 'green',
      });

      setSelectedStreamIds([]);
    } catch (e) {
      console.error('[ChannelManager] assign error:', e);
    }
  }, [selectedChannelId, selectedStreamIds]);

  // --- Drag-and-drop handler ---
  const handleDragEnd = useCallback(async ({ active, over }) => {
    if (!over) return;
    const streamId = active.id;
    const targetChannelId = over.id;

    try {
      const channelStreamsRes = await API.getChannelStreams(targetChannelId);
      const existingIds = Array.isArray(channelStreamsRes)
        ? channelStreamsRes.map((s) => s.stream_id ?? s.stream?.id ?? s.id)
        : [];

      if (!existingIds.includes(streamId)) {
        await API.updateChannel({
          id: targetChannelId,
          streams: [...existingIds, streamId],
        });
        notifications.show({
          title: 'Stream assigned',
          message: 'Stream successfully added to channel.',
          color: 'green',
        });
      }
    } catch (e) {
      console.error('[ChannelManager] drag-assign error:', e);
    }
  }, []);

  // --- Resizable divider ---
  const handleDividerMouseDown = useCallback((e) => {
    e.preventDefault();
    isDraggingDivider.current = true;

    const onMove = (me) => {
      if (!isDraggingDivider.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const newWidth = Math.max(MIN_LEFT, Math.min(MAX_LEFT, me.clientX - rect.left));
      setLeftWidth(newWidth);
    };

    const onUp = () => {
      isDraggingDivider.current = false;
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
    };

    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  }, []);

  return (
    <DndContext collisionDetection={pointerWithin} onDragEnd={handleDragEnd}>
      <div className="cm-container" ref={containerRef}>
        {/* Left: channels pane */}
        <div className="cm-left-pane" style={{ width: leftWidth }}>
          <ChannelsTreePane
            selectedChannelId={selectedChannelId}
            onSelectChannel={setSelectedChannelId}
          />
        </div>

        {/* Resizable divider */}
        <div className="cm-divider" onMouseDown={handleDividerMouseDown} />

        {/* Right: streams pane */}
        <div className="cm-right-pane">
          <StreamsTreePane
            selectedStreamIds={selectedStreamIds}
            onToggleStreamSelection={handleToggleStreamSelection}
            onSelectAllInGroup={handleSelectAllInGroup}
            onAssign={handleAssign}
          />
        </div>
      </div>
    </DndContext>
  );
};

export default ChannelManagerLayout;
