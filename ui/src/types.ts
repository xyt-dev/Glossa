export type Mode = "translate" | "chat";
export type Role = "user" | "assistant";
export type MarkKind = "word" | "usage" | "sentence";

export interface Example {
  en: string;
  zh: string;
}

export interface WordEntry {
  word: string;
  ipa?: string | null;
  pos?: string | null;
  meaning?: string | null;
  ielts_band?: number | null;
  /** legacy (≤v0.2) per-word native note; still rendered for old sessions */
  native_usage?: string | null;
  examples: Example[];
}

export interface SentencePair {
  src: string;
  dst: string;
}

/** Native expression found in the source sentence, its own card. */
export interface UsageEntry {
  usage: string;
  explanation?: string | null;
  examples: Example[];
}

export interface TranslationResult {
  translation: string;
  sentences: SentencePair[];
  source_lang?: string | null;
  target_lang?: string | null;
  words: WordEntry[];
  usages: UsageEntry[];
}

export interface Message {
  role: Role;
  mode: Mode;
  text?: string | null;
  result?: TranslationResult | null;
  raw?: string | null;
  ts: string;
}

export interface SessionMeta {
  id: string;
  title: string;
  created: string;
  updated: string;
}

export interface Session extends SessionMeta {
  messages: Message[];
}

export type SendEvent =
  | { type: "delta"; text: string }
  | { type: "parsed"; result: TranslationResult }
  | { type: "fallback"; raw: string }
  | { type: "done"; session: SessionMeta }
  | { type: "error"; message: string };

export interface MarkInput {
  word: string;
  kind: MarkKind;
  ipa?: string | null;
  pos?: string | null;
  meaning?: string | null;
  native_usage?: string | null;
  examples: Example[];
}

export interface VocabEntry {
  word: string;
  kind: MarkKind;
  ipa?: string | null;
  pos?: string | null;
  meaning?: string | null;
  native_usage?: string | null;
  examples: Example[];
  /** legacy contexts from ≤v0.2; new word/usage entries expand examples instead */
  contexts: string[];
  first_marked: string;
  last_marked: string;
}

export interface VocabMemory {
  profile_summary: string;
  words: VocabEntry[];
}

export interface Profile {
  name: string;
  base_url: string;
  api_key: string;
  api_key_env: string;
  model: string;
  translate_effort?: string | null;
  chat_effort?: string | null;
  provider?: string | null;
  temperature?: number | null;
  extra?: Record<string, unknown> | null;
}

export interface Config {
  active_profile: string;
  ui: { theme: string; zoom: number };
  memory: {
    path?: string | null;
    min_ielts_band: number;
    max_context_words: number;
  };
  session: {
    default_mode: Mode;
    max_context_messages: number;
  };
  profiles: Profile[];
}

export const THEMES = [
  { id: "gruvbox-light", label: "Gruvbox Light" },
  { id: "gruvbox-dark", label: "Gruvbox Dark" },
  { id: "catppuccin-latte", label: "Catppuccin Light" },
  { id: "catppuccin-mocha", label: "Catppuccin Dark" },
] as const;
