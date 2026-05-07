import { ChangeDetectionStrategy, Component, inject, OnInit, ChangeDetectorRef, Output, EventEmitter } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormBuilder, ReactiveFormsModule, FormsModule, Validators, FormControl } from '@angular/forms';
import { ApiService } from '../../api.service';
import { Subject } from 'rxjs';
import { debounceTime, distinctUntilChanged, switchMap } from 'rxjs/operators';


@Component({
  selector: 'app-create-channel-dialog',
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule, FormsModule],
  templateUrl: './create-channel-dialog.html',
  styleUrl: './create-channel-dialog.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CreateChannelDialogComponent implements OnInit {
  /**
   * Emits when the dialog should close.
   * true  = a channel was successfully created (parent should refresh data)
   * false = user cancelled (no changes made)
   */
  @Output() dialogClose = new EventEmitter<boolean>();

  private readonly api = inject(ApiService);
  private readonly fb = inject(FormBuilder);
  private readonly cdr = inject(ChangeDetectorRef);

  // ─── Main Form ───
  readonly form = this.fb.group({
    name: ['', Validators.required],
    channel_number: [null as number | null],
    channel_group_id: [null as number | null],
    stream_profile_id: [null as number | null],
    user_level: [3],
    logo_id: [null as number | null],
    is_adult: [false],
    tvg_id: [''],
    tvc_guide_stationid: [''],
    epg_data_id: [null as number | null]
  });

  // ─── Search & Filter Controls ───
  groupSearchControl = new FormControl('');
  logoSearchControl = new FormControl('');
  newLogoUrlControl = new FormControl('');
  epgSearchControl = new FormControl('');       // channel_data.db search
  epgFilterControl = new FormControl('');        // EPG selector filter
  channelSearchControl = new FormControl('');    // Existing channels sidebar filter

  // ─── UI State ───
  showOnlyCustomGroups = false;
  browsingMatches = false;               // legacy flag for auto-suggest suppression
  dbBrowserExpanded = false;             // Channel Data Lookup section collapsed by default
  dbSearchError: string | null = null;   // error message shown in the Channel Data Lookup section
  epgDropdownOpen = false;               // EPG assignment dropdown
  addingGroup = false;                   // inline group creation mode
  newGroupName = '';                     // inline group name input
  submitting = false;
  searchingEPG = false;
  uploadingLogo = false;
  addingLogo = false;

  // ─── Data Collections ───
  userLevels = [
    { id: 1, name: 'Admin' },
    { id: 2, name: 'Standard User' },
    { id: 3, name: 'Streamer' }
  ];

  groups: any[] = [];
  filteredGroups: any[] = [];            // filtered by search + custom toggle
  profiles: any[] = [];
  logos: any[] = [];
  epgData: any[] = [];                   // all EPG data records from epg_epgdata table
  filteredEpgData: any[] = [];           // filtered by search in EPG dropdown
  existingChannels: any[] = [];          // from channels summary
  filteredExistingChannels: any[] = [];  // filtered by sidebar search

  // ─── Selection State ───
  selectedStation: any = null;           // selected station from channel_data.db (for shortcuts)
  selectedMatch: any = null;             // highlighted match in the db browser
  selectedLogoPreview: string | null = null;
  suggestions: any[] = [];              // auto-suggest results as user types channel name
  dbSearchResults: any[] = [];          // explicit search results in db browser section

  // ─── Computed display for EPG selector ───
  get selectedEpgDisplay(): string {
    const epgId = this.form.get('epg_data_id')?.value;
    if (!epgId) return 'None selected';
    const match = this.epgData.find(e => e.id === epgId);
    return match ? `${match.name} (${match.tvg_id})` : 'None selected';
  }

  // ─── Computed display for filtered groups (search + custom toggle) ───
  // Searches group name AND M3U account names so users can find groups
  // by provider name (e.g. typing "iptv" finds groups from that provider).
  get displayedGroups(): any[] {
    const search = (this.groupSearchControl.value || '').toLowerCase();
    let list = this.showOnlyCustomGroups
      ? this.groups.filter(g => g.is_custom === true)
      : this.groups;
    if (search) {
      list = list.filter(g => {
        // Match against group name
        if (g.name.toLowerCase().includes(search)) return true;
        // Match against any associated M3U account name
        if (g.m3u_accounts && Array.isArray(g.m3u_accounts)) {
          return g.m3u_accounts.some((acc: string) => acc.toLowerCase().includes(search));
        }
        return false;
      });
    }
    return list;
  }

  private nameSearchSubject = new Subject<string>();

  // ═══════════════════════════════════════════════════════════
  // LIFECYCLE
  // ═══════════════════════════════════════════════════════════

  ngOnInit() {
    this.loadGroups();
    this.loadExistingChannels();

    // Load stream profiles
    this.api.getStreamProfiles().subscribe((res: any) => {
      this.profiles = res.results || res;
      this.cdr.markForCheck();
    });

    // Load logos
    this.api.getLogos().subscribe((res: any) => {
      this.logos = res.results || res;
      this.cdr.markForCheck();
    });

    // Load EPG data for the EPG assignment dropdown
    this.api.getEpgData().subscribe({
      next: (res: any) => {
        this.epgData = res.results || res;
        this.filteredEpgData = this.epgData.slice(0, 100); // render first 100
        this.cdr.markForCheck();
      },
      error: () => {
        // EPG data might not be available — degrade gracefully
        this.epgData = [];
        this.filteredEpgData = [];
        this.cdr.markForCheck();
      }
    });

    // EPG dropdown filter: filter epgData as user types
    this.epgFilterControl.valueChanges.pipe(
      debounceTime(200),
      distinctUntilChanged()
    ).subscribe(val => {
      const search = (val || '').toLowerCase();
      if (!search) {
        this.filteredEpgData = this.epgData.slice(0, 100);
      } else {
        this.filteredEpgData = this.epgData
          .filter(e => e.name.toLowerCase().includes(search) || e.tvg_id.toLowerCase().includes(search))
          .slice(0, 100);
      }
      this.cdr.markForCheck();
    });

    // Logo ID change → update preview
    this.form.get('logo_id')?.valueChanges.subscribe(val => {
      const logo = this.logos.find(l => l.id === val);
      this.selectedLogoPreview = logo ? (logo.cache_url || logo.url) : null;
      this.cdr.markForCheck();
    });

    // Channel name auto-suggest from channel_data.db (debounced)
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

    // When user types in channel name, trigger auto-suggest
    this.form.get('name')?.valueChanges.subscribe(val => {
      if (val && val.length > 2 && !this.browsingMatches) {
        this.nameSearchSubject.next(val);
      } else {
        this.suggestions = [];
        this.cdr.markForCheck();
      }
    });

    // Sidebar filter: filter existing channels as user types
    this.channelSearchControl.valueChanges.pipe(
      debounceTime(200),
      distinctUntilChanged()
    ).subscribe(val => {
      this.filterExistingChannels(val || '');
    });
  }

  // ═══════════════════════════════════════════════════════════
  // DATA LOADING
  // ═══════════════════════════════════════════════════════════

  loadGroups() {
    this.api.getChannelGroups().subscribe({
      next: (res: any) => {
        const allGroups = res.results || res;
        this.groups = allGroups.sort((a: any, b: any) => a.name.localeCompare(b.name));
        this.cdr.markForCheck();
      },
      error: (err: any) => {
        // Degrade gracefully — show empty group list if the API fails
        console.error('Failed to load channel groups:', err);
        this.groups = [];
        this.cdr.markForCheck();
      }
    });
  }

  loadExistingChannels() {
    this.api.getChannelsSummary().subscribe({
      next: (res: any) => {
        const channels = res.results || res;
        // Sort by channel number ascending
        this.existingChannels = channels.sort((a: any, b: any) =>
          (a.channel_number || 0) - (b.channel_number || 0)
        );
        this.filteredExistingChannels = this.existingChannels;
        this.cdr.markForCheck();
      },
      error: () => {
        this.existingChannels = [];
        this.filteredExistingChannels = [];
        this.cdr.markForCheck();
      }
    });
  }

  filterExistingChannels(search: string) {
    if (!search) {
      this.filteredExistingChannels = this.existingChannels;
    } else {
      const lower = search.toLowerCase();
      this.filteredExistingChannels = this.existingChannels.filter(ch =>
        ch.name.toLowerCase().includes(lower) ||
        String(ch.channel_number).includes(lower)
      );
    }
    this.cdr.markForCheck();
  }

  filterGroups() {
    // Trigger change detection — displayedGroups getter handles actual filtering
    this.cdr.markForCheck();
  }

  // ═══════════════════════════════════════════════════════════
  // GROUP SELECTION & CREATION
  // ═══════════════════════════════════════════════════════════

  selectGroup(group: any) {
    this.form.patchValue({ channel_group_id: group.id });
    this.cdr.markForCheck();
  }

  startAddGroup() {
    this.addingGroup = true;
    this.newGroupName = '';
    this.cdr.markForCheck();
  }

  confirmAddGroup() {
    const name = this.newGroupName.trim();
    if (!name) return;

    this.api.createChannelGroup(name).subscribe({
      next: (res: any) => {
        // Add the new group, re-sort, select it, and close inline input
        this.groups.push(res);
        this.groups.sort((a: any, b: any) => a.name.localeCompare(b.name));
        this.form.patchValue({ channel_group_id: res.id });
        this.addingGroup = false;
        this.newGroupName = '';
        this.cdr.markForCheck();
      },
      error: () => {
        this.addingGroup = false;
        this.cdr.markForCheck();
      }
    });
  }

  // ═══════════════════════════════════════════════════════════
  // CHANNEL NUMBER SHORTCUTS
  // ═══════════════════════════════════════════════════════════

  /** Find the lowest unused channel number */
  useFirstAvailable() {
    const used = new Set(this.existingChannels.map((ch: any) => ch.channel_number));
    let num = 1;
    while (used.has(num)) num++;
    this.form.patchValue({ channel_number: num });
    this.cdr.markForCheck();
  }

  /** One above the current highest channel number */
  useHighestPlusOne() {
    if (this.existingChannels.length === 0) {
      this.form.patchValue({ channel_number: 1 });
    } else {
      const max = Math.max(...this.existingChannels.map((ch: any) => ch.channel_number || 0));
      this.form.patchValue({ channel_number: max + 1 });
    }
    this.cdr.markForCheck();
  }

  /** Find a gap near the user's current input (or near 1 if blank) */
  useSmartRange() {
    const current = this.form.get('channel_number')?.value || 1;
    const used = new Set(this.existingChannels.map((ch: any) => ch.channel_number));
    // Search upward from current value for the nearest gap
    let num = current;
    while (used.has(num)) num++;
    this.form.patchValue({ channel_number: num });
    this.cdr.markForCheck();
  }

  // ═══════════════════════════════════════════════════════════
  // CHANNEL_DATA.DB BROWSER (separate from EPG selector)
  // ═══════════════════════════════════════════════════════════

  /** Auto-suggest callback: user clicked a suggestion under channel name */
  selectSuggestion(station: any) {
    this.selectedStation = station;
    this.form.patchValue({
      name: station.name,
      tvg_id: station.call_sign || station.name,
      tvc_guide_stationid: station.station_id || ''
    });
    this.suggestions = [];
    this.cdr.markForCheck();
  }

  /** Explicit search in the Channel Data Lookup section */
  performEpgSearch(query: string) {
    if (!query) return;
    this.searchingEPG = true;
    this.dbSearchError = null;
    this.cdr.markForCheck();
    this.api.suggestMatches(query).subscribe({
      next: (res: any) => {
        this.dbSearchResults = (res.matches || []).map((m: any) => m.station);
        if (this.dbSearchResults.length > 0) {
          this.selectedMatch = this.dbSearchResults[0];
        } else {
          // No results returned — inform the user
          this.dbSearchError = `No matches found for "${query}". The channel data database may not contain this station.`;
        }
        this.searchingEPG = false;
        this.cdr.markForCheck();
      },
      error: (err: any) => {
        // Surface the error to the user so they know *why* results are empty.
        // Common causes: 503 (database not loaded), network error, backend crash.
        const status = err?.status || 0;
        const detail = err?.error?.error || err?.message || 'Unknown error';
        if (status === 503) {
          this.dbSearchError = 'Channel data database is not available. Place channel_data.db in the data/ directory or set CHANNEL_DB_PATH.';
        } else {
          this.dbSearchError = `Search failed (HTTP ${status}): ${detail}`;
        }
        console.error('Channel Data Lookup search failed:', status, detail);
        this.dbSearchResults = [];
        this.searchingEPG = false;
        this.cdr.markForCheck();
      }
    });
  }

  selectMatch(match: any) {
    this.selectedMatch = match;
    this.cdr.markForCheck();
  }

  /** Apply a field (or all fields) from a channel_data.db match to the form */
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

    // Store the match as the "selected station" so shortcuts work
    this.selectedStation = this.selectedMatch;
    this.cdr.markForCheck();
  }

  // ═══════════════════════════════════════════════════════════
  // EPG SELECTOR (actual EPG data from epg_epgdata table)
  // ═══════════════════════════════════════════════════════════

  selectEpg(epg: any | null) {
    if (epg) {
      this.form.patchValue({ epg_data_id: epg.id });
    } else {
      this.form.patchValue({ epg_data_id: null });
    }
    this.epgDropdownOpen = false;
    this.cdr.markForCheck();
  }

  useDummyEpg() {
    this.form.patchValue({ tvg_id: 'dummy', tvc_guide_stationid: 'dummy', epg_data_id: null });
    this.selectedStation = { name: 'Dummy EPG', station_id: 'dummy' };
    this.cdr.markForCheck();
  }

  // ═══════════════════════════════════════════════════════════
  // SHORTCUT LOGIC (linked to selectedStation from channel_data.db)
  // ═══════════════════════════════════════════════════════════

  useEpgName() {
    if (this.selectedStation) {
      this.form.patchValue({ name: this.selectedStation.name });
    }
  }

  useEpgTvgId() {
    if (this.selectedStation) {
      this.form.patchValue({ tvg_id: this.selectedStation.call_sign || this.selectedStation.name });
    }
  }

  useEpgLogo() {
    if (this.selectedStation?.logo_uri) {
      this.newLogoUrlControl.setValue(this.selectedStation.logo_uri);
      this.addLogoFromUrl();
    }
  }

  // ═══════════════════════════════════════════════════════════
  // LOGO HANDLING
  // ═══════════════════════════════════════════════════════════

  onLogoError() {
    this.selectedLogoPreview = null;
    this.cdr.markForCheck();
  }

  /** Add a logo from a URL (used by db match apply and EPG logo shortcut) */
  addLogoFromUrl() {
    const url = this.newLogoUrlControl.value?.trim();
    if (!url) return;
    this.addingLogo = true;
    this.api.createLogo({ name: url.split('/').pop() || 'Imported Logo', url }).subscribe({
      next: (res: any) => {
        this.logos.unshift(res);
        this.form.patchValue({ logo_id: res.id });
        this.newLogoUrlControl.setValue('');
        this.addingLogo = false;
        this.cdr.markForCheck();
      },
      error: () => {
        this.addingLogo = false;
        this.cdr.markForCheck();
      }
    });
  }

  /** Upload a logo file — shows immediate local preview via FileReader before upload completes */
  uploadLogoFile(event: Event) {
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file) return;

    // Validate file type
    if (!file.type.startsWith('image/')) {
      console.warn('Invalid file type. Please select an image file.');
      return;
    }

    // Immediate local preview via FileReader (shows image before upload finishes)
    const reader = new FileReader();
    reader.onload = (e: ProgressEvent<FileReader>) => {
      this.selectedLogoPreview = e.target?.result as string;
      this.cdr.markForCheck();
    };
    reader.readAsDataURL(file);

    // Upload to backend
    this.uploadingLogo = true;
    this.api.uploadLogo(file).subscribe({
      next: (res: any) => {
        this.logos.unshift(res);
        this.form.patchValue({ logo_id: res.id });
        // Preview will be updated by the logo_id valueChanges subscriber
        this.uploadingLogo = false;
        this.cdr.markForCheck();
      },
      error: () => {
        this.uploadingLogo = false;
        this.selectedLogoPreview = null; // Clear preview if upload fails
        this.cdr.markForCheck();
      }
    });

    // Reset the input so the same file can be re-selected
    (event.target as HTMLInputElement).value = '';
  }

  // ═══════════════════════════════════════════════════════════
  // FORM SUBMISSION
  // ═══════════════════════════════════════════════════════════

  submit() {
    if (this.form.invalid) return;
    this.submitting = true;
    this.api.createChannel(this.form.value).subscribe({
      next: () => this.dialogClose.emit(true),
      error: () => {
        this.submitting = false;
        this.cdr.markForCheck();
      }
    });
  }

  cancel() {
    this.dialogClose.emit(false);
  }
}
