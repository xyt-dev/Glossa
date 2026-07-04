import { useMemo, useState } from "react";
import type { MarkKind, VocabEntry, VocabMemory } from "../types";
import Dropdown from "./Dropdown";

type SortMode = "added" | "alpha";

interface Props {
  memory: VocabMemory;
  onClose: () => void;
  onRemove: (word: string, kind: MarkKind) => void;
}

interface ScoredEntry {
  entry: VocabEntry;
  index: number;
  score: number;
}

function normalize(value: string | null | undefined): string {
  return (value ?? "").trim().toLowerCase();
}

function fuzzyScoreText(value: string | null | undefined, query: string): number {
  const text = normalize(value);
  if (!text || !query) return 0;

  const exactIndex = text.indexOf(query);
  if (exactIndex >= 0) {
    return 1200 + query.length * 24 - exactIndex;
  }

  let score = 0;
  let cursor = 0;
  let streak = 0;
  let firstMatch = -1;

  for (const char of query) {
    const found = text.indexOf(char, cursor);
    if (found === -1) return 0;
    if (firstMatch === -1) firstMatch = found;

    if (found === cursor) {
      streak += 1;
      score += 18 * streak;
    } else {
      streak = 1;
      score += 8;
    }

    if (found === 0) score += 12;
    cursor = found + 1;
  }

  return score - Math.max(firstMatch, 0);
}

function entryScore(entry: VocabEntry, query: string): number {
  const word = normalize(entry.word);
  const wordScore = fuzzyScoreText(entry.word, query) * 4 + (word.startsWith(query) ? 600 : 0);
  const meaningScore = fuzzyScoreText(entry.meaning, query) * 2;
  const usageScore = fuzzyScoreText(entry.native_usage, query) * 2;
  const ipaScore = fuzzyScoreText(entry.ipa, query);
  const posScore = fuzzyScoreText(entry.pos, query);
  const contextScore = Math.max(0, ...entry.contexts.map((ctx) => fuzzyScoreText(ctx, query)));

  return Math.max(wordScore, meaningScore, usageScore, ipaScore, posScore, contextScore);
}

function formatDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  return `${date.getMonth() + 1}/${date.getDate()}`;
}


export default function VocabBook({ memory, onClose, onRemove }: Props) {
  const [query, setQuery] = useState("");
  const [sortMode, setSortMode] = useState<SortMode>("added");
  const normalizedQuery = normalize(query);

  const entries = useMemo<ScoredEntry[]>(() => {
    const base = memory.words.map((entry, index) => ({
      entry,
      index,
      score: normalizedQuery ? entryScore(entry, normalizedQuery) : 0,
    }));

    if (normalizedQuery) {
      return base
        .filter((item) => item.score > 0)
        .sort((a, b) => b.score - a.score || a.index - b.index);
    }

    if (sortMode === "alpha") {
      return [...base].sort(
        (a, b) =>
          a.entry.word.localeCompare(b.entry.word, undefined, { sensitivity: "base" }) ||
          a.entry.kind.localeCompare(b.entry.kind) ||
          a.index - b.index,
      );
    }

    return base;
  }, [memory.words, normalizedQuery, sortMode]);

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal vocab-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <div>
            <h2>生词本</h2>
            <div className="vocab-summary">
              共 {memory.words.length} 条{normalizedQuery ? `，匹配 ${entries.length} 条` : ""}
            </div>
          </div>
          <button className="close-btn" onClick={onClose}>
            ×
          </button>
        </div>

        <div className="vocab-toolbar">
          <input
            className="vocab-search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="模糊搜索单词、释义、native 用法或上下文…"
            autoFocus
          />
          <Dropdown
            className="vocab-sort"
            value={sortMode}
            disabled={Boolean(normalizedQuery)}
            title={normalizedQuery ? "搜索时按匹配度排序" : "排序方式"}
            options={[
              { value: "added", label: "添加顺序" },
              { value: "alpha", label: "字母顺序" },
            ]}
            onChange={(v) => setSortMode(v as SortMode)}
          />
        </div>
        {normalizedQuery && <div className="vocab-search-note">搜索结果已按匹配度排序</div>}

        <div className="vocab-list">
          {entries.length === 0 ? (
            <div className="vocab-empty">
              {memory.words.length === 0 ? "还没有标记生词。" : "没有匹配的生词。"}
            </div>
          ) : (
            entries.map(({ entry, index }) => (
              <article className="vocab-entry" key={`${entry.kind}:${entry.word}:${index}`}>
                <div className="vocab-entry-head">
                  <div className="vocab-word-line">
                    <span className="vocab-word">{entry.word}</span>
                    {entry.ipa && <span className="ipa">{entry.ipa}</span>}
                    {entry.pos && <span className="pos">{entry.pos}</span>}
                  </div>
                  <div className="vocab-meta">
                    <span className={`vocab-kind ${entry.kind}`}>
                      {entry.kind === "word" ? "生词" : entry.kind === "usage" ? "用法" : "句子"}
                    </span>
                    <span>{formatDate(entry.first_marked)}</span>
                    <button
                      className="vocab-remove"
                      title="从生词本删除"
                      onClick={() => onRemove(entry.word, entry.kind)}
                    >
                      ×
                    </button>
                  </div>
                </div>

                {entry.meaning && <div className="vocab-meaning">{entry.meaning}</div>}
                {entry.native_usage && <div className="vocab-native">{entry.native_usage}</div>}
                {entry.contexts.length > 0 && (
                  <div className="vocab-context" title={entry.contexts[entry.contexts.length - 1]}>
                    {entry.contexts[entry.contexts.length - 1]}
                  </div>
                )}
              </article>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
