import { ChangeDetectionStrategy, ChangeDetectorRef, Component, EventEmitter, inject, Input, OnInit, Output } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormControl, FormsModule, ReactiveFormsModule } from '@angular/forms';
import { ApiService } from '../../api.service';
import { TuiAccordion } from '@taiga-ui/kit';
import { TuiLoader } from '@taiga-ui/core';

@Component({
  selector: 'app-streams-pane',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    ReactiveFormsModule,
    TuiAccordion,
    TuiLoader
  ],
  templateUrl: './streams-pane.html',
  styleUrl: './streams-pane.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class StreamsPaneComponent implements OnInit {
  private readonly api = inject(ApiService);
  private readonly cdr = inject(ChangeDetectorRef);

  @Input() selectedStreamIds: number[] = [];
  @Output() toggleStreamSelection = new EventEmitter<number>();
  @Output() selectAllInGroup = new EventEmitter<{streams: any[], isSelected: boolean}>();
  @Output() assignClicked = new EventEmitter<void>();

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
      this.cdr.markForCheck();
    });

    this.api.getStreamGroups().subscribe((res: any) => {
      const results = Array.isArray(res) ? res : res?.results || [];
      this.groups = results.map((g: any) => g.name || g.group_name || String(g));
      this.cdr.markForCheck();
    });
  }

  fetchStreams() {
    this.loading = true;
    this.cdr.markForCheck();
    
    const params: any = {};

    if (this.providerControl.value?.length) {
      params.m3u_account = this.providerControl.value.join('::');
    }
    
    if (this.groupControl.value?.length) {
      params.channel_group = this.groupControl.value.join('::');
    }

    if (this.searchQuery) {
      params.search = this.searchQuery;
    }

    this.api.getStreams(params).subscribe({
      next: (res: any) => {
        const streams = Array.isArray(res) ? res : res?.results || [];
        this.groupStreams(streams);
        this.loading = false;
        this.cdr.markForCheck();
      },
      error: (err) => {
        console.error('Error fetching streams', err);
        this.loading = false;
        this.cdr.markForCheck();
      }
    });
  }

  private groupStreams(streams: any[]) {
    const grouped: { [key: string]: any[] } = {};
    streams.forEach(s => {
      const groupName = s.channel_group || s.channel_group_id || 'Ungrouped';
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

  isStreamSelected(id: number): boolean {
    return this.selectedStreamIds.includes(id);
  }

  isGroupFullySelected(group: any): boolean {
    return group.streams.length > 0 && group.streams.every((s: any) => this.isStreamSelected(s.id));
  }

  onToggleStream(id: number, event: Event) {
    event.stopPropagation();
    this.toggleStreamSelection.emit(id);
  }

  onToggleGroup(group: any, event: Event) {
    event.stopPropagation();
    const isSelected = !this.isGroupFullySelected(group);
    this.selectAllInGroup.emit({ streams: group.streams, isSelected });
  }

  onAssign() {
    this.assignClicked.emit();
  }

  handleDragStart(event: DragEvent, stream: any) {
    if (event.dataTransfer) {
      // If the dragged stream is part of the selection, drag all selected. Otherwise just drag this one.
      const idsToDrag = this.selectedStreamIds.includes(stream.id) 
        ? this.selectedStreamIds 
        : [stream.id];
        
      // Use 'application/json' as the data type — browser normalizes custom types to lowercase
      event.dataTransfer.setData('application/json', JSON.stringify(idsToDrag));
      event.dataTransfer.effectAllowed = 'copy';
      
      // Custom drag image showing count
      const dragEl = document.createElement('div');
      dragEl.textContent = `${idsToDrag.length} stream(s)`;
      dragEl.style.cssText = 'position: absolute; top: -1000px; background: #646cff; color: white; padding: 4px 12px; border-radius: 6px; font-size: 13px; font-weight: 500;';
      document.body.appendChild(dragEl);
      event.dataTransfer.setDragImage(dragEl, 0, 0);
      setTimeout(() => document.body.removeChild(dragEl), 0);
    }
  }
}
