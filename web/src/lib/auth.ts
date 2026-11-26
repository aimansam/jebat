import { api } from "./api";

export function getToken(): string | null {
  return localStorage.getItem("jebat_token");
}

export function setToken(token: string, role: string) {
  localStorage.setItem("jebat_token", token);
  localStorage.setItem("jebat_role", role);
}

export function clearToken() {
  localStorage.removeItem("jebat_token");
  localStorage.removeItem("jebat_role");
}

export function getRole(): string {
  return localStorage.getItem("jebat_role") || "";
}

export async function checkAuth(): Promise<boolean> {
  const token = getToken();
  if (!token) return false;
  try {
    const res = await api.verify(token);
    return res.valid;
  } catch {
    clearToken();
    return false;
  }
}
