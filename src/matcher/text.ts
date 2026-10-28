/** Small, dependency free text matching helpers. */

const STOPWORDS = new Set([
  "move", "moves", "put", "set", "go", "snap", "send", "shift", "place", "make", "resize",
  "window", "windows", "this", "that", "it", "the", "a", "an", "to", "of", "on", "in", "at",
  "my", "screen", "side", "please", "pls", "now", "into", "onto", "all", "way", "for", "me",
]);

export function normalize(s: string): string {
  return s
    .toLowerCase()
    .replace(/%/g, " percent ")
    .replace(/(\d)\s*\/\s*(\d)/g, "$1/$2")
    .replace(/[^a-z0-9/\s]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function tokens(s: string): string[] {
  return normalize(s).split(" ").filter(Boolean);
}

/** Query tokens with filler words removed (kept if nothing else would remain). */
export function contentTokens(s: string): string[] {
  const all = tokens(s);
  const kept = all.filter((t) => !STOPWORDS.has(t));
  return kept.length ? kept : all;
}

export function levenshtein(a: string, b: string): number {
  if (a === b) return 0;
  if (!a.length) return b.length;
  if (!b.length) return a.length;
  let prev = Array.from({ length: b.length + 1 }, (_, i) => i);
  for (let i = 1; i <= a.length; i++) {
    const cur = [i];
    for (let j = 1; j <= b.length; j++) {
      cur[j] = Math.min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (a[i - 1] === b[j - 1] ? 0 : 1));
    }
    prev = cur;
  }
  return prev[b.length];
}

function isSubsequence(q: string, t: string): boolean {
  let i = 0;
  for (const c of t) if (c === q[i]) i++;
  return i === q.length;
}

/** How well one query token matches one target token, 0 to 1. */
export function tokenScore(q: string, t: string): number {
  if (q === t) return 1;
  if (t.startsWith(q)) return q.length === 1 ? 0.6 : 0.9;
  if (q.length >= 4 && levenshtein(q, t) <= (q.length >= 10 ? 2 : 1)) return 0.75;
  if (q.length >= 3 && q[0] === t[0] && isSubsequence(q, t)) return 0.65;
  return 0;
}

/**
 * Score query tokens against one target phrase. Every query token must match some
 * target token or the score collapses, so "left half" does not pick "left third".
 */
export function phraseScore(query: string[], target: string[]): number {
  if (!query.length || !target.length) return 0;
  let sum = 0;
  let matched = 0;
  const used = new Set<number>();
  for (const q of query) {
    let best = 0;
    let bestIdx = -1;
    target.forEach((t, i) => {
      const s = tokenScore(q, t) * (used.has(i) ? 0.5 : 1);
      if (s > best) {
        best = s;
        bestIdx = i;
      }
    });
    if (best > 0) {
      matched++;
      used.add(bestIdx);
    }
    sum += best;
  }
  if (matched < query.length) return 0;
  const coverage = used.size / target.length;
  return (sum / query.length) * (0.85 + 0.15 * coverage);
}
