const BASE_URL = '/api';

async function fetchApi(path: string, options: RequestInit = {}) {
  // In dev, Vite handles proxying. In prod, the Rust server serves the files.
  const response = await fetch(path, options);
  if (!response.ok) {
    const errorData = await response.json().catch(() => ({}));
    throw new Error(errorData.message || `API Error: ${response.status} ${response.statusText}`);
  }
  return response.json();
}

export const api = {
  // =================== CHANNELS ===================
  getChannelGroups: () => fetchApi('/api/channels/groups/'),
  createChannelGroup: (name: string) => fetchApi('/api/channels/groups/', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name })
  }),
  getChannels: (params: Record<string, any> = {}) => {
    const urlParams = new URLSearchParams();
    Object.entries(params).forEach(([key, value]) => {
      if (Array.isArray(value)) value.forEach(v => urlParams.append(key, v));
      else urlParams.set(key, value);
    });
    const q = urlParams.toString();
    return fetchApi(`/api/channels/channels/${q ? '?' + q : ''}`);
  },
  getChannelStreams: (channelId: number) => fetchApi(`/api/channels/channels/${channelId}/`),
  updateChannel: (id: number, data: any) => fetchApi(`/api/channels/channels/${id}/`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  createChannel: (data: any) => fetchApi('/api/channels/channels/', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  getChannelsSummary: () => fetchApi('/api/channels/channels/summary/'),

  // =================== EPG ===================
  getEpgData: () => fetchApi('/api/epg/epgdata/'),
  getEpgSources: () => fetchApi('/api/epg/sources/'),
  getEPGLcnByTvgId: (tvgId: string) => fetchApi(`/api/epg/lcn?tvg_id=${encodeURIComponent(tvgId)}`),
  suggestMatches: (channelName: string) => fetchApi('/api/channel-db/match/suggest/', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ channel_name: channelName })
  }),
  getLogos: () => fetchApi('/api/channels/logos/'),
  createLogo: (data: { name: string; url: string }) => fetchApi('/api/channels/logos/', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  uploadLogo: (file: File) => {
    const formData = new FormData();
    formData.append('file', file);
    formData.append('name', file.name);
    return fetchApi('/api/channels/logos/upload', {
      method: 'POST',
      body: formData
    });
  },
  getStreamProfiles: () => fetchApi('/api/core/streamprofiles/'),

  // =================== STREAMS ===================
  getPlaylists: () => fetchApi('/api/m3u/accounts/'),
  refreshM3UAccount: (id: number) => fetchApi(`/api/m3u/refresh/${id}/`, { method: 'POST' }),
  getStreamGroups: () => fetchApi('/api/channels/groups/'),
  getStreams: (params: Record<string, any> = {}) => {
    const urlParams = new URLSearchParams();
    Object.entries(params).forEach(([key, value]) => {
      if (Array.isArray(value)) value.forEach(v => urlParams.append(key, v));
      else urlParams.set(key, value);
    });
    const q = urlParams.toString();
    return fetchApi(`/api/channels/streams/${q ? '?' + q : ''}`);
  },
  getStreamFilterOptions: () => fetchApi('/api/channels/streams/filter-options/'),

  // =================== STREAM CHECKER ===================
  testStream: (streamId: number) => fetchApi(`/api/streams/${streamId}/check/`, { method: 'POST' }),
  bulkCheckStreams: (streamIds: number[]) => fetchApi('/api/streams/bulk-check/', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ stream_ids: streamIds })
  }),
  getBulkCheckStatus: () => fetchApi('/api/streams/bulk-check/status/'),
  cancelBulkCheck: () => fetchApi('/api/streams/bulk-check/cancel/', { method: 'POST' }),

  // =================== STREAM SORTING ===================
  bulkSortStreams: (channelIds: number[]) => fetchApi('/api/channels/bulk-sort-streams/', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ channel_ids: channelIds })
  }),
  reorderChannelStreams: (channelId: number, streamIds: number[]) => fetchApi(`/api/channels/channels/${channelId}/`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ streams: streamIds })
  }),

  // =================== SETTINGS ===================
  getSettings: () => fetchApi('/api/settings/'),
  updateSetting: (id: number, data: any) => fetchApi(`/api/settings/${id}/`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  createSetting: (data: any) => fetchApi('/api/settings/', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  })
};
