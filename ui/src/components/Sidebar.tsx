import { memo, useEffect, useRef, useState } from "react";
import type { MouseEvent as ReactMouseEvent, TouchEvent } from "react";
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
  /** 正在流式生成的会话 id 集合，用于在 tab 上显示进度指示 */
  streamingIds: Set<string>;
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

// memo：流式期间 props 引用稳定（见 App streamingIds 稳定化 + useCallback 回调），
// 侧边栏不再随每个 token 重渲染。
function Sidebar({
  sessions,
  activeId,
  streamingIds,
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

  // 触屏长按检测：桌面右键与手机长按弹同一个菜单
  const longPressTimer = useRef<number | null>(null);
  const longPressFired = useRef(false);
  const touchStart = useRef({ x: 0, y: 0 });

  const commitRename = (id: string) => {
    const title = draft.trim();
    setEditingId(null);
    if (title) onRename(id, title);
  };

  // fixed 定位在 web 的 CSS zoom 下会被再缩放，坐标除回去；并夹住不出屏
  const openMenu = (clientX: number, clientY: number, id: string, title: string) => {
    const maxX = Math.max(8, window.innerWidth / uiScale - 170);
    const maxY = Math.max(8, window.innerHeight / uiScale - 110);
    setMenu({
      x: Math.max(8, Math.min(clientX / uiScale, maxX)),
      y: Math.max(8, Math.min(clientY / uiScale, maxY)),
      id,
      title,
    });
  };

  const onTabTouchStart = (e: TouchEvent<HTMLDivElement>, s: SessionMeta) => {
    if (editingId === s.id) return;
    cancelLongPress();
    longPressFired.current = false;
    const t = e.touches[0];
    const point = { x: t.clientX, y: t.clientY };
    touchStart.current = point;
    longPressTimer.current = window.setTimeout(() => {
      longPressFired.current = true;
      longPressTimer.current = null;
      navigator.vibrate?.(10); // Android 触感反馈；iOS 无害忽略
      openMenu(point.x, point.y, s.id, s.title);
    }, 500);
  };

  const cancelLongPress = () => {
    if (longPressTimer.current) {
      clearTimeout(longPressTimer.current);
      longPressTimer.current = null;
    }
  };

  const onTabTouchMove = (e: TouchEvent<HTMLDivElement>) => {
    // 移动超过阈值视为滚动，取消长按
    const t = e.touches[0];
    if (
      Math.abs(t.clientX - touchStart.current.x) > 10 ||
      Math.abs(t.clientY - touchStart.current.y) > 10
    ) {
      cancelLongPress();
    }
  };

  const onTabTouchEnd = () => {
    cancelLongPress();
  };

  const onTabClick = (e: ReactMouseEvent<HTMLDivElement>, id: string) => {
    if (longPressFired.current) {
      e.preventDefault();
      e.stopPropagation();
      longPressFired.current = false;
      return;
    }
    if (id !== activeId) onSelect(id);
  };

  useEffect(() => {
    if (!menu) return;
    const close = () => {
      longPressFired.current = false;
      setMenu(null);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("mousedown", close);
    window.addEventListener("touchstart", close);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("touchstart", close);
      window.removeEventListener("keydown", onKey);
    };
  }, [menu]);

  useEffect(() => cancelLongPress, []);

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
      <button className="new-session" onClick={onNew}>
        ＋ 新会话
      </button>
      <nav className="session-list">
        {sessions.map((s) => (
          <div
            key={s.id}
            className={`session-tab${s.id === activeId ? " active" : ""}`}
            onClick={(e) => onTabClick(e, s.id)}
            onDoubleClick={() => {
              setEditingId(s.id);
              setDraft(s.title);
            }}
            onTouchStart={(e) => onTabTouchStart(e, s)}
            onTouchMove={onTabTouchMove}
            onTouchEnd={onTabTouchEnd}
            onTouchCancel={onTabTouchEnd}
            onContextMenu={(e) => {
              e.preventDefault();
              openMenu(e.clientX, e.clientY, s.id, s.title);
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
                {streamingIds.has(s.id) ? (
                  <span
                    className="session-streaming"
                    role="img"
                    aria-label="生成中"
                  />
                ) : (
                  <span className="session-date">{shortDate(s.updated)}</span>
                )}
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
          onTouchStart={(e) => e.stopPropagation()}
        >
          <button
            onClick={() => {
              setEditingId(menu.id);
              setDraft(menu.title);
              longPressFired.current = false;
              setMenu(null);
            }}
          >
            重命名
          </button>
          <button
            className="danger"
            onClick={() => {
              onDelete(menu.id);
              longPressFired.current = false;
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

export default memo(Sidebar);
