import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ChannelsPaneComponent } from './channels-pane/channels-pane';
import { StreamsPaneComponent } from './streams-pane/streams-pane';
import { ApiService } from '../api.service';
import { firstValueFrom } from 'rxjs';
import { CdkDropListGroup } from '@angular/cdk/drag-drop';

@Component({
  selector: 'app-channel-manager',
  standalone: true,
  imports: [CommonModule, ChannelsPaneComponent, StreamsPaneComponent, CdkDropListGroup],
  templateUrl: './channel-manager.component.html',
  styleUrl: './channel-manager.component.less',
  changeDetection: ChangeDetectionStrategy.Default,
})
export class ChannelManagerComponent {
  private api = inject(ApiService);

  selectedChannelId: number | null = null;
  selectedStreamIds: number[] = [];

  onChannelSelected(channelId: number) {
    this.selectedChannelId = channelId;
  }

  onToggleStreamSelection(streamId: number) {
    if (this.selectedStreamIds.includes(streamId)) {
      this.selectedStreamIds = this.selectedStreamIds.filter(id => id !== streamId);
    } else {
      this.selectedStreamIds = [...this.selectedStreamIds, streamId];
    }
  }

  onSelectAllInGroup(event: { streams: any[], isSelected: boolean }) {
    if (event.isSelected) {
      const newIds = new Set(this.selectedStreamIds);
      event.streams.forEach(s => newIds.add(s.id));
      this.selectedStreamIds = Array.from(newIds);
    } else {
      const streamIdsToDeselect = new Set(event.streams.map(s => s.id));
      this.selectedStreamIds = this.selectedStreamIds.filter(id => !streamIdsToDeselect.has(id));
    }
  }

  async onAssign() {
    if (!this.selectedChannelId) {
      alert('Please select a channel on the left first.');
      return;
    }

    try {
      // Get current streams for the selected channel
      const channelStreamsRes: any = await firstValueFrom(this.api.getChannelStreams(this.selectedChannelId));
      const existingStreamIds = Array.isArray(channelStreamsRes) 
        ? channelStreamsRes.map((s: any) => s.stream_id ?? s.stream?.id ?? s.id) 
        : [];

      // Add new streams
      const newStreamIds = Array.from(new Set([...existingStreamIds, ...this.selectedStreamIds]));

      await firstValueFrom(this.api.updateChannel({
        id: this.selectedChannelId,
        streams: newStreamIds,
      }));

      // In a real app we'd use Taiga UI alerts, for now simple alert
      console.log(`Successfully assigned ${this.selectedStreamIds.length} stream(s)`);
      
      // Clear selection
      this.selectedStreamIds = [];
      
      // TODO: refresh channels or stream counts if necessary
    } catch (err) {
      console.error('Failed to assign streams', err);
      alert('Failed to assign streams');
    }
  }
}
