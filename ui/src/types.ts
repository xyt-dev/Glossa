export type Mode = "translate" | "chat";
export type Role = "user" | "assistant";
export type MarkKind = "word" | "usage";

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
  native_usage?: string | null;
  examples: Example[];
}

export interface TranslationResult {
  translation: string;
  source_lang?: string | null;
  target_lang?: string | null;
  words: WordEntry[];
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
  context?: string | null;
}

export interface VocabEntry {
  word: string;
  kind: MarkKind;
  ipa?: string | null;
  pos?: string | null;
  meaning?: string | null;
  native_usage?: string | null;
  contexts: string[];
  marked_count: number;
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
  effort?: string | null;
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
  { id: "catppuccin-mocha", label: "Catppuccin Mocha（深）" },
  { id: "catppuccin-latte", label: "Catppuccin Latte（浅）" },
  { id: "gruvbox-dark", label: "Gruvbox Dark" },
  { id: "gruvbox-light", label: "Gruvbox Light" },
] as const;
