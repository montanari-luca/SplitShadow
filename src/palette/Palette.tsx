import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export function Palette() {
  const [query, setQuery] = useState("");
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const un = listen("palette://show", () => {
      setQuery("");
      input.current?.focus();
    });
    return () => void un.then((f) => f());
  }, []);

  return (
    <div className="h-full w-full p-1" onMouseDown={(e) => e.target === e.currentTarget && invoke("palette_hide")}>
      <div className="pop overflow-hidden rounded-[14px] border" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
        <input
          ref={input}
          autoFocus
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Escape" && invoke("palette_hide")}
          placeholder="Move, resize, split, or open a workspace..."
          className="h-16 w-full bg-transparent px-5 text-[19px] outline-none placeholder:opacity-50"
        />
      </div>
    </div>
  );
}
