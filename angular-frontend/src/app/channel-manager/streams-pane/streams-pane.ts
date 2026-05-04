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

  @Input() selectedStreamIds: number[] = [];
  @Output() toggleStreamSelection = new EventEmitter<number>();
  @Output() selectAllInGroup = new EventEmitter<{streams: any[], isSelected: boolean}>();
  @Output() assignClicked = new EventEmitter<void>();

  // =================== DATA ===================
  
  m3uViews: M3UView[] = [];
  allM3Us: any[] = [];
  allGroups: string[] = [];
  
  loading = false;
  refreshingIds: Set<number> = new Set();

  // =================== FILTERS ===================

  searchQuery = '';
  selectedProviderFilters: Set<number> = new Set();
  selectedGroupFilters: Set<string> = new Set();

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

    // Fetch playlists (providers)
    this.api.getPlaylists().subscribe((m3us: any) => {
      this.allM3Us = Array.isArray(m3us) ? m3us : m3us?.results || [];
      
      // Fetch all streams to build the tree
      // Using a large page_size to get enough data for the tree view
      this.api.getStreams({ page_size: 5000 }).subscribe({
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
  }

  private buildTree(m3us: any[], streams: any[]) {
    // Collect all unique group names for the filter
    const groupsSet = new Set<string>();
    streams.forEach(s => {
      if (s.channel_group) groupsSet.add(s.channel_group);
    });
    this.allGroups = Array.from(groupsSet).sort();

    // Group streams by M3U Account ID, then by Group Name
    const streamsByM3U = new Map<number, Map<string, any[]>>();

    streams.forEach(s => {
      const m3uId = s.m3u_account_id;
      if (m3uId === undefined || m3uId === null) return;

      if (!streamsByM3U.has(m3uId)) {
        streamsByM3U.set(m3uId, new Map<string, any[]>());
      }
      
      const groupName = s.channel_group || 'Ungrouped';
      const m3uGroups = streamsByM3U.get(m3uId)!;
      
      if (!m3uGroups.has(groupName)) {
        m3uGroups.set(groupName, []);
      }
      m3uGroups.get(groupName)!.push(s);
    });

    this.m3uViews = m3us.map(m => {
      const m3uGroups = streamsByM3U.get(m.id) || new Map<string, any[]>();
      const groups: M3UGroupView[] = Array.from(m3uGroups.keys()).sort().map(name => ({
        name,
        expanded: false,
        streams: m3uGroups.get(name)!
      }));

      const totalStreams = groups.reduce((acc, g) => acc + g.streams.length, 0);

      return {
        id: m.id,
        name: m.name,
        status: m.status || 'idle',
        last_refreshed: m.updated_at || m.last_refreshed,
        expanded: false,
        groups,
        total_streams: totalStreams
      };
    }).filter(m => m.total_streams > 0); // Only show providers with streams
  }

  // =================== FILTERS ===================

  get filteredM3UViews(): M3UView[] {
    return this.m3uViews.filter(m => {
      // Filter by provider
      if (this.selectedProviderFilters.size > 0 && !this.selectedProviderFilters.has(m.id)) {
        return false;
      }

      // If we have group filters or search query, we need to check nested items
      const hasGroupFilters = this.selectedGroupFilters.size > 0;
      const hasSearch = !!this.searchQuery;

      if (!hasGroupFilters && !hasSearch) return true;

      // Check if any group matches
      const matchingGroups = m.groups.filter(g => {
        if (hasGroupFilters && !this.selectedGroupFilters.has(g.name)) return false;
        
        if (hasSearch) {
          return g.streams.some(s => s.name.toLowerCase().includes(this.searchQuery.toLowerCase()));
        }
        return true;
      });

      return matchingGroups.length > 0;
    }).map(m => {
      // If searching or filtering groups, we only show matching groups/streams
      const hasGroupFilters = this.selectedGroupFilters.size > 0;
      const hasSearch = !!this.searchQuery;

      if (!hasGroupFilters && !hasSearch) return m;

      const filteredGroups = m.groups.filter(g => {
        if (hasGroupFilters && !this.selectedGroupFilters.has(g.name)) return false;
        if (hasSearch) {
          return g.streams.some(s => s.name.toLowerCase().includes(this.searchQuery.toLowerCase()));
        }
        return true;
      }).map(g => {
        if (!hasSearch) return g;
        return {
          ...g,
          expanded: true, // Auto-expand when searching
          streams: g.streams.filter(s => s.name.toLowerCase().includes(this.searchQuery.toLowerCase()))
        };
      });

      return {
        ...m,
        expanded: hasSearch || m.expanded,
        groups: filteredGroups
      };
    });
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
        // We'll wait a bit then reload data
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

  onAssign() {
    this.assignClicked.emit();
  }

  // =================== SELECTION ===================

  isStreamSelected(id: number): boolean {
    return this.selectedStreamIds.includes(id);
  }

  onToggleStream(id: number, event: Event) {
    event.stopPropagation();
    this.toggleStreamSelection.emit(id);
  }

  onToggleM3UGroup(group: M3UGroupView, event: Event) {
    event.stopPropagation();
    const isSelected = !group.streams.every(s => this.isStreamSelected(s.id));
    this.selectAllInGroup.emit({ streams: group.streams, isSelected });
  }

  // =================== HELPERS ===================

  toggleM3UExpand(m3u: M3UView) {
    m3u.expanded = !m3u.expanded;
    this.cdr.markForCheck();
  }

  toggleGroupExpand(group: M3UGroupView, event: Event) {
    event.stopPropagation();
    group.expanded = !group.expanded;
    this.cdr.markForCheck();
  }

  onDocumentClick() {
    this.showProviderDropdown = false;
    this.showGroupDropdown = false;
    this.cdr.markForCheck();
  }

  handleDragStart(event: DragEvent, stream: any) {
    if (event.dataTransfer) {
      const idsToDrag = this.selectedStreamIds.includes(stream.id) 
        ? this.selectedStreamIds 
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
