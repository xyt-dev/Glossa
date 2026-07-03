import type { MarkInput, TranslationResult } from "../types";
import WordCard from "./WordCard";

interface Props {
  result?: TranslationResult | null;
  raw?: string | null;
  context: string | null;
  markedSet: Set<string>;
  onToggleMark: (input: MarkInput, marked: boolean) => void;
}

export default function TranslationBlock({
  result,
  raw,
  context,
  markedSet,
  onToggleMark,
}: Props) {
  if (!result) {
    return (
      <div className="translation-block fallback">
        <div className="fallback-note">未能解析为结构化结果，以下为模型原始输出：</div>
        <pre className="raw-output">{raw ?? ""}</pre>
      </div>
    );
  }
  return (
    <div className="translation-block">
      <div className="translation-text">{result.translation}</div>
      {result.words.length > 0 && (
        <div className="word-cards">
          {result.words.map((w, i) => (
            <WordCard
              key={`${w.word}-${i}`}
              entry={w}
              context={context}
              markedSet={markedSet}
              onToggleMark={onToggleMark}
            />
          ))}
        </div>
      )}
    </div>
  );
}
