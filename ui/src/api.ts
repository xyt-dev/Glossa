import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  Config,
  MarkInput,
  MarkKind,
  Mode,
  SendEvent,
  Session,
  SessionMeta,
  VocabEntry,
  VocabMemory,
} from "./types";

export const api = {
  listSessions: () => invoke<SessionMeta[]>("list_sessions"),
  createSession: () => invoke<Session>("create_session"),
  loadSession: (id: string) => invoke<Session>("load_session", { id }),
  deleteSession: (id: string) => invoke<void>("delete_session", { id }),
  renameSession: (id: string, title: string) =>
    invoke<Session>("rename_session", { id, title }),

  /** Streams SendEvents into `onEvent`; resolves when the turn is persisted. */
  sendMessage: (
    sessionId: string,
    text: string,
    mode: Mode,
    onEvent: (e: SendEvent) => void,
  ) => {
    const channel = new Channel<SendEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("send_message", { sessionId, text, mode, onEvent: channel });
  },

  markWord: (input: MarkInput) => invoke<VocabEntry>("mark_word", { input }),
  unmarkWord: (word: string, kind: MarkKind) =>
    invoke<void>("unmark_word", { word, kind }),
  getMemory: () => invoke<VocabMemory>("get_memory"),

  getConfig: () => invoke<Config>("get_config"),
  setConfig: (config: Config) => invoke<void>("set_config", { config }),
  setZoom: (zoom: number) => invoke<void>("set_zoom", { zoom }),
};
