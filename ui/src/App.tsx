import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api";
import type {
  Config,
  MarkInput,
  Mode,
  Session,
  SessionMeta,
  VocabMemory,
} from "./types";
import Sidebar from "./components/Sidebar";
import Conversation from "./components/Conversation";
import InputBar from "./components/InputBar";
import Settings from "./components/Settings";

function applyTheme(theme: string) {
  document.documentElement.dataset.theme = theme;
}

export default function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [sessions, setSessions] = useState<SessionMeta[]>([]);
  const [active, setActive] = useState<Session | null>(null);
  const [memory, setMemory] = useState<VocabMemory>({ profile_summary: "", words: [] });
  const [mode, setMode] = useState<Mode>("translate");
  const [busy, setBusy] = useState(false);
  const [streamText, setStreamText] = useState("");
  const [streamMode, setStreamMode] = useState<Mode>("translate");
  const [error, setError] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const didInit = useRef(false);

  useEffect(() => {
    if (didInit.current) return; // StrictMode double-mount guard
    didInit.current = true;
    (async () => {
      try {
        const cfg = await api.getConfig();
        setConfig(cfg);
        setMode(cfg.session.default_mode);
        applyTheme(cfg.ui.theme);
        api.setZoom(cfg.ui.zoom).catch(() => {});
        setMemory(await api.getMemory());
        const list = await api.listSessions();
        if (list.length === 0) {
          const s = await api.createSession();
          setSessions([s]);
          setActive(s);
        } else {
          setSessions(list);
          setActive(await api.loadSession(list[0].id));
        }
      } catch (e) {
        setError(String(e));
      }
    })();
  }, []);

  // Ctrl+M toggles input mode anywhere
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key.toLowerCase() === "m") {
        e.preventDefault();
        setMode((m) => (m === "translate" ? "chat" : "translate"));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const refreshSessions = useCallback(async () => {
    setSessions(await api.listSessions());
  }, []);

  const selectSession = useCallback(
    async (id: string) => {
      if (busy) return;
      try {
        setActive(await api.loadSession(id));
        setError(null);
      } catch (e) {
        setError(String(e));
      }
    },
    [busy],
  );

  const newSession = useCallback(async () => {
    if (busy) return;
    try {
      const s = await api.createSession();
      await refreshSessions();
      setActive(s);
    } catch (e) {
      setError(String(e));
    }
  }, [busy, refreshSessions]);

  const removeSession = useCallback(
    async (id: string) => {
      try {
        await api.deleteSession(id);
        const list = await api.listSessions();
        setSessions(list);
        if (active?.id === id) {
          setActive(list.length ? await api.loadSession(list[0].id) : null);
        }
      } catch (e) {
        setError(String(e));
      }
    },
    [active],
  );

  const renameSession = useCallback(
    async (id: string, title: string) => {
      try {
        await api.renameSession(id, title);
        await refreshSessions();
        setActive((a) => (a && a.id === id ? { ...a, title } : a));
      } catch (e) {
        setError(String(e));
      }
    },
    [refreshSessions],
  );

  const send = useCallback(
    async (text: string) => {
      if (!active || busy) return;
      const sessionId = active.id;
      setBusy(true);
      setError(null);
      setStreamText("");
      setStreamMode(mode);
      setActive((a) =>
        a
          ? {
              ...a,
              messages: [
                ...a.messages,
                { role: "user", mode, text, ts: new Date().toISOString() },
              ],
            }
          : a,
      );
      try {
        await api.sendMessage(sessionId, text, mode, (e) => {
          if (e.type === "delta") setStreamText((prev) => prev + e.text);
          else if (e.type === "error") setError(e.message);
        });
      } catch (e) {
        setError(String(e));
      }
      // reload canonical state (turn is persisted before `done`)
      try {
        setActive(await api.loadSession(sessionId));
        await refreshSessions();
      } catch {
        // session may have been deleted meanwhile; ignore
      }
      setStreamText("");
      setBusy(false);
    },
    [active, busy, mode, refreshSessions],
  );

  const toggleMark = useCallback(async (input: MarkInput, marked: boolean) => {
    try {
      if (marked) await api.unmarkWord(input.word, input.kind);
      else await api.markWord(input);
      setMemory(await api.getMemory());
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const saveConfig = useCallback(async (cfg: Config) => {
    await api.setConfig(cfg);
    setConfig(cfg);
    applyTheme(cfg.ui.theme);
    api.setZoom(cfg.ui.zoom).catch(() => {});
  }, []);

  const markedSet = useMemo(
    () => new Set(memory.words.map((w) => `${w.kind}:${w.word.toLowerCase()}`)),
    [memory],
  );

  return (
    <div className="app">
      <Sidebar
        sessions={sessions}
        activeId={active?.id ?? null}
        busy={busy}
        onSelect={selectSession}
        onNew={newSession}
        onDelete={removeSession}
        onRename={renameSession}
        onSettings={() => setShowSettings(true)}
      />
      <main className="main">
        {error && (
          <div className="error-banner">
            <span>{error}</span>
            <button onClick={() => setError(null)}>×</button>
          </div>
        )}
        <Conversation
          session={active}
          busy={busy}
          streamText={streamText}
          streamMode={streamMode}
          markedSet={markedSet}
          onToggleMark={toggleMark}
        />
        <InputBar
          mode={mode}
          onModeChange={setMode}
          busy={busy}
          disabled={!active}
          onSend={send}
        />
      </main>
      {showSettings && config && (
        <Settings
          config={config}
          onSave={saveConfig}
          onClose={() => setShowSettings(false)}
          onPreviewTheme={applyTheme}
          onPreviewZoom={(z) => api.setZoom(z).catch(() => {})}
        />
      )}
    </div>
  );
}
