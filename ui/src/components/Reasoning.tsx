import { useState } from "react";

interface Props {
  text: string;
  /** True while the model is still thinking (no answer content yet). */
  streaming?: boolean;
}

/** Collapsible "thinking" section at the top of a reply bubble.
 *  Always starts collapsed; only a click toggles it (no auto expand/collapse). */
export default function Reasoning({ text, streaming = false }: Props) {
  const [open, setOpen] = useState(false);

  return (
    <div className={`reasoning${open ? " open" : ""}`}>
      <button className="reasoning-head" onClick={() => setOpen((o) => !o)}>
        {/* CSS 画的小三角（不是字体字形），配 align-items:center 才能稳稳垂直居中 */}
        <span className="reasoning-caret" aria-hidden />
        {streaming && <span className="spinner reasoning-spinner" />}
        <span className="reasoning-label">{streaming ? "思考中…" : "思考过程"}</span>
      </button>
      {open && <div className="reasoning-body">{text}</div>}
    </div>
  );
}
