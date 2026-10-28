import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { search } from "../matcher/search";
import { emptyContext, type SearchContext, type Suggestion } from "../matcher/types";
import { completionFor } from "./complete";
import { GlyphView } from "./Glyph";

interface ShowPayload {
  trusted: boolean;
  targetApp: string | null;
  targetTitle: string | null;
}

const isMac = navigator.platform.toLowerCase().includes("mac");
const MOD = isMac ? "⌘" : "Ctrl+";

export function Palette() {
  const [query, setQuery] = useState("");
  const [ctx, setCtx] = useState<SearchContext>(emptyContext);
  const [info, setInfo] = useState<ShowPayload>({ trusted: true, targetApp: null, targetTitle: null });
  const [selected, setSelected] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState<Suggestion | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLDivElement>(null);

  const suggestions = useMemo(() => search(query, ctx), [query, ctx]);

  const refresh = useCallback(async () => {
    try {
      setCtx(await invoke<SearchContext>("palette_context"));
    } catch {
      /* keep the previous context */
    }
  }, []);

  useEffect(() => {
    const un = listen<ShowPayload>("palette://show", (e) => {
      setQuery("");
      setSelected(0);
      setError(null);
      setPending(null);
      setInfo(e.payload);
      input.current?.focus();
      void refresh();
    });
    return () => void un.then((f) => f());
  }, [refresh]);

  useEffect(() => setSelected(0), [query]);
  useEffect(() => {
    list.current?.querySelector<HTMLElement>(`[data-i="${selected}"]`)?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  const close = () => void invoke("palette_hide");

  const run = async (s: Suggestion | undefined) => {
    if (!s) return;
    if (s.command.type === "complete") {
      setQuery(s.command.text);
      return;
    }
    if (s.confirm && pending?.key !== s.key) {
      setPending(s);
      return;
    }
    setPending(null);
    try {
      await invoke("run_action", { action: s.command });
    } catch (e) {
      setError(String(e));
    }
  };

  const onKey = (e: React.KeyboardEvent) => {
    const mod = isMac ? e.metaKey : e.ctrlKey;
    if (e.key === "Escape") {
      e.preventDefault();
      if (pending) setPending(null);
      else close();
    } else if (e.key === "ArrowDown" || (e.ctrlKey && e.key === "n")) {
      e.preventDefault();
      setSelected((i) => Math.min(i + 1, suggestions.length - 1));
    } else if (e.key === "ArrowUp" || (e.ctrlKey && e.key === "p")) {
      e.preventDefault();
      setSelected((i) => Math.max(i - 1, 0));
    } else if (e.key === "Tab") {
      e.preventDefault();
      const s = suggestions[selected];
      if (s) setQuery(completionFor(s));
    } else if (e.key === "Enter") {
      e.preventDefault();
      void run(pending ?? suggestions[selected]);
    } else if (mod && /^[1-9]$/.test(e.key)) {
      e.preventDefault();
      void run(suggestions[Number(e.key) - 1]);
    }
  };

  const ghost = useMemo(() => {
    const s = suggestions[selected];
    if (!s || !query) return "";
    const full = completionFor(s);
    return full.toLowerCase().startsWith(query.toLowerCase()) ? full.slice(query.length) : "";
  }, [suggestions, selected, query]);

  const rows = pending ? [pending] : suggestions;

  return (
    <div className="h-full w-full p-1" onMouseDown={(e) => e.target === e.currentTarget && close()}>
      <div className="pop overflow-hidden rounded-[14px] border" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
        <div className="relative flex h-16 items-center px-5">
          <div className="relative flex-1 text-[19px]">
            <div className="pointer-events-none absolute inset-0 flex items-center whitespace-pre" aria-hidden>
              <span className="invisible">{query}</span>
              <span style={{ color: "var(--muted)", opacity: 0.6 }}>{ghost}</span>
            </div>
            <input
              ref={input}
              autoFocus
              spellCheck={false}
              autoCorrect="off"
              autoCapitalize="off"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={onKey}
              placeholder="Move, resize, split, or open a workspace"
              className="relative h-10 w-full bg-transparent outline-none placeholder:opacity-50"
              style={{ color: "var(--text)", caretColor: "var(--glyph-fill)" }}
            />
          </div>
          {info.targetApp && (
            <div className="ml-3 max-w-[180px] truncate rounded-md px-2 py-1 text-xs" style={{ background: "var(--chip)", color: "var(--muted)" }} title={info.targetTitle ?? undefined}>
              {info.targetApp}
            </div>
          )}
        </div>

        {!info.trusted && (
          <button
            className="mx-3 mb-2 flex w-[calc(100%-1.5rem)] items-center justify-between rounded-lg px-3 py-2 text-left text-[13px]"
            style={{ background: "var(--row-active)" }}
            onClick={() => void invoke("request_accessibility")}
          >
            <span>SplitShadow needs Accessibility access to move windows.</span>
            <span style={{ color: "var(--muted)" }}>Grant access</span>
          </button>
        )}
        {error && (
          <div className="mx-3 mb-2 rounded-lg px-3 py-2 text-[13px]" style={{ background: "rgba(255,69,58,0.15)", color: "#ff6b61" }}>
            {error}
          </div>
        )}

        {rows.length > 0 && (
          <div ref={list} className="max-h-[368px] overflow-y-auto border-t px-2 py-2" style={{ borderColor: "var(--border)" }}>
            {rows.map((s, i) => {
              const active = pending ? true : i === selected;
              return (
                <div
                  key={s.key}
                  data-i={i}
                  onMouseMove={() => !pending && setSelected(i)}
                  onClick={() => void run(s)}
                  className="flex h-11 items-center gap-3 rounded-lg px-3"
                  style={{ background: active ? "var(--row-active)" : "transparent" }}
                >
                  <div className="flex w-8 justify-center">
                    <GlyphView glyph={s.glyph} />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[14.5px]">{pending ? `${s.title}?` : s.title}</div>
                    {(pending || s.subtitle) && (
                      <div className="truncate text-xs" style={{ color: "var(--muted)" }}>
                        {pending ? "Press Enter to confirm, Esc to cancel" : s.subtitle}
                      </div>
                    )}
                  </div>
                  {!pending && i < 9 && (
                    <kbd className="rounded-md px-1.5 py-0.5 font-sans text-[11px]" style={{ background: "var(--chip)", color: "var(--muted)" }}>
                      {MOD}
                      {i + 1}
                    </kbd>
                  )}
                </div>
              );
            })}
          </div>
        )}

        <div className="flex h-9 items-center gap-4 border-t px-4 text-[11.5px]" style={{ borderColor: "var(--border)", color: "var(--muted)" }}>
          <Hint k={"↑↓"} label="move" />
          <Hint k="Tab" label="complete" />
          <Hint k={"↵"} label="run" />
          <Hint k="Esc" label="close" />
          <span className="ml-auto">{MOD}1-9 quick run</span>
        </div>
      </div>
    </div>
  );
}

function Hint({ k, label }: { k: string; label: string }) {
  return (
    <span className="flex items-center gap-1.5">
      <kbd className="rounded px-1.5 py-px font-sans" style={{ background: "var(--chip)" }}>
        {k}
      </kbd>
      {label}
    </span>
  );
}
