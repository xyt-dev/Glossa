import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getVersion } from "@tauri-apps/api/app";
import { api } from "./api";
import { isTauri } from "./platform";
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

// getCurrentWindow() throws outside Tauri — only touch it on desktop
const appWindow = isTauri ? getCurrentWindow() : null;

const NARROW = 768;
const RELEASES_API = "https://api.github.com/repos/xyt-dev/Glossa/releases/latest";

function isNewer(current: string, latest: string): boolean {
  const a = current.split(".").map((n) => parseInt(n, 10) || 0);
  const b = latest.split(".").map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    if ((b[i] ?? 0) !== (a[i] ?? 0)) return (b[i] ?? 0) > (a[i] ?? 0);
  }
  return false;
}

export default function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [sessions, setSessions] = useState<SessionMeta[]>([]);
  const [active, setActive] = useState<Session | null>(null);
  const [memory, setMemory] = useState<VocabMemory>({ profile_summary: "", words: [] });
  const [mode, setMode] = useState<Mode>("translate");
  // 每个会话独立的进行中流式状态，按会话 id 存放；不同会话互不阻塞，可并行生成。
  // user 是本轮已发送的问题：它绑定在流式状态上（而非 active），这样切走再切回、
  // 甚至在后端持久化用户消息前回读，都不会丢失问题气泡。
  const [streams, setStreams] = useState<
    Record<string, { text: string; reasoning: string; mode: Mode; user: string }>
  >({});
  const [error, setError] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [showVocab, setShowVocab] = useState(false);
  const [confirm, setConfirm] = useState<ConfirmReq | null>(null);
  const [updateTag, setUpdateTag] = useState<string | null>(null);
  // 启动始终默认展开（窄屏抽屉除外），不记忆上次的折叠状态
  const [sidebarOpen, setSidebarOpen] = useState(() => window.innerWidth >= NARROW);
  const didInit = useRef(false);

  const toggleSidebar = useCallback(() => {
    setSidebarOpen((open) => !open);
  }, []);

  // 稳定引用，保证 memo(Sidebar) 在流式期间不被这两个回调的新身份击穿
  const openSettings = useCallback(() => {
    setShowSettings(true);
    // 窄屏抽屉：打开弹窗时收起侧边栏，避免遮挡
    if (window.innerWidth < NARROW) setSidebarOpen(false);
  }, []);
  const openVocab = useCallback(() => {
    setShowVocab(true);
    if (window.innerWidth < NARROW) setSidebarOpen(false);
  }, []);

  useEffect(() => {
    if (didInit.current) return; // StrictMode double-mount guard
    didInit.current = true;
    (async () => {
      try {
        const cfg = await api.getConfig();
        setConfig(cfg);
        setMode(cfg.session.default_mode);
        applyTheme(cfg.ui.theme);
        // web 端不用应用内 zoom，交给浏览器缩放
        if (isTauri) api.setZoom(cfg.ui.zoom).catch(() => {});
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
  // earlier in the bubble phase anyway. Browser (web) keeps its own menu.
  useEffect(() => {
    if (!isTauri) return;
    const onCtx = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      if (target?.closest("input, textarea")) return;
      e.preventDefault();
    };
    window.addEventListener("contextmenu", onCtx);
    return () => window.removeEventListener("contextmenu", onCtx);
  }, []);

  // 桌面端启动时静默检查新版本（web 端由服务端保证是最新构建）
  useEffect(() => {
    if (!isTauri) return;
    (async () => {
      try {
        const current = await getVersion();
        const res = await fetch(RELEASES_API);
        if (!res.ok) return;
        const tag = ((await res.json()).tag_name as string | undefined)?.replace(/^v/, "");
        if (tag && isNewer(current, tag)) setUpdateTag(tag);
      } catch {
        // 离线或 API 限流，静默跳过
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

  const selectSession = useCallback(async (id: string) => {
    // 生成中也允许切换会话：正在流式的会话在后台继续，切回来仍能看到
    try {
      setActive(await api.loadSession(id));
      setError(null);
      // 窄屏抽屉：选中会话后自动收起
      if (window.innerWidth < NARROW) setSidebarOpen(false);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const newSession = useCallback(async () => {
    try {
      const s = await api.createSession();
      await refreshSessions();
      setActive(s);
    } catch (e) {
      setError(String(e));
    }
  }, [refreshSessions]);

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
      if (!active) return;
      const sessionId = active.id;
      // 同一会话正在生成时不重复发送（输入框已按 activeBusy 禁用，这里兜底）。
      // 后端按会话串行保存，同一会话并发发送会相互覆盖历史。
      const turnMode = mode;
      setError(null);
      // 问题气泡从这里渲染（Conversation 据 streamUser），不再乐观改写 active，
      // 从根上避免“切走再切回时问题消失”的时序竞态。
      setStreams((s) => ({
        ...s,
        [sessionId]: { text: "", reasoning: "", mode: turnMode, user: text },
      }));
      // 节流批量刷新：把每个 token 的 setState 合并成「每 ~60ms 一次」（≈16fps）。
      // 流式文本是可瞥读的，不需要 60fps；关键是别让高频重渲染+文字增长的重排占满主线程——
      // 那正是聊天思考转圈「跳跳卡卡、hover 卡死」的原因（翻译框静态文字无此问题）。
      let bufText = "";
      let bufReason = "";
      let timer: ReturnType<typeof setTimeout> | null = null;
      const flush = () => {
        timer = null;
        const t = bufText;
        const r = bufReason;
        bufText = "";
        bufReason = "";
        if (!t && !r) return;
        setStreams((s) =>
          s[sessionId]
            ? {
                ...s,
                [sessionId]: {
                  ...s[sessionId],
                  text: s[sessionId].text + t,
                  reasoning: s[sessionId].reasoning + r,
                },
              }
            : s,
        );
      };
      const schedule = () => {
        if (timer == null) timer = setTimeout(flush, 60);
      };
      try {
        await api.sendMessage(sessionId, text, turnMode, (e) => {
          if (e.type === "delta") {
            bufText += e.text;
            schedule();
          } else if (e.type === "reasoning") {
            bufReason += e.text;
            schedule();
          } else if (e.type === "error") setError(e.message);
        });
      } catch (e) {
        setError(String(e));
      }
      if (timer != null) clearTimeout(timer);
      flush(); // 落尾：应用最后一批还没刷的缓冲
      // 回读权威状态（turn 在 done 之前已持久化）；仅在仍停留在该会话时替换视图
      try {
        const reloaded = await api.loadSession(sessionId);
        setActive((a) => (a && a.id === sessionId ? reloaded : a));
        await refreshSessions();
      } catch {
        // 会话可能同时被删除，忽略
      }
      setStreams((s) => {
        const next = { ...s };
        delete next[sessionId];
        return next;
      });
    },
    [active, mode, refreshSessions],
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
    if (isTauri) api.setZoom(cfg.ui.zoom).catch(() => {});
  }, []);

  const markedSet = useMemo(
    () => new Set(memory.words.map((w) => `${w.kind}:${w.word.toLowerCase()}`)),
    [memory],
  );

  // 侧边栏据此在各会话 tab 上显示“生成中”指示。身份只在“哪些会话在流式”这个集合
  // 变化时更新——文字增量（每 token）不重建，配合 memo(Sidebar) 避免每 token 重渲染。
  const streamKey = Object.keys(streams).sort().join(" ");
  const streamingIds = useMemo(
    () => new Set(streamKey ? streamKey.split(" ") : []),
    [streamKey],
  );
  const activeStream = active ? streams[active.id] : undefined;
  const activeBusy = activeStream != null;

  return (
    <div className={`app${sidebarOpen ? "" : " sidebar-collapsed"}`}>
      {isTauri && appWindow && (
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
      )}
      {sidebarOpen && window.innerWidth < NARROW && (
        <div className="sidebar-backdrop" onClick={toggleSidebar} />
      )}
      <Sidebar
        sessions={sessions}
        activeId={active?.id ?? null}
        streamingIds={streamingIds}
        onSelect={selectSession}
        onNew={newSession}
        onDelete={removeSession}
        onRename={renameSession}
        onSettings={openSettings}
        onVocab={openVocab}
        onCollapse={toggleSidebar}
      />
      <main className="main">
        {!sidebarOpen && (
          <button
            className="sidebar-expand"
            aria-label="展开侧边栏"
            onClick={toggleSidebar}
          >
            ☰
          </button>
        )}
        {updateTag && (
          <div className="error-banner update-banner">
            <span>
              新版本 v{updateTag} 已发布 — Linux/macOS 重新运行安装脚本即可更新，
              其他平台见 GitHub Releases
            </span>
            <button onClick={() => setUpdateTag(null)}>×</button>
          </div>
        )}
        {error && (
          <div className="error-banner">
            <span>{error}</span>
            <button onClick={() => setError(null)}>×</button>
          </div>
        )}
        <Conversation
          session={active}
          busy={activeBusy}
          streamText={activeStream?.text ?? ""}
          streamReasoning={activeStream?.reasoning ?? ""}
          streamMode={activeStream?.mode ?? mode}
          streamUser={activeStream?.user ?? null}
          markedSet={markedSet}
          onToggleMark={toggleMark}
        />
        <InputBar
          mode={mode}
          onModeChange={setMode}
          busy={activeBusy}
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
