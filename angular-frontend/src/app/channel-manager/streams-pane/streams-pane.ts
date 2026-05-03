import { ChangeDetectionStrategy, Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormControl, FormsModule, ReactiveFormsModule } from '@angular/forms';
import { ApiService } from '../../api.service';
import { TuiAccordion, TuiMultiSelect } from '@taiga-ui/kit';
import { TuiLoader, TuiTextfield } from '@taiga-ui/core';

@Component({
  selector: 'app-streams-pane',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    ReactiveFormsModule,
    TuiAccordion,
    TuiMultiSelect,
    TuiLoader,
    TuiTextfield,
  ],
  templateUrl: './streams-pane.html',
  styleUrl: './streams-pane.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class StreamsPaneComponent implements OnInit {
  private readonly api = inject(ApiService);

  readonly providerControl = new FormControl<string[]>([]);
  readonly groupControl = new FormControl<string[]>([]);
  searchQuery = '';

  providers: string[] = [];
  groups: string[] = [];
  
  streamGroups: any[] = [];
  loading = false;

  ngOnInit() {
    this.fetchFilterData();
    this.fetchStreams();
  }

  private fetchFilterData() {
    this.api.getPlaylists().subscribe((res: any) => {
      const results = Array.isArray(res) ? res : res?.results || [];
      this.providers = results.map((p: any) => p.name || String(p.id));
    });

    this.api.getStreamGroups().subscribe((res: any) => {
      const results = Array.isArray(res) ? res : res?.results || [];
      this.groups = results.map((g: any) => g.name || g.group_name || String(g));
    });
  }

  fetchStreams() {
    this.loading = true;
    const params: any = {
      page_size: 100,
    };

    if (this.providerControl.value?.length) {
      params.m3u_account = this.providerControl.value;
    }
    
    if (this.groupControl.value?.length) {
      params.channel_group = this.groupControl.value;
    }

    if (this.searchQuery) {
      params.search = this.searchQuery;
    }

    this.api.queryStreams(params).subscribe({
      next: (res: any) => {
        const streams = Array.isArray(res) ? res : res?.results || [];
        this.groupStreams(streams);
        this.loading = false;
      },
      error: (err) => {
        console.error('Error fetching streams', err);
        this.loading = false;
      }
    });
  }

  private groupStreams(streams: any[]) {
    const grouped: { [key: string]: any[] } = {};
    streams.forEach(s => {
      const groupName = s.channel_group || 'Ungrouped';
      if (!grouped[groupName]) grouped[groupName] = [];
      grouped[groupName].push(s);
    });

    this.streamGroups = Object.keys(grouped).map(name => ({
      name,
      streams: grouped[name],
      expanded: !!this.searchQuery // Auto-expand on search
    }));
  }

  onSearchChange(query: string) {
    this.searchQuery = query;
    this.fetchStreams();
  }
}
