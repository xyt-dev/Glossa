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
  const exampleScore = Math.max(
    0,
    ...entry.examples.map((ex) => fuzzyScoreText(`${ex.en} ${ex.zh}`, query)),
  );

  return Math.max(wordScore, meaningScore, usageScore, ipaScore, posScore, exampleScore);
}

function formatDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  return `${date.getMonth() + 1}/${date.getDate()}`;
}


export default function VocabBook({ memory, onClose, onRemove }: Props) {
  const [query, setQuery] = useState("");
  const [sortMode, setSortMode] = useState<SortMode>("added");
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const normalizedQuery = normalize(query);

  const toggleExpanded = (key: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  const entries = useMemo<ScoredEntry[]>(() => {
    const base = memory.words.map((entry, index) => ({
      entry,
      index,
      score: normalizedQuery ? entryScore(entry, normalizedQuery) : 0,
    }));

    if (normalizedQuery) {
      // tie-break: newest first, matching the default list order
      return base
        .filter((item) => item.score > 0)
        .sort((a, b) => b.score - a.score || b.index - a.index);
    }

    if (sortMode === "alpha") {
      return [...base].sort(
        (a, b) =>
          a.entry.word.localeCompare(b.entry.word, undefined, { sensitivity: "base" }) ||
          a.entry.kind.localeCompare(b.entry.kind) ||
          b.index - a.index,
      );
    }

    // 添加顺序：最新标记的在最上方
    return [...base].reverse();
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
            placeholder="模糊搜索单词、释义、native 用法或例句…"
            autoFocus
          />
          <Dropdown
            className="vocab-sort"
            value={sortMode}
            disabled={Boolean(normalizedQuery)}
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
                    <span className={`vocab-kind kind-${entry.kind}`}>
                      {entry.kind === "word" ? "生词" : entry.kind === "usage" ? "用法" : "句子"}
                    </span>
                    <span>{formatDate(entry.first_marked)}</span>
                    <button
                      className="vocab-remove"
                      onClick={() => onRemove(entry.word, entry.kind)}
                    >
                      ×
                    </button>
                  </div>
                </div>

                {entry.meaning && <div className="vocab-meaning">{entry.meaning}</div>}
                {entry.native_usage && <div className="vocab-native">{entry.native_usage}</div>}
                {entry.examples.length > 0 &&
                  (() => {
                    const key = `${entry.kind}:${entry.word.toLowerCase()}`;
                    const open = expanded.has(key);
                    return open ? (
                      <div
                        className="vocab-examples-open"
                        onClick={() => toggleExpanded(key)}
                      >
                        <div className="vocab-examples-title">
                          <span className="ctx-chevron">▾</span>
                          例句
                        </div>
                        {entry.examples.map((ex, i) => (
                          <div key={i} className="vocab-example-item">
                            <span className="vocab-example-en">{ex.en}</span>
                            <span className="vocab-example-zh">{ex.zh}</span>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <div className="vocab-examples-toggle" onClick={() => toggleExpanded(key)}>
                        <span className="ctx-chevron">▸</span>
                        <span className="ctx-text">展开例句（{entry.examples.length}）</span>
                      </div>
                    );
                  })()}
              </article>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
