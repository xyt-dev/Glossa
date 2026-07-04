/** Runtime platform detection: same bundle serves Tauri desktop and browser. */
export const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/**
 * Web renders the desktop-tuned layout ~15% smaller (browser zoom is the
 * user-facing zoom there, and the desktop sizing reads oversized in a page).
 * Fixed-position popups (dropdown/context menu) must divide viewport
 * coordinates by this factor because CSS `zoom` on `.app` rescales them.
 */
export const uiScale = isTauri ? 1 : 0.85;
