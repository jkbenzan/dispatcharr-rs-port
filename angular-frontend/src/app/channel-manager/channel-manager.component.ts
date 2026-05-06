import { ChangeDetectionStrategy, Component, HostListener, inject, ViewChild, ChangeDetectorRef } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ChannelsPaneComponent } from './channels-pane/channels-pane';
import { StreamsPaneComponent } from './streams-pane/streams-pane';
import { CreateChannelDialogComponent } from './create-channel-dialog/create-channel-dialog';

/**
 * Channel Manager orchestrator — manages the resizable two-pane layout.
 *
 * Left pane: Channels pane (nested Group → Channel → Stream tree, self-contained).
 * Right pane: Streams pane (available stream catalog, independent).
 */
@Component({
  selector: 'app-channel-manager',
  standalone: true,
  imports: [CommonModule, ChannelsPaneComponent, StreamsPaneComponent, CreateChannelDialogComponent],
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

  // Create Channel Dialog state — rendered as an inline overlay
  // instead of using TuiDialogService (which wraps content in its own
  // dialog chrome, causing a "double modal" effect).
  showCreateChannelDialog = false;

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

  /**
   * Open the Create Channel dialog as a custom overlay.
   * We avoid TuiDialogService here because it wraps content in its own
   * dialog chrome (header, backdrop, sizing), which duplicates our
   * component's custom dialog UI and causes positioning issues on
   * smaller screens.
   */
  openCreateChannelDialog() {
    this.showCreateChannelDialog = true;
    this.cdr.markForCheck();
  }

  /**
   * Handle the result from the Create Channel dialog.
   * @param created - true if a channel was successfully created, false if cancelled.
   */
  onCreateChannelDialogClose(created: boolean) {
    this.showCreateChannelDialog = false;
    if (created) {
      // Refresh the channels pane to show the newly created channel
      this.channelsPane.loadData();
    }
    this.cdr.markForCheck();
  }

  /**
   * Close dialog when Escape is pressed (dismissible behavior).
   */
  @HostListener('document:keydown.escape')
  onEscapeKey() {
    if (this.showCreateChannelDialog) {
      this.showCreateChannelDialog = false;
      this.cdr.markForCheck();
    }
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
