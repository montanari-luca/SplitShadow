import type { Command, Glyph, LayoutId } from "./types";

export interface ActionDef {
  id: string;
  title: string;
  subtitle?: string;
  glyph: Glyph;
  command: Command;
  /** Extra phrases that should find this action. */
  synonyms: string[];
  /** Tie-break and empty-query order, lower first. */
  rank: number;
  /** Shown in the empty-query list. */
  featured?: boolean;
}

const layout = (
  id: LayoutId,
  title: string,
  rect: [number, number, number, number],
  synonyms: string[],
  rank: number,
  featured = false,
): ActionDef => ({ id, title, glyph: { rect }, command: { type: "layout", layout: id }, synonyms, rank, featured });

export const layoutTitles: Record<LayoutId, string> = {
  "left-half": "Left half",
  "right-half": "Right half",
  "top-half": "Top half",
  "bottom-half": "Bottom half",
  "left-third": "Left third",
  "center-third": "Center third",
  "right-third": "Right third",
  "left-two-thirds": "Left two thirds",
  "right-two-thirds": "Right two thirds",
  "top-left": "Top left quarter",
  "top-right": "Top right quarter",
  "bottom-left": "Bottom left quarter",
  "bottom-right": "Bottom right quarter",
  maximize: "Maximize",
  "almost-maximize": "Almost maximize",
  center: "Center window",
};

export const actions: ActionDef[] = [
  layout("left-half", "Left half", [0, 0, 0.5, 1], ["left", "half left", "left side", "west"], 1, true),
  layout("right-half", "Right half", [0.5, 0, 0.5, 1], ["right", "half right", "right side", "east"], 2, true),
  layout("top-half", "Top half", [0, 0, 1, 0.5], ["top", "up", "upper half", "north"], 3, true),
  layout("bottom-half", "Bottom half", [0, 0.5, 1, 0.5], ["bottom", "down", "lower half", "south"], 4, true),
  layout("maximize", "Maximize", [0, 0, 1, 1], ["max", "full", "fullscreen", "full screen", "fill screen", "biggest"], 5, true),
  layout("almost-maximize", "Almost maximize", [0.05, 0.05, 0.9, 0.9], ["nearly maximize", "almost full", "90 percent", "90%", "big"], 6, true),
  layout("center", "Center window", [0.25, 0.2, 0.5, 0.6], ["center", "centre", "middle", "centered"], 7, true),
  layout("left-third", "Left third", [0, 0, 1 / 3, 1], ["third left", "1/3 left", "one third left", "left 1/3"], 10),
  layout("center-third", "Center third", [1 / 3, 0, 1 / 3, 1], ["third center", "middle third", "1/3 center", "center 1/3"], 11),
  layout("right-third", "Right third", [2 / 3, 0, 1 / 3, 1], ["third right", "1/3 right", "one third right", "right 1/3"], 12),
  layout("left-two-thirds", "Left two thirds", [0, 0, 2 / 3, 1], ["two thirds left", "2/3 left", "left 2/3", "66 percent left"], 13),
  layout("right-two-thirds", "Right two thirds", [1 / 3, 0, 2 / 3, 1], ["two thirds right", "2/3 right", "right 2/3", "66 percent right"], 14),
  layout("top-left", "Top left quarter", [0, 0, 0.5, 0.5], ["upper left", "top left corner", "quarter top left"], 20),
  layout("top-right", "Top right quarter", [0.5, 0, 0.5, 0.5], ["upper right", "top right corner", "quarter top right"], 21),
  layout("bottom-left", "Bottom left quarter", [0, 0.5, 0.5, 0.5], ["lower left", "bottom left corner", "quarter bottom left"], 22),
  layout("bottom-right", "Bottom right quarter", [0.5, 0.5, 0.5, 0.5], ["lower right", "bottom right corner", "quarter bottom right"], 23),
  {
    id: "minimize",
    title: "Hide window",
    subtitle: "Minimize",
    glyph: { icon: "minimize" },
    command: { type: "minimize" },
    synonyms: ["hide", "minimize", "minimise", "tuck away", "dismiss"],
    rank: 30,
    featured: true,
  },
  {
    id: "restore",
    title: "Restore size",
    subtitle: "Undo the last move",
    glyph: { icon: "restore" },
    command: { type: "restore" },
    synonyms: ["restore", "undo", "previous size", "original size", "revert", "back"],
    rank: 31,
  },
  {
    id: "larger",
    title: "Make larger",
    subtitle: "Grow by 10%",
    glyph: { icon: "larger" },
    command: { type: "resize", step: 0.1 },
    synonyms: ["larger", "bigger", "grow", "expand", "increase size", "enlarge"],
    rank: 32,
  },
  {
    id: "smaller",
    title: "Make smaller",
    subtitle: "Shrink by 10%",
    glyph: { icon: "smaller" },
    command: { type: "resize", step: -0.1 },
    synonyms: ["smaller", "shrink", "reduce", "decrease size", "tinier"],
    rank: 33,
  },
  {
    id: "next-display",
    title: "Next display",
    subtitle: "Send to the display on the right",
    glyph: { icon: "display" },
    command: { type: "display", delta: 1 },
    synonyms: ["next monitor", "other display", "other monitor", "next screen", "second monitor", "move display"],
    rank: 34,
  },
  {
    id: "prev-display",
    title: "Previous display",
    subtitle: "Send to the display on the left",
    glyph: { icon: "display" },
    command: { type: "display", delta: -1 },
    synonyms: ["previous monitor", "previous screen", "last display", "prev display"],
    rank: 35,
  },
  {
    id: "settings",
    title: "Settings",
    subtitle: "Hotkey, workspaces, about",
    glyph: { icon: "settings" },
    command: { type: "settings" },
    synonyms: ["preferences", "options", "config", "configure", "about", "hotkey", "shortcut"],
    rank: 90,
  },
  {
    id: "check-update",
    title: "Check for updates",
    glyph: { icon: "download" },
    command: { type: "check-update" },
    synonyms: ["update app", "new version", "upgrade", "updates"],
    rank: 91,
  },
  {
    id: "quit",
    title: "Quit SplitShadow",
    glyph: { icon: "quit" },
    command: { type: "quit" },
    synonyms: ["exit", "close app", "stop"],
    rank: 99,
  },
];
