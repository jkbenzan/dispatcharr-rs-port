import React, { useState, useEffect, useCallback } from 'react';
import { useDroppable } from '@dnd-kit/core';
import API from '../../api';
import './ChannelManager.css';

// --- Droppable channel row ---
const DroppableChannel = React.memo(({ channel, isSelected, onSelectChannel }) => {
  const { setNodeRef, isOver } = useDroppable({ id: channel.id });

  return (
    <div
      ref={setNodeRef}
      className={`cm-channel-item${isSelected ? ' selected' : ''}${isOver ? ' drop-over' : ''}`}
      onClick={() => onSelectChannel(channel.id)}
      title={`${channel.name}${channel.channel_number ? ` (#${channel.channel_number})` : ''}`}
    >
      <span className="cm-channel-item-name">{channel.name}</span>
      {channel.channel_number && (
        <span className="cm-channel-item-num">#{channel.channel_number}</span>
      )}
    </div>
  );
});

// --- Channel group section ---
const ChannelGroupSection = ({ group, selectedChannelId, onSelectChannel }) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const [channels, setChannels] = useState(null);  // null = not loaded yet
  const [loading, setLoading] = useState(false);

  const handleToggle = useCallback(async () => {
    if (!isExpanded && channels === null) {
      // Lazy-load channels for this group
      setLoading(true);
      try {
        const params = new URLSearchParams();
        params.set('channel_group', group.id);
        params.set('page_size', '500');
        params.set('ordering', 'channel_number');
        const response = await API.queryChannels(params);
        const list = Array.isArray(response) ? response : response?.results || [];
        setChannels(list);
      } catch (e) {
        console.error('[ChannelsTreePane] error loading group channels:', e);
        setChannels([]);
      } finally {
        setLoading(false);
      }
    }
    setIsExpanded((v) => !v);
  }, [isExpanded, channels, group.id]);

  const channelCount = channels?.length ?? group.channel_count ?? null;

  return (
    <div className="cm-channel-group">
      <div className="cm-channel-group-header" onClick={handleToggle}>
        <span
          className="cm-group-chevron"
          style={{ transform: isExpanded ? 'rotate(90deg)' : undefined }}
        >
          ▶
        </span>
        <span className="cm-group-name">{group.name}</span>
        {channelCount !== null && (
          <span className="cm-group-count">{channelCount}</span>
        )}
      </div>

      {isExpanded && (
        <div>
          {loading && <div className="cm-loading">Loading…</div>}
          {!loading && channels !== null && channels.length === 0 && (
            <div className="cm-empty">No channels in this group.</div>
          )}
          {!loading && channels && channels.map((channel) => (
            <DroppableChannel
              key={channel.id}
              channel={channel}
              isSelected={selectedChannelId === channel.id}
              onSelectChannel={onSelectChannel}
            />
          ))}
        </div>
      )}
    </div>
  );
};

// --- Main ChannelsTreePane ---
const ChannelsTreePane = ({ selectedChannelId, onSelectChannel }) => {
  const [channelGroups, setChannelGroups] = useState([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');

  useEffect(() => {
    const fetchGroups = async () => {
      try {
        const response = await API.getChannelGroups();
        const groups = Array.isArray(response) ? response : response?.results || [];
        setChannelGroups(groups);
      } catch (e) {
        console.error('[ChannelsTreePane] error fetching groups:', e);
      } finally {
        setLoading(false);
      }
    };
    fetchGroups();
  }, []);

  const filteredGroups = search
    ? channelGroups.filter((g) =>
        g.name.toLowerCase().includes(search.toLowerCase())
      )
    : channelGroups;

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div className="cm-pane-header">
        <h3 className="cm-pane-title">Channels</h3>
      </div>

      {/* Search */}
      <div style={{ padding: '10px 12px', borderBottom: '1px solid rgba(255,255,255,0.07)', flexShrink: 0 }}>
        <input
          type="text"
          className="cm-search-input"
          placeholder="Filter groups…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          style={{ width: '100%' }}
        />
      </div>

      {/* Group list */}
      <div className="cm-scroll">
        {loading && <div className="cm-loading">Loading channel groups…</div>}
        {!loading && filteredGroups.length === 0 && (
          <div className="cm-empty">No channel groups found.</div>
        )}
        {!loading && filteredGroups.map((group) => (
          <ChannelGroupSection
            key={group.id}
            group={group}
            selectedChannelId={selectedChannelId}
            onSelectChannel={onSelectChannel}
          />
        ))}
      </div>
    </div>
  );
};

export default ChannelsTreePane;
