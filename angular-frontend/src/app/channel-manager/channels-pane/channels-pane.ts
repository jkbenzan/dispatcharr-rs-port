import { ChangeDetectionStrategy, ChangeDetectorRef, Component, EventEmitter, inject, Input, OnInit, Output } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormControl, FormGroup, FormsModule, ReactiveFormsModule, Validators } from '@angular/forms';
import { ApiService } from '../../api.service';
import { WebSocketService } from '../../websocket.service';
import { TuiLoader, TuiButton, TuiDialogService, TuiTextfield } from '@taiga-ui/core';
import { PolymorpheusContent } from '@taiga-ui/polymorpheus';
import { ChannelListItemComponent } from '../channel-list-item/channel-list-item';
import { firstValueFrom, forkJoin } from 'rxjs';

interface ChannelGroupView {
  id: number | null;
  name: string;
  channels: any[];
  expanded: boolean;
  channelRange: { min: number | null; max: number | null };
}

@Component({
  selector: 'app-channels-pane',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    ReactiveFormsModule,
    TuiLoader,
    TuiButton,
    TuiTextfield,
    ChannelListItemComponent
  ],
  templateUrl: './channels-pane.html',
  styleUrl: './channels-pane.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ChannelsPaneComponent implements OnInit {
  private readonly api = inject(ApiService);
  private readonly ws = inject(WebSocketService);
  private readonly dialogs = inject(TuiDialogService);
  private readonly cdr = inject(ChangeDetectorRef);
  
  @Input() selectedChannelId: number | null = null;
  @Output() channelSelected = new EventEmitter<number>();

  allChannels: any[] = [];
  allGroups: any[] = [];
  groupViews: ChannelGroupView[] = [];
  loading = true;
  searchQuery = '';
  totalChannels = 0;

  // Group filter
  selectedGroupIds: Set<number> = new Set();
  groupFilterOpen = false;

  createChannelForm = new FormGroup({
    name: new FormControl('', Validators.required),
    channel_number: new FormControl<number | null>(null),
  });

  ngOnInit() {
    this.fetchAll();

    this.ws.messages$.subscribe(msg => {
      if (msg.type === 'playlist_created' || msg.type === 'm3u_refresh_done' || msg.type === 'channel_updated') {
        this.fetchAll();
      }
    });
  }

  fetchAll() {
    this.loading = true;
    this.cdr.markForCheck();

    // Fetch groups and ALL channels in parallel
    forkJoin({
      groups: this.api.getChannelGroups(),
      channels: this.api.getChannels({ page_size: 5000 }),
    }).subscribe({
      next: ({ groups, channels }) => {
        this.allGroups = Array.isArray(groups) ? groups : groups?.results || [];
        this.allChannels = channels?.results || [];
        this.totalChannels = this.allChannels.length;
        
        // Initialize group filter: select all groups
        if (this.selectedGroupIds.size === 0) {
          this.allGroups.forEach((g: any) => this.selectedGroupIds.add(g.id));
          this.selectedGroupIds.add(-1); // "Uncategorized" pseudo-group
        }

        this.buildGroupViews();
        this.loading = false;
        this.cdr.markForCheck();
      },
      error: (err) => {
        console.error('Error fetching channel data', err);
        this.loading = false;
        this.cdr.markForCheck();
      }
    });
  }

  private buildGroupViews() {
    const search = this.searchQuery.toLowerCase();
    
    // Filter channels by search
    let filteredChannels = this.allChannels;
    if (search) {
      filteredChannels = filteredChannels.filter((c: any) =>
        c.name?.toLowerCase().includes(search) ||
        String(c.channel_number || '').includes(search)
      );
    }

    // Build a map of group_id -> channels
    const channelsByGroup = new Map<number | null, any[]>();
    filteredChannels.forEach((ch: any) => {
      const gid = ch.channel_group_id ?? null;
      if (!channelsByGroup.has(gid)) channelsByGroup.set(gid, []);
      channelsByGroup.get(gid)!.push(ch);
    });

    // Build group views
    const views: ChannelGroupView[] = [];

    // Named groups
    for (const g of this.allGroups) {
      if (!this.selectedGroupIds.has(g.id)) continue;
      
      const channels = (channelsByGroup.get(g.id) || []).sort(
        (a: any, b: any) => (a.channel_number ?? 9999) - (b.channel_number ?? 9999)
      );
      const nums = channels.map((c: any) => c.channel_number).filter((n: any) => n != null);

      views.push({
        id: g.id,
        name: g.name,
        channels,
        expanded: false,
        channelRange: {
          min: nums.length ? Math.min(...nums) : null,
          max: nums.length ? Math.max(...nums) : null,
        },
      });
    }

    // Uncategorized
    if (this.selectedGroupIds.has(-1)) {
      const uncategorized = (channelsByGroup.get(null) || []).sort(
        (a: any, b: any) => (a.channel_number ?? 9999) - (b.channel_number ?? 9999)
      );
      if (uncategorized.length > 0 || !search) {
        views.unshift({
          id: null,
          name: 'Uncategorized',
          channels: uncategorized,
          expanded: false,
          channelRange: { min: null, max: null },
        });
      }
    }

    // Only show groups that have channels
    this.groupViews = views.filter(v => v.channels.length > 0);
  }

  toggleGroup(group: ChannelGroupView) {
    group.expanded = !group.expanded;
  }

  onSearchChange(query: string) {
    this.searchQuery = query;
    this.buildGroupViews();
    this.cdr.markForCheck();
  }

  clearSearch() {
    this.searchQuery = '';
    this.buildGroupViews();
    this.cdr.markForCheck();
  }

  // Group filter
  toggleGroupFilter() {
    this.groupFilterOpen = !this.groupFilterOpen;
  }

  isGroupSelected(groupId: number): boolean {
    return this.selectedGroupIds.has(groupId);
  }

  toggleGroupSelection(groupId: number) {
    if (this.selectedGroupIds.has(groupId)) {
      this.selectedGroupIds.delete(groupId);
    } else {
      this.selectedGroupIds.add(groupId);
    }
    this.buildGroupViews();
    this.cdr.markForCheck();
  }

  selectAllGroups() {
    this.allGroups.forEach((g: any) => this.selectedGroupIds.add(g.id));
    this.selectedGroupIds.add(-1);
    this.buildGroupViews();
    this.cdr.markForCheck();
  }

  deselectAllGroups() {
    this.selectedGroupIds.clear();
    this.buildGroupViews();
    this.cdr.markForCheck();
  }

  get groupFilterLabel(): string {
    const total = this.allGroups.length + 1; // +1 for uncategorized
    if (this.selectedGroupIds.size === total) return `All groups`;
    if (this.selectedGroupIds.size === 0) return `No groups`;
    return `${this.selectedGroupIds.size} groups selected`;
  }

  onChannelClick(channel: any) {
    this.channelSelected.emit(channel.id);
  }

  handleDragOver(event: DragEvent) {
    event.preventDefault();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'copy';
    }
  }

  async handleDropNative(event: DragEvent, channel: any) {
    event.preventDefault();
    if (!event.dataTransfer) return;

    try {
      const streamIdsStr = event.dataTransfer.getData('application/json');
      if (!streamIdsStr) return;
      const streamIds: number[] = JSON.parse(streamIdsStr);

      const existingStreamIds = (channel.streams || []).map((s: any) => s.id);
      const newStreamIds = Array.from(new Set([...existingStreamIds, ...streamIds]));

      if (newStreamIds.length > existingStreamIds.length) {
        await firstValueFrom(this.api.updateChannel(channel.id, {
          streams: newStreamIds,
        }));
        
        console.log(`Assigned ${newStreamIds.length - existingStreamIds.length} stream(s) to "${channel.name}"`);
        this.fetchAll();
      }
    } catch (err) {
      console.error('Drag assignment failed:', err);
    }
  }

  showCreateDialog(content: PolymorpheusContent<any>): void {
    this.createChannelForm.reset();
    this.dialogs.open(content, { dismissible: true }).subscribe();
  }

  async submitCreateChannel(observer: any) {
    if (this.createChannelForm.invalid) return;

    try {
      const vals = this.createChannelForm.value;
      const payload: any = { name: vals.name };
      if (vals.channel_number != null) payload.channel_number = vals.channel_number;

      await firstValueFrom(this.api.createChannel(payload));
      observer.complete();
      this.fetchAll();
    } catch (err) {
      console.error('Error creating channel', err);
    }
  }
}
