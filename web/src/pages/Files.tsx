import { useEffect, useState } from "react";

interface LootItem {
  id: number;
  agent_id: string;
  loot_type: string;
  filename: string;
  size_bytes: number;
  created_at: string;
}

export default function Files() {
  const [loot, setLoot] = useState<LootItem[]>([]);

  useEffect(() => {
    fetch("/api/loot")
      .then((r) => r.json())
      .then((data) => setLoot(data.loot || []))
      .catch(() => {});
  }, []);

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div>
      <h2 style={styles.h2}>Files &amp; Loot ({loot.length})</h2>
      {loot.length === 0 ? (
        <p style={styles.empty}>
          No loot collected yet. Files from file_upload, file_download, and
          cred_dump tasks are automatically stored here.
        </p>
      ) : (
        <table style={styles.table}>
          <thead>
            <tr>
              <th style={styles.th}>ID</th>
              <th style={styles.th}>Agent</th>
              <th style={styles.th}>Type</th>
              <th style={styles.th}>Filename</th>
              <th style={styles.th}>Size</th>
              <th style={styles.th}>Collected</th>
              <th style={styles.th}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {loot.map((l) => (
              <tr key={l.id}>
                <td style={styles.td}>#{l.id}</td>
                <td style={{ ...styles.td, fontFamily: "monospace" }}>
                  {l.agent_id.slice(0, 8)}
                </td>
                <td style={{ ...styles.td, color: typeColor(l.loot_type) }}>
                  {l.loot_type}
                </td>
                <td style={styles.td}>{l.filename}</td>
                <td style={styles.td}>{formatSize(l.size_bytes)}</td>
                <td style={styles.td}>
                  {new Date(l.created_at).toLocaleString()}
                </td>
                <td style={styles.td}>
                  <a
                    href={`/api/loot/${l.id}/download`}
                    style={styles.dlBtn}
                    download
                  >
                    Download
                  </a>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

function typeColor(t: string): string {
  const colors: Record<string, string> = {
    credentials: "#f44",
    files: "#08f",
  };
  return colors[t] || "#888";
}

const styles: Record<string, React.CSSProperties> = {
  h2: { color: "#0f0", margin: "0 0 16px", fontSize: 18 },
  empty: { color: "#666", fontSize: 13 },
  table: { width: "100%", borderCollapse: "collapse", fontSize: 13 },
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
  dlBtn: {
    color: "#0f0",
    textDecoration: "none",
    fontSize: 12,
  },
};
