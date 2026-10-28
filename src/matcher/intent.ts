import { contentTokens, tokenScore } from "./text";
import type { LayoutId } from "./types";

type Dir = "L" | "R" | "T" | "B" | "C";
type Frac = "half" | "third" | "two" | "quarter";

const DIRS: Record<Dir, string[]> = {
  L: ["left", "lft", "west"],
  R: ["right", "rght", "east"],
  T: ["top", "up", "upper", "north"],
  B: ["bottom", "down", "lower", "south", "btm"],
  C: ["center", "centre", "middle", "centered"],
};

const FRACS: Record<Frac, string[]> = {
  half: ["half", "1/2", "50"],
  third: ["third", "thirds", "1/3", "33"],
  two: ["twothirds", "2/3", "66", "67"],
  quarter: ["quarter", "quarters", "corner", "1/4", "25"],
};

const MAX = ["max", "maximize", "maximise", "full", "fullscreen", "fill", "biggest"];
const ALMOST = ["almost", "nearly", "90"];

const match = (t: string, words: string[]) => words.some((w) => t === w || (t.length >= 3 && tokenScore(t, w) >= 0.75));

interface Entities {
  dirs: Dir[];
  frac: Frac | null;
  max: boolean;
  almost: boolean;
}

function extract(toks: string[]): Entities {
  const dirs: Dir[] = [];
  let frac: Frac | null = null;
  let max = false;
  let almost = false;
  for (let i = 0; i < toks.length; i++) {
    const t = toks[i];
    if (match(t, ALMOST)) almost = true;
    else if (match(t, MAX)) max = true;
    else if (t === "two" && /^thirds?$/.test(toks[i + 1] ?? "")) {
      frac = "two";
      i++;
    } else if (t === "one" && /^third$/.test(toks[i + 1] ?? "")) {
      frac = "third";
      i++;
    } else {
      const f = (Object.keys(FRACS) as Frac[]).find((k) => match(t, FRACS[k]));
      if (f) {
        // "thirds" after "two" was consumed above; a bare plural means two thirds only with "two".
        frac = f;
        continue;
      }
      const d = (Object.keys(DIRS) as Dir[]).find((k) => match(t, DIRS[k]));
      if (d && !dirs.includes(d)) dirs.push(d);
    }
  }
  return { dirs, frac, max, almost };
}

/**
 * Turn a natural language request into the most likely layouts, best first.
 * Returns nothing when the text does not describe a layout; plain fuzzy search
 * covers single words, this handles word order and combinations
 * ("upper right corner", "two thirds on the left").
 */
export function resolveLayouts(query: string): LayoutId[] {
  const e = extract(contentTokens(query));
  const has = (d: Dir) => e.dirs.includes(d);
  const vertical = has("T") || has("B");
  const horizontal = has("L") || has("R");

  if (e.almost) return ["almost-maximize"];
  if (e.max && !e.dirs.length) return ["maximize"];

  if (e.frac === "quarter" || (vertical && horizontal)) {
    const rows: Array<"top" | "bottom"> = has("T") ? ["top"] : has("B") ? ["bottom"] : ["top", "bottom"];
    const cols: Array<"left" | "right"> = has("L") ? ["left"] : has("R") ? ["right"] : ["left", "right"];
    if (!e.dirs.length) return [];
    return rows.flatMap((r) => cols.map((c) => `${r}-${c}` as LayoutId));
  }

  if (e.frac === "two") {
    if (has("L")) return ["left-two-thirds"];
    if (has("R")) return ["right-two-thirds"];
    return ["left-two-thirds", "right-two-thirds"];
  }

  if (e.frac === "third") {
    if (has("L")) return ["left-third"];
    if (has("C")) return ["center-third"];
    if (has("R")) return ["right-third"];
    if (!e.dirs.length) return ["left-third", "center-third", "right-third"];
    // No vertical thirds exist, fall through to halves for top/bottom.
  }

  if (has("C") && !horizontal && !vertical) return ["center"];
  if (e.frac === "half" || e.dirs.length) {
    const out: LayoutId[] = [];
    if (has("L")) out.push("left-half");
    if (has("R")) out.push("right-half");
    if (has("T")) out.push("top-half");
    if (has("B")) out.push("bottom-half");
    return out;
  }
  return [];
}
