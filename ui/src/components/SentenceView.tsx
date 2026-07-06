import { useState } from "react";
import type { MarkInput, SentencePair } from "../types";
import Popover, { type PopoverAnchor } from "./Popover";

interface Props {
  sentences: SentencePair[];
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
export default function SentenceView({ sentences, markedSet, onToggleMark }: Props) {
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

  return (
    <div className="sentence-view">
      {sentences.map((p, i) => (
        <span
          key={i}
          className={`sentence-chip${pop?.index === i ? " active" : ""}`}
          onClick={(e) => openPop(e, i)}
        >
          {p.src}
        </span>
      ))}
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
