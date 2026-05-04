import { Component, Input, Output, EventEmitter, ChangeDetectionStrategy } from '@angular/core';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-channel-list-item',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './channel-list-item.html',
  styles: [`
    .channel-item {
      display: flex;
      align-items: center;
      padding: 8px 12px;
      border-bottom: 1px solid var(--tui-base-03);
      cursor: pointer;
      gap: 8px;
      background: var(--tui-base-01);
      transition: background 0.2s;
    }
    .channel-item:hover {
      background: var(--tui-base-02);
    }
    .channel-item.selected {
      background: var(--tui-primary-hover);
      color: var(--tui-base-01);
    }
    .expand-icon {
      flex-shrink: 0;
      color: var(--tui-text-03);
      transition: color 0.15s;
    }
    .expand-icon:hover {
      color: var(--tui-text-01);
    }
    .channel-logo {
      width: 48px;
      height: 36px;
      object-fit: contain;
      background: var(--tui-base-02);
      border-radius: 4px;
      flex-shrink: 0;
    }
    .channel-info {
      flex: 1;
      display: flex;
      flex-direction: column;
      overflow: hidden;
    }
    .channel-name {
      font-weight: 500;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .channel-meta {
      font-size: 12px;
      color: var(--tui-text-02);
      display: flex;
      gap: 8px;
    }
    .channel-item.selected .channel-meta {
      color: var(--tui-base-02);
    }
    .badge {
      background: var(--tui-base-03);
      padding: 2px 6px;
      border-radius: 4px;
      font-size: 10px;
      font-weight: bold;
    }
    .stream-count {
      color: var(--tui-success-fill);
      font-weight: 500;
    }

    /* Assigned streams sub-list */
    .assigned-streams {
      background: rgba(255, 255, 255, 0.03);
      border-bottom: 1px solid var(--tui-base-03);
      padding: 4px 0 4px 36px;
    }
    .assigned-stream-item {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 4px 12px;
      font-size: 12px;
      border-radius: 4px;
      transition: background 0.15s;
    }
    .assigned-stream-item:hover {
      background: var(--tui-base-02);
    }
    .assigned-stream-info {
      display: flex;
      flex-direction: column;
      overflow: hidden;
    }
    .assigned-stream-name {
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      font-weight: 500;
    }
    .assigned-stream-provider {
      font-size: 11px;
      opacity: 0.5;
    }
  `],
  changeDetection: ChangeDetectionStrategy.OnPush
})
export class ChannelListItemComponent {
  @Input() channel: any;
  @Input() isSelected = false;
  @Output() clicked = new EventEmitter<void>();
  
  @Output() itemDragOver = new EventEmitter<DragEvent>();
  @Output() itemDrop = new EventEmitter<DragEvent>();

  expanded = false;

  toggleExpand(event: Event) {
    event.stopPropagation(); // Don't select the channel when expanding
    this.expanded = !this.expanded;
  }
}
