import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { Listener } from "../lib/api";

export default function Listeners() {
  const [listeners, setListeners] = useState<Listener[]>([]);

  useEffect(() => {
    api.listListeners().then((r) => setListeners(r.listeners)).catch(() => {});
  }, []);

  return (
    <div>
      <h2 style={styles.h2}>Listeners</h2>
      {listeners.length === 0 ? (
        <p style={styles.empty}>
          Listeners are configured in <code style={styles.code}>config/jebat.toml</code> and started
          on server launch. Management API coming in Phase 5.
        </p>
      ) : (
        <table style={styles.table}>
          <thead>
            <tr>
              <th style={styles.th}>ID</th>
              <th style={styles.th}>Protocol</th>
              <th style={styles.th}>Bind</th>
              <th style={styles.th}>Port</th>
            </tr>
          </thead>
          <tbody>
            {listeners.map((l) => (
              <tr key={l.id}>
                <td style={styles.td}>{l.id}</td>
                <td style={{ ...styles.td, color: "#08f" }}>{l.protocol}</td>
                <td style={styles.td}>{l.bind}</td>
                <td style={styles.td}>{l.port}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  h2: { color: "#0f0", margin: "0 0 16px", fontSize: 18 },
  empty: { color: "#666", fontSize: 13 },
  code: { color: "#0f0", background: "#111", padding: "2px 6px", borderRadius: 3 },
  table: { width: "100%", borderCollapse: "collapse", fontSize: 13 },
  th: {
    textAlign: "left",
    color: "#666",
    padding: "8px 10px",
    borderBottom: "1px solid #222",
    fontSize: 11,
    textTransform: "uppercase" as const,
  },
  td: { padding: "8px 10px", borderBottom: "1px solid #1a1a1a", color: "#bbb" },
};
