import {
  ChangeDetectionStrategy, ChangeDetectorRef, Component, EventEmitter,
  inject, OnInit, Output
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { ApiService } from '../../api.service';
import { VideoPlayerComponent } from '../video-player/video-player.component';
import { firstValueFrom } from 'rxjs';

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
  imports: [CommonModule, FormsModule, VideoPlayerComponent],
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

  // =================== TESTING STATE ===================
  /** Stream IDs currently being tested (show spinner) */
  testingStreamIds: Set<number> = new Set();
  /** Channel IDs currently being bulk-tested */
  testingChannelIds: Set<number> = new Set();

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
   */
  private buildGroupViews(groups: any[], channels: any[]) {
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
        expanded: false,
        channels: groupChannels.map(ch => this.buildChannelView(ch)),
      });
    });

    // Add ungrouped channels if any
    if (ungroupedChannels.length > 0) {
      views.push({
        id: -1,
        name: 'Ungrouped',
        expanded: false,
        channels: ungroupedChannels.map(ch => this.buildChannelView(ch)),
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
    if (this.selectedGroupFilters.size === 0) return 'All groups';
    if (this.selectedGroupFilters.size === 1) {
      const id = [...this.selectedGroupFilters][0];
      const g = this.allGroupsWithChannels.find(g => g.id === id);
      return g ? g.name : 'All groups';
    }
    return `${this.selectedGroupFilters.size} groups`;
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

  toggleKebab(channelId: number, event: Event) {
    event.stopPropagation();
    this.openKebabChannelId = this.openKebabChannelId === channelId ? null : channelId;
    this.cdr.markForCheck();
  }

  closeKebab() {
    this.openKebabChannelId = null;
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
   * Test all streams in a channel, then sort them based on sorting rules.
   * Uses bulk check + bulk sort endpoints.
   */
  testChannel(channel: ChannelView, event: Event) {
    event.stopPropagation();
    this.closeKebab();
    if (channel.streams.length === 0) return;
    if (this.testingChannelIds.has(channel.id)) return;

    this.testingChannelIds.add(channel.id);
    this.cdr.markForCheck();

    const streamIds = channel.streams.map(s => s.id);

    this.api.bulkCheckStreams(streamIds).subscribe({
      next: () => {
        // Poll for completion
        this.pollBulkCheckAndSort(channel);
      },
      error: (err) => {
        console.error('Bulk check start failed:', err);
        this.testingChannelIds.delete(channel.id);
        this.cdr.markForCheck();
      }
    });
  }

  /**
   * Test all streams in a channel individually (sequential, one-by-one).
   * Unlike testChannel() which uses the bulk-check endpoint, this tests each
   * stream separately via POST /api/streams/:id/check/ and updates stats
   * in-place as each completes — giving the user per-stream progress feedback.
   */
  async testAllStreams(channel: ChannelView, event: Event) {
    event.stopPropagation();
    this.closeKebab();

    // Guard: no streams or already testing
    if (channel.streams.length === 0) return;
    if (this.testingChannelIds.has(channel.id)) return;

    // Mark the entire channel as in-progress
    this.testingChannelIds.add(channel.id);

    // Mark all streams as testing so spinners appear on each row
    channel.streams.forEach(s => this.testingStreamIds.add(s.id));

    // Auto-expand the channel so the user can see per-stream progress
    channel.expanded = true;
    this.cdr.markForCheck();

    // Test each stream sequentially — we use firstValueFrom to await each HTTP call.
    // Sequential testing avoids overwhelming the backend with concurrent ffprobe calls.
    for (const stream of channel.streams) {
      try {
        const res: any = await firstValueFrom(this.api.testStream(stream.id));
        // Update stream stats in-place on success
        if (res?.success && res?.stream) {
          stream.stream_stats = res.stream.stream_stats || res.stream.custom_properties?.stream_stats;
          stream.stream_stats_updated_at = res.stream.stream_stats_updated_at;
        }
      } catch (err) {
        console.error(`Test All Streams: stream ${stream.id} (${stream.name}) failed:`, err);
      } finally {
        // Remove per-stream spinner regardless of success/failure
        this.testingStreamIds.delete(stream.id);
        this.cdr.markForCheck();
      }
    }

    // All streams done — remove channel-level testing state
    this.testingChannelIds.delete(channel.id);
    this.cdr.markForCheck();
  }

  /**
   * Poll bulk check status until done, then trigger sort + refresh.
   */
  private pollBulkCheckAndSort(channel: ChannelView) {
    const poll = setInterval(() => {
      this.api.getBulkCheckStatus().subscribe({
        next: (status: any) => {
          if (!status.is_running) {
            clearInterval(poll);
            // Sort streams based on sorting rules
            this.api.bulkSortStreams([channel.id]).subscribe({
              next: () => {
                // Refresh channel data to get updated order + stats
                this.api.getChannelStreams(channel.id).subscribe({
                  next: (updated: any) => {
                    // Rebuild the channel's stream list
                    const newStreams: StreamView[] = (updated.streams || []).map((s: any, idx: number) => ({
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
                    newStreams.sort((a: StreamView, b: StreamView) => a.order - b.order);
                    channel.streams = newStreams;
                    this.testingChannelIds.delete(channel.id);
                    this.cdr.markForCheck();
                  },
                  error: () => {
                    this.testingChannelIds.delete(channel.id);
                    this.cdr.markForCheck();
                  }
                });
              },
              error: () => {
                this.testingChannelIds.delete(channel.id);
                this.cdr.markForCheck();
              }
            });
          }
        },
        error: () => {
          clearInterval(poll);
          this.testingChannelIds.delete(channel.id);
          this.cdr.markForCheck();
        }
      });
    }, 2000); // Poll every 2 seconds
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

  /** Close kebab when clicking outside */
  onDocumentClick(event: Event) {
    if (this.openKebabChannelId !== null) {
      this.closeKebab();
    }
    if (this.showGroupFilterDropdown) {
      this.showGroupFilterDropdown = false;
      this.cdr.markForCheck();
    }
  }
}
