export type LayoutId =
  | "left-half"
  | "right-half"
  | "top-half"
  | "bottom-half"
  | "left-third"
  | "center-third"
  | "right-third"
  | "left-two-thirds"
  | "right-two-thirds"
  | "top-left"
  | "top-right"
  | "bottom-left"
  | "bottom-right"
  | "maximize"
  | "almost-maximize"
  | "center";

export type Command =
  | { type: "layout"; layout: LayoutId }
  | { type: "minimize" }
  | { type: "resize"; step: number }
  | { type: "display"; delta: 1 | -1 }
  | { type: "restore" }
  | { type: "split"; windowId: number }
  | { type: "unsplit" }
  | { type: "ws-save"; name: string }
  | { type: "ws-open"; name: string }
  | { type: "ws-update"; name?: string }
  | { type: "ws-delete"; name: string }
  | { type: "settings" }
  | { type: "quit" }
  | { type: "install-update" }
  | { type: "check-update" }
  /** UI only: replace the query text (used to ask for a missing argument). */
  | { type: "complete"; text: string };

export type IconName =
  | "minimize"
  | "larger"
  | "smaller"
  | "display"
  | "restore"
  | "split"
  | "unsplit"
  | "workspace"
  | "save"
  | "update"
  | "delete"
  | "settings"
  | "quit"
  | "download"
  | "window";

/** Either a rectangle drawn inside a tiny screen (fractions x, y, w, h) or a named icon. */
export type Glyph = { rect: [number, number, number, number] } | { icon: IconName };

export interface Suggestion {
  key: string;
  title: string;
  subtitle?: string;
  glyph: Glyph;
  command: Command;
  /** Ask for confirmation before running. */
  confirm?: boolean;
  score: number;
}

export interface WindowRef {
  id: number;
  app: string;
  title: string;
}

export interface SearchContext {
  windows: WindowRef[];
  workspaces: string[];
  activeWorkspace: string | null;
  /** Window the palette will act on, excluded from split partners. */
  targetId: number | null;
  /** Version string when an update is available. */
  update: string | null;
  /** Whether the target window is currently in a linked split. */
  linked: boolean;
}

export const emptyContext: SearchContext = {
  windows: [],
  workspaces: [],
  activeWorkspace: null,
  targetId: null,
  update: null,
  linked: false,
};
