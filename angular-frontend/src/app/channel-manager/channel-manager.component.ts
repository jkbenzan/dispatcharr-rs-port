import { ChangeDetectionStrategy, Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ChannelsPaneComponent } from './channels-pane/channels-pane';
import { StreamsPaneComponent } from './streams-pane/streams-pane';

@Component({
  selector: 'app-channel-manager',
  standalone: true,
  imports: [CommonModule, ChannelsPaneComponent, StreamsPaneComponent],
  templateUrl: './channel-manager.component.html',
  styleUrl: './channel-manager.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ChannelManagerComponent {}
