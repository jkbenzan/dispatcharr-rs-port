import { ChangeDetectionStrategy, Component, HostListener, inject } from '@angular/core';
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

  // Resizable pane state — default 40% left
  leftPaneWidth = 40;
  private isResizing = false;

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

  // --- Resize logic ---

  onDividerMouseDown(event: MouseEvent) {
    event.preventDefault();
    this.isResizing = true;
    document.body.classList.add('resizing');
  }

  @HostListener('document:mousemove', ['$event'])
  onMouseMove(event: MouseEvent) {
    if (!this.isResizing) return;
    const container = document.querySelector('.split-pane') as HTMLElement;
    if (!container) return;

    const rect = container.getBoundingClientRect();
    const x = event.clientX - rect.left;
    let pct = (x / rect.width) * 100;

    // Clamp between 15% and 75%
    pct = Math.max(15, Math.min(75, pct));
    this.leftPaneWidth = pct;
  }

  @HostListener('document:mouseup')
  onMouseUp() {
    if (this.isResizing) {
      this.isResizing = false;
      document.body.classList.remove('resizing');
    }
  }
}
