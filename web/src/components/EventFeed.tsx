import type { WsEvent } from "../lib/ws";

export default function EventFeed({ events }: { events: WsEvent[] }) {
  if (events.length === 0) {
    return <div style={styles.empty}>Waiting for events...</div>;
  }

  return (
    <div style={styles.feed}>
      {events.map((ev, i) => (
        <div key={i} style={styles.row}>
          <span style={styles.time}>
            {new Date(ev.timestamp).toLocaleTimeString()}
          </span>
          <span style={typeColor(ev.event_type)}>{ev.event_type}</span>
          {ev.agent_id && (
            <span style={styles.agent}>{ev.agent_id.slice(0, 8)}</span>
          )}
          <span style={styles.detail}>{ev.detail}</span>
        </div>
      ))}
    </div>
  );
}

function typeColor(t: string): React.CSSProperties {
  const colors: Record<string, string> = {
    implant_new: "#0f0",
    implant_checkin: "#0a0",
    task_complete: "#08f",
    task_queued: "#fa0",
  };
  return { color: colors[t] || "#888", fontWeight: 600, minWidth: 120 };
}

const styles: Record<string, React.CSSProperties> = {
  empty: { color: "#444", fontSize: 12, padding: 8 },
  feed: {
    background: "#0d0d0d",
    border: "1px solid #1a1a1a",
    borderRadius: 4,
    maxHeight: 300,
    overflow: "auto",
    fontSize: 12,
  },
  row: {
    display: "flex",
    gap: 12,
    padding: "4px 8px",
    borderBottom: "1px solid #151515",
    alignItems: "center",
  },
  time: { color: "#555", minWidth: 70 },
  agent: { color: "#666", fontFamily: "monospace" },
  detail: { color: "#aaa", flex: 1 },
};
