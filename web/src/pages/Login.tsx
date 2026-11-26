import { useState } from "react";
import { api } from "../lib/api";
import { setToken } from "../lib/auth";

export default function Login({ onLogin }: { onLogin: () => void }) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    try {
      const res = await api.login(username, password);
      setToken(res.token, res.role);
      onLogin();
    } catch {
      setError("Invalid credentials");
    }
  };

  return (
    <div style={styles.wrap}>
      <form style={styles.form} onSubmit={submit}>
        <h1 style={styles.title}>JEBAT</h1>
        <p style={styles.sub}>C2 Server</p>
        {error && <div style={styles.error}>{error}</div>}
        <input
          style={styles.input}
          placeholder="Username"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          autoFocus
        />
        <input
          style={styles.input}
          type="password"
          placeholder="Password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
        />
        <button style={styles.btn} type="submit">
          Login
        </button>
      </form>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  wrap: {
    display: "flex",
    justifyContent: "center",
    alignItems: "center",
    height: "100vh",
    background: "#0a0a0a",
    fontFamily: "monospace",
  },
  form: {
    display: "flex",
    flexDirection: "column",
    gap: 12,
    width: 300,
    padding: 32,
    background: "#111",
    border: "1px solid #222",
    borderRadius: 8,
  },
  title: {
    color: "#0f0",
    margin: 0,
    textAlign: "center",
    letterSpacing: 4,
  },
  sub: {
    color: "#555",
    margin: 0,
    textAlign: "center",
    fontSize: 12,
    marginBottom: 8,
  },
  error: {
    color: "#f44",
    fontSize: 12,
    textAlign: "center",
  },
  input: {
    background: "#0a0a0a",
    border: "1px solid #333",
    color: "#e0e0e0",
    padding: "8px 12px",
    borderRadius: 4,
    fontSize: 14,
    outline: "none",
  },
  btn: {
    background: "#0f0",
    color: "#000",
    border: "none",
    padding: "8px 0",
    borderRadius: 4,
    fontWeight: 700,
    fontSize: 14,
    cursor: "pointer",
  },
};
