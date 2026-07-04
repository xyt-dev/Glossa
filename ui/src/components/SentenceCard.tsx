import type { MarkInput, SentencePair } from "../types";

interface Props {
  pair: SentencePair;
  markedSet: Set<string>;
  onToggleMark: (input: MarkInput, marked: boolean) => void;
}

/** One aligned sentence pair, saveable to the vocab book (kind = sentence). */
export default function SentenceCard({ pair, markedSet, onToggleMark }: Props) {
  const marked = markedSet.has(`sentence:${pair.src.toLowerCase()}`);

  const input: MarkInput = {
    word: pair.src,
    kind: "sentence",
    ipa: null,
    pos: null,
    meaning: pair.dst,
    native_usage: null,
    context: null,
  };

  return (
    <div className="word-card sentence-card">
      <div className="sent-row">
        <div className="sent-text">
          <div className="sent-src">
            <span className="kind-tag sent-tag">句</span>
            {pair.src}
          </div>
          <div className="sent-dst">{pair.dst}</div>
        </div>
        <button
          className={`mark-btn${marked ? " on" : ""}`}
          title={marked ? "取消收藏句子" : "收藏句子到生词本"}
          onClick={() => onToggleMark(input, marked)}
        >
          {marked ? "★ 句子" : "☆ 句子"}
        </button>
      </div>
    </div>
  );
}
