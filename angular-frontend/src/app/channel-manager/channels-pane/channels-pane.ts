import { ChangeDetectionStrategy, Component, EventEmitter, inject, Input, OnInit, Output } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ApiService } from '../../api.service';
import { WebSocketService } from '../../websocket.service';
import { TuiAccordion } from '@taiga-ui/kit';
import { TuiLoader } from '@taiga-ui/core';
import { CdkDropList, CdkDragDrop } from '@angular/cdk/drag-drop';
import { firstValueFrom } from 'rxjs';

interface ChannelGroup {
  id: number;
  name: string;
  channel_count?: number;
  channels?: any[];
  loading?: boolean;
}

@Component({
  selector: 'app-channels-pane',
  standalone: true,
  imports: [CommonModule, TuiAccordion, TuiLoader, CdkDropList],
  templateUrl: './channels-pane.html',
  styleUrl: './channels-pane.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ChannelsPaneComponent implements OnInit {
  private readonly api = inject(ApiService);
  private readonly ws = inject(WebSocketService);
  
  @Input() selectedChannelId: number | null = null;
  @Output() channelSelected = new EventEmitter<number>();

  groups: ChannelGroup[] = [];
  loading = true;

  ngOnInit() {
    this.fetchGroups();

    this.ws.messages$.subscribe(msg => {
      if (msg.type === 'playlist_created' || msg.type === 'm3u_refresh_done' || msg.type === 'channel_updated') {
        this.fetchGroups();
      }
    });
  }

  fetchGroups() {
    this.api.getChannelGroups().subscribe({
      next: (res: any) => {
        const results = Array.isArray(res) ? res : res?.results || [];
        this.groups = results.map((g: any) => ({ ...g, channels: null, loading: false }));
        this.loading = false;
      },
      error: (err) => {
        console.error('Error fetching channel groups', err);
        this.loading = false;
      }
    });
  }

  onGroupExpand(group: ChannelGroup) {
    if (group.channels !== null) return;

    group.loading = true;
    this.api.queryChannels({ channel_group: group.id, page_size: 1000 }).subscribe({
      next: (res: any) => {
        group.channels = Array.isArray(res) ? res : res?.results || [];
        group.loading = false;
      },
      error: (err) => {
        console.error(`Error fetching channels for group ${group.name}`, err);
        group.loading = false;
        group.channels = [];
      }
    });
  }

  onChannelClick(channel: any) {
    this.channelSelected.emit(channel.id);
  }

  async handleDrop(event: CdkDragDrop<any>, channel: any) {
    const stream = event.item.data;
    if (!stream) return;

    try {
      const channelStreamsRes: any = await firstValueFrom(this.api.getChannelStreams(channel.id));
      const existingStreamIds = Array.isArray(channelStreamsRes) 
        ? channelStreamsRes.map((s: any) => s.stream_id ?? s.stream?.id ?? s.id) 
        : [];

      if (!existingStreamIds.includes(stream.id)) {
        await firstValueFrom(this.api.updateChannel({
          id: channel.id,
          streams: [...existingStreamIds, stream.id],
        }));
        
        console.log(`Assigned stream ${stream.name} to channel ${channel.name}`);
        // TODO: Notification
      }
    } catch (err) {
      console.error('Drag assignment failed:', err);
    }
  }
}
