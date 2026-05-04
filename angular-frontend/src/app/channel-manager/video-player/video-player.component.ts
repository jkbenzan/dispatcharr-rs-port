import {
  Component, Input, Output, EventEmitter,
  ViewChild, ElementRef, OnChanges, OnDestroy,
  SimpleChanges, ChangeDetectionStrategy, ChangeDetectorRef
} from '@angular/core';
import { CommonModule } from '@angular/common';

// mpegts.js is imported dynamically to avoid SSR issues
declare var mpegts: any;

/**
 * In-app floating video player for previewing live streams and channels.
 * Uses mpegts.js for MPEG-TS live playback and native HTML5 <video> for VOD.
 *
 * Usage:
 *   <app-video-player
 *     [streamUrl]="'/stream/abc123/'"
 *     [title]="'Channel Name'"
 *     [contentType]="'live'"
 *     (closed)="onPlayerClosed()"
 *   ></app-video-player>
 */
@Component({
  selector: 'app-video-player',
  standalone: true,
  imports: [CommonModule],
  template: `
    <!-- Overlay backdrop -->
    <div class="player-overlay" *ngIf="streamUrl" (click)="close()">
      <div class="player-container" (click)="$event.stopPropagation()">
        <!-- Header bar -->
        <div class="player-header">
          <span class="player-title">{{ title || 'Preview' }}</span>
          <button class="player-close" (click)="close()" title="Close player">
            <span class="material-icons">close</span>
          </button>
        </div>

        <!-- Video element -->
        <div class="player-video-wrapper">
          <video #videoElement
            autoplay
            playsinline
            [muted]="false"
            controls
            style="width: 100%; height: 100%; background: #000;">
          </video>

          <!-- Loading spinner overlay -->
          <div class="player-loading" *ngIf="loading">
            <div class="spinner"></div>
            <span>Connecting...</span>
          </div>

          <!-- Error overlay -->
          <div class="player-error" *ngIf="errorMsg">
            <span class="material-icons" style="font-size: 32px;">error_outline</span>
            <span>{{ errorMsg }}</span>
          </div>
        </div>
      </div>
    </div>
  `,
  styles: [`
    .player-overlay {
      position: fixed;
      top: 0; left: 0; right: 0; bottom: 0;
      background: rgba(0, 0, 0, 0.7);
      z-index: 10000;
      display: flex;
      align-items: center;
      justify-content: center;
      animation: fadeIn 0.15s ease;
    }
    @keyframes fadeIn {
      from { opacity: 0; }
      to { opacity: 1; }
    }
    .player-container {
      width: 640px;
      max-width: 90vw;
      background: #1a1a2e;
      border-radius: 8px;
      overflow: hidden;
      box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
    }
    .player-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 8px 12px;
      background: #16213e;
    }
    .player-title {
      font-size: 13px;
      font-weight: 600;
      color: #e0e0e0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .player-close {
      background: none;
      border: none;
      color: #888;
      cursor: pointer;
      padding: 4px;
      border-radius: 4px;
      transition: color 0.15s, background 0.15s;
    }
    .player-close:hover {
      color: #fff;
      background: rgba(255, 255, 255, 0.1);
    }
    .player-video-wrapper {
      position: relative;
      width: 100%;
      aspect-ratio: 16 / 9;
      background: #000;
    }
    .player-loading {
      position: absolute;
      top: 0; left: 0; right: 0; bottom: 0;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 12px;
      color: rgba(255, 255, 255, 0.7);
      font-size: 13px;
      background: rgba(0, 0, 0, 0.5);
    }
    .spinner {
      width: 32px; height: 32px;
      border: 3px solid rgba(255, 255, 255, 0.2);
      border-top-color: #646cff;
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
    }
    @keyframes spin {
      to { transform: rotate(360deg); }
    }
    .player-error {
      position: absolute;
      top: 0; left: 0; right: 0; bottom: 0;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 8px;
      color: #ff6b6b;
      font-size: 13px;
      background: rgba(0, 0, 0, 0.7);
    }
  `],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class VideoPlayerComponent implements OnChanges, OnDestroy {
  @Input() streamUrl: string | null = null;
  @Input() title: string = 'Preview';
  @Input() contentType: 'live' | 'vod' = 'live';
  @Output() closed = new EventEmitter<void>();

  @ViewChild('videoElement') videoEl!: ElementRef<HTMLVideoElement>;

  loading = false;
  errorMsg: string | null = null;
  private player: any = null;
  private mpegtsModule: any = null;

  constructor(private cdr: ChangeDetectorRef) {}

  ngOnChanges(changes: SimpleChanges) {
    if (changes['streamUrl']) {
      // Tear down old player, start new one after view updates
      this.destroyPlayer();
      if (this.streamUrl) {
        this.loading = true;
        this.errorMsg = null;
        this.cdr.markForCheck();
        // Wait for the view to render the <video> element
        setTimeout(() => this.initPlayer(), 50);
      }
    }
  }

  ngOnDestroy() {
    this.destroyPlayer();
  }

  close() {
    this.destroyPlayer();
    this.streamUrl = null;
    this.closed.emit();
  }

  private async initPlayer() {
    if (!this.streamUrl || !this.videoEl?.nativeElement) {
      this.loading = false;
      this.cdr.markForCheck();
      return;
    }

    const video = this.videoEl.nativeElement;

    if (this.contentType === 'vod') {
      // Native HTML5 playback for VOD content
      this.initVod(video);
    } else {
      // mpegts.js for live MPEG-TS streams
      await this.initLive(video);
    }
  }

  private initVod(video: HTMLVideoElement) {
    video.src = this.streamUrl!;
    video.load();

    video.oncanplay = () => {
      this.loading = false;
      this.cdr.markForCheck();
      video.play().catch(() => {});
    };

    video.onerror = () => {
      this.loading = false;
      this.errorMsg = 'Failed to load video. The stream may be offline.';
      this.cdr.markForCheck();
    };

    // Store a pseudo-player for cleanup
    this.player = {
      destroy: () => {
        video.removeAttribute('src');
        video.load();
        video.oncanplay = null;
        video.onerror = null;
      }
    };
  }

  private async initLive(video: HTMLVideoElement) {
    try {
      // Dynamically import mpegts.js
      if (!this.mpegtsModule) {
        this.mpegtsModule = (await import('mpegts.js' as any)).default;
      }
      const mpegts = this.mpegtsModule;

      if (!mpegts.getFeatureList().mseLivePlayback) {
        this.loading = false;
        this.errorMsg = 'Your browser does not support live stream playback.';
        this.cdr.markForCheck();
        return;
      }

      // Build absolute URL if relative
      let url = this.streamUrl!;
      if (url.startsWith('/') && typeof window !== 'undefined') {
        url = `${window.location.origin}${url}`;
      }

      const player = mpegts.createPlayer(
        { type: 'mpegts', url, isLive: true, cors: true },
        {
          enableWorker: true,
          enableStashBuffer: false,
          liveBufferLatencyChasing: false,
          liveSync: false,
          autoCleanupSourceBuffer: true,
          autoCleanupMaxBackwardDuration: 120,
          autoCleanupMinBackwardDuration: 60,
          reuseRedirectedURL: true,
        }
      );

      player.attachMediaElement(video);

      player.on(mpegts.Events.MEDIA_INFO, () => {
        this.loading = false;
        this.cdr.markForCheck();
        player.play().catch(() => {});
      });

      player.on(mpegts.Events.ERROR, (errorType: string, errorDetail: string) => {
        console.error('mpegts.js error:', errorType, errorDetail);
        this.loading = false;
        this.errorMsg = `Stream error: ${errorType}`;
        this.cdr.markForCheck();
      });

      player.load();
      this.player = player;
    } catch (err: any) {
      console.error('Failed to initialize live player:', err);
      this.loading = false;
      this.errorMsg = `Player init failed: ${err.message || err}`;
      this.cdr.markForCheck();
    }
  }

  private destroyPlayer() {
    if (this.player) {
      try {
        if (typeof this.player.pause === 'function') {
          this.player.pause();
        }
        this.player.destroy();
      } catch (e) {
        // Ignore cleanup errors
      }
      this.player = null;
    }
    this.loading = false;
    this.errorMsg = null;
  }
}
