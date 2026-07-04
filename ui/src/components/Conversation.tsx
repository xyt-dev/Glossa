import { useEffect, useRef } from "react";
import type { MarkInput, Mode, Session } from "../types";
import ChatBubble from "./ChatBubble";
import TranslationBlock from "./TranslationBlock";

interface Props {
  session: Session | null;
  busy: boolean;
  streamText: string;
  streamMode: Mode;
  markedSet: Set<string>;
  onToggleMark: (input: MarkInput, marked: boolean) => void;
}

export default function Conversation({
  session,
  busy,
  streamText,
  streamMode,
  markedSet,
  onToggleMark,
}: Props) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [session?.messages.length, streamText]);

  if (!session) {
    return <div className="conversation empty-hint">新建一个会话开始使用</div>;
  }

  const items = session.messages.map((m, i) => {
    if (m.role === "user") {
      return (
        <div key={i} className="turn user">
          <div className="user-bubble">
            <span className={`mode-chip ${m.mode}`}>
              {m.mode === "translate" ? "译" : "聊"}
            </span>
            <span className="user-text">{m.text}</span>
          </div>
        </div>
      );
    }
    return (
      <div key={i} className="turn assistant">
        {m.mode === "translate" ? (
          <TranslationBlock
            result={m.result}
            raw={m.raw}
            markedSet={markedSet}
            onToggleMark={onToggleMark}
          />
        ) : (
          <ChatBubble text={m.text ?? ""} />
        )}
      </div>
    );
  });

  return (
    <div className="conversation">
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
      {busy && (
        <div className="turn assistant">
          {streamMode === "chat" ? (
            <ChatBubble text={streamText} streaming />
          ) : (
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
      <div ref={bottomRef} />
    </div>
  );
}
