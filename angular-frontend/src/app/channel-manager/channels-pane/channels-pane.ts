import {
  ChangeDetectionStrategy, ChangeDetectorRef, Component, EventEmitter,
  inject, OnInit, Output, ViewChild, TemplateRef
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { ApiService } from '../../api.service';
import { VideoPlayerComponent } from '../video-player/video-player.component';
import { TuiSheetDialog } from '@taiga-ui/addon-mobile';
import { StreamCheckerSheetComponent } from '../stream-checker-sheet/stream-checker-sheet';

// =================== INTERFACES ===================

/** A channel group containing its channels */
interface GroupView {
  id: number;
  name: string;
  expanded: boolean;
  channels: ChannelView[];
}

/** A channel within a group, containing its assigned streams */
interface ChannelView {
  id: number;
  uuid: string;
  name: string;
  channel_number: number | null;
  logo_url: string | null;
  epg_id: string | null;
  expanded: boolean;
  streams: StreamView[];
}

/** A stream assigned to a channel */
interface StreamView {
  id: number;
  channel_stream_id: number;
  order: number;
  name: string;
  logo_url: string | null;
  url: string | null;
  m3u_account_id: number | null;
  m3u_account_name: string;
  stream_stats: any;
  stream_stats_updated_at: string | null;
}

@Component({
  selector: 'app-channels-pane',
  standalone: true,
  imports: [CommonModule, FormsModule, VideoPlayerComponent, TuiSheetDialog, StreamCheckerSheetComponent],
  templateUrl: './channels-pane.html',
  styleUrl: './channels-pane.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ChannelsPaneComponent implements OnInit {
  private readonly api = inject(ApiService);
  private readonly cdr = inject(ChangeDetectorRef);

  // =================== DATA ===================

  /** The full nested tree: groups → channels → streams */
  groupViews: GroupView[] = [];
  /** All groups (for the filter dropdown) — only groups with channels */
  allGroupsWithChannels: { id: number; name: string }[] = [];
  /** M3U account name lookup: accountId → name */
  m3uAccountNames: Map<number, string> = new Map();

  loading = true;

  // =================== FILTERS ===================

  searchQuery = '';
  /** Selected group IDs for filtering (multi-select). Empty = show all. */
  selectedGroupFilters: Set<number> = new Set();
  showGroupFilterDropdown = false;

  // =================== SELECTION ===================

  /** Selected channel IDs (checkboxes) */
  selectedChannelIds: Set<number> = new Set();
  /** Selected stream IDs (checkboxes) */
  selectedStreamIds: Set<number> = new Set();
  /** For shift-click range selection on the flat channel list */
  private lastClickedChannelIdx = -1;
  /** Flat list of channel IDs for shift-click computation */
  private flatChannelIds: number[] = [];

  // =================== KEBAB MENU ===================

  /** Currently open kebab menu for a channel (channel ID, or null) */
  openKebabChannelId: number | null = null;
  /** Currently open kebab menu for a group (group ID, or null) */
  openKebabGroupId: number | null = null;

  // =================== IN-APP VIDEO PLAYER ===================

  playerUrl: string | null = null;
  playerTitle = '';
  playerContentType: 'live' | 'vod' = 'live';

  // =================== STREAM DRAG REORDER ===================

  /** The channel whose stream is being dragged */
  private dragSourceChannelId: number | null = null;
  /** Index within that channel's streams array */
  private dragSourceIndex: number | null = null;
  /** The stream ID being dragged */
  draggedStreamId: number | null = null;
  /** The target index for the drop indicator */
  dragOverIndex: number | null = null;
  dragOverChannelId: number | null = null;

  // =================== CROSS-PANE DROP TARGET ===================

  /** Channel ID currently being hovered over during a cross-pane drag */
  crossPaneDropTargetId: number | null = null;

  // =================== TESTING STATE ===================
  /** Stream IDs currently being tested (show spinner) */
  testingStreamIds: Set<number> = new Set();
  /** Channel IDs currently being bulk-tested */
  testingChannelIds: Set<number> = new Set();
  /** Group IDs currently being tested (all channels in group) */
  testingGroupIds: Set<number> = new Set();

  // =================== SHEET DIALOG STATE ===================
  /** Whether the stream checker sheet is open */
  sheetOpen = false;
  /** SheetDialog configuration — two stop levels for collapsed/expanded */
  sheetOptions = { stops: ['6rem', '14rem'] };
  /** Stream IDs to pass to the stream checker sheet */
  sheetStreamIds: number[] = [];
  /** Channel IDs to pass to the stream checker sheet for auto-sort */
  sheetChannelIds: number[] = [];
  /** Label shown in the sheet header */
  sheetLabel = 'Stream Checker';
  /**
   * Tracks which row (group or channel) triggered the current check.
   * Used to show a retrieval badge after the sheet is dismissed.
   * Format: { type: 'group'|'channel', id: number, name: string }
   */
  activeCheckSource: { type: 'group' | 'channel'; id: number; name: string } | null = null;

  // =================== LIFECYCLE ===================

  ngOnInit() {
    this.loadData();
  }

  /**
   * Load all data: groups, channels (with streams), and M3U account names.
   * Builds the nested tree structure for the template.
   */
  private loadData() {
    this.loading = true;
    this.cdr.markForCheck();

    // Load M3U account names for display
    this.api.getPlaylists().subscribe((res: any) => {
      const accounts = Array.isArray(res) ? res : res?.results || [];
      accounts.forEach((a: any) => this.m3uAccountNames.set(a.id, a.name));
    });

    // Load groups first, then channels
    this.api.getChannelGroups().subscribe({
      next: (groupsRes: any) => {
        const groups = Array.isArray(groupsRes) ? groupsRes : groupsRes?.results || [];

        // Fetch ALL channels (large page_size for complete data)
        this.api.getChannels({ page_size: 5000 }).subscribe({
          next: (channelsRes: any) => {
            const channels = Array.isArray(channelsRes) ? channelsRes : channelsRes?.results || [];
            this.buildGroupViews(groups, channels);
            this.loading = false;
            this.cdr.markForCheck();
          },
          error: (err) => {
            console.error('Failed to fetch channels:', err);
            this.loading = false;
            this.cdr.markForCheck();
          }
        });
      },
      error: (err) => {
        console.error('Failed to fetch groups:', err);
        this.loading = false;
        this.cdr.markForCheck();
      }
    });
  }

  /**
   * Build the nested GroupView[] from raw groups and channels data.
   * Only groups with at least one channel are included.
   * Preserves previous expansion state when rebuilding.
   */
  private buildGroupViews(groups: any[], channels: any[]) {
    // Capture previous expansion state before rebuilding
    const prevExpandedGroups = new Set(this.groupViews.filter(g => g.expanded).map(g => g.id));
    const prevExpandedChannels = new Set<number>();
    this.groupViews.forEach(g => {
      g.channels.forEach(ch => {
        if (ch.expanded) prevExpandedChannels.add(ch.id);
      });
    });

    // Map channels to their group ID
    const channelsByGroup = new Map<number, any[]>();
    const ungroupedChannels: any[] = [];

    channels.forEach(ch => {
      const groupId = ch.channel_group_id || ch.channel_group;
      if (groupId) {
        if (!channelsByGroup.has(groupId)) channelsByGroup.set(groupId, []);
        channelsByGroup.get(groupId)!.push(ch);
      } else {
        ungroupedChannels.push(ch);
      }
    });

    const views: GroupView[] = [];
    const groupsWithChannels: { id: number; name: string }[] = [];

    // Build group views — only include groups that have channels
    groups.forEach((g: any) => {
      const groupChannels = channelsByGroup.get(g.id) || [];
      if (groupChannels.length === 0) return; // Skip empty groups

      groupsWithChannels.push({ id: g.id, name: g.name });
      views.push({
        id: g.id,
        name: g.name,
        expanded: prevExpandedGroups.has(g.id),
        channels: groupChannels.map(ch => {
          const view = this.buildChannelView(ch);
          view.expanded = prevExpandedChannels.has(ch.id);
          return view;
        }),
      });
    });

    // Add ungrouped channels if any
    if (ungroupedChannels.length > 0) {
      views.push({
        id: -1,
        name: 'Ungrouped',
        expanded: prevExpandedGroups.has(-1),
        channels: ungroupedChannels.map(ch => {
          const view = this.buildChannelView(ch);
          view.expanded = prevExpandedChannels.has(ch.id);
          return view;
        }),
      });
    }

    this.groupViews = views;
    this.allGroupsWithChannels = groupsWithChannels;

    // Build flat channel ID list for shift-click
    this.flatChannelIds = [];
    views.forEach(g => g.channels.forEach(ch => this.flatChannelIds.push(ch.id)));
  }

  /** Convert raw channel JSON into a ChannelView with streams */
  private buildChannelView(ch: any): ChannelView {
    const streams: StreamView[] = (ch.streams || []).map((s: any, idx: number) => ({
      id: s.id,
      channel_stream_id: s.channel_stream_id,
      order: s.order ?? idx,
      name: s.name,
      logo_url: s.logo_url || null,
      url: s.url || null,
      m3u_account_id: s.m3u_account_id || s.m3u_account || null,
      m3u_account_name: this.getM3uAccountName(s.m3u_account_id || s.m3u_account),
      stream_stats: s.stream_stats || null,
      stream_stats_updated_at: s.stream_stats_updated_at || null,
    }));

    // Sort streams by their order field
    streams.sort((a, b) => a.order - b.order);

    return {
      id: ch.id,
      uuid: ch.uuid,
      name: ch.name,
      channel_number: ch.channel_number,
      logo_url: ch.logo_url || null,
      epg_id: ch.epg_id || null,
      expanded: false,
      streams,
    };
  }

  /** Get M3U account name from the lookup map */
  private getM3uAccountName(accountId: number | null): string {
    if (!accountId) return '';
    return this.m3uAccountNames.get(accountId) || `Provider #${accountId}`;
  }

  // =================== FILTERING ===================

  /** Get the filtered group views based on search + group filter */
  get filteredGroupViews(): GroupView[] {
    let views = this.groupViews;

    // Apply group filter
    if (this.selectedGroupFilters.size > 0) {
      views = views.filter(g => this.selectedGroupFilters.has(g.id));
    }

    // Apply search filter
    if (this.searchQuery.trim()) {
      const q = this.searchQuery.toLowerCase().trim();
      views = views.map(g => {
        const matchedChannels = g.channels.filter(ch =>
          ch.name.toLowerCase().includes(q) ||
          (ch.channel_number != null && String(ch.channel_number).includes(q))
        );
        if (matchedChannels.length === 0) return null;
        return { ...g, channels: matchedChannels, expanded: true };
      }).filter(Boolean) as GroupView[];
    }

    return views;
  }

  /** Total channel count across all visible groups */
  get totalChannelCount(): number {
    return this.filteredGroupViews.reduce((sum, g) => sum + g.channels.length, 0);
  }

  toggleGroupFilter(groupId: number) {
    if (this.selectedGroupFilters.has(groupId)) {
      this.selectedGroupFilters.delete(groupId);
    } else {
      this.selectedGroupFilters.add(groupId);
    }
    this.cdr.markForCheck();
  }

  isGroupFilterSelected(groupId: number): boolean {
    return this.selectedGroupFilters.has(groupId);
  }

  clearGroupFilters() {
    this.selectedGroupFilters.clear();
    this.cdr.markForCheck();
  }

  get groupFilterLabel(): string {
    if (this.selectedGroupFilters.size === 0) return 'All Groups';
    if (this.selectedGroupFilters.size === 1) {
      const id = [...this.selectedGroupFilters][0];
      const g = this.allGroupsWithChannels.find(g => g.id === id);
      return g ? g.name : 'All Groups';
    }
    return `${this.selectedGroupFilters.size} Groups`;
  }

  // =================== EXPAND / COLLAPSE ===================

  toggleGroupExpand(group: GroupView) {
    group.expanded = !group.expanded;
    this.cdr.markForCheck();
  }

  toggleChannelExpand(channel: ChannelView, event: Event) {
    event.stopPropagation();
    channel.expanded = !channel.expanded;
    this.cdr.markForCheck();
  }

  expandAll() {
    this.filteredGroupViews.forEach(g => {
      g.expanded = true;
      g.channels.forEach(ch => ch.expanded = true);
    });
    this.cdr.markForCheck();
  }

  collapseAll() {
    this.filteredGroupViews.forEach(g => {
      g.expanded = false;
      g.channels.forEach(ch => ch.expanded = false);
    });
    this.cdr.markForCheck();
  }

  // =================== SELECTION ===================

  /** Toggle a channel checkbox */
  toggleChannelSelect(channelId: number, event: MouseEvent) {
    const idx = this.flatChannelIds.indexOf(channelId);

    if (event.shiftKey && this.lastClickedChannelIdx >= 0) {
      // Shift-click range selection
      const start = Math.min(this.lastClickedChannelIdx, idx);
      const end = Math.max(this.lastClickedChannelIdx, idx);
      for (let i = start; i <= end; i++) {
        this.selectedChannelIds.add(this.flatChannelIds[i]);
      }
    } else {
      if (this.selectedChannelIds.has(channelId)) {
        this.selectedChannelIds.delete(channelId);
      } else {
        this.selectedChannelIds.add(channelId);
      }
    }

    this.lastClickedChannelIdx = idx;
    this.cdr.markForCheck();
  }

  isChannelSelected(id: number): boolean {
    return this.selectedChannelIds.has(id);
  }

  /** Select all visible channels */
  selectAllChannels() {
    this.flatChannelIds.forEach(id => this.selectedChannelIds.add(id));
    this.cdr.markForCheck();
  }

  /** Deselect all channels */
  deselectAllChannels() {
    this.selectedChannelIds.clear();
    this.lastClickedChannelIdx = -1;
    this.cdr.markForCheck();
  }

  /** Returns true if every visible channel is selected */
  get allChannelsSelected(): boolean {
    return this.flatChannelIds.length > 0 &&
           this.flatChannelIds.every(id => this.selectedChannelIds.has(id));
  }

  // --- Group-level inline select-all (checkbox in group row) ---

  /** True if ALL channels in this group are selected */
  isGroupAllSelected(group: GroupView): boolean {
    return group.channels.length > 0 &&
           group.channels.every(ch => this.selectedChannelIds.has(ch.id));
  }

  /** True if SOME (but not all) channels in this group are selected */
  isGroupPartiallySelected(group: GroupView): boolean {
    if (group.channels.length === 0) return false;
    const selectedCount = group.channels.filter(ch => this.selectedChannelIds.has(ch.id)).length;
    return selectedCount > 0 && selectedCount < group.channels.length;
  }

  /** Toggle all channels in this group on/off */
  toggleGroupSelectAll(group: GroupView, event: Event) {
    event.stopPropagation();
    const allSelected = this.isGroupAllSelected(group);
    group.channels.forEach(ch => {
      if (allSelected) {
        this.selectedChannelIds.delete(ch.id);
      } else {
        this.selectedChannelIds.add(ch.id);
      }
    });
    this.cdr.markForCheck();
  }

  /** Toggle a stream checkbox */
  toggleStreamSelect(streamId: number, event?: Event) {
    event?.stopPropagation();
    if (this.selectedStreamIds.has(streamId)) {
      this.selectedStreamIds.delete(streamId);
    } else {
      this.selectedStreamIds.add(streamId);
    }
    this.cdr.markForCheck();
  }

  isStreamSelected(id: number): boolean {
    return this.selectedStreamIds.has(id);
  }

  // =================== KEBAB MENU ===================

  /** Toggle the channel-level kebab menu */
  toggleKebab(channelId: number, event: Event) {
    event.stopPropagation();
    this.openKebabGroupId = null; // Close group kebab if open
    this.openKebabChannelId = this.openKebabChannelId === channelId ? null : channelId;
    this.cdr.markForCheck();
  }

  /** Toggle the group-level kebab menu */
  toggleGroupKebab(groupId: number, event: Event) {
    event.stopPropagation();
    this.openKebabChannelId = null; // Close channel kebab if open
    this.openKebabGroupId = this.openKebabGroupId === groupId ? null : groupId;
    this.cdr.markForCheck();
  }

  closeKebab() {
    this.openKebabChannelId = null;
    this.openKebabGroupId = null;
    this.cdr.markForCheck();
  }

  // =================== ACTIONS: PLAY / PREVIEW ===================

  /**
   * Play a channel — opens the in-app video player.
   * Uses the proxy route: /stream/{channel_uuid}/
   */
  playChannel(channel: ChannelView, event: Event) {
    event.stopPropagation();
    this.closeKebab();
    this.playerUrl = `/stream/${channel.uuid}/`;
    this.playerTitle = channel.name;
    this.playerContentType = 'live';
    this.cdr.markForCheck();
  }

  /**
   * Preview a single stream — opens the in-app video player.
   * Uses the stream's direct URL.
   */
  previewStream(stream: StreamView, event: Event) {
    event.stopPropagation();
    if (!stream.url) return;
    this.playerUrl = stream.url;
    this.playerTitle = stream.name;
    this.playerContentType = 'live';
    this.cdr.markForCheck();
  }

  closePlayer() {
    this.playerUrl = null;
    this.playerTitle = '';
    this.cdr.markForCheck();
  }

  // =================== ACTIONS: TEST STREAM / CHANNEL ===================

  /**
   * Test a single stream via POST /api/streams/:id/check/.
   * Updates the stream's stats in-place on success.
   */
  testStream(stream: StreamView, channel: ChannelView, event: Event) {
    event.stopPropagation();
    if (this.testingStreamIds.has(stream.id)) return; // Already testing

    this.testingStreamIds.add(stream.id);
    this.cdr.markForCheck();

    this.api.testStream(stream.id).subscribe({
      next: (res: any) => {
        if (res.success && res.stream) {
          // Update stream stats in-place
          stream.stream_stats = res.stream.stream_stats || res.stream.custom_properties?.stream_stats;
          stream.stream_stats_updated_at = res.stream.stream_stats_updated_at;
        }
        this.testingStreamIds.delete(stream.id);
        this.cdr.markForCheck();
      },
      error: (err) => {
        console.error('Stream test failed:', err);
        this.testingStreamIds.delete(stream.id);
        this.cdr.markForCheck();
      }
    });
  }

  /**
   * Test all streams in a channel via the bulk check engine + SheetDialog.
   * Opens a bottom sheet showing real-time progress.
   */
  testChannel(channel: ChannelView, event: Event) {
    event.stopPropagation();
    this.closeKebab();
    if (channel.streams.length === 0) return;

    const streamIds = channel.streams.map(s => s.id);
    this.openCheckerSheet(streamIds, [channel.id], `Testing: ${channel.name}`, 'channel', channel.id, channel.name);
  }

  /**
   * Test all channels in a group via the bulk check engine + SheetDialog.
   * Collects all stream IDs across all channels and runs a single bulk check.
   */
  testGroupChannels(group: GroupView, event: Event) {
    event.stopPropagation();
    this.closeKebab();
    if (group.channels.length === 0) return;

    // Collect all stream IDs and channel IDs in this group
    const streamIds: number[] = [];
    const channelIds: number[] = [];
    for (const ch of group.channels) {
      if (ch.streams.length > 0) {
        channelIds.push(ch.id);
        ch.streams.forEach(s => streamIds.push(s.id));
      }
    }

    if (streamIds.length === 0) return;

    this.openCheckerSheet(streamIds, channelIds, `Testing: ${group.name}`, 'group', group.id, group.name);
  }

  /**
   * Open the stream checker SheetDialog with the given stream/channel IDs.
   * This is the shared entry point for both channel-level and group-level testing.
   */
  private openCheckerSheet(
    streamIds: number[],
    channelIds: number[],
    label: string,
    sourceType: 'group' | 'channel',
    sourceId: number,
    sourceName: string
  ) {
    this.sheetStreamIds = streamIds;
    this.sheetChannelIds = channelIds;
    this.sheetLabel = label;
    this.activeCheckSource = { type: sourceType, id: sourceId, name: sourceName };
    this.sheetOpen = true;
    this.cdr.markForCheck();
  }

  /**
   * Handle sheet dismiss — the badge persists on the source row
   * so the user can re-open the sheet to check progress.
   */
  onSheetDismiss(open: boolean) {
    this.sheetOpen = open;
    // activeCheckSource remains set so the badge stays visible
    this.cdr.markForCheck();
  }

  /**
   * Re-open the sheet from the badge on the group/channel row.
   * Does NOT start a new check — just shows the existing sheet
   * with the status polling panel.
   */
  reopenCheckerSheet(event: Event) {
    event.stopPropagation();
    // Re-open with empty streamIds (won't start a new check, just observe)
    this.sheetStreamIds = [];
    this.sheetOpen = true;
    this.cdr.markForCheck();
  }

  /**
   * Called by the stream checker sheet when the bulk check completes.
   * Refreshes the channel data and clears testing state.
   */
  onCheckComplete() {
    // Clear testing state
    this.testingGroupIds.clear();
    this.testingChannelIds.clear();
    this.testingStreamIds.clear();
    // Clear the badge since the check is done
    this.activeCheckSource = null;
    // Reload all channel data to get updated stats and ordering
    this.loadData();
  }

  // =================== STREAM DRAG REORDER ===================

  /**
   * When a stream row starts being dragged, record source info.
   */
  onStreamDragStart(event: DragEvent, channel: ChannelView, streamIdx: number) {
    this.dragSourceChannelId = channel.id;
    this.dragSourceIndex = streamIdx;
    this.draggedStreamId = channel.streams[streamIdx].id;

    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
      event.dataTransfer.setData('text/plain', String(channel.streams[streamIdx].id));
    }
  }

  /**
   * When dragging over a stream row, show the drop indicator.
   */
  onStreamDragOver(event: DragEvent, channel: ChannelView, streamIdx: number) {
    // Only allow reorder within the same channel
    if (this.dragSourceChannelId !== channel.id) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
    this.dragOverChannelId = channel.id;
    this.dragOverIndex = streamIdx;
    this.cdr.markForCheck();
  }

  /**
   * Handle dropping a stream onto a new position — reorder and persist.
   */
  onStreamDrop(event: DragEvent, channel: ChannelView, targetIdx: number) {
    event.preventDefault();
    if (this.dragSourceChannelId !== channel.id || this.dragSourceIndex === null) return;

    const srcIdx = this.dragSourceIndex;
    if (srcIdx === targetIdx) {
      this.clearDragState();
      return;
    }

    // Reorder the array in-place
    const [moved] = channel.streams.splice(srcIdx, 1);
    channel.streams.splice(targetIdx, 0, moved);

    // Recalculate order numbers
    channel.streams.forEach((s, i) => s.order = i);

    // Persist to backend
    const orderedStreamIds = channel.streams.map(s => s.id);
    this.api.reorderChannelStreams(channel.id, orderedStreamIds).subscribe({
      error: (err) => console.error('Failed to persist stream reorder:', err)
    });

    this.clearDragState();
    this.cdr.markForCheck();
  }

  onStreamDragEnd() {
    this.clearDragState();
    this.cdr.markForCheck();
  }

  private clearDragState() {
    this.dragSourceChannelId = null;
    this.dragSourceIndex = null;
    this.draggedStreamId = null;
    this.dragOverIndex = null;
    this.dragOverChannelId = null;
  }

  isDragTarget(channelId: number, streamIdx: number): boolean {
    return this.dragOverChannelId === channelId && this.dragOverIndex === streamIdx;
  }

  // =================== CROSS-PANE DROP (Streams Pane → Channel) ===================

  /**
   * Allow drops on channel rows from the Streams Pane.
   * Only activates when the drag payload is 'application/json' (cross-pane),
   * not 'text/plain' (internal reorder).
   */
  onChannelDragOver(event: DragEvent, channel: ChannelView) {
    // Accept cross-pane drops (application/json from Streams Pane)
    if (event.dataTransfer?.types.includes('application/json')) {
      event.preventDefault();
      event.dataTransfer.dropEffect = 'copy';
      this.crossPaneDropTargetId = channel.id;
      this.cdr.markForCheck();
    }
  }

  /**
   * Clear the drop highlight when the cursor leaves the channel row.
   */
  onChannelDragLeave(event: DragEvent, channel: ChannelView) {
    // Only clear if the leave target is not a child element
    const relatedTarget = event.relatedTarget as HTMLElement;
    const currentTarget = event.currentTarget as HTMLElement;
    if (currentTarget && relatedTarget && currentTarget.contains(relatedTarget)) {
      return; // Still inside the same channel row, don't clear
    }
    if (this.crossPaneDropTargetId === channel.id) {
      this.crossPaneDropTargetId = null;
      this.cdr.markForCheck();
    }
  }

  /**
   * Handle dropping streams from the Streams Pane onto a channel.
   * Extracts stream IDs from dataTransfer, appends them (avoiding duplicates),
   * and persists via PATCH.
   */
  onChannelDrop(event: DragEvent, channel: ChannelView) {
    event.preventDefault();
    this.crossPaneDropTargetId = null;

    const jsonData = event.dataTransfer?.getData('application/json');
    if (!jsonData) return;

    let droppedStreamIds: number[];
    try {
      droppedStreamIds = JSON.parse(jsonData);
      if (!Array.isArray(droppedStreamIds)) return;
    } catch {
      return;
    }

    // Filter out stream IDs that are already assigned to this channel
    const existingIds = new Set(channel.streams.map(s => s.id));
    const newIds = droppedStreamIds.filter(id => !existingIds.has(id));
    if (newIds.length === 0) {
      console.log('All dropped streams are already assigned to this channel.');
      this.cdr.markForCheck();
      return;
    }

    // Build the full ordered stream list: existing + newly dropped
    const allStreamIds = [...channel.streams.map(s => s.id), ...newIds];

    // Persist via PATCH — the backend replaces the channel's stream list
    this.api.updateChannel(channel.id, { streams: allStreamIds }).subscribe({
      next: () => {
        // Reload data to get fresh stream details (names, logos, stats)
        this.loadData();
      },
      error: (err) => {
        console.error('Failed to assign streams to channel:', err);
        this.cdr.markForCheck();
      }
    });
  }

  // =================== HELPERS ===================

  /** Format stream stats into a human-readable summary */
  formatStreamStats(stats: any): string {
    if (!stats) return 'No stats';
    const parts: string[] = [];
    if (stats.resolution && stats.resolution !== '0x0') parts.push(stats.resolution);
    if (stats.video_codec) parts.push(stats.video_codec);
    if (stats.video_bitrate) parts.push(`${(stats.video_bitrate / 1000).toFixed(1)} Mbps`);
    if (stats.status) parts.push(stats.status);
    return parts.length > 0 ? parts.join(' · ') : 'No stats';
  }

  /** Track-by for ngFor performance */
  trackByGroupId(_: number, g: GroupView) { return g.id; }
  trackByChannelId(_: number, ch: ChannelView) { return ch.id; }
  trackByStreamId(_: number, s: StreamView) { return s.id; }

  /** Close kebab menus when clicking outside */
  onDocumentClick(event: Event) {
    if (this.openKebabChannelId !== null || this.openKebabGroupId !== null) {
      this.closeKebab();
    }
    if (this.showGroupFilterDropdown) {
      this.showGroupFilterDropdown = false;
      this.cdr.markForCheck();
    }
  }
}
