const BASE_URL = '/api';

/**
 * Core fetch wrapper.
 * - Throws on non-2xx responses, surfacing the backend error message.
 * - Returns {} for 204 No Content (e.g. delete operations).
 */
async function fetchApi(path: string, options: RequestInit = {}) {
  const response = await fetch(path, options);
  if (!response.ok) {
    const errorData = await response.json().catch(() => ({}));
    throw new Error(errorData.error || errorData.message || `API Error: ${response.status} ${response.statusText}`);
  }

  if (response.status === 204) return {};

  const text = await response.text();
  return text ? JSON.parse(text) : {};
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
  /** DELETE /api/channels/channels/:id/ — removes channel and its stream assignments */
  deleteChannel: (id: number) => fetchApi(`/api/channels/channels/${id}/`, {
    method: 'DELETE'
  }),
  createChannel: (data: any) => fetchApi('/api/channels/channels/', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  getChannelsSummary: () => fetchApi('/api/channels/channels/summary/'),
  bulkUpdateChannels: (data: any[]) => fetchApi('/api/channels/channels/edit/bulk/', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  updateChannelGroup: (id: number, data: { name: string }) => fetchApi(`/api/channels/groups/${id}/`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  deleteChannelGroup: (id: number) => fetchApi(`/api/channels/groups/${id}/`, {
    method: 'DELETE'
  }),

  // =================== EPG ===================
  getEpgData: () => fetchApi('/api/epg/epgdata/'),
  getEpgSources: () => fetchApi('/api/epg/sources/'),
  createEpgSource: (data: any) => fetchApi('/api/epg/sources/', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  updateEpgSource: (id: number, data: any) => fetchApi(`/api/epg/sources/${id}/`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  deleteEpgSource: (id: number) => fetchApi(`/api/epg/sources/${id}/`, { method: 'DELETE' }),
  refreshEpgSource: (id: number) => fetchApi(`/api/epg/refresh/${id}/`, { method: 'POST' }),
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
  addM3UAccount: (data: any) => fetchApi('/api/m3u/accounts/', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  updateM3UAccount: (id: number, data: any) => fetchApi(`/api/m3u/accounts/${id}/`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  deleteM3UAccount: (id: number) => fetchApi(`/api/m3u/accounts/${id}/`, { method: 'DELETE' }),
  refreshM3UAccount: (id: number) => fetchApi(`/api/m3u/refresh/${id}/`, { method: 'POST' }),
  updateM3UGroupSettings: (id: number, data: any) => fetchApi(`/api/m3u/accounts/${id}/group-settings/`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  refreshAllM3uAccounts: () => fetchApi('/api/m3u/refresh-all/', { method: 'POST' }),
  refreshAllEpgSources: () => fetchApi('/api/epg/refresh-all/', { method: 'POST' }),
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
  /** Fetch all sorting rules, ordered by priority */
  getSortingRules: () => fetchApi('/api/stream-checker/sorting-rules/'),
  /** Create a new sorting rule */
  createSortingRule: (data: any) => fetchApi('/api/stream-checker/sorting-rules/', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  /** Update an existing sorting rule by ID */
  updateSortingRule: (id: number, data: any) => fetchApi(`/api/stream-checker/sorting-rules/${id}/`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  /** Delete a sorting rule by ID */
  deleteSortingRule: (id: number) => fetchApi(`/api/stream-checker/sorting-rules/${id}/`, {
    method: 'DELETE'
  }),
  /** Reorder channel streams by health score using sorting rules */
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

  // =================== SETTINGS & DASHBOARD ===================
  getSettings: () => fetchApi('/api/core/settings/'),
  getComskipConfig: () => fetchApi('/api/channels/dvr/comskip-config/'),
  getDashboardStats: () => fetchApi('/api/stats'),

  // =================== ACTIVE CONNECTIONS / STATS ===================
  getActiveConnections: () => fetchApi('/proxy/ts/status'),
  stopChannel: (channelId: string) => fetchApi(`/proxy/ts/stop/${channelId}`, { method: 'DELETE' }),
  stopClient: (channelId: string, clientId: string) => fetchApi(`/proxy/ts/stop/${channelId}/${clientId}`, { method: 'DELETE' }),
  updateSetting: (id: number, data: any) => fetchApi(`/api/core/settings/${id}/`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),
  createSetting: (data: any) => fetchApi('/api/core/settings/', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  }),

  // =================== ACTIVITY / LOGS ===================
  getSystemEvents: (limit: number = 100, offset: number = 0) =>
    fetchApi(`/api/core/system-events/?limit=${limit}&offset=${offset}`),
  clearSystemEvents: () => fetchApi('/api/core/system-events/clear/', { method: 'DELETE' }),

  // =================== INTEGRATIONS & PLUGINS ===================
  getIntegrations: () => fetchApi('/api/connect/integrations/'),
  getPlugins: () => fetchApi('/api/plugins/plugins/'),

  // =================== TV GUIDE / EPG ===================
  getEpgGrid: (start: string, end: string, channel_uuids: string[]) =>
    fetchApi(`/api/epg/grid/?start=${start}&end=${end}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ channel_uuids })
    }),

  // =================== VOD ===================
  getVodCategories: () => fetchApi('/api/vod/categories/'),
  getVodMovies: (limit: number = 24, offset: number = 0, search: string = '', categoryId: number | null = null) => {
    // Backend uses page and page_size for VOD
    const page = Math.floor(offset / limit) + 1;
    let url = `/api/vod/movies/?page=${page}&page_size=${limit}`;
    if (search) url += `&search=${encodeURIComponent(search)}`;
    if (categoryId) url += `&category_id=${categoryId}`;
    return fetchApi(url);
  },
  getVodSeries: (limit: number = 24, offset: number = 0, search: string = '', categoryId: number | null = null) => {
    const page = Math.floor(offset / limit) + 1;
    let url = `/api/vod/series/?page=${page}&page_size=${limit}`;
    if (search) url += `&search=${encodeURIComponent(search)}`;
    if (categoryId) url += `&category_id=${categoryId}`;
    return fetchApi(url);
  }
};
