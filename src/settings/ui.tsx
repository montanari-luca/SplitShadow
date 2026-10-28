import type { ReactNode } from "react";

export const isMac = navigator.platform.toLowerCase().includes("mac");

export function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mb-8">
      <h2 className="mb-3 text-[13px] font-semibold uppercase tracking-wide" style={{ color: "var(--muted)" }}>
        {title}
      </h2>
      <div className="overflow-hidden rounded-xl border" style={{ borderColor: "var(--border)", background: "var(--row-active)" }}>
        {children}
      </div>
    </section>
  );
}

export function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="flex min-h-14 items-center justify-between gap-6 border-b px-4 py-3 last:border-b-0" style={{ borderColor: "var(--border)" }}>
      <div className="min-w-0">
        <div className="text-[14px]">{label}</div>
        {hint && (
          <div className="mt-0.5 text-[12.5px]" style={{ color: "var(--muted)" }}>
            {hint}
          </div>
        )}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

export function Button({ children, onClick, kind = "default", disabled }: { children: ReactNode; onClick?: () => void; kind?: "default" | "primary" | "danger"; disabled?: boolean }) {
  const bg = kind === "primary" ? "var(--glyph-fill)" : kind === "danger" ? "rgba(255,69,58,0.18)" : "var(--chip)";
  const color = kind === "primary" ? "#07130a" : kind === "danger" ? "#ff6b61" : "var(--text)";
  return (
    <button
      disabled={disabled}
      onClick={onClick}
      className="rounded-lg px-3.5 py-1.5 text-[13px] font-medium disabled:opacity-40"
      style={{ background: bg, color }}
    >
      {children}
    </button>
  );
}

export function Toggle({ on, onChange }: { on: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      role="switch"
      aria-checked={on}
      onClick={() => onChange(!on)}
      className="relative h-[22px] w-10 rounded-full transition-colors"
      style={{ background: on ? "var(--glyph-fill)" : "var(--chip)" }}
    >
      <span className="absolute top-0.5 h-[18px] w-[18px] rounded-full bg-white shadow transition-all" style={{ left: on ? 20 : 2 }} />
    </button>
  );
}

export function Kbd({ parts }: { parts: string[] }) {
  return (
    <span className="inline-flex gap-1">
      {parts.map((p, i) => (
        <kbd key={i} className="min-w-6 rounded-md px-2 py-1 text-center font-sans text-[13px]" style={{ background: "var(--chip)" }}>
          {p}
        </kbd>
      ))}
    </span>
  );
}
