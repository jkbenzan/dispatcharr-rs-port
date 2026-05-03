import { ChangeDetectionStrategy, Component, EventEmitter, inject, Input, OnInit, Output } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormControl, FormGroup, FormsModule, ReactiveFormsModule, Validators } from '@angular/forms';
import { ApiService } from '../../api.service';
import { WebSocketService } from '../../websocket.service';
import { TuiAccordion, TuiDataListWrapper, TuiSelect } from '@taiga-ui/kit';
import { TuiLoader, TuiButton, TuiDialogService, TuiDataList, TuiTextfield } from '@taiga-ui/core';
import { PolymorpheusContent } from '@taiga-ui/polymorpheus';
import { ChannelListItemComponent } from '../channel-list-item/channel-list-item';
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
  imports: [
    CommonModule,
    FormsModule,
    ReactiveFormsModule,
    TuiAccordion,
    TuiLoader,
    TuiButton,
    TuiTextfield,
    TuiSelect,
    TuiDataList,
    TuiDataListWrapper,
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
  
  @Input() selectedChannelId: number | null = null;
  @Output() channelSelected = new EventEmitter<number>();

  groups: ChannelGroup[] = [];
  loading = true;

  createChannelForm = new FormGroup({
    name: new FormControl('', Validators.required),
    channel_number: new FormControl<number | null>(null),
    group_id: new FormControl<number | null>(null),
  });

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

  handleDragOver(event: DragEvent) {
    if (event.dataTransfer?.types.includes('streamids') || event.dataTransfer?.types.includes('text/plain')) {
      event.preventDefault();
      event.dataTransfer.dropEffect = 'copy';
    }
  }

  async handleDropNative(event: DragEvent, channel: any) {
    event.preventDefault();
    if (!event.dataTransfer) return;

    try {
      const streamIdsStr = event.dataTransfer.getData('streamIds');
      if (!streamIdsStr) return;
      const streamIds: number[] = JSON.parse(streamIdsStr);

      const channelStreamsRes: any = await firstValueFrom(this.api.getChannelStreams(channel.id));
      const existingStreamIds = Array.isArray(channelStreamsRes) 
        ? channelStreamsRes.map((s: any) => s.stream_id ?? s.stream?.id ?? s.id) 
        : [];

      // Combine and deduplicate
      const newStreamIds = Array.from(new Set([...existingStreamIds, ...streamIds]));

      if (newStreamIds.length > existingStreamIds.length) {
        await firstValueFrom(this.api.updateChannel({
          id: channel.id,
          streams: newStreamIds,
        }));
        
        console.log(`Assigned streams to channel ${channel.name}`);
        // TODO: Notification
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
      
      // If the group_id is actually a group object from the select
      if (vals.group_id) {
        const groupObj = vals.group_id as any;
        payload.channel_group = groupObj.id ? groupObj.id : groupObj;
      }

      await firstValueFrom(this.api.createChannel(payload));
      
      observer.complete();
      this.fetchGroups(); // Refresh list
    } catch (err) {
      console.error('Error creating channel', err);
    }
  }
}
