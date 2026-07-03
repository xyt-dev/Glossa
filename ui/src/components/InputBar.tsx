import { useState } from "react";
import type { Mode } from "../types";

interface Props {
  mode: Mode;
  busy: boolean;
  disabled: boolean;
  onModeChange: (m: Mode) => void;
  onSend: (text: string) => void;
}

export default function InputBar({ mode, busy, disabled, onModeChange, onSend }: Props) {
  const [text, setText] = useState("");

  const submit = () => {
    const t = text.trim();
    if (!t || busy || disabled) return;
    setText("");
    onSend(t);
  };

  const rows = Math.min(6, Math.max(1, text.split("\n").length));

  return (
    <div className="input-bar">
      <div className="mode-toggle" title="Ctrl+M 切换模式">
        <button
          className={mode === "translate" ? "on" : ""}
          onClick={() => onModeChange("translate")}
        >
          严格翻译
        </button>
        <button
          className={mode === "chat" ? "on" : ""}
          onClick={() => onModeChange("chat")}
        >
          聊天
        </button>
      </div>
      <textarea
        className="input-text"
        rows={rows}
        value={text}
        placeholder={
          mode === "translate"
            ? "输入要翻译的文本，Enter 发送，Shift+Enter 换行"
            : "就翻译内容继续提问…"
        }
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
            e.preventDefault();
            submit();
          }
        }}
      />
      <button className="send-btn" onClick={submit} disabled={busy || disabled || !text.trim()}>
        {busy ? "…" : "发送"}
      </button>
    </div>
  );
}
