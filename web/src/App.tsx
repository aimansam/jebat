import { useEffect, useState } from "react";
import { Routes, Route, Navigate, Link, useLocation } from "react-router-dom";
import { checkAuth, clearToken, getRole } from "./lib/auth";
import { connectWs } from "./lib/ws";
import Login from "./pages/Login";
import Dashboard from "./pages/Dashboard";
import Implants from "./pages/Implants";
import Interact from "./pages/Interact";
import Tasks from "./pages/Tasks";
import Listeners from "./pages/Listeners";

function App() {
  const [authed, setAuthed] = useState<boolean | null>(null);
  const location = useLocation();

  useEffect(() => {
    checkAuth().then(setAuthed);
  }, []);

  useEffect(() => {
    if (authed) connectWs();
  }, [authed]);

  if (authed === null) return <div style={styles.loading}>Loading...</div>;

  if (!authed) {
    return <Login onLogin={() => setAuthed(true)} />;
  }

  const nav = [
    { to: "/", label: "Dashboard" },
    { to: "/implants", label: "Implants" },
    { to: "/tasks", label: "Tasks" },
    { to: "/listeners", label: "Listeners" },
  ];

  return (
    <div style={styles.app}>
      <nav style={styles.nav}>
        <span style={styles.brand}>JEBAT</span>
        <div style={styles.links}>
          {nav.map((n) => (
            <Link
              key={n.to}
              to={n.to}
              style={{
                ...styles.link,
                ...(location.pathname === n.to ? styles.activeLink : {}),
              }}
            >
              {n.label}
            </Link>
          ))}
        </div>
        <div style={styles.right}>
          <span style={styles.role}>{getRole()}</span>
          <button
            style={styles.logout}
            onClick={() => {
              clearToken();
              setAuthed(false);
            }}
          >
            Logout
          </button>
        </div>
      </nav>
      <main style={styles.main}>
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/implants" element={<Implants />} />
          <Route path="/implants/:id" element={<Interact />} />
          <Route path="/tasks" element={<Tasks />} />
          <Route path="/listeners" element={<Listeners />} />
          <Route path="*" element={<Navigate to="/" />} />
        </Routes>
      </main>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  loading: {
    display: "flex",
    justifyContent: "center",
    alignItems: "center",
    height: "100vh",
    color: "#0f0",
    background: "#0a0a0a",
    fontFamily: "monospace",
  },
  app: {
    minHeight: "100vh",
    background: "#0a0a0a",
    color: "#e0e0e0",
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
  },
  nav: {
    display: "flex",
    alignItems: "center",
    padding: "0 20px",
    height: 48,
    background: "#111",
    borderBottom: "1px solid #222",
    gap: 16,
  },
  brand: {
    color: "#0f0",
    fontWeight: 700,
    fontSize: 16,
    letterSpacing: 2,
  },
  links: { display: "flex", gap: 8, flex: 1 },
  link: {
    color: "#888",
    textDecoration: "none",
    padding: "6px 12px",
    borderRadius: 4,
    fontSize: 13,
  },
  activeLink: {
    color: "#0f0",
    background: "#1a1a1a",
  },
  right: { display: "flex", alignItems: "center", gap: 12 },
  role: { color: "#666", fontSize: 12, textTransform: "uppercase" as const },
  logout: {
    background: "transparent",
    border: "1px solid #333",
    color: "#888",
    padding: "4px 10px",
    borderRadius: 4,
    cursor: "pointer",
    fontSize: 12,
  },
  main: { padding: 20 },
};

export default App;
