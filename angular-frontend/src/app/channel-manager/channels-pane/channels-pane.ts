import { ChangeDetectionStrategy, Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ApiService } from '../../api.service';
import { TuiAccordion } from '@taiga-ui/kit';
import { TuiLoader } from '@taiga-ui/core';

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
  imports: [CommonModule, TuiAccordion, TuiLoader],
  templateUrl: './channels-pane.html',
  styleUrl: './channels-pane.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ChannelsPaneComponent implements OnInit {
  private readonly api = inject(ApiService);
  groups: ChannelGroup[] = [];
  loading = true;

  ngOnInit() {
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
}
