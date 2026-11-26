import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../lib/api";
import type { Implant } from "../lib/api";
import { onWsEvent } from "../lib/ws";

export default function Implants() {
  const [implants, setImplants] = useState<Implant[]>([]);

  const refresh = () => {
    api.listImplants().then((r) => setImplants(r.implants)).catch(() => {});
  };

  useEffect(() => {
    refresh();
    return onWsEvent((ev) => {
      if (ev.event_type.startsWith("implant_")) refresh();
    });
  }, []);

  const statusColor = (s: string) =>
    s === "active" ? "#0f0" : s === "stale" ? "#fa0" : "#f44";

  return (
    <div>
      <h2 style={styles.h2}>Implants ({implants.length})</h2>
      <table style={styles.table}>
        <thead>
          <tr>
            <th style={styles.th}>Agent ID</th>
            <th style={styles.th}>Hostname</th>
            <th style={styles.th}>Username</th>
            <th style={styles.th}>IP</th>
            <th style={styles.th}>Sleep</th>
            <th style={styles.th}>Last Checkin</th>
            <th style={styles.th}>Status</th>
            <th style={styles.th}>Actions</th>
          </tr>
        </thead>
        <tbody>
          {implants.map((imp) => (
            <tr key={imp.agent_id}>
              <td style={styles.td}>
                <Link to={`/implants/${imp.agent_id}`} style={styles.link}>
                  {imp.agent_id.slice(0, 8)}...
                </Link>
              </td>
              <td style={styles.td}>{imp.hostname}</td>
              <td style={styles.td}>{imp.username || "-"}</td>
              <td style={styles.td}>{imp.internal_ip || imp.external_ip || "-"}</td>
              <td style={styles.td}>{(imp.sleep_ms / 1000).toFixed(0)}s</td>
              <td style={styles.td}>
                {new Date(imp.last_checkin).toLocaleString()}
              </td>
              <td style={{ ...styles.td, color: statusColor(imp.status) }}>
                {imp.status}
              </td>
              <td style={styles.td}>
                <Link to={`/implants/${imp.agent_id}`} style={styles.interactBtn}>
                  Interact
                </Link>
                <button
                  style={styles.delBtn}
                  onClick={async () => {
                    await api.deleteImplant(imp.agent_id);
                    refresh();
                  }}
                >
                  Remove
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  h2: { color: "#0f0", margin: "0 0 16px", fontSize: 18 },
  table: {
    width: "100%",
    borderCollapse: "collapse",
    fontSize: 13,
  },
  th: {
    textAlign: "left",
    color: "#666",
    padding: "8px 10px",
    borderBottom: "1px solid #222",
    fontSize: 11,
    textTransform: "uppercase" as const,
  },
  td: {
    padding: "8px 10px",
    borderBottom: "1px solid #1a1a1a",
    color: "#bbb",
  },
  link: { color: "#0f0", textDecoration: "none" },
  interactBtn: {
    color: "#08f",
    textDecoration: "none",
    marginRight: 8,
    fontSize: 12,
  },
  delBtn: {
    background: "transparent",
    border: "1px solid #333",
    color: "#f44",
    padding: "2px 8px",
    borderRadius: 3,
    cursor: "pointer",
    fontSize: 11,
  },
};
