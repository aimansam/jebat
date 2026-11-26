const BASE = "";

async function request<T>(path: string, opts?: RequestInit): Promise<T> {
  const token = localStorage.getItem("jebat_token");
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  };
  const res = await fetch(`${BASE}${path}`, { ...opts, headers });
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return res.json();
}

export const api = {
  login: (username: string, password: string) =>
    request<{ token: string; role: string }>("/api/auth/login", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    }),

  verify: (token: string) =>
    request<{ valid: boolean; username: string; role: string }>(
      "/api/auth/verify",
      { method: "POST", body: JSON.stringify({ token }) },
    ),

  listImplants: () =>
    request<{ implants: Implant[] }>("/api/implants"),

  getImplant: (id: string) =>
    request<{ implant: Implant }>(`/api/implants/${id}`),

  deleteImplant: (id: string) =>
    request<void>(`/api/implants/${id}`, { method: "DELETE" }),

  queueTask: (agentId: string, taskType: number, data: string) =>
    request<{ task_id: number }>("/api/tasks", {
      method: "POST",
      body: JSON.stringify({
        agent_id: agentId,
        task_type: taskType,
        data,
      }),
    }),

  getTask: (id: number) =>
    request<{ task: Task }>(`/api/tasks/${id}`),

  listTasksForImplant: (agentId: string) =>
    request<{ tasks: Task[] }>(`/api/implants/${agentId}/tasks`),

  listListeners: () =>
    request<{ listeners: Listener[] }>("/api/listeners"),
};

export interface Implant {
  agent_id: string;
  hostname: string;
  os_version: string;
  username: string;
  internal_ip: string;
  external_ip: string;
  first_seen: string;
  last_checkin: string;
  sleep_ms: number;
  jitter_pct: number;
  listener_id: string;
  status: string;
}

export interface Task {
  id: number;
  agent_id: string;
  task_type: number;
  task_type_name: string;
  status: string;
  created_at: string;
  dispatched_at: string | null;
  completed_at: string | null;
  output: string | null;
  success: boolean | null;
}

export interface Listener {
  id: string;
  protocol: string;
  bind: string;
  port: number;
}
