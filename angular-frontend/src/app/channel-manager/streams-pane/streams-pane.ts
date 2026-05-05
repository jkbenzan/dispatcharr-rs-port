import { ChangeDetectionStrategy, ChangeDetectorRef, Component, EventEmitter, inject, Input, OnInit, OnDestroy, Output } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { ApiService } from '../../api.service';
import { TuiLoader } from '@taiga-ui/core';
import { VideoPlayerComponent } from '../video-player/video-player.component';

// =================== INTERFACES ===================

interface M3UView {
  id: number;
  name: string;
  status: string;
  last_refreshed: string | null;
  expanded: boolean;
  groups: M3UGroupView[];
  total_streams: number;
}

interface M3UGroupView {
  name: string;
  expanded: boolean;
  streams: any[];
}

@Component({
  selector: 'app-streams-pane',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    TuiLoader,
    VideoPlayerComponent
  ],
  templateUrl: './streams-pane.html',
  styleUrl: './streams-pane.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class StreamsPaneComponent implements OnInit, OnDestroy {
  private readonly api = inject(ApiService);
  private readonly cdr = inject(ChangeDetectorRef);

  // =================== DATA ===================
  
  m3uViews: M3UView[] = [];
  allM3Us: any[] = [];
  allGroups: string[] = [];
  
  /** Lookup: channel_group_id → group name (resolved from /api/channels/groups/) */
  private channelGroupNames: Map<number, string> = new Map();

  /** Set of stream IDs that are already assigned to at least one channel */
  assignedStreamIds: Set<number> = new Set();

  loading = false;
  refreshingIds: Set<number> = new Set();

  // =================== SELECTION (self-managed) ===================

  /** Internally managed set of selected stream IDs */
  selectedStreamIds: Set<number> = new Set();

  /** Flat ordered list of all visible stream IDs for shift-click range selection */
  private flatStreamIds: number[] = [];
  /** Index of the last clicked stream for shift-click computation */
  private lastClickedStreamIdx = -1;

  // =================== FILTERS ===================

  searchQuery = '';
  selectedProviderFilters: Set<number> = new Set();
  selectedGroupFilters: Set<string> = new Set();

  /** When true, streams already assigned to a channel are hidden */
  hideAssigned = false;

  showProviderDropdown = false;
  showGroupDropdown = false;

  providerSearch = '';
  groupSearch = '';

  // =================== VIDEO PLAYER ===================

  playerUrl: string | null = null;
  playerTitle = '';

  ngOnInit() {
    this.loadData();
  }

  ngOnDestroy() {
    // Cleanup if needed
  }

  private loadData() {
    this.loading = true;
    this.cdr.markForCheck();

    // Step 1: Fetch channel groups so we can resolve IDs → names
    this.api.getChannelGroups().subscribe((groupsRes: any) => {
      const groups = Array.isArray(groupsRes) ? groupsRes : groupsRes?.results || [];
      this.channelGroupNames.clear();
      groups.forEach((g: any) => this.channelGroupNames.set(g.id, g.name));

      // Step 2: Fetch channels to build the set of assigned stream IDs
      this.api.getChannels({ page_size: 5000 }).subscribe((channelsRes: any) => {
        const channels = Array.isArray(channelsRes) ? channelsRes : channelsRes?.results || [];
        this.assignedStreamIds.clear();
        channels.forEach((ch: any) => {
          const streams = ch.streams || [];
          streams.forEach((s: any) => this.assignedStreamIds.add(s.id));
        });

        // Step 3: Fetch playlists (providers)
        this.api.getPlaylists().subscribe((m3us: any) => {
          this.allM3Us = Array.isArray(m3us) ? m3us : m3us?.results || [];
          
          // Step 4: Fetch all streams to build the tree
          this.api.getStreams({ page_size: 10000 }).subscribe({
            next: (streamsRes: any) => {
              const streams = Array.isArray(streamsRes) ? streamsRes : streamsRes?.results || [];
              this.buildTree(this.allM3Us, streams);
              this.loading = false;
              this.cdr.markForCheck();
            },
            error: (err) => {
              console.error('Failed to fetch streams:', err);
              this.loading = false;
              this.cdr.markForCheck();
            }
          });
        });
      });
    });
  }

  private buildTree(m3us: any[], streams: any[]) {
    // Collect all unique group names for the filter dropdown
    const groupsSet = new Set<string>();
    streams.forEach(s => {
      const groupName = this.resolveGroupName(s.channel_group);
      if (groupName) groupsSet.add(groupName);
    });
    this.allGroups = Array.from(groupsSet).sort();

    // Group streams by M3U Account ID, then by resolved Group Name
    const streamsByM3U = new Map<number, Map<string, any[]>>();

    streams.forEach(s => {
      const m3uId = s.m3u_account_id ?? s.m3u_account;
      if (m3uId === undefined || m3uId === null) return;

      if (!streamsByM3U.has(m3uId)) {
        streamsByM3U.set(m3uId, new Map<string, any[]>());
      }
      
      const groupName = this.resolveGroupName(s.channel_group) || 'Ungrouped';
      const m3uGroups = streamsByM3U.get(m3uId)!;
      
      if (!m3uGroups.has(groupName)) {
        m3uGroups.set(groupName, []);
      }
      m3uGroups.get(groupName)!.push(s);
    });

    // Preserve existing expansion state when rebuilding the tree
    const previousExpandedM3U = new Set(this.m3uViews.filter(m => m.expanded).map(m => m.id));
    const previousExpandedGroups = new Set<string>();
    this.m3uViews.forEach(m => {
      m.groups.forEach(g => {
        if (g.expanded) previousExpandedGroups.add(`${m.id}::${g.name}`);
      });
    });

    // Build M3U views — show ALL providers, even those with 0 streams
    this.m3uViews = m3us.map(m => {
      const m3uGroups = streamsByM3U.get(m.id) || new Map<string, any[]>();
      const groups: M3UGroupView[] = Array.from(m3uGroups.keys()).sort().map(name => ({
        name,
        expanded: previousExpandedGroups.has(`${m.id}::${name}`),
        streams: m3uGroups.get(name)!
      }));

      const totalStreams = groups.reduce((acc, g) => acc + g.streams.length, 0);

      return {
        id: m.id,
        name: m.name,
        status: m.status || 'idle',
        last_refreshed: m.updated_at || m.last_refreshed,
        expanded: previousExpandedM3U.has(m.id),
        groups,
        total_streams: totalStreams
      };
    });

    // Rebuild flat stream ID list for shift-click
    this.rebuildFlatStreamIds();
  }

  /**
   * Resolve a channel_group value to a human-readable group name.
   */
  private resolveGroupName(channelGroup: any): string {
    if (channelGroup === null || channelGroup === undefined) return '';
    if (typeof channelGroup === 'number') {
      return this.channelGroupNames.get(channelGroup) || `Group #${channelGroup}`;
    }
    return String(channelGroup);
  }

  /**
   * Rebuild the flat ordered list of stream IDs from the current filtered view.
   * Used for shift-click range selection.
   */
  private rebuildFlatStreamIds() {
    this.flatStreamIds = [];
    this.filteredM3UViews.forEach(m => {
      m.groups.forEach(g => {
        g.streams.forEach(s => this.flatStreamIds.push(s.id));
      });
    });
  }

  // =================== FILTERS ===================

  get filteredM3UViews(): M3UView[] {
    return this.m3uViews.filter(m => {
      // Filter by provider
      if (this.selectedProviderFilters.size > 0 && !this.selectedProviderFilters.has(m.id)) {
        return false;
      }

      const hasGroupFilters = this.selectedGroupFilters.size > 0;
      const hasSearch = !!this.searchQuery;

      if (!hasGroupFilters && !hasSearch && !this.hideAssigned) return true;

      // Check if any group matches
      const matchingGroups = m.groups.filter(g => {
        if (hasGroupFilters && !this.selectedGroupFilters.has(g.name)) return false;
        
        // Filter out assigned streams if toggle is on
        let filteredStreams = g.streams;
        if (this.hideAssigned) {
          filteredStreams = filteredStreams.filter(s => !this.assignedStreamIds.has(s.id));
        }

        if (hasSearch) {
          filteredStreams = filteredStreams.filter(s => 
            s.name.toLowerCase().includes(this.searchQuery.toLowerCase())
          );
        }
        return filteredStreams.length > 0;
      });

      return matchingGroups.length > 0;
    }).map(m => {
      const hasGroupFilters = this.selectedGroupFilters.size > 0;
      const hasSearch = !!this.searchQuery;

      if (!hasGroupFilters && !hasSearch && !this.hideAssigned) return m;

      const filteredGroups = m.groups.filter(g => {
        if (hasGroupFilters && !this.selectedGroupFilters.has(g.name)) return false;
        
        let filteredStreams = g.streams;
        if (this.hideAssigned) {
          filteredStreams = filteredStreams.filter(s => !this.assignedStreamIds.has(s.id));
        }
        if (hasSearch) {
          filteredStreams = filteredStreams.filter(s => 
            s.name.toLowerCase().includes(this.searchQuery.toLowerCase())
          );
        }
        return filteredStreams.length > 0;
      }).map(g => {
        let filteredStreams = g.streams;
        if (this.hideAssigned) {
          filteredStreams = filteredStreams.filter(s => !this.assignedStreamIds.has(s.id));
        }
        if (hasSearch) {
          filteredStreams = filteredStreams.filter(s => 
            s.name.toLowerCase().includes(this.searchQuery.toLowerCase())
          );
        }
        return {
          ...g,
          expanded: hasSearch ? true : g.expanded,
          streams: filteredStreams
        };
      });

      return {
        ...m,
        expanded: hasSearch || m.expanded,
        groups: filteredGroups
      };
    });
  }

  /** Total stream count across all visible groups/providers */
  get totalVisibleStreamCount(): number {
    return this.filteredM3UViews.reduce((sum, m) =>
      sum + m.groups.reduce((gs, g) => gs + g.streams.length, 0), 0);
  }

  get providerFilterLabel(): string {
    if (this.selectedProviderFilters.size === 0) return 'All Providers';
    if (this.selectedProviderFilters.size === 1) {
      const id = Array.from(this.selectedProviderFilters)[0];
      return this.allM3Us.find(m => m.id === id)?.name || '1 Provider';
    }
    return `${this.selectedProviderFilters.size} Providers`;
  }

  get groupFilterLabel(): string {
    if (this.selectedGroupFilters.size === 0) return 'All Groups';
    if (this.selectedGroupFilters.size === 1) return Array.from(this.selectedGroupFilters)[0];
    return `${this.selectedGroupFilters.size} Groups`;
  }

  toggleProviderFilter(id: number) {
    if (this.selectedProviderFilters.has(id)) {
      this.selectedProviderFilters.delete(id);
    } else {
      this.selectedProviderFilters.add(id);
    }
    this.cdr.markForCheck();
  }

  toggleGroupFilter(name: string) {
    if (this.selectedGroupFilters.has(name)) {
      this.selectedGroupFilters.delete(name);
    } else {
      this.selectedGroupFilters.add(name);
    }
    this.cdr.markForCheck();
  }

  clearProviderFilters() {
    this.selectedProviderFilters.clear();
    this.cdr.markForCheck();
  }

  clearGroupFilters() {
    this.selectedGroupFilters.clear();
    this.cdr.markForCheck();
  }

  toggleHideAssigned() {
    this.hideAssigned = !this.hideAssigned;
    this.cdr.markForCheck();
  }

  get filteredProviders() {
    return this.allM3Us.filter(m => 
      m.name.toLowerCase().includes(this.providerSearch.toLowerCase())
    );
  }

  get filteredGroups() {
    return this.allGroups.filter(g => 
      g.toLowerCase().includes(this.groupSearch.toLowerCase())
    );
  }

  // =================== ACTIONS ===================

  refreshM3U(m3uId: number, event: Event) {
    event.stopPropagation();
    if (this.refreshingIds.has(m3uId)) return;

    this.refreshingIds.add(m3uId);
    this.cdr.markForCheck();

    this.api.refreshM3UAccount(m3uId).subscribe({
      next: () => {
        setTimeout(() => {
          this.refreshingIds.delete(m3uId);
          this.loadData();
        }, 2000);
      },
      error: (err) => {
        console.error('Refresh failed:', err);
        this.refreshingIds.delete(m3uId);
        this.cdr.markForCheck();
      }
    });
  }

  previewStream(stream: any, event: Event) {
    event.stopPropagation();
    this.playerUrl = `/stream/${stream.id}/`;
    this.playerTitle = stream.name;
    this.cdr.markForCheck();
  }

  // =================== SELECTION ===================

  isStreamSelected(id: number): boolean {
    return this.selectedStreamIds.has(id);
  }

  /**
   * Toggle a single stream's selection. Supports shift-click range selection.
   */
  onToggleStream(id: number, event: MouseEvent) {
    event.stopPropagation();

    // Rebuild flat IDs on every click to match current filter state
    this.rebuildFlatStreamIds();
    const idx = this.flatStreamIds.indexOf(id);

    if (event.shiftKey && this.lastClickedStreamIdx >= 0) {
      // Shift-click: select/deselect the range between last click and this one
      const start = Math.min(this.lastClickedStreamIdx, idx);
      const end = Math.max(this.lastClickedStreamIdx, idx);
      for (let i = start; i <= end; i++) {
        this.selectedStreamIds.add(this.flatStreamIds[i]);
      }
    } else {
      // Normal click: toggle single stream
      if (this.selectedStreamIds.has(id)) {
        this.selectedStreamIds.delete(id);
      } else {
        this.selectedStreamIds.add(id);
      }
    }

    this.lastClickedStreamIdx = idx;
    this.cdr.markForCheck();
  }

  /** Select all visible streams */
  selectAllStreams() {
    this.rebuildFlatStreamIds();
    this.flatStreamIds.forEach(id => this.selectedStreamIds.add(id));
    this.cdr.markForCheck();
  }

  /** Deselect all streams */
  deselectAllStreams() {
    this.selectedStreamIds.clear();
    this.lastClickedStreamIdx = -1;
    this.cdr.markForCheck();
  }

  /** Returns true if all visible streams are currently selected */
  get allVisibleSelected(): boolean {
    if (this.totalVisibleStreamCount === 0) return false;
    this.rebuildFlatStreamIds();
    return this.flatStreamIds.every(id => this.selectedStreamIds.has(id));
  }

  // =================== EXPAND / COLLAPSE ===================

  toggleM3UExpand(m3u: M3UView) {
    m3u.expanded = !m3u.expanded;
    this.cdr.markForCheck();
  }

  toggleGroupExpand(group: M3UGroupView, event: Event) {
    event.stopPropagation();
    group.expanded = !group.expanded;
    this.cdr.markForCheck();
  }

  expandAll() {
    this.filteredM3UViews.forEach(m => {
      m.expanded = true;
      m.groups.forEach(g => g.expanded = true);
    });
    this.cdr.markForCheck();
  }

  collapseAll() {
    this.filteredM3UViews.forEach(m => {
      m.expanded = false;
      m.groups.forEach(g => g.expanded = false);
    });
    this.cdr.markForCheck();
  }

  // =================== HELPERS ===================

  onDocumentClick() {
    this.showProviderDropdown = false;
    this.showGroupDropdown = false;
    this.cdr.markForCheck();
  }

  handleDragStart(event: DragEvent, stream: any) {
    if (event.dataTransfer) {
      // If the dragged stream is selected, drag ALL selected streams.
      // Otherwise, drag just the single stream.
      const idsToDrag = this.selectedStreamIds.has(stream.id) 
        ? Array.from(this.selectedStreamIds)
        : [stream.id];
        
      event.dataTransfer.setData('application/json', JSON.stringify(idsToDrag));
      event.dataTransfer.effectAllowed = 'copy';
      
      const dragEl = document.createElement('div');
      dragEl.textContent = `${idsToDrag.length} stream(s)`;
      dragEl.style.cssText = 'position: absolute; top: -1000px; background: #646cff; color: white; padding: 4px 12px; border-radius: 6px; font-size: 13px; font-weight: 500;';
      document.body.appendChild(dragEl);
      event.dataTransfer.setDragImage(dragEl, 0, 0);
      setTimeout(() => document.body.removeChild(dragEl), 0);
    }
  }

  trackByM3UId(_: number, m: M3UView) { return m.id; }
  trackByGroupName(_: number, g: M3UGroupView) { return g.name; }
  trackByStreamId(_: number, s: any) { return s.id; }
}
