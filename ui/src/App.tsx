import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
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
import VocabBook from "./components/VocabBook";
import ConfirmDialog from "./components/ConfirmDialog";

interface ConfirmReq {
  title: string;
  detail?: string;
  action: () => void | Promise<void>;
}

function applyTheme(theme: string) {
  document.documentElement.dataset.theme = theme;
}

const appWindow = getCurrentWindow();

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
  const [showVocab, setShowVocab] = useState(false);
  const [confirm, setConfirm] = useState<ConfirmReq | null>(null);
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

  // Suppress the webview's native context menu (mismatched with the theme);
  // inputs keep it for paste. Components with custom menus preventDefault
  // earlier in the bubble phase anyway.
  useEffect(() => {
    const onCtx = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      if (target?.closest("input, textarea")) return;
      e.preventDefault();
    };
    window.addEventListener("contextmenu", onCtx);
    return () => window.removeEventListener("contextmenu", onCtx);
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

  const doRemoveSession = useCallback(
    async (id: string) => {
      await api.deleteSession(id);
      const list = await api.listSessions();
      setSessions(list);
      if (active?.id === id) {
        setActive(list.length ? await api.loadSession(list[0].id) : null);
      }
    },
    [active],
  );

  const removeSession = useCallback(
    (id: string) => {
      const meta = sessions.find((s) => s.id === id);
      setConfirm({
        title: `删除会话「${meta?.title ?? "未命名"}」？`,
        detail: "会话内容将被永久删除，无法恢复。",
        action: () => doRemoveSession(id),
      });
    },
    [sessions, doRemoveSession],
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
      <header className="titlebar">
        <div className="titlebar-drag" data-tauri-drag-region="">
          <span className="titlebar-title" data-tauri-drag-region="">
            Glossa
          </span>
        </div>
        <div className="window-controls">
          <button
            className="window-control"
            type="button"
            aria-label="最小化"
            onClick={() => void appWindow.minimize()}
          >
            −
          </button>
          <button
            className="window-control"
            type="button"
            aria-label="最大化/还原"
            onClick={() => void appWindow.toggleMaximize()}
          >
            □
          </button>
          <button
            className="window-control close"
            type="button"
            aria-label="关闭"
            onClick={() => void appWindow.close()}
          >
            ×
          </button>
        </div>
      </header>
      <Sidebar
        sessions={sessions}
        activeId={active?.id ?? null}
        busy={busy}
        onSelect={selectSession}
        onNew={newSession}
        onDelete={removeSession}
        onRename={renameSession}
        onSettings={() => setShowSettings(true)}
        onVocab={() => setShowVocab(true)}
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
      {showVocab && (
        <VocabBook
          memory={memory}
          onClose={() => setShowVocab(false)}
          onRemove={(word, kind) => {
            const kindLabel = kind === "word" ? "生词" : kind === "usage" ? "用法" : "句子";
            setConfirm({
              title: `从生词本删除${kindLabel}「${word}」？`,
              detail: "删除后无法恢复，模型的水平画像上下文中也会移除这条记录。",
              action: async () => {
                await api.unmarkWord(word, kind);
                setMemory(await api.getMemory());
              },
            });
          }}
        />
      )}
      {confirm && (
        <ConfirmDialog
          title={confirm.title}
          detail={confirm.detail}
          onCancel={() => setConfirm(null)}
          onConfirm={async () => {
            try {
              await confirm.action();
            } catch (e) {
              setError(String(e));
            }
            setConfirm(null);
          }}
        />
      )}
    </div>
  );
}
