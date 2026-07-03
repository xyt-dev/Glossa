import type { MarkInput, WordEntry } from "../types";

interface Props {
  entry: WordEntry;
  context: string | null;
  markedSet: Set<string>;
  onToggleMark: (input: MarkInput, marked: boolean) => void;
}

export default function WordCard({ entry, context, markedSet, onToggleMark }: Props) {
  const key = entry.word.toLowerCase();
  const wordMarked = markedSet.has(`word:${key}`);
  const usageMarked = markedSet.has(`usage:${key}`);

  const input = (kind: "word" | "usage"): MarkInput => ({
    word: entry.word,
    kind,
    ipa: entry.ipa,
    pos: entry.pos,
    meaning: entry.meaning,
    native_usage: entry.native_usage,
    context,
  });

  return (
    <div className="word-card">
      <div className="word-head">
        <span className="word">{entry.word}</span>
        {entry.ipa && <span className="ipa">{entry.ipa}</span>}
        {entry.pos && <span className="pos">{entry.pos}</span>}
        {entry.ielts_band != null && (
          <span className="band" title="IELTS band">
            {entry.ielts_band}
          </span>
        )}
        <span className="word-actions">
          <button
            className={`mark-btn${wordMarked ? " on" : ""}`}
            title={wordMarked ? "取消标记生词" : "标记为生词"}
            onClick={() => onToggleMark(input("word"), wordMarked)}
          >
            {wordMarked ? "★ 生词" : "☆ 生词"}
          </button>
          <button
            className={`mark-btn${usageMarked ? " on" : ""}`}
            title={usageMarked ? "取消标记用法" : "标记 native 用法"}
            onClick={() => onToggleMark(input("usage"), usageMarked)}
          >
            {usageMarked ? "◆ 用法" : "◇ 用法"}
          </button>
        </span>
      </div>
      {entry.meaning && <div className="meaning">{entry.meaning}</div>}
      {entry.native_usage && (
        <div className="native-usage">
          <span className="label">Native</span>
          {entry.native_usage}
        </div>
      )}
      {entry.examples.length > 0 && (
        <ul className="examples">
          {entry.examples.map((ex, i) => (
            <li key={i}>
              <span className="en">{ex.en}</span>
              <span className="zh">{ex.zh}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
