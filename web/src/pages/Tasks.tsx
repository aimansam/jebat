import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { Implant, Task } from "../lib/api";
import { onWsEvent } from "../lib/ws";

export default function Tasks() {
  const [implants, setImplants] = useState<Implant[]>([]);
  const [selected, setSelected] = useState("");
  const [tasks, setTasks] = useState<Task[]>([]);
  const [expanded, setExpanded] = useState<number | null>(null);

  useEffect(() => {
    api.listImplants().then((r) => {
      setImplants(r.implants);
      if (r.implants.length > 0 && !selected) {
        setSelected(r.implants[0].agent_id);
      }
    }).catch(() => {});
  }, []);

  useEffect(() => {
    if (!selected) return;
    api.listTasksForImplant(selected).then((r) => setTasks(r.tasks)).catch(() => {});
  }, [selected]);

  useEffect(() => {
    return onWsEvent((ev) => {
      if (ev.event_type === "task_complete" && ev.agent_id === selected) {
        api.listTasksForImplant(selected).then((r) => setTasks(r.tasks)).catch(() => {});
      }
    });
  }, [selected]);

  return (
    <div>
      <h2 style={styles.h2}>Tasks</h2>
      <div style={styles.selector}>
        <label style={styles.label}>Implant:</label>
        <select
          style={styles.select}
          value={selected}
          onChange={(e) => setSelected(e.target.value)}
        >
          {implants.map((imp) => (
            <option key={imp.agent_id} value={imp.agent_id}>
              {imp.hostname} ({imp.agent_id.slice(0, 8)})
            </option>
          ))}
        </select>
      </div>
      <table style={styles.table}>
        <thead>
          <tr>
            <th style={styles.th}>ID</th>
            <th style={styles.th}>Type</th>
            <th style={styles.th}>Status</th>
            <th style={styles.th}>Created</th>
            <th style={styles.th}>Completed</th>
            <th style={styles.th}>Success</th>
          </tr>
        </thead>
        <tbody>
          {tasks.map((t) => (
            <>
              <tr
                key={t.id}
                style={{ ...styles.tr, cursor: t.output ? "pointer" : "default" }}
                onClick={() => t.output && setExpanded(expanded === t.id ? null : t.id)}
              >
                <td style={styles.td}>#{t.id}</td>
                <td style={{ ...styles.td, color: "#08f" }}>{t.task_type_name}</td>
                <td
                  style={{
                    ...styles.td,
                    color:
                      t.status === "completed" ? "#0f0" : t.status === "pending" ? "#fa0" : "#888",
                  }}
                >
                  {t.status}
                </td>
                <td style={styles.td}>{new Date(t.created_at).toLocaleString()}</td>
                <td style={styles.td}>
                  {t.completed_at ? new Date(t.completed_at).toLocaleString() : "-"}
                </td>
                <td style={styles.td}>
                  {t.success === null ? "-" : t.success ? "yes" : "no"}
                </td>
              </tr>
              {expanded === t.id && t.output && (
                <tr key={`${t.id}-out`}>
                  <td colSpan={6} style={styles.output}>
                    <pre style={styles.pre}>{t.output}</pre>
                  </td>
                </tr>
              )}
            </>
          ))}
        </tbody>
      </table>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  h2: { color: "#0f0", margin: "0 0 16px", fontSize: 18 },
  selector: { display: "flex", gap: 8, marginBottom: 16, alignItems: "center" },
  label: { color: "#666", fontSize: 12 },
  select: {
    background: "#111",
    border: "1px solid #333",
    color: "#e0e0e0",
    padding: "4px 8px",
    borderRadius: 4,
    fontSize: 13,
  },
  table: { width: "100%", borderCollapse: "collapse", fontSize: 13 },
  th: {
    textAlign: "left",
    color: "#666",
    padding: "8px 10px",
    borderBottom: "1px solid #222",
    fontSize: 11,
    textTransform: "uppercase" as const,
  },
  tr: {},
  td: { padding: "8px 10px", borderBottom: "1px solid #1a1a1a", color: "#bbb" },
  output: {
    padding: "0 10px 8px",
    background: "#0d0d0d",
  },
  pre: {
    color: "#aaa",
    margin: 0,
    fontSize: 12,
    whiteSpace: "pre-wrap",
    wordBreak: "break-all",
  },
};
