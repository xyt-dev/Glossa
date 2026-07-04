import { useEffect, useState } from "react";
import { uiScale } from "../platform";
import type { SessionMeta } from "../types";
import Logo from "./Logo";

interface CtxMenu {
  x: number;
  y: number;
  id: string;
  title: string;
}

interface Props {
  sessions: SessionMeta[];
  activeId: string | null;
  busy: boolean;
  onSelect: (id: string) => void;
  onNew: () => void;
  onDelete: (id: string) => void;
  onRename: (id: string, title: string) => void;
  onSettings: () => void;
  onVocab: () => void;
  onCollapse: () => void;
}

function shortDate(iso: string): string {
  const d = new Date(iso);
  if (isNaN(d.getTime())) return "";
  const now = new Date();
  const sameDay = d.toDateString() === now.toDateString();
  return sameDay
    ? d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
    : `${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export default function Sidebar({
  sessions,
  activeId,
  busy,
  onSelect,
  onNew,
  onDelete,
  onRename,
  onSettings,
  onVocab,
  onCollapse,
}: Props) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [menu, setMenu] = useState<CtxMenu | null>(null);

  const commitRename = (id: string) => {
    const title = draft.trim();
    setEditingId(null);
    if (title) onRename(id, title);
  };

  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", onKey);
    };
  }, [menu]);

  return (
    <aside className="sidebar">
      <div className="sidebar-head">
        <span className="wordmark" aria-label="Glossa">
          <Logo size={24} />
          <span className="wordmark-text">lossa</span>
        </span>
        <button
          className="sidebar-collapse"
          aria-label="收起侧边栏"
          onClick={onCollapse}
        >
          «
        </button>
      </div>
      <button className="new-session" onClick={onNew} disabled={busy}>
        ＋ 新会话
      </button>
      <nav className="session-list">
        {sessions.map((s) => (
          <div
            key={s.id}
            className={`session-tab${s.id === activeId ? " active" : ""}`}
            onClick={() => s.id !== activeId && onSelect(s.id)}
            onDoubleClick={() => {
              setEditingId(s.id);
              setDraft(s.title);
            }}
            onContextMenu={(e) => {
              e.preventDefault();
              // fixed 定位在 CSS zoom 下会被再缩放，坐标除回去
              setMenu({
                x: Math.min(e.clientX / uiScale, window.innerWidth / uiScale - 170),
                y: Math.min(e.clientY / uiScale, window.innerHeight / uiScale - 110),
                id: s.id,
                title: s.title,
              });
            }}
          >
            {editingId === s.id ? (
              <input
                className="rename-input"
                value={draft}
                autoFocus
                onChange={(e) => setDraft(e.target.value)}
                onBlur={() => commitRename(s.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") commitRename(s.id);
                  if (e.key === "Escape") setEditingId(null);
                }}
                onClick={(e) => e.stopPropagation()}
              />
            ) : (
              <>
                <span className="session-title">{s.title}</span>
                <span className="session-date">{shortDate(s.updated)}</span>
              </>
            )}
          </div>
        ))}
      </nav>
      {menu && (
        <div
          className="context-menu"
          style={{ left: menu.x, top: menu.y }}
          onMouseDown={(e) => e.stopPropagation()}
        >
          <button
            onClick={() => {
              setEditingId(menu.id);
              setDraft(menu.title);
              setMenu(null);
            }}
          >
            重命名
          </button>
          <button
            className="danger"
            onClick={() => {
              onDelete(menu.id);
              setMenu(null);
            }}
          >
            删除
          </button>
        </div>
      )}
      <div className="sidebar-foot">
        <button className="settings-btn" onClick={onVocab}>
          <span className="btn-icon">{""}</span>生词本
        </button>
        <button className="settings-btn" onClick={onSettings}>
          <span className="btn-icon">⚙</span>设置
        </button>
      </div>
    </aside>
  );
}
