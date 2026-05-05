import { ChangeDetectionStrategy, Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormBuilder, ReactiveFormsModule, Validators } from '@angular/forms';
import { ApiService } from '../../api.service';
import { Subject } from 'rxjs';
import { debounceTime, distinctUntilChanged, switchMap } from 'rxjs/operators';
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

  groups: any[] = [];
  profiles: any[] = [];
  logos: any[] = [];
  suggestions: any[] = [];
  
  private searchSubject = new Subject<string>();

  ngOnInit() {
    this.api.getChannelGroups().subscribe((res: any) => {
      this.groups = res.results || res;
    });

    this.api.getStreamProfiles().subscribe((res: any) => {
      this.profiles = res.results || res;
    });

    this.api.getLogos().subscribe((res: any) => {
      this.logos = res.results || res;
    });

    this.searchSubject.pipe(
      debounceTime(500),
      distinctUntilChanged(),
      switchMap(query => this.api.suggestMatches(query))
    ).subscribe((res: any) => {
      if (res && res.matches) {
        this.suggestions = res.matches.map((m: any) => m.station);
      }
    });

    this.form.get('name')?.valueChanges.subscribe(val => {
      if (val && val.length > 2) {
        this.searchSubject.next(val);
      } else {
        this.suggestions = [];
      }
    });
  }

  selectSuggestion(station: any) {
    this.form.patchValue({
      name: station.name,
      tvg_id: station.call_sign || station.name,
      tvc_guide_stationid: station.station_id || ''
    });
    this.suggestions = [];
  }

  submit() {
    if (this.form.invalid) return;

    this.api.createChannel(this.form.value).subscribe({
      next: () => {
        this.context.completeWith(true);
      },
      error: (err) => {
        console.error('Failed to create channel', err);
        // Toast notification could go here
      }
    });
  }

  cancel() {
    this.context.completeWith(false);
  }
}
