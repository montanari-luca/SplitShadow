import type { Glyph, IconName } from "../matcher/types";

const ICONS: Record<IconName, string[]> = {
  minimize: ["M5 14h10"],
  larger: ["M12 4h4v4", "M8 16H4v-4", "M16 4l-5 5", "M4 16l5-5"],
  smaller: ["M16 8h-4V4", "M4 12h4v4", "M12 8l5-5", "M8 12l-5 5"],
  display: ["M3 4.5h14v9H3z", "M7 17h6", "M10 13.5V17"],
  restore: ["M4 10a6 6 0 1 0 2-4.5", "M4 4v3h3"],
  split: ["M3 4.5h14v11H3z", "M10 4.5v11"],
  unsplit: ["M3 4.5h14v11H3z", "M10 4.5v4", "M10 11.5v4", "M4 17L16 3"],
  workspace: ["M3 3h6v6H3z", "M11 3h6v6h-6z", "M3 11h6v6H3z", "M11 11h6v6h-6z"],
  save: ["M5 3h8l3 3v11H5z", "M8 3v4h5", "M8 17v-5h5v5"],
  update: ["M4 10a6 6 0 0 1 10-4.5", "M16 4v3h-3", "M16 10a6 6 0 0 1-10 4.5", "M4 16v-3h3"],
  delete: ["M4.5 6h11", "M8 6V4h4v2", "M6 6l.8 10h6.4L14 6"],
  settings: ["M10 7.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5z", "M10 3v2", "M10 15v2", "M3 10h2", "M15 10h2", "M5 5l1.5 1.5", "M13.5 13.5L15 15", "M15 5l-1.5 1.5", "M6.5 13.5L5 15"],
  quit: ["M10 3v6", "M6 6a6 6 0 1 0 8 0"],
  download: ["M10 3v9", "M6 9l4 4 4-4", "M4 16h12"],
  window: ["M3 4.5h14v11H3z", "M3 7.5h14"],
};

export function GlyphView({ glyph }: { glyph: Glyph }) {
  if ("rect" in glyph) {
    const [x, y, w, h] = glyph.rect;
    return (
      <svg width="30" height="22" viewBox="0 0 30 22" aria-hidden>
        <rect x="0.5" y="0.5" width="29" height="21" rx="3.5" fill="none" stroke="var(--glyph)" />
        <rect x={2.5 + x * 25} y={2.5 + y * 17} width={w * 25} height={h * 17} rx="1.8" fill="var(--glyph-fill)" />
      </svg>
    );
  }
  return (
    <svg width="22" height="22" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden style={{ opacity: 0.8 }}>
      {ICONS[glyph.icon].map((d, i) => (
        <path key={i} d={d} />
      ))}
    </svg>
  );
}
