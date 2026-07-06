import { useEffect, useMemo, useRef } from "react";
import type { MarkInput, Mode, Session } from "../types";
import ChatBubble from "./ChatBubble";
import TranslationBlock from "./TranslationBlock";

interface Props {
  session: Session | null;
  busy: boolean;
  streamText: string;
  /** 进行中的推理（思考）文本；有内容后自动折叠 */
  streamReasoning: string;
  streamMode: Mode;
  /** 本轮进行中的问题；后端持久化后由 canonical 接管，二者 de-dup */
  streamUser: string | null;
  markedSet: Set<string>;
  onToggleMark: (input: MarkInput, marked: boolean) => void;
}

export default function Conversation({
  session,
  busy,
  streamText,
  streamReasoning,
  streamMode,
  streamUser,
  markedSet,
  onToggleMark,
}: Props) {
  const scrollerRef = useRef<HTMLDivElement>(null);
  // 用户是否"贴着底部"。贴底才跟随流式；滚上去看历史则解除，不再被拽回底部。
  const pinnedRef = useRef(true);
  // 程序化滚动会自己触发一次 scroll 事件——打标记让 onScroll 忽略它，
  // 否则它会在用户上滑还没生效前就把 pinned 重新置真，导致流式期间“怎么都滚不上去”。
  const selfScrollRef = useRef(false);

  const stickToBottom = () => {
    const el = scrollerRef.current;
    if (!el) return;
    selfScrollRef.current = true;
    el.scrollTop = el.scrollHeight;
  };

  const onScroll = () => {
    if (selfScrollRef.current) {
      selfScrollRef.current = false;
      return; // 忽略我们自己造成的滚动
    }
    const el = scrollerRef.current;
    if (!el) return;
    // 用户主动滚动：离底稍远即解除跟随，滚回底部附近再恢复
    pinnedRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
  };

  // 切换会话：直接跳到底部看最新，并重置为贴底
  useEffect(() => {
    pinnedRef.current = true;
    stickToBottom();
  }, [session?.id]);

  // 流式增量 / 新消息：仅当用户贴着底部时才跟随（回复中可自由上滚看历史）
  useEffect(() => {
    if (pinnedRef.current) stickToBottom();
  }, [session?.messages.length, streamText, streamReasoning, streamUser]);

  // 历史消息独立 memo：流式期间 active 不变 → items 元素引用不变，React 跳过所有
  // 历史气泡的重渲染（含 react-markdown 重解析）；每 token 只重渲染底部流式气泡。
  const items = useMemo(() => {
    // key 带上会话 id：切换会话时 key 变化 → React 重新挂载而非按位置复用旧实例，
    // 否则思考框的展开/折叠状态会串到另一个会话的同位置消息上。
    const sid = session?.id ?? "";
    return (session?.messages ?? []).map((m, i) => {
      if (m.role === "user") {
        return (
          <div key={`${sid}:${i}`} className="turn user">
            <div className="user-bubble">
              <span className={`mode-chip ${m.mode}`}>
                {m.mode === "translate" ? "译" : "聊"}
              </span>
              <span className="user-text">{m.text}</span>
            </div>
          </div>
        );
      }
      const prev = i > 0 ? session?.messages[i - 1] : null;
      const sourceText =
        m.mode === "translate" &&
        prev?.role === "user" &&
        prev.mode === "translate"
          ? prev.text
          : null;
      return (
        <div key={`${sid}:${i}`} className="turn assistant">
          {m.mode === "translate" ? (
            <TranslationBlock
              result={m.result}
              raw={m.raw}
              sourceText={sourceText}
              markedSet={markedSet}
              onToggleMark={onToggleMark}
            />
          ) : (
            <ChatBubble text={m.text ?? ""} reasoning={m.reasoning} />
          )}
        </div>
      );
    });
  }, [session?.id, session?.messages, markedSet, onToggleMark]);

  if (!session) {
    return <div className="conversation empty-hint">新建一个会话开始使用</div>;
  }

  // 问题气泡：仅当 canonical 尚未收录本轮问题时才从流式状态补渲染，避免与
  // 后端已持久化的用户消息重复（切走再切回、或回读晚于持久化都能正确 de-dup）。
  const last = session.messages[session.messages.length - 1];
  const pendingUser =
    busy &&
    streamUser != null &&
    !(last && last.role === "user" && last.text === streamUser && last.mode === streamMode)
      ? streamUser
      : null;

  return (
    <div className="conversation" ref={scrollerRef} onScroll={onScroll}>
      {session.messages.length === 0 && !busy && (
        <div className="empty-hint">
          <div className="hint-title">翻译 + 词汇讲解</div>
          <div className="hint-body">
            输入中文或英文回车翻译；切到聊天模式（Ctrl+M）可就翻译内容继续追问。
            词卡上可标记生词 / native 用法，模型会据此校准讲解深度。
          </div>
        </div>
      )}
      {items}
      {pendingUser != null && (
        <div className="turn user">
          <div className="user-bubble">
            <span className={`mode-chip ${streamMode}`}>
              {streamMode === "translate" ? "译" : "聊"}
            </span>
            <span className="user-text">{pendingUser}</span>
          </div>
        </div>
      )}
      {busy && (
        <div className="turn assistant">
          {/* key=会话 id：每个会话的实时气泡各自独立，思考折叠状态不串会话 */}
          {streamMode === "chat" ? (
            <ChatBubble
              key={session.id}
              text={streamText}
              streaming
              reasoning={streamReasoning || null}
              reasoningStreaming={streamText.length === 0}
            />
          ) : (
            // 翻译模式不展示思考部分（即使模型开了思考）
            <div className="generating">
              <span className="spinner" />
              {streamText ? (
                <pre className="raw-stream">{streamText}</pre>
              ) : (
                <span className="thinking">翻译中…</span>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
