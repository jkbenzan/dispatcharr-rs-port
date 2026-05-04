import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';

@Injectable({
  providedIn: 'root'
})
export class ApiService {
  private http = inject(HttpClient);

  // =================== CHANNELS ===================

  getChannelGroups(): Observable<any> {
    return this.http.get('/api/channels/groups/');
  }

  getChannels(params: any = {}): Observable<any> {
    let httpParams = new HttpParams();
    Object.keys(params).forEach(key => {
      const value = params[key];
      if (Array.isArray(value)) {
        value.forEach(v => httpParams = httpParams.append(key, v));
      } else {
        httpParams = httpParams.set(key, value);
      }
    });
    return this.http.get('/api/channels/channels/', { params: httpParams });
  }

  getChannelStreams(channelId: number): Observable<any> {
    // Channel data from getChannels already includes streams array
    // But if we need to fetch separately:
    return this.http.get(`/api/channels/channels/${channelId}/`);
  }

  updateChannel(id: number, channelData: any): Observable<any> {
    return this.http.patch(`/api/channels/channels/${id}/`, channelData);
  }

  createChannel(channelData: any): Observable<any> {
    return this.http.post('/api/channels/channels/', channelData);
  }

  // =================== STREAMS ===================

  getPlaylists(): Observable<any> {
    return this.http.get('/api/m3u/accounts/');
  }

  getStreamGroups(): Observable<any> {
    return this.http.get('/api/channels/groups/');
  }

  getStreams(params: any = {}): Observable<any> {
    let httpParams = new HttpParams();
    Object.keys(params).forEach(key => {
      const value = params[key];
      if (Array.isArray(value)) {
        value.forEach(v => httpParams = httpParams.append(key, v));
      } else {
        httpParams = httpParams.set(key, value);
      }
    });
    return this.http.get('/api/channels/streams/', { params: httpParams });
  }

  getStreamFilterOptions(): Observable<any> {
    return this.http.get('/api/channels/streams/filter-options/');
  }

  // =================== STREAM CHECKER ===================

  /**
   * Test a single stream using ffprobe + ffmpeg.
   * POST /api/streams/:id/check/
   * Returns { success, stream } with updated stream_stats.
   */
  testStream(streamId: number): Observable<any> {
    return this.http.post(`/api/streams/${streamId}/check/`, {});
  }

  /**
   * Start a bulk check of multiple streams.
   * POST /api/streams/bulk-check/
   * Body: { stream_ids: number[] }
   */
  bulkCheckStreams(streamIds: number[]): Observable<any> {
    return this.http.post('/api/streams/bulk-check/', { stream_ids: streamIds });
  }

  /**
   * Poll the status of a running bulk check.
   * GET /api/streams/bulk-check/status/
   */
  getBulkCheckStatus(): Observable<any> {
    return this.http.get('/api/streams/bulk-check/status/');
  }

  // =================== STREAM SORTING ===================

  /**
   * Trigger automated sorting on one or more channels based on sorting rules.
   * POST /api/channels/bulk-sort-streams/
   * Body: { channel_ids: number[] }
   */
  bulkSortStreams(channelIds: number[]): Observable<any> {
    return this.http.post('/api/channels/bulk-sort-streams/', { channel_ids: channelIds });
  }

  /**
   * Reorder streams within a channel by sending the full ordered stream ID list.
   * PATCH /api/channels/channels/:id/
   * Body: { streams: number[] }
   */
  reorderChannelStreams(channelId: number, streamIds: number[]): Observable<any> {
    return this.http.patch(`/api/channels/channels/${channelId}/`, { streams: streamIds });
  }

  // =================== SETTINGS ===================

  /**
   * Fetch all application settings.
   * GET /api/settings/
   */
  getSettings(): Observable<any> {
    return this.http.get('/api/settings/');
  }

  /**
   * Update a setting by ID.
   * PUT /api/settings/:id/
   */
  updateSetting(id: number, data: any): Observable<any> {
    return this.http.put(`/api/settings/${id}/`, data);
  }

  /**
   * Create a new setting.
   * POST /api/settings/
   */
  createSetting(data: any): Observable<any> {
    return this.http.post('/api/settings/', data);
  }
}
