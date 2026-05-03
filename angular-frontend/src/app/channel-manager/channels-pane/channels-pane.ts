import { ChangeDetectionStrategy, ChangeDetectorRef, Component, EventEmitter, inject, Input, OnInit, Output } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormControl, FormGroup, FormsModule, ReactiveFormsModule, Validators } from '@angular/forms';
import { ApiService } from '../../api.service';
import { WebSocketService } from '../../websocket.service';
import { TuiLoader, TuiButton, TuiDialogService, TuiTextfield } from '@taiga-ui/core';
import { PolymorpheusContent } from '@taiga-ui/polymorpheus';
import { ChannelListItemComponent } from '../channel-list-item/channel-list-item';
import { firstValueFrom } from 'rxjs';

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

  channels: any[] = [];
  loading = true;
  searchQuery = '';
  
  // Pagination
  totalCount = 0;
  currentPage = 1;
  hasNextPage = false;

  createChannelForm = new FormGroup({
    name: new FormControl('', Validators.required),
    channel_number: new FormControl<number | null>(null),
  });

  ngOnInit() {
    this.fetchChannels();

    this.ws.messages$.subscribe(msg => {
      if (msg.type === 'playlist_created' || msg.type === 'm3u_refresh_done' || msg.type === 'channel_updated') {
        this.fetchChannels();
      }
    });
  }

  fetchChannels() {
    this.loading = true;
    const params: any = {
      page: this.currentPage,
      ordering: 'channel_number',
    };
    if (this.searchQuery) {
      params.search = this.searchQuery;
    }

    this.api.getChannels(params).subscribe({
      next: (res: any) => {
        this.channels = res?.results || [];
        this.totalCount = res?.count || 0;
        this.hasNextPage = !!res?.next;
        this.loading = false;
        this.cdr.markForCheck();
      },
      error: (err) => {
        console.error('Error fetching channels', err);
        this.loading = false;
        this.cdr.markForCheck();
      }
    });
  }

  onSearchChange(query: string) {
    this.searchQuery = query;
    this.currentPage = 1;
    this.fetchChannels();
  }

  nextPage() {
    if (this.hasNextPage) {
      this.currentPage++;
      this.fetchChannels();
    }
  }

  prevPage() {
    if (this.currentPage > 1) {
      this.currentPage--;
      this.fetchChannels();
    }
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

      // The channel object from getChannels already has a streams array
      const existingStreamIds = (channel.streams || []).map((s: any) => s.id);

      // Combine and deduplicate
      const newStreamIds = Array.from(new Set([...existingStreamIds, ...streamIds]));

      if (newStreamIds.length > existingStreamIds.length) {
        await firstValueFrom(this.api.updateChannel(channel.id, {
          streams: newStreamIds,
        }));
        
        console.log(`Assigned ${newStreamIds.length - existingStreamIds.length} stream(s) to channel "${channel.name}"`);
        this.fetchChannels(); // Refresh to show updated stream counts
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
      this.fetchChannels(); // Refresh list
    } catch (err) {
      console.error('Error creating channel', err);
    }
  }
}
