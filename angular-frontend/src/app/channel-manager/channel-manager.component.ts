import { ChangeDetectionStrategy, Component, HostListener, inject } from '@angular/core';
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
  // Resizable pane state — default 40% left, 60% right
  leftPaneWidth = 40;
  private isResizing = false;

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
