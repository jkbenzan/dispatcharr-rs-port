import { browser } from '$app/environment';

export const wsStore = $state({
  messages: [] as any[],
  lastMessage: null as any,
  connected: false
});

let socket: WebSocket | null = null;

export function connectWS() {
  if (!browser || socket) return;

  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const wsUrl = `${protocol}//${window.location.host}/ws`;
  
  socket = new WebSocket(wsUrl);

  socket.onopen = () => {
    wsStore.connected = true;
    console.log('WebSocket connected');
  };

  socket.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      wsStore.lastMessage = data;
      wsStore.messages = [...wsStore.messages.slice(-99), data]; // Keep last 100
    } catch (e) {
      console.error('WS parse error:', e);
    }
  };

  socket.onclose = () => {
    wsStore.connected = false;
    socket = null;
    console.log('WS closed, reconnecting in 5s...');
    setTimeout(connectWS, 5000);
  };

  socket.onerror = (err) => {
    console.error('WS error:', err);
    socket?.close();
  };
}

export function sendWS(msg: any) {
  if (socket && socket.readyState === WebSocket.OPEN) {
    socket.send(JSON.stringify(msg));
  }
}
