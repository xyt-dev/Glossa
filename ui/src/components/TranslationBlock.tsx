import type { MarkInput, TranslationResult } from "../types";
import SentenceView from "./SentenceView";
import UsageCard from "./UsageCard";
import WordCard from "./WordCard";

interface Props {
  result?: TranslationResult | null;
  raw?: string | null;
  markedSet: Set<string>;
  onToggleMark: (input: MarkInput, marked: boolean) => void;
}

export default function TranslationBlock({
  result,
  raw,
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

  const pairs = result.sentences ?? [];
  const hasWords = result.words.length > 0;
  const hasUsages = (result.usages ?? []).length > 0;

  return (
    <article className="translation-block translation-article">
      <section className="translation-section translation-source-section">
        {pairs.length > 0 ? (
          <SentenceView
            sentences={pairs}
            markedSet={markedSet}
            onToggleMark={onToggleMark}
          />
        ) : (
          // legacy results (≤v0.2) have no sentence pairs
          <div className="translation-text">{result.translation}</div>
        )}
      </section>

      {hasWords && (
        <section className="translation-section translation-notes-section">
          <div className="translation-section-head">
            <div className="translation-section-title">词汇讲解</div>
            <div className="translation-section-note">挑出值得学的词，像阅读批注一样辅助理解。</div>
          </div>
          <div className="word-cards translation-cards">
            {result.words.map((w, i) => (
              <WordCard
                key={`${w.word}-${i}`}
                entry={w}
                markedSet={markedSet}
                onToggleMark={onToggleMark}
              />
            ))}
          </div>
        </section>
      )}

      {hasUsages && (
        <section className="translation-section translation-notes-section">
          <div className="translation-section-head">
            <div className="translation-section-title">地道表达</div>
            <div className="translation-section-note">把句子里真正值得学的习语、搭配、句式单独提出来。</div>
          </div>
          <div className="word-cards translation-cards">
            {result.usages.map((u, i) => (
              <UsageCard
                key={`${u.usage}-${i}`}
                entry={u}
                markedSet={markedSet}
                onToggleMark={onToggleMark}
              />
            ))}
          </div>
        </section>
      )}
    </article>
  );
}
