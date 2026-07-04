import type { MarkInput, WordEntry } from "../types";

interface Props {
  entry: WordEntry;
  markedSet: Set<string>;
  onToggleMark: (input: MarkInput, marked: boolean) => void;
}

export default function WordCard({ entry, markedSet, onToggleMark }: Props) {
  const marked = markedSet.has(`word:${entry.word.toLowerCase()}`);

  const input: MarkInput = {
    word: entry.word,
    kind: "word",
    ipa: entry.ipa,
    pos: entry.pos,
    meaning: entry.meaning,
    native_usage: entry.native_usage,
    examples: entry.examples,
  };

  return (
    <div className="word-card">
      <div className="word-head">
        <span className="kind-tag word-tag">词</span>
        <span className="word">{entry.word}</span>
        {entry.ipa && <span className="ipa">{entry.ipa}</span>}
        {entry.pos && <span className="pos">{entry.pos}</span>}
        {entry.ielts_band != null && (
          <span className="band" data-tip="IELTS band">
            {entry.ielts_band}
          </span>
        )}
        <span className="word-actions">
          <button
            className={`mark-btn${marked ? " on" : ""}`}
            onClick={() => onToggleMark(input, marked)}
          >
            {marked ? "★ 生词" : "☆ 生词"}
          </button>
        </span>
      </div>
      {entry.meaning && <div className="meaning">{entry.meaning}</div>}
      {entry.native_usage && (
        <div className="native-usage">
          <span className="native-tag">Native</span>
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
