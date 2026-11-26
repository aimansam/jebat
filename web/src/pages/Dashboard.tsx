import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { Implant } from "../lib/api";
import { onWsEvent } from "../lib/ws";
import type { WsEvent } from "../lib/ws";
import EventFeed from "../components/EventFeed";

export default function Dashboard() {
  const [implants, setImplants] = useState<Implant[]>([]);
  const [events, setEvents] = useState<WsEvent[]>([]);

  useEffect(() => {
    api.listImplants().then((r) => setImplants(r.implants)).catch(() => {});
    return onWsEvent((ev) => {
      setEvents((prev) => [ev, ...prev].slice(0, 50));
      if (ev.event_type === "implant_new" || ev.event_type === "implant_checkin") {
        api.listImplants().then((r) => setImplants(r.implants)).catch(() => {});
      }
    });
  }, []);

  const active = implants.filter((i) => i.status === "active").length;
  const stale = implants.filter((i) => i.status === "stale").length;
  const dead = implants.filter((i) => i.status === "dead").length;

  return (
    <div>
      <h2 style={styles.h2}>Dashboard</h2>
      <div style={styles.cards}>
        <Card label="Total" value={implants.length} color="#0f0" />
        <Card label="Active" value={active} color="#0f0" />
        <Card label="Stale" value={stale} color="#fa0" />
        <Card label="Dead" value={dead} color="#f44" />
      </div>
      <h3 style={styles.h3}>Live Events</h3>
      <EventFeed events={events} />
    </div>
  );
}

function Card({ label, value, color }: { label: string; value: number; color: string }) {
  return (
    <div style={styles.card}>
      <div style={{ ...styles.cardValue, color }}>{value}</div>
      <div style={styles.cardLabel}>{label}</div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  h2: { color: "#0f0", margin: "0 0 16px", fontSize: 18 },
  h3: { color: "#888", margin: "24px 0 8px", fontSize: 14 },
  cards: { display: "flex", gap: 12 },
  card: {
    background: "#111",
    border: "1px solid #222",
    borderRadius: 6,
    padding: "16px 24px",
    textAlign: "center",
    minWidth: 100,
  },
  cardValue: { fontSize: 28, fontWeight: 700 },
  cardLabel: { color: "#666", fontSize: 11, marginTop: 4, textTransform: "uppercase" as const },
};
