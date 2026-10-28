import type { Suggestion } from "../matcher/types";

/** Text that Tab should put in the input for a suggestion. */
export function completionFor(s: Suggestion): string {
  const c = s.command;
  switch (c.type) {
    case "ws-open":
      return `open workspace ${c.name}`;
    case "ws-update":
      return c.name ? `update workspace ${c.name}` : "update workspace";
    case "ws-delete":
      return `delete workspace ${c.name}`;
    case "ws-save":
      return `save workspace as ${c.name}`;
    case "split":
      return `split with ${s.title.replace(/^Split with /, "")}`;
    case "complete":
      return c.text;
    default:
      return s.title.toLowerCase();
  }
}
