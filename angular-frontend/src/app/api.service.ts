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

  queryChannels(params: any): Observable<any> {
    let httpParams = new HttpParams();
    Object.keys(params).forEach(key => {
      const value = params[key];
      if (Array.isArray(value)) {
        value.forEach(v => httpParams = httpParams.append(key, v));
      } else {
        httpParams = httpParams.set(key, value);
      }
    });
    return this.http.get('/api/channels/', { params: httpParams });
  }

  getChannelStreams(channelId: number): Observable<any> {
    return this.http.get(`/api/channels/${channelId}/streams/`);
  }

  updateChannel(channelData: any): Observable<any> {
    return this.http.put(`/api/channels/${channelData.id}/`, channelData);
  }

  // Streams
  getPlaylists(): Observable<any> {
    return this.http.get('/api/m3u_accounts/');
  }

  getStreamGroups(): Observable<any> {
    return this.http.get('/api/streams/groups/');
  }

  queryStreams(params: any): Observable<any> {
    let httpParams = new HttpParams();
    Object.keys(params).forEach(key => {
      const value = params[key];
      if (Array.isArray(value)) {
        value.forEach(v => httpParams = httpParams.append(key, v));
      } else {
        httpParams = httpParams.set(key, value);
      }
    });
    return this.http.get('/api/streams/', { params: httpParams });
  }
}
