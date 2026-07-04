import { Channel, invoke } from "@tauri-apps/api/core";
import { isTauri } from "./platform";
import type {
  Config,
  MarkInput,
  MarkKind,
  Mode,
  SendEvent,
  Session,
  SessionMeta,
  VocabEntry,
  VocabMemory,
} from "./types";

interface Backend {
  listSessions(): Promise<SessionMeta[]>;
  createSession(): Promise<Session>;
  loadSession(id: string): Promise<Session>;
  deleteSession(id: string): Promise<void>;
  renameSession(id: string, title: string): Promise<Session>;
  /** Streams SendEvents into `onEvent`; resolves when the turn is persisted. */
  sendMessage(
    sessionId: string,
    text: string,
    mode: Mode,
    onEvent: (e: SendEvent) => void,
  ): Promise<void>;
  markWord(input: MarkInput): Promise<VocabEntry>;
  unmarkWord(word: string, kind: MarkKind): Promise<void>;
  getMemory(): Promise<VocabMemory>;
  getConfig(): Promise<Config>;
  setConfig(config: Config): Promise<void>;
  /** Desktop-only; browsers use native zoom. */
  setZoom(zoom: number): Promise<void>;
}

const tauriBackend: Backend = {
  listSessions: () => invoke<SessionMeta[]>("list_sessions"),
  createSession: () => invoke<Session>("create_session"),
  loadSession: (id) => invoke<Session>("load_session", { id }),
  deleteSession: (id) => invoke<void>("delete_session", { id }),
  renameSession: (id, title) => invoke<Session>("rename_session", { id, title }),
  sendMessage: (sessionId, text, mode, onEvent) => {
    const channel = new Channel<SendEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("send_message", { sessionId, text, mode, onEvent: channel });
  },
  markWord: (input) => invoke<VocabEntry>("mark_word", { input }),
  unmarkWord: (word, kind) => invoke<void>("unmark_word", { word, kind }),
  getMemory: () => invoke<VocabMemory>("get_memory"),
  getConfig: () => invoke<Config>("get_config"),
  setConfig: (config) => invoke<void>("set_config", { config }),
  setZoom: (zoom) => invoke<void>("set_zoom", { zoom }),
};

/** Web token: taken from `?token=` once, then kept in sessionStorage. */
function webToken(): string | null {
  const fromUrl = new URLSearchParams(window.location.search).get("token");
  if (fromUrl) {
    sessionStorage.setItem("glossa-token", fromUrl);
    // strip the token from the visible URL
    const url = new URL(window.location.href);
    url.searchParams.delete("token");
    window.history.replaceState(null, "", url);
  }
  return sessionStorage.getItem("glossa-token");
}

function httpBackend(): Backend {
  const token = webToken();
  const headers: Record<string, string> = { "content-type": "application/json" };
  if (token) headers.authorization = `Bearer ${token}`;

  async function call<T>(path: string, init?: RequestInit): Promise<T> {
    const res = await fetch(`/api${path}`, { ...init, headers });
    if (!res.ok) throw new Error(await res.text());
    const text = await res.text();
    return (text ? JSON.parse(text) : undefined) as T;
  }

  return {
    listSessions: () => call("/sessions"),
    createSession: () => call("/sessions", { method: "POST" }),
    loadSession: (id) => call(`/sessions/${id}`),
    deleteSession: (id) => call(`/sessions/${id}`, { method: "DELETE" }),
    renameSession: (id, title) =>
      call(`/sessions/${id}`, { method: "PATCH", body: JSON.stringify({ title }) }),

    async sendMessage(sessionId, text, mode, onEvent) {
      const res = await fetch(`/api/sessions/${sessionId}/messages`, {
        method: "POST",
        headers,
        body: JSON.stringify({ text, mode }),
      });
      if (!res.ok || !res.body) throw new Error(await res.text());
      const reader = res.body.getReader();
      const decoder = new TextDecoder();
      let buf = "";
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        buf += decoder.decode(value, { stream: true });
        let nl;
        while ((nl = buf.indexOf("\n")) >= 0) {
          const line = buf.slice(0, nl).trim();
          buf = buf.slice(nl + 1);
          if (line) onEvent(JSON.parse(line) as SendEvent);
        }
      }
    },

    markWord: (input) =>
      call("/memory/mark", { method: "POST", body: JSON.stringify(input) }),
    unmarkWord: (word, kind) =>
      call("/memory/unmark", { method: "POST", body: JSON.stringify({ word, kind }) }),
    getMemory: () => call("/memory"),
    getConfig: () => call("/config"),
    setConfig: (config) =>
      call("/config", { method: "PUT", body: JSON.stringify(config) }),
    setZoom: async () => {},
  };
}

export const api: Backend = isTauri ? tauriBackend : httpBackend();
