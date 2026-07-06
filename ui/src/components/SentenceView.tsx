import { useState } from "react";
import type { MarkInput, SentencePair } from "../types";
import Popover, { type PopoverAnchor } from "./Popover";

interface Props {
  sentences: SentencePair[];
  /** 原文原样展示文本：保留换行 / 空行 / 缩进等排版。 */
  sourceDisplay?: string;
  markedSet: Set<string>;
  onToggleMark: (input: MarkInput, marked: boolean) => void;
}

interface PopState {
  index: number;
  anchor: PopoverAnchor;
}

/**
 * 顶部句子区：原文按句渲染，hover 高亮，点击某句弹出浮动窗口（通用 Popover）
 * 查看译文并可收藏（kind = sentence）。
 */
export default function SentenceView({
  sentences,
  sourceDisplay,
  markedSet,
  onToggleMark,
}: Props) {
  const [pop, setPop] = useState<PopState | null>(null);

  const openPop = (e: React.MouseEvent, i: number) => {
    e.stopPropagation();
    if (pop?.index === i) {
      setPop(null); // 点同一句收起
      return;
    }
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    setPop({
      index: i,
      anchor: { cx: r.left + r.width / 2, top: r.top, bottom: r.bottom },
    });
  };

  const active = pop ? sentences[pop.index] : null;
  const marked = active
    ? markedSet.has(`sentence:${active.src.toLowerCase()}`)
    : false;

  // 用原文原样文本切片：句子本身仍可点击，但句子之间的空格 / 换行 / 空行全部按原样保留。
  // 若定位失败（模型 src 与用户原文略有差异），回退到旧的句子平铺方式。
  const display = sourceDisplay ?? sentences.map((p) => p.src).join("");
  let cursor = 0;
  let fallback = false;
  const pieces: Array<{ type: "text"; text: string } | { type: "sentence"; text: string; index: number }> = [];
  for (let i = 0; i < sentences.length; i++) {
    const src = sentences[i].src;
    const idx = display.indexOf(src, cursor);
    if (idx < 0) {
      fallback = true;
      break;
    }
    if (idx > cursor) pieces.push({ type: "text", text: display.slice(cursor, idx) });
    pieces.push({ type: "sentence", text: src, index: i });
    cursor = idx + src.length;
  }
  if (!fallback && cursor < display.length) {
    pieces.push({ type: "text", text: display.slice(cursor) });
  }

  return (
    <div className="sentence-view">
      {fallback
        ? sentences.map((p, i) => (
            <span
              key={i}
              className={`sentence-chip${pop?.index === i ? " active" : ""}`}
              onClick={(e) => openPop(e, i)}
            >
              {p.src}
            </span>
          ))
        : pieces.map((piece, i) =>
            piece.type === "text" ? (
              <span key={`t-${i}`} className="sentence-gap">
                {piece.text}
              </span>
            ) : (
              <span
                key={`s-${piece.index}`}
                className={`sentence-chip${pop?.index === piece.index ? " active" : ""}`}
                onClick={(e) => openPop(e, piece.index)}
              >
                {piece.text}
              </span>
            ),
          )}
      {pop && active && (
        <Popover
          anchor={pop.anchor}
          onClose={() => setPop(null)}
          footer={
            <button
              className={`mark-btn pop-mark${marked ? " on" : ""}`}
              onClick={() =>
                onToggleMark(
                  {
                    word: active.src,
                    kind: "sentence",
                    ipa: null,
                    pos: null,
                    meaning: active.dst,
                    native_usage: null,
                    examples: [],
                  },
                  marked,
                )
              }
            >
              {marked ? "★ 已收藏" : "☆ 收藏句子"}
            </button>
          }
        >
          <div className="pop-block">
            <div className="pop-label">原文</div>
            <div className="pop-src">{active.src}</div>
          </div>
          <div className="pop-block">
            <div className="pop-label">译文</div>
            <div className="pop-dst">{active.dst}</div>
          </div>
        </Popover>
      )}
    </div>
  );
}
