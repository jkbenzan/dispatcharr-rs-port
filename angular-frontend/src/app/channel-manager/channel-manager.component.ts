import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ChannelsPaneComponent } from './channels-pane/channels-pane';
import { StreamsPaneComponent } from './streams-pane/streams-pane';
import { ApiService } from '../api.service';
import { firstValueFrom } from 'rxjs';

@Component({
  selector: 'app-channel-manager',
  standalone: true,
  imports: [CommonModule, ChannelsPaneComponent, StreamsPaneComponent],
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
      // Fetch current channel data to get existing stream ids
      const channelRes: any = await firstValueFrom(this.api.getChannelStreams(this.selectedChannelId));
      const channelData = channelRes?.results?.[0] || channelRes;
      const existingStreamIds = (channelData?.streams || []).map((s: any) => s.id);

      // Add new streams
      const newStreamIds = Array.from(new Set([...existingStreamIds, ...this.selectedStreamIds]));

      await firstValueFrom(this.api.updateChannel(this.selectedChannelId, {
        streams: newStreamIds,
      }));

      console.log(`Successfully assigned ${this.selectedStreamIds.length} stream(s)`);
      
      // Clear selection
      this.selectedStreamIds = [];
    } catch (err) {
      console.error('Failed to assign streams', err);
      alert('Failed to assign streams');
    }
  }
}
