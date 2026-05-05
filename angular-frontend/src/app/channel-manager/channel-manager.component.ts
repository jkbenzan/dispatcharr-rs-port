import { ChangeDetectionStrategy, Component, HostListener, inject, ViewChild, ChangeDetectorRef } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ChannelsPaneComponent } from './channels-pane/channels-pane';
import { StreamsPaneComponent } from './streams-pane/streams-pane';

/**
 * Channel Manager orchestrator — manages the resizable two-pane layout.
 *
 * Left pane: Channels pane (nested Group → Channel → Stream tree, self-contained).
 * Right pane: Streams pane (available stream catalog, independent).
 */
@Component({
  selector: 'app-channel-manager',
  standalone: true,
  imports: [CommonModule, ChannelsPaneComponent, StreamsPaneComponent],
  templateUrl: './channel-manager.component.html',
  styleUrl: './channel-manager.component.less',
  changeDetection: ChangeDetectionStrategy.Default,
})
export class ChannelManagerComponent {
  private readonly cdr = inject(ChangeDetectorRef);

  @ViewChild(ChannelsPaneComponent) channelsPane!: ChannelsPaneComponent;
  @ViewChild(StreamsPaneComponent) streamsPane!: StreamsPaneComponent;

  // Resizable pane state — default 40% left
  leftPaneWidth = 40;
  private isResizing = false;

  // Toolbar state
  toolbarExpanded = false;

  // Selection tracking (for enabling/disabling toolbar buttons)
  selectedStreamIds: Set<number> = new Set();
  selectedChannelIds: Set<number> = new Set();

  onStreamSelectionChange(ids: Set<number>) {
    this.selectedStreamIds = ids;
    this.cdr.markForCheck();
  }

  onChannelSelectionChange(ids: Set<number>) {
    this.selectedChannelIds = ids;
    this.cdr.markForCheck();
  }

  /**
   * Bulk assign all selected streams from the Streams Pane
   * to the single selected channel in the Channels Pane.
   */
  assignSelected() {
    console.log('[Assign] streams:', this.selectedStreamIds.size, 'channels:', this.selectedChannelIds.size);
    if (this.selectedStreamIds.size === 0 || this.selectedChannelIds.size !== 1) {
      console.warn('[Assign] Blocked — need exactly 1 channel and ≥1 stream selected');
      return;
    }

    const streamIds = Array.from(this.selectedStreamIds);
    this.channelsPane.assignSelectedStreams(streamIds).subscribe({
      next: (res) => {
        if (res !== null) {
          // Success: clear the stream selection
          this.streamsPane.deselectAllStreams();
        }
      },
      error: (err) => console.error('Bulk assignment failed:', err)
    });
  }

  // --- Toolbar logic ---

  toggleToolbar() {
    this.toolbarExpanded = !this.toolbarExpanded;
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
