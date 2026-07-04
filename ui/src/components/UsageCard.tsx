import type { MarkInput, UsageEntry } from "../types";

interface Props {
  entry: UsageEntry;
  markedSet: Set<string>;
  onToggleMark: (input: MarkInput, marked: boolean) => void;
}

/** Card for a native expression quoted from the source sentence. */
export default function UsageCard({ entry, markedSet, onToggleMark }: Props) {
  const marked = markedSet.has(`usage:${entry.usage.toLowerCase()}`);

  const input: MarkInput = {
    word: entry.usage,
    kind: "usage",
    ipa: null,
    pos: null,
    meaning: null,
    native_usage: entry.explanation,
    examples: entry.examples,
  };

  return (
    <div className="word-card usage-card">
      <div className="word-head">
        <span className="native-tag">Native</span>
        <span className="usage-phrase">{entry.usage}</span>
        <span className="word-actions">
          <button
            className={`mark-btn${marked ? " on" : ""}`}
            onClick={() => onToggleMark(input, marked)}
          >
            {marked ? "◆ 用法" : "◇ 用法"}
          </button>
        </span>
      </div>
      {entry.explanation && <div className="usage-explanation">{entry.explanation}</div>}
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
