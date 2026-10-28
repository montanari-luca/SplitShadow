import { actions, type ActionDef } from "./actions";
import { resolveLayouts } from "./intent";
import { contentTokens, levenshtein, normalize, phraseScore, tokens } from "./text";
import type { Command, SearchContext, Suggestion } from "./types";

const THRESHOLD = 0.45;
const LIMIT = 8;

const VERBS = {
  save: ["save", "create", "new", "store"],
  open: ["open", "load", "switch", "launch", "start", "activate"],
  update: ["update", "overwrite", "replace", "refresh", "resave"],
  delete: ["delete", "remove", "rm", "erase", "trash"],
  split: ["split", "pair", "tile", "link"],
  unsplit: ["unsplit", "unlink"],
} as const;

type Verb = keyof typeof VERBS;

const NOUNS = ["workspace", "workspaces", "ws", "layout", "layouts"];
const NOISE = new Set([...NOUNS, "as", "named", "called", "with", "the", "my", "current", "active", "to", "a", "this", "window", "and"]);

function verbOf(word: string): Verb | null {
  const w = word.toLowerCase();
  const verbs = Object.keys(VERBS) as Verb[];
  // Exact words win over fuzzy ones: "unsplit" is its own verb, not a typo of "split".
  const exact = verbs.find((v) => (VERBS[v] as readonly string[]).includes(w));
  if (exact) return exact;
  if (w.length < 3) return null;
  return verbs.find((v) => (VERBS[v] as readonly string[]).some((x) => x.startsWith(w) || levenshtein(w, x) <= 1)) ?? null;
}

/** Raw (case preserved) argument text after the verb, minus filler words. */
function argumentOf(raw: string): string {
  return raw
    .trim()
    .split(/\s+/)
    .slice(1)
    .filter((w) => !NOISE.has(w.toLowerCase()))
    .join(" ")
    .trim();
}

function matchName(arg: string, name: string): number {
  if (!arg) return 0.5;
  const q = contentTokens(arg);
  return phraseScore(q, tokens(name));
}

function actionRows(query: string): Suggestion[] {
  const q = contentTokens(query);
  const layouts = resolveLayouts(query);
  const rows: Suggestion[] = [];
  for (const a of actions) {
    let score = 0;
    for (const phrase of [a.title.replace(/splitshadow/i, ""), ...a.synonyms]) score = Math.max(score, phraseScore(q, tokens(phrase)));
    const li = a.command.type === "layout" ? layouts.indexOf(a.command.layout) : -1;
    if (li >= 0) score = Math.max(score, 1.5 - li * 0.05);
    if (score >= THRESHOLD) rows.push(fromAction(a, score));
  }
  return rows;
}

function fromAction(a: ActionDef, score: number): Suggestion {
  return { key: a.id, title: a.title, subtitle: a.subtitle, glyph: a.glyph, command: a.command, score };
}

const openRow = (name: string, score: number, active: boolean): Suggestion => ({
  key: `ws-open:${name}`,
  title: `Open workspace ${name}`,
  subtitle: active ? "Currently active" : undefined,
  glyph: { icon: "workspace" },
  command: { type: "ws-open", name },
  score,
});

function splitRows(arg: string, ctx: SearchContext, base: number): Suggestion[] {
  return ctx.windows
    .filter((w) => w.id !== ctx.targetId)
    .map((w) => {
      const s = arg ? Math.max(matchName(arg, w.app), matchName(arg, w.title) * 0.9) : 0.5;
      return {
        key: `split:${w.id}`,
        title: `Split with ${w.app}`,
        subtitle: w.title && w.title !== w.app ? w.title : undefined,
        glyph: { icon: "split" } as const,
        command: { type: "split", windowId: w.id } as Command,
        score: base + s,
      };
    })
    .filter((r) => r.score >= base + THRESHOLD);
}

function workspaceVerbRows(verb: Verb, raw: string, ctx: SearchContext): Suggestion[] {
  const arg = argumentOf(raw);
  const names = ctx.workspaces;
  const scored = names
    .map((n) => ({ n, s: matchName(arg, n) }))
    .filter((x) => x.s >= THRESHOLD)
    .sort((a, b) => b.s - a.s);

  switch (verb) {
    case "save": {
      if (!arg) {
        return [
          {
            key: "ws-save:prompt",
            title: "Save workspace",
            subtitle: "Press Enter, then type a name",
            glyph: { icon: "save" },
            command: { type: "complete", text: "save workspace as " },
            score: 3,
          },
        ];
      }
      const exists = names.some((n) => n.toLowerCase() === arg.toLowerCase());
      return [
        {
          key: `ws-save:${arg}`,
          title: `Save workspace as "${arg}"`,
          subtitle: exists ? "Replaces the existing workspace" : "Saves the windows on this display",
          glyph: { icon: "save" },
          command: { type: "ws-save", name: arg },
          score: 3,
        },
      ];
    }
    case "open":
      return scored.map((x, i) => openRow(x.n, 3 + x.s - i * 0.01, ctx.activeWorkspace === x.n));
    case "update": {
      const rows: Suggestion[] = [];
      if (!arg && ctx.activeWorkspace) {
        rows.push({
          key: "ws-update:current",
          title: `Update current workspace (${ctx.activeWorkspace})`,
          subtitle: "Overwrite it with the windows as they are now",
          glyph: { icon: "update" },
          command: { type: "ws-update" },
          score: 3.5,
        });
      }
      for (const x of scored) {
        if (!arg && x.n === ctx.activeWorkspace) continue;
        rows.push({
          key: `ws-update:${x.n}`,
          title: `Update workspace ${x.n}`,
          subtitle: "Overwrite it with the windows as they are now",
          glyph: { icon: "update" },
          command: { type: "ws-update", name: x.n },
          score: 3 + x.s,
        });
      }
      if (!rows.length && !names.length) return [emptyWorkspaces()];
      return rows;
    }
    case "delete":
      return scored.map((x) => ({
        key: `ws-delete:${x.n}`,
        title: `Delete workspace ${x.n}`,
        subtitle: "Asks for confirmation",
        glyph: { icon: "delete" } as const,
        command: { type: "ws-delete", name: x.n } as Command,
        confirm: true,
        score: 3 + x.s,
      }));
    case "split":
      return splitRows(arg, ctx, 3);
    case "unsplit":
      return [unsplitRow(3)];
  }
}

const emptyWorkspaces = (): Suggestion => ({
  key: "ws-none",
  title: "No workspaces yet",
  subtitle: 'Press Enter to save one: "save workspace as ..."',
  glyph: { icon: "save" },
  command: { type: "complete", text: "save workspace as " },
  score: 2,
});

const unsplitRow = (score: number): Suggestion => ({
  key: "unsplit",
  title: "Unlink split",
  subtitle: "Windows stop following each other",
  glyph: { icon: "unsplit" },
  command: { type: "unsplit" },
  score,
});

function defaults(ctx: SearchContext): Suggestion[] {
  const rows: Suggestion[] = [];
  if (ctx.update) {
    rows.push({
      key: "install-update",
      title: `Update to v${ctx.update}`,
      subtitle: "Download, install and relaunch",
      glyph: { icon: "download" },
      command: { type: "install-update" },
      score: 100,
    });
  }
  if (ctx.linked) rows.push(unsplitRow(90));
  for (const a of actions.filter((x) => x.featured)) rows.push(fromAction(a, 50 - a.rank));
  ctx.workspaces.slice(0, 3).forEach((n, i) => rows.push(openRow(n, 20 - i, ctx.activeWorkspace === n)));
  return rows;
}

export function search(query: string, ctx: SearchContext, limit = LIMIT): Suggestion[] {
  const raw = query.trim();
  let rows: Suggestion[];

  if (!raw) {
    rows = defaults(ctx);
  } else {
    const first = raw.split(/\s+/)[0];
    const verb = verbOf(first);
    const nounOnly = tokens(raw).length === 1 && NOUNS.includes(normalize(raw));
    rows = [];

    if (nounOnly) {
      rows.push(...ctx.workspaces.map((n, i) => openRow(n, 3 - i * 0.01, ctx.activeWorkspace === n)));
      if (!ctx.workspaces.length) rows.push(emptyWorkspaces());
    } else if (verb) {
      rows.push(...workspaceVerbRows(verb, raw, ctx));
    }

    rows.push(...actionRows(raw));

    // A bare workspace or app name is a shortcut for opening or splitting.
    if (!verb && !nounOnly) {
      for (const n of ctx.workspaces) {
        const s = matchName(raw, n);
        if (s >= THRESHOLD) rows.push(openRow(n, 0.9 * s, ctx.activeWorkspace === n));
      }
      for (const r of splitRows(raw, ctx, 0)) rows.push({ ...r, score: r.score * 0.85 });
    }

    if (ctx.update && /^(upd|upg|inst|new ver|downl)/.test(normalize(raw))) {
      rows.push(defaults(ctx)[0]);
    }
    if (ctx.linked && /^(unl|uns|stop|break|detach)/.test(normalize(raw))) rows.push(unsplitRow(1.4));
  }

  const seen = new Set<string>();
  return rows
    .filter((r) => (seen.has(r.key) ? false : (seen.add(r.key), true)))
    .sort((a, b) => b.score - a.score || rankOf(a) - rankOf(b))
    .slice(0, limit);
}

const rankOf = (s: Suggestion) => actions.find((a) => a.id === s.key)?.rank ?? 1000;
