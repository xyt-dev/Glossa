import { useState } from "react";
import type { SessionMeta } from "../types";

interface Props {
  sessions: SessionMeta[];
  activeId: string | null;
  busy: boolean;
  onSelect: (id: string) => void;
  onNew: () => void;
  onDelete: (id: string) => void;
  onRename: (id: string, title: string) => void;
  onSettings: () => void;
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
}: Props) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState("");

  const commitRename = (id: string) => {
    const title = draft.trim();
    setEditingId(null);
    if (title) onRename(id, title);
  };

  return (
    <aside className="sidebar">
      <div className="sidebar-head">
        <span className="logo">译</span>
        <span className="app-name">Glossa</span>
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
            title={s.title}
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
                <button
                  className="session-delete"
                  title="删除会话"
                  onClick={(e) => {
                    e.stopPropagation();
                    onDelete(s.id);
                  }}
                >
                  ×
                </button>
              </>
            )}
          </div>
        ))}
      </nav>
      <div className="sidebar-foot">
        <button className="settings-btn" onClick={onSettings}>
          ⚙ 设置
        </button>
      </div>
    </aside>
  );
}
