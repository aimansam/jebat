export interface WsEvent {
  event_type: string;
  agent_id: string | null;
  task_id: number | null;
  detail: string;
  timestamp: string;
}

type Handler = (event: WsEvent) => void;

let socket: WebSocket | null = null;
const handlers = new Set<Handler>();

export function connectWs() {
  if (socket && socket.readyState === WebSocket.OPEN) return;

  const proto = location.protocol === "https:" ? "wss:" : "ws:";
  socket = new WebSocket(`${proto}//${location.host}/api/ws`);

  socket.onmessage = (msg) => {
    try {
      const event: WsEvent = JSON.parse(msg.data);
      handlers.forEach((h) => h(event));
    } catch {}
  };

  socket.onclose = () => {
    setTimeout(connectWs, 3000);
  };
}

export function onWsEvent(handler: Handler): () => void {
  handlers.add(handler);
  return () => handlers.delete(handler);
}
