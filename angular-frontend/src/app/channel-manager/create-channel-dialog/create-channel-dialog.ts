import { ChangeDetectionStrategy, Component, inject, OnInit, ChangeDetectorRef } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormBuilder, ReactiveFormsModule, Validators, FormControl } from '@angular/forms';
import { ApiService } from '../../api.service';
import { Subject } from 'rxjs';
import { debounceTime, distinctUntilChanged, switchMap } from 'rxjs/operators';
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
    channel_group_id: [null as any],
    stream_profile_id: [null as any],
    user_level: [1],
    logo_id: [null as any],
    is_adult: [false],
    tvg_id: [''],
    tvc_guide_stationid: [''],
    epg_data_id: [null as any]
  });

  groupSearchControl = new FormControl('');
  logoSearchControl = new FormControl('');
  newLogoUrlControl = new FormControl('');
  epgSearchControl = new FormControl('');
  
  showOnlyCustomGroups = false;
  browsingMatches = false;
  selectedMatch: any = null;

  userLevels = [
    { id: 1, name: 'Admin' },
    { id: 2, name: 'Standard User' },
    { id: 3, name: 'Streamer' }
  ];

  groups: any[] = [];
  profiles: any[] = [];
  logos: any[] = [];
  filteredLogos: any[] = [];
  suggestions: any[] = [];
  selectedStation: any = null; 
  
  selectedLogoPreview: string | null = null;
  addingLogo = false;
  uploadingLogo = false;
  addingGroup = false;
  submitting = false;
  searchingEPG = false;

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

    this.groupSearchControl.valueChanges.subscribe(val => {
      const group = this.groups.find(g => g.name === val);
      if (group) {
        this.form.get('channel_group_id')?.setValue(group.id, { emitEvent: false });
      } else {
        this.form.get('channel_group_id')?.setValue(null, { emitEvent: false });
      }
      this.cdr.markForCheck();
    });

    this.logoSearchControl.valueChanges.pipe(
      debounceTime(300),
      distinctUntilChanged()
    ).subscribe(val => {
      this.filterLogos(val || '');
    });

    this.form.get('logo_id')?.valueChanges.subscribe(val => {
      const logo = this.logos.find(l => l.id === val);
      this.selectedLogoPreview = logo ? (logo.cache_url || logo.url) : null;
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
      if (val && val.length > 2 && !this.browsingMatches) {
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
      this.groups = allGroups.sort((a: any, b: any) => a.name.localeCompare(b.name));
      this.cdr.markForCheck();
    });
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
    this.selectedStation = station;
    this.form.patchValue({
      name: station.name,
      tvg_id: station.call_sign || station.name,
      tvc_guide_stationid: station.station_id || ''
    });
    this.suggestions = [];
    this.browsingMatches = false;
    this.cdr.markForCheck();
  }

  // --- EPG Browser Logic ---

  openEpgBrowser() {
    this.browsingMatches = true;
    const name = this.form.get('name')?.value;
    if (name) {
      this.epgSearchControl.setValue(name);
      this.performEpgSearch(name);
    }
    this.cdr.markForCheck();
  }

  performEpgSearch(query: string) {
    if (!query) return;
    this.searchingEPG = true;
    this.cdr.markForCheck();
    this.api.suggestMatches(query).subscribe((res: any) => {
      this.suggestions = res.matches.map((m: any) => m.station);
      if (this.suggestions.length > 0) {
        this.selectedMatch = this.suggestions[0];
      }
      this.searchingEPG = false;
      this.cdr.markForCheck();
    });
  }

  selectMatch(match: any) {
    this.selectedMatch = match;
    this.cdr.markForCheck();
  }

  applyMatchField(field: 'name' | 'tvg_id' | 'tvc_guide_stationid' | 'logo' | 'all') {
    if (!this.selectedMatch) return;

    if (field === 'name' || field === 'all') {
      this.form.patchValue({ name: this.selectedMatch.name });
    }
    if (field === 'tvg_id' || field === 'all') {
      this.form.patchValue({ tvg_id: this.selectedMatch.call_sign || this.selectedMatch.name });
    }
    if (field === 'tvc_guide_stationid' || field === 'all') {
      this.form.patchValue({ tvc_guide_stationid: this.selectedMatch.station_id });
    }
    if (field === 'logo' || field === 'all') {
      if (this.selectedMatch.logo_uri) {
        this.newLogoUrlControl.setValue(this.selectedMatch.logo_uri);
        this.addLogoFromUrl();
      }
    }
    
    this.selectedStation = this.selectedMatch;
    if (field === 'all') {
      this.browsingMatches = false;
    }
    this.cdr.markForCheck();
  }

  // --- Shortcut Logic (linked to selectedStation) ---

  useEpgName() { if (this.selectedStation) this.form.patchValue({ name: this.selectedStation.name }); }
  useEpgTvgId() { if (this.selectedStation) this.form.patchValue({ tvg_id: this.selectedStation.call_sign || this.selectedStation.name }); }
  useEpgLogo() { if (this.selectedStation?.logo_uri) { this.newLogoUrlControl.setValue(this.selectedStation.logo_uri); this.addLogoFromUrl(); } }

  useDummyEpg() {
    this.form.patchValue({ tvg_id: 'dummy', tvc_guide_stationid: 'dummy', epg_data_id: null });
    this.selectedStation = { name: 'Dummy EPG', station_id: 'dummy' };
    this.cdr.markForCheck();
  }

  clearEpg() {
    this.selectedStation = null;
    this.form.patchValue({ epg_data_id: null });
    this.cdr.markForCheck();
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
        }
      });
    }
  }

  // --- Logo / Group Logic ---

  onLogoError() { this.selectedLogoPreview = null; this.cdr.markForCheck(); }

  addLogoFromUrl() {
    const url = this.newLogoUrlControl.value?.trim();
    if (!url) return;
    this.addingLogo = true;
    this.api.createLogo({ name: url.split('/').pop() || 'Imported Logo', url }).subscribe({
      next: (res: any) => {
        this.logos.unshift(res);
        this.filterLogos(this.logoSearchControl.value || '');
        this.form.patchValue({ logo_id: res.id });
        this.newLogoUrlControl.setValue('');
        this.addingLogo = false;
        this.cdr.markForCheck();
      },
      error: () => { this.addingLogo = false; this.cdr.markForCheck(); }
    });
  }

  uploadLogoFile(event: Event) {
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file) return;
    this.uploadingLogo = true;
    this.api.uploadLogo(file).subscribe({
      next: (res: any) => {
        this.logos.unshift(res);
        this.filterLogos(this.logoSearchControl.value || '');
        this.form.patchValue({ logo_id: res.id });
        this.uploadingLogo = false;
        this.cdr.markForCheck();
      },
      error: () => { this.uploadingLogo = false; this.cdr.markForCheck(); }
    });
  }

  addGroup() {
    const name = prompt("Enter new Group name:");
    if (!name) return;
    this.addingGroup = true;
    this.api.createChannelGroup(name).subscribe({
      next: (res: any) => {
        this.groups.push(res);
        this.groups.sort((a: any, b: any) => a.name.localeCompare(b.name));
        this.groupSearchControl.setValue(res.name);
        this.form.patchValue({ channel_group_id: res.id });
        this.addingGroup = false;
        this.cdr.markForCheck();
      },
      error: () => { this.addingGroup = false; this.cdr.markForCheck(); }
    });
  }

  submit() {
    if (this.form.invalid) return;
    this.submitting = true;
    this.api.createChannel(this.form.value).subscribe({
      next: () => this.context.completeWith(true),
      error: () => { this.submitting = false; this.cdr.markForCheck(); }
    });
  }

  cancel() { this.context.completeWith(false); }
}
