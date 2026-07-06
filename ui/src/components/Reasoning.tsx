import { useLayoutEffect, useRef, useState } from "react";

interface Props {
  text: string;
  /** True while the model is still thinking (no answer content yet). */
  streaming?: boolean;
}

/** Collapsible "thinking" section at the top of a reply bubble.
 *  Open-state is DERIVED from `streaming` (open while thinking, collapsed once the
 *  answer starts) so it's correct regardless of mount timing — no reliance on
 *  catching a transition. A manual click overrides until the next thinking→done,
 *  which force-resets back to auto (collapsed). */
export default function Reasoning({ text, streaming = false }: Props) {
  // null = 跟随自动（思考时展开、出正文即折叠）；true/false = 用户手动指定
  const [userOpen, setUserOpen] = useState<boolean | null>(null);
  const open = userOpen ?? streaming;

  // 思考完成（streaming 由 true→false）时**无论如何强制回到自动=折叠**（即便用户手动展开过）。
  // 用 layout effect：在正式内容绘制前折叠，不闪一帧“展开+内容”。
  const wasStreaming = useRef(streaming);
  useLayoutEffect(() => {
    if (wasStreaming.current && !streaming) setUserOpen(null);
    wasStreaming.current = streaming;
  }, [streaming]);

  return (
    <div className={`reasoning${open ? " open" : ""}`}>
      <button className="reasoning-head" onClick={() => setUserOpen(!open)}>
        <span className="reasoning-caret" aria-hidden>
          {open ? "▾" : "▸"}
        </span>
        <span className="reasoning-label">{streaming ? "思考中…" : "思考过程"}</span>
        {streaming && <span className="spinner reasoning-spinner" />}
      </button>
      {open && <div className="reasoning-body">{text}</div>}
    </div>
  );
}
