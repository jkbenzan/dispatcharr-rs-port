import React, { useState, useEffect, useCallback, useRef } from 'react';
import { useDraggable } from '@dnd-kit/core';
import { CSS } from '@dnd-kit/utilities';
import { FixedSizeList as List } from 'react-window';
import AutoSizer from 'react-virtualized-auto-sizer';
import API from '../../api';
import './ChannelManager.css';

// --- Draggable stream row ---
const DraggableStream = React.memo(({ stream, isSelected, onToggle }) => {
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: stream.id,
    data: { stream },
  });

  return (
    <div
      ref={setNodeRef}
      className={`cm-stream-item${isSelected ? ' selected' : ''}`}
      style={{ transform: CSS.Translate.toString(transform), opacity: isDragging ? 0.5 : 1 }}
    >
      <span className="cm-stream-grip" {...listeners} {...attributes} title="Drag to assign">⠿</span>
      <input
        type="checkbox"
        className="cm-stream-checkbox"
        checked={isSelected}
        onChange={() => onToggle(stream.id)}
        onClick={(e) => e.stopPropagation()}
      />
      <span className="cm-stream-name" title={stream.name}>{stream.name}</span>
    </div>
  );
});

// --- Virtualized row renderer for react-window ---
const StreamRow = ({ index, style, data }) => {
  const stream = data.streams[index];
  return (
    <div style={style}>
      <DraggableStream
        stream={stream}
        isSelected={data.selectedStreamIds.has(stream.id)}
        onToggle={data.onToggle}
      />
    </div>
  );
};

// --- Custom multi-select dropdown (provider or group) ---
const FilterDropdown = ({ label, items, selectedValues, onChangeValues, searchable = false }) => {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');
  const dropdownRef = useRef(null);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target)) {
        setOpen(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  // Clear search on close
  useEffect(() => {
    if (!open) setSearch('');
  }, [open]);

  const filteredItems = searchable
    ? items.filter((it) => it.label.toLowerCase().includes(search.toLowerCase()))
    : items;

  const buttonLabel =
    selectedValues.length === 0
      ? label
      : `${selectedValues.length} ${selectedValues.length === 1 ? label.replace(/s$/, '') : label.toLowerCase()}`;

  const selectAll = () => {
    const toAdd = filteredItems.map((it) => it.value);
    onChangeValues([...new Set([...selectedValues, ...toAdd])]);
  };

  const clearAll = () => {
    if (searchable && search) {
      const filtered = new Set(filteredItems.map((it) => it.value));
      onChangeValues(selectedValues.filter((v) => !filtered.has(v)));
    } else {
      onChangeValues([]);
    }
  };

  return (
    <div className="cm-filter-dropdown" ref={dropdownRef}>
      <button
        className={`cm-filter-btn${open ? ' open' : ''}`}
        onClick={() => setOpen((o) => !o)}
        type="button"
      >
        <span className="cm-filter-btn-label">{buttonLabel}</span>
        <span className="cm-filter-btn-arrow">▾</span>
      </button>

      {open && (
        <div className="cm-filter-menu">
          {searchable && (
            <div className="cm-filter-menu-search">
              <span className="cm-filter-menu-search-icon">🔍</span>
              <input
                autoFocus
                placeholder="Search…"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                onClick={(e) => e.stopPropagation()}
              />
            </div>
          )}
          <div className="cm-filter-menu-actions">
            <button className="cm-filter-action-btn" onClick={selectAll} type="button">
              Select All{searchable && search ? ' Visible' : ''}
            </button>
            <button className="cm-filter-action-btn" onClick={clearAll} type="button">
              {searchable && search ? 'Clear Visible' : 'Clear All'}
            </button>
          </div>
          <div className="cm-filter-menu-options">
            {filteredItems.length === 0 && (
              <div className="cm-filter-empty">No matches</div>
            )}
            {filteredItems.map((item) => (
              <label key={item.value} className="cm-filter-option">
                <input
                  type="checkbox"
                  checked={selectedValues.includes(item.value)}
                  onChange={(e) => {
                    if (e.target.checked) {
                      onChangeValues([...selectedValues, item.value]);
                    } else {
                      onChangeValues(selectedValues.filter((v) => v !== item.value));
                    }
                  }}
                />
                <span className="cm-filter-option-name">{item.label}</span>
              </label>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

// --- Group header component ---
const GroupHeader = ({ group, isExpanded, onToggle, streams, selectedIds, onToggleGroup }) => {
  const loadedCount = streams.length;
  const allSelected = loadedCount > 0 && streams.every((s) => selectedIds.has(s.id));
  const someSelected = !allSelected && streams.some((s) => selectedIds.has(s.id));

  return (
    <div className="cm-group-header" onClick={() => onToggle(group.name)}>
      <span className={`cm-group-chevron${isExpanded ? ' expanded' : ''}`}>▶</span>
      <input
        type="checkbox"
        className="cm-group-checkbox"
        checked={allSelected}
        ref={(el) => { if (el) el.indeterminate = someSelected; }}
        onChange={(e) => { e.stopPropagation(); onToggleGroup(group.name, streams, !allSelected); }}
        onClick={(e) => e.stopPropagation()}
        title="Select all in group"
      />
      <span className="cm-group-name">{group.name}</span>
      <span className="cm-group-count">{group.count ?? loadedCount}</span>
    </div>
  );
};

// --- Main StreamsTreePane ---
const StreamsTreePane = ({ selectedStreamIds: selectedIdsArray, onToggleStreamSelection, onSelectAllInGroup, onAssign }) => {
  // Use a Set internally for O(1) lookup
  const selectedIds = new Set(selectedIdsArray || []);

  const [m3uAccounts, setM3uAccounts] = useState([]);       // [{value, label}]
  const [streamGroupMeta, setStreamGroupMeta] = useState([]); // [{name, count}] — top-level only, no streams yet
  const [selectedProviders, setSelectedProviders] = useState([]); // string[] of m3u account IDs
  const [selectedGroups, setSelectedGroups] = useState([]);   // string[] of group names

  const [searchQuery, setSearchQuery] = useState('');
  const [isSearching, setIsSearching] = useState(false);

  const [loading, setLoading] = useState(true);
  const [expandedGroups, setExpandedGroups] = useState(new Set());
  const [loadedGroups, setLoadedGroups] = useState({});   // groupName -> true when streams fetched
  const [streamsData, setStreamsData] = useState({});     // groupName -> stream[]

  // ---- Initial data fetch: providers + group metadata only ----
  useEffect(() => {
    const init = async () => {
      try {
        const [m3uRes, groupRes] = await Promise.all([
          API.getPlaylists(),
          API.getStreamGroups(),
        ]);

        const m3us = Array.isArray(m3uRes) ? m3uRes : m3uRes?.results || [];
        setM3uAccounts(
          m3us
            .filter((m) => m.id != null)
            .map((m) => ({ value: String(m.id), label: String(m.name || m.id) }))
        );

        // getStreamGroups() returns [{name, count}] or [{group_name, count}] depending on API version
        const groups = Array.isArray(groupRes) ? groupRes : groupRes?.results || [];
        setStreamGroupMeta(
          groups
            .map((g) => ({
              name: String(g.name ?? g.group_name ?? g),
              count: g.count ?? 0,
            }))
            .filter((g) => g.name)
        );
      } catch (e) {
        console.error('[StreamsTreePane] init error:', e);
      } finally {
        setLoading(false);
      }
    };
    init();
  }, []);

  // ---- Reload group metadata when provider filter changes ----
  useEffect(() => {
    if (loading) return; // skip during initial load
    const reload = async () => {
      try {
        const params = new URLSearchParams();
        selectedProviders.forEach((id) => params.append('m3u_account', id));
        const groupRes = await API.getStreamGroups();
        const groups = Array.isArray(groupRes) ? groupRes : groupRes?.results || [];
        setStreamGroupMeta(
          groups
            .map((g) => ({
              name: String(g.name ?? g.group_name ?? g),
              count: g.count ?? 0,
            }))
            .filter((g) => g.name)
        );
        // Reset any groups loaded under old filters
        setLoadedGroups({});
        setStreamsData({});
        setExpandedGroups(new Set());
      } catch (e) {
        console.error('[StreamsTreePane] provider filter reload error:', e);
      }
    };
    reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedProviders]);

  // ---- Lazy-load a group's streams on expand ----
  const loadGroupStreams = useCallback(async (groupName) => {
    if (loadedGroups[groupName]) return;
    setLoadedGroups((prev) => ({ ...prev, [groupName]: 'loading' }));
    try {
      const params = new URLSearchParams();
      selectedProviders.forEach((id) => params.append('m3u_account', id));
      params.set('channel_group', groupName);
      params.set('page_size', '1000');
      const response = await API.queryStreams(params);
      const streams = Array.isArray(response) ? response : response?.results || [];
      setStreamsData((prev) => ({ ...prev, [groupName]: streams }));
      setLoadedGroups((prev) => ({ ...prev, [groupName]: true }));
    } catch (e) {
      console.error('[StreamsTreePane] loadGroupStreams error:', e);
      setLoadedGroups((prev) => ({ ...prev, [groupName]: false }));
    }
  }, [loadedGroups, selectedProviders]);

  const toggleGroup = useCallback((groupName) => {
    setExpandedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(groupName)) {
        next.delete(groupName);
      } else {
        next.add(groupName);
        // Trigger lazy load
        loadGroupStreams(groupName);
      }
      return next;
    });
  }, [loadGroupStreams]);

  // ---- Intelligent search: Enter key triggers API search + auto-expand best matches ----
  const handleSearch = useCallback(async () => {
    const q = searchQuery.trim();
    if (!q) {
      // Clear search state and restore normal view
      setIsSearching(false);
      setStreamsData({});
      setLoadedGroups({});
      setExpandedGroups(new Set());
      return;
    }
    setIsSearching(true);
    try {
      const params = new URLSearchParams();
      params.set('search', q);
      selectedProviders.forEach((id) => params.append('m3u_account', id));
      params.set('page_size', '500');

      const response = await API.queryStreams(params);
      const streams = Array.isArray(response) ? response : response?.results || [];

      // Group results and auto-expand
      const groupMap = {};
      streams.forEach((s) => {
        const gn = s.channel_group || s.channel_group_name || 'Ungrouped';
        if (!groupMap[gn]) groupMap[gn] = [];
        groupMap[gn].push(s);
      });

      setStreamsData((prev) => ({ ...prev, ...groupMap }));
      setLoadedGroups((prev) => {
        const next = { ...prev };
        Object.keys(groupMap).forEach((k) => { next[k] = true; });
        return next;
      });
      setExpandedGroups(new Set(Object.keys(groupMap)));
    } catch (e) {
      console.error('[StreamsTreePane] search error:', e);
    }
  }, [searchQuery, selectedProviders]);

  // ---- Group selection toggle ----
  const handleToggleGroup = useCallback((groupName, streams, selectAll) => {
    const ids = streams.map((s) => s.id);
    if (selectAll) {
      onSelectAllInGroup(streams, true);
    } else {
      onSelectAllInGroup(streams, false);
    }
  }, [onSelectAllInGroup]);

  // ---- Compute displayed groups ----
  const displayedGroups = (() => {
    let groups = streamGroupMeta;
    // Filter by selected group names
    if (selectedGroups.length > 0) {
      groups = groups.filter((g) => selectedGroups.includes(g.name));
    }
    // If searching, only show groups that have results
    if (isSearching) {
      groups = groups.filter((g) => streamsData[g.name] && streamsData[g.name].length > 0);
    }
    return groups;
  })();

  if (loading) return <div className="cm-loading">Loading streams…</div>;

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div className="cm-pane-header">
        <h3 className="cm-pane-title">Streams</h3>
        {selectedIdsArray?.length > 0 && (
          <button
            className="cm-assign-btn"
            onClick={onAssign}
            disabled={!selectedIdsArray?.length}
            title="Assign selected streams to selected channel"
          >
            Assign {selectedIdsArray.length} selected
          </button>
        )}
      </div>

      {/* Filters */}
      <div className="cm-filters">
        {/* Search row */}
        <div className="cm-search-row">
          <input
            type="text"
            className="cm-search-input"
            placeholder="Search streams… (Enter to search)"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onKeyDown={(e) => { if (e.key === 'Enter') handleSearch(); }}
          />
          {(searchQuery || isSearching) && (
            <button
              type="button"
              style={{
                padding: '6px 10px',
                background: 'rgba(255,255,255,0.07)',
                border: '1px solid rgba(255,255,255,0.1)',
                borderRadius: 6,
                color: 'rgba(255,255,255,0.5)',
                cursor: 'pointer',
                fontSize: 12,
                flexShrink: 0,
              }}
              onClick={() => {
                setSearchQuery('');
                setIsSearching(false);
                setStreamsData({});
                setLoadedGroups({});
                setExpandedGroups(new Set());
              }}
              title="Clear search"
            >
              ✕
            </button>
          )}
        </div>

        {/* Provider + Group multi-select filter row */}
        <div className="cm-filter-row">
          <FilterDropdown
            label="Providers"
            items={m3uAccounts}
            selectedValues={selectedProviders}
            onChangeValues={setSelectedProviders}
            searchable={false}
          />
          <FilterDropdown
            label="Groups"
            items={streamGroupMeta.map((g) => ({ value: g.name, label: g.name }))}
            selectedValues={selectedGroups}
            onChangeValues={setSelectedGroups}
            searchable={true}
          />
        </div>
      </div>

      {/* Group tree */}
      <div className="cm-scroll">
        {displayedGroups.length === 0 && (
          <div className="cm-empty">
            {isSearching ? 'No streams matched your search.' : 'No stream groups found.'}
          </div>
        )}

        {displayedGroups.map((group) => {
          const isExpanded = expandedGroups.has(group.name);
          const streams = streamsData[group.name] || [];
          const isLoading = loadedGroups[group.name] === 'loading';

          return (
            <div key={group.name} className="cm-group">
              <GroupHeader
                group={group}
                isExpanded={isExpanded}
                onToggle={toggleGroup}
                streams={streams}
                selectedIds={selectedIds}
                onToggleGroup={handleToggleGroup}
              />

              {isExpanded && (
                <div className="cm-group-items">
                  {isLoading && <div className="cm-loading">Loading…</div>}
                  {!isLoading && streams.length === 0 && loadedGroups[group.name] === true && (
                    <div className="cm-empty">No streams in this group.</div>
                  )}
                  {!isLoading && streams.length > 0 && (
                    <>
                      {/* Actions bar */}
                      <div className="cm-group-actions">
                        <label className="cm-select-all-check">
                          <input
                            type="checkbox"
                            style={{ width: 13, height: 13, accentColor: '#339af0' }}
                            checked={streams.length > 0 && streams.every((s) => selectedIds.has(s.id))}
                            ref={(el) => {
                              if (el) el.indeterminate = streams.some((s) => selectedIds.has(s.id)) && !streams.every((s) => selectedIds.has(s.id));
                            }}
                            onChange={(e) => onSelectAllInGroup(streams, e.target.checked)}
                          />
                          Select all ({streams.length})
                        </label>
                      </div>

                      {/* Virtualized stream list */}
                      <div style={{ height: Math.min(streams.length * 38, 380), width: '100%' }}>
                        <AutoSizer>
                          {({ height, width }) => (
                            <List
                              height={height}
                              itemCount={streams.length}
                              itemSize={38}
                              width={width}
                              itemData={{
                                streams,
                                selectedIds,
                                onToggle: onToggleStreamSelection,
                              }}
                            >
                              {StreamRow}
                            </List>
                          )}
                        </AutoSizer>
                      </div>
                    </>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default StreamsTreePane;
