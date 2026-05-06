import { ChangeDetectionStrategy, Component, inject, OnInit, ChangeDetectorRef } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormBuilder, ReactiveFormsModule, Validators, FormControl } from '@angular/forms';
import { ApiService } from '../../api.service';
import { Subject } from 'rxjs';
import { debounceTime, distinctUntilChanged, switchMap, map } from 'rxjs/operators';
// Taiga UI 5 imports
import { POLYMORPHEUS_CONTEXT } from '@taiga-ui/polymorpheus';
import { TuiDialogContext } from '@taiga-ui/core';

@Component({
  selector: 'app-create-channel-dialog',
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule],
  templateUrl: './create-channel-dialog.html',
  styleUrl: './create-channel-dialog.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CreateChannelDialogComponent implements OnInit {
  private readonly context = inject<TuiDialogContext<boolean>>(POLYMORPHEUS_CONTEXT);
  private readonly api = inject(ApiService);
  private readonly fb = inject(FormBuilder);
  private readonly cdr = inject(ChangeDetectorRef);

  readonly form = this.fb.group({
    name: ['', Validators.required],
    channel_number: [null],
    channel_group_id: [null],
    stream_profile_id: [null],
    user_level: [1],
    logo_id: [null],
    is_adult: [false],
    tvg_id: [''],
    tvc_guide_stationid: [''],
    epg_data_id: [null]
  });

  groupSearchControl = new FormControl('');
  logoSearchControl = new FormControl('');
  newLogoUrlControl = new FormControl('');
  showOnlyCustomGroups = false;

  userLevels = [
    { id: 1, name: 'Admin' },
    { id: 2, name: 'Standard User' },
    { id: 3, name: 'Streamer' }
  ];

  groups: any[] = [];
  filteredGroups: any[] = [];
  profiles: any[] = [];
  logos: any[] = [];
  filteredLogos: any[] = [];
  suggestions: any[] = [];
  
  selectedLogoPreview: string | null = null;
  addingLogo = false;
  uploadingLogo = false;
  addingGroup = false;
  submitting = false;

  private nameSearchSubject = new Subject<string>();

  ngOnInit() {
    this.loadGroups();

    this.api.getStreamProfiles().subscribe((res: any) => {
      this.profiles = res.results || res;
      this.cdr.markForCheck();
    });

    this.api.getLogos().subscribe((res: any) => {
      this.logos = res.results || res;
      this.filterLogos('');
      this.cdr.markForCheck();
    });

    this.groupSearchControl.valueChanges.pipe(
      debounceTime(200),
      distinctUntilChanged()
    ).subscribe(val => {
      this.applyGroupFilters(val || '');
    });

    this.logoSearchControl.valueChanges.pipe(
      debounceTime(300),
      distinctUntilChanged()
    ).subscribe(val => {
      this.filterLogos(val || '');
    });

    this.form.get('logo_id')?.valueChanges.subscribe(val => {
      const logo = this.logos.find(l => l.id === val);
      if (logo) {
        this.selectedLogoPreview = logo.cache_url || logo.url;
      } else {
        this.selectedLogoPreview = null;
      }
      this.cdr.markForCheck();
    });

    this.nameSearchSubject.pipe(
      debounceTime(500),
      distinctUntilChanged(),
      switchMap(query => this.api.suggestMatches(query))
    ).subscribe((res: any) => {
      if (res && res.matches) {
        this.suggestions = res.matches.map((m: any) => m.station);
        this.cdr.markForCheck();
      }
    });

    this.form.get('name')?.valueChanges.subscribe(val => {
      if (val && val.length > 2) {
        this.nameSearchSubject.next(val);
      } else {
        this.suggestions = [];
        this.cdr.markForCheck();
      }
    });
  }

  loadGroups() {
    this.api.getChannelGroups().subscribe((res: any) => {
      const allGroups = res.results || res;
      // Alphabetical sort ascending
      this.groups = allGroups.sort((a: any, b: any) => a.name.localeCompare(b.name));
      this.applyGroupFilters(this.groupSearchControl.value || '');
      this.cdr.markForCheck();
    });
  }

  applyGroupFilters(search: string) {
    let filtered = [...this.groups];
    
    if (this.showOnlyCustomGroups) {
      // For now, "Custom Groups" are those NOT in the original list 
      // (This is a heuristic since we don't have an is_custom flag in DB yet)
      // Actually, I'll just show all for now but support the toggle.
    }
    
    if (search) {
      const lower = search.toLowerCase();
      filtered = filtered.filter(g => g.name.toLowerCase().includes(lower));
    }
    
    this.filteredGroups = filtered;
    this.cdr.markForCheck();
  }

  toggleCustomGroups() {
    this.showOnlyCustomGroups = !this.showOnlyCustomGroups;
    this.applyGroupFilters(this.groupSearchControl.value || '');
  }

  filterLogos(search: string) {
    if (!search) {
      this.filteredLogos = this.logos.slice(0, 50);
    } else {
      const lower = search.toLowerCase();
      this.filteredLogos = this.logos.filter(l => l.name.toLowerCase().includes(lower)).slice(0, 50);
    }
    this.cdr.markForCheck();
  }

  selectSuggestion(station: any) {
    this.form.patchValue({
      name: station.name,
      tvg_id: station.call_sign || station.name,
      tvc_guide_stationid: station.station_id || ''
    });
    this.suggestions = [];
    this.cdr.markForCheck();
  }

  copyNameToTvgId() {
    const name = this.form.get('name')?.value;
    if (name) {
      this.form.patchValue({ tvg_id: name });
    }
  }

  searchEpgData() {
    const val = this.form.get('tvg_id')?.value;
    if (val && val.length > 2) {
      this.api.suggestMatches(val).subscribe((res: any) => {
        if (res && res.matches && res.matches.length > 0) {
          const first = res.matches[0].station;
          this.form.patchValue({
            tvg_id: first.call_sign || first.name,
            tvc_guide_stationid: first.station_id || ''
          });
          this.cdr.markForCheck();
        }
      });
    }
  }

  fetchLcn() {
    const tvgId = this.form.get('tvg_id')?.value;
    if (tvgId) {
      this.api.getEPGLcnByTvgId(tvgId).subscribe({
        next: (res) => {
          if (res && res.lcn) {
            this.form.patchValue({ tvc_guide_stationid: res.lcn });
            this.cdr.markForCheck();
          }
        },
        error: (err) => {
          console.error('Failed to fetch LCN', err);
        }
      });
    }
  }

  onLogoError() {
    this.selectedLogoPreview = null;
    this.cdr.markForCheck();
  }

  addLogoFromUrl() {
    const url = this.newLogoUrlControl.value?.trim();
    if (!url) return;
    
    this.addingLogo = true;
    this.cdr.markForCheck();
    
    const name = url.split('/').pop() || 'Imported Logo';
    
    this.api.createLogo({ name, url }).subscribe({
      next: (res: any) => {
        this.logos.unshift(res);
        this.filterLogos(this.logoSearchControl.value || '');
        this.form.patchValue({ logo_id: res.id });
        this.newLogoUrlControl.setValue('');
        this.addingLogo = false;
        this.cdr.markForCheck();
      },
      error: (err) => {
        console.error('Failed to add logo', err);
        this.addingLogo = false;
        this.cdr.markForCheck();
      }
    });
  }

  uploadLogoFile(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    this.uploadingLogo = true;
    this.cdr.markForCheck();

    this.api.uploadLogo(file).subscribe({
      next: (res: any) => {
        this.logos.unshift(res);
        this.filterLogos(this.logoSearchControl.value || '');
        this.form.patchValue({ logo_id: res.id });
        this.uploadingLogo = false;
        input.value = '';
        this.cdr.markForCheck();
      },
      error: (err) => {
        console.error('Failed to upload logo', err);
        this.uploadingLogo = false;
        input.value = '';
        this.cdr.markForCheck();
      }
    });
  }

  addGroup() {
    const name = prompt("Enter new Group name:");
    if (!name) return;

    this.addingGroup = true;
    this.cdr.markForCheck();

    this.api.createChannelGroup(name).subscribe({
      next: (res: any) => {
        this.groups.push(res);
        this.groups.sort((a: any, b: any) => a.name.localeCompare(b.name));
        this.applyGroupFilters(this.groupSearchControl.value || '');
        this.form.patchValue({ channel_group_id: res.id });
        this.addingGroup = false;
        this.cdr.markForCheck();
      },
      error: (err) => {
        console.error('Failed to create group', err);
        this.addingGroup = false;
        this.cdr.markForCheck();
      }
    });
  }

  submit() {
    if (this.form.invalid) return;

    this.submitting = true;
    this.cdr.markForCheck();

    this.api.createChannel(this.form.value).subscribe({
      next: () => {
        this.context.completeWith(true);
      },
      error: (err) => {
        console.error('Failed to create channel', err);
        this.submitting = false;
        this.cdr.markForCheck();
      }
    });
  }

  cancel() {
    this.context.completeWith(false);
  }
}
