import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';

@Injectable({
  providedIn: 'root'
})
export class ApiService {
  private http = inject(HttpClient);

  // Channels
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

  // Streams
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
}
