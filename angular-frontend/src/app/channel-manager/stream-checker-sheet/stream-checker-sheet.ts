import {
  Component,
  ChangeDetectionStrategy,
  ChangeDetectorRef,
  OnInit,
  OnDestroy,
  Input,
  Output,
  EventEmitter,
  inject,
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { ApiService } from '../../api.service';

// =================== INTERFACES ===================

/** Status of a single M3U provider worker in the bulk check engine */
export interface WorkerStatus {
  m3u_account_id: number;
  m3u_account_name: string;
  current_stream_name: string;
  completed: number;
  total: number;
}

/** Result of a single stream test in the activity log */
export interface StreamResult {
  id: number;
  name: string;
  stream_stats?: {
    reachable?: boolean;
    resolution?: string;
    video_codec?: string;
    status?: string;
  };
  status?: string;
}

/** Overall bulk check status from the backend */
export interface BulkCheckStatus {
  is_running: boolean;
  total: number;
  completed: number;
  successful: number;
  failed: number;
  current_stream_id: number | null;
  current_stream_name: string | null;
  workers: WorkerStatus[];
  last_results: StreamResult[];
}

/**
 * StreamCheckerSheetComponent
 *
 * Renders the stream checker analytics panel inside a SheetDialog.
 * Replicates the React StreamChecker.jsx analytics sidebar:
 * - Progress bar with percentage
 * - Stats badges (total, success, failed)
 * - Worker cards showing per-provider progress
 * - Live activity log showing latest test results
 *
 * This component is used inside a <ng-template [(tuiSheetDialog)]>
 * in the channels-pane. It receives stream IDs to test and channel
 * IDs to sort after testing completes.
 */
@Component({
  selector: 'app-stream-checker-sheet',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './stream-checker-sheet.html',
  styleUrls: ['./stream-checker-sheet.less'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class StreamCheckerSheetComponent implements OnInit, OnDestroy {
  private api = inject(ApiService);
  private cdr = inject(ChangeDetectorRef);

  /** Stream IDs to test — passed in by the parent when opening the sheet */
  @Input() streamIds: number[] = [];

  /** Channel IDs to auto-sort after bulk check completes */
  @Input() channelIds: number[] = [];

  /** Label shown in the sheet header (e.g., "Testing: Group Name") */
  @Input() label = 'Stream Checker';

  /** Whether to auto-sort channels after the check completes */
  @Input() autoSort = true;

  /** Emits when the check completes so the parent can refresh data */
  @Output() checkComplete = new EventEmitter<void>();

  /** Current bulk check status from the backend */
  status: BulkCheckStatus = {
    is_running: false,
    total: 0,
    completed: 0,
    successful: 0,
    failed: 0,
    current_stream_id: null,
    current_stream_name: null,
    workers: [],
    last_results: [],
  };

  /** Computed progress percentage (0-100) */
  progressPercent = 0;

  /** Whether a check was started by this component instance */
  startedByUs = false;

  /** Whether the auto-sort has already been triggered (prevent double-sort) */
  private sortTriggered = false;

  /** Polling interval handle */
  private pollInterval: any = null;

  /** Error message if bulk check fails to start */
  errorMessage: string | null = null;

  /** Whether a cancel request has been sent (show "Cancelling..." indicator) */
  cancelling = false;

  ngOnInit() {
    // Start polling immediately to pick up any existing check
    this.startPolling();

    // If stream IDs were provided, kick off the bulk check
    if (this.streamIds.length > 0) {
      this.startBulkCheck();
    }
  }

  ngOnDestroy() {
    this.stopPolling();
  }

  // =================== BULK CHECK LIFECYCLE ===================

  /**
   * Start a new bulk check via the backend engine.
   * If a check is already running, we just observe it instead.
   */
  private startBulkCheck() {
    this.api.bulkCheckStreams(this.streamIds).subscribe({
      next: (res: any) => {
        if (res.success) {
          this.startedByUs = true;
          this.errorMessage = null;
        } else {
          // A check is already running — we'll just observe it
          this.errorMessage = res.message || null;
        }
        this.cdr.markForCheck();
      },
      error: (err) => {
        // 400 means a check is already running — not an error, just observe
        if (err.status === 400) {
          this.errorMessage = 'A bulk check is already in progress. Showing its status.';
        } else {
          this.errorMessage = 'Failed to start bulk check.';
          console.error('Bulk check start failed:', err);
        }
        this.cdr.markForCheck();
      },
    });
  }

  // =================== POLLING ===================

  /** Poll GET /api/streams/bulk-check/status/ every 1s */
  private startPolling() {
    // Fetch once immediately
    this.fetchStatus();

    this.pollInterval = setInterval(() => {
      this.fetchStatus();
    }, 1000);
  }

  private stopPolling() {
    if (this.pollInterval) {
      clearInterval(this.pollInterval);
      this.pollInterval = null;
    }
  }

  private fetchStatus() {
    this.api.getBulkCheckStatus().subscribe({
      next: (res: BulkCheckStatus) => {
        const wasRunning = this.status.is_running;
        this.status = res;
        this.progressPercent = res.total > 0
          ? Math.round((res.completed / res.total) * 100)
          : 0;

        // Check if the run just completed
        if (wasRunning && !res.is_running) {
          this.onCheckComplete();
        }

        this.cdr.markForCheck();
      },
      error: (err) => {
        console.error('Failed to poll bulk check status:', err);
      },
    });
  }

  // =================== COMPLETION ===================

  /**
   * Called when the bulk check transitions from running → not running.
   * Triggers auto-sort if configured, then notifies the parent.
   */
  private onCheckComplete() {
    // Stop frequent polling — check is done
    this.stopPolling();

    // Clear cancelling state if it was set
    this.cancelling = false;

    // Auto-sort the channels if we started the check and have channel IDs
    if (this.autoSort && this.channelIds.length > 0 && !this.sortTriggered) {
      this.sortTriggered = true;
      this.api.bulkSortStreams(this.channelIds).subscribe({
        next: () => {
          console.log('Auto-sort completed for channels:', this.channelIds);
          this.checkComplete.emit();
        },
        error: (err) => {
          console.error('Auto-sort failed:', err);
          this.checkComplete.emit(); // Still emit so parent can refresh
        },
      });
    } else {
      this.checkComplete.emit();
    }
  }

  // =================== CANCELLATION ===================

  /**
   * Request cancellation of the running bulk check.
   * Sets a cooperative flag on the backend; the current stream test
   * will complete but no new streams will be started.
   */
  cancelCheck() {
    this.cancelling = true;
    this.cdr.markForCheck();

    this.api.cancelBulkCheck().subscribe({
      next: () => {
        console.log('Bulk check cancel requested');
        // The poll loop will detect is_running → false and trigger onCheckComplete
      },
      error: (err) => {
        console.error('Cancel request failed:', err);
        this.cancelling = false;
        this.cdr.markForCheck();
      },
    });
  }

  // =================== TEMPLATE HELPERS ===================

  /** Compute worker progress percentage */
  workerProgress(worker: WorkerStatus): number {
    return worker.total > 0 ? Math.round((worker.completed / worker.total) * 100) : 0;
  }

  /** Returns the reversed last_results for newest-first display */
  get reversedResults(): StreamResult[] {
    return (this.status.last_results || []).slice().reverse();
  }

  trackByWorkerId(_: number, w: WorkerStatus) { return w.m3u_account_id; }
  trackByResultId(idx: number, r: StreamResult) { return `${r.id}-${idx}`; }
}
