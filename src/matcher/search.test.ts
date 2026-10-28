import { describe, expect, it } from "vitest";
import { search } from "./search";
import { emptyContext, type SearchContext } from "./types";

const ctx: SearchContext = {
  ...emptyContext,
  targetId: 1,
  windows: [
    { id: 1, app: "Notes", title: "Shopping" },
    { id: 2, app: "Slack", title: "general | MintFlow" },
    { id: 3, app: "Safari", title: "Docs" },
  ],
  workspaces: ["Dev", "Writing"],
  activeWorkspace: "Dev",
};

const top = (q: string, c = ctx) => search(q, c)[0];
const topKey = (q: string, c = ctx) => top(q, c)?.key;

describe("layout phrases", () => {
  it.each([
    ["left", "left-half"],
    ["right", "right-half"],
    ["top", "top-half"],
    ["bottom", "bottom-half"],
    ["move window to the left of the screen", "left-half"],
    ["Move window to the right of the screen", "right-half"],
    ["put this on the left half", "left-half"],
    ["snap lft", "left-half"],
    ["lef", "left-half"],
    ["send it to the top", "top-half"],
    ["top left", "top-left"],
    ["upper right corner", "top-right"],
    ["bottom left quarter", "bottom-left"],
    ["lower right", "bottom-right"],
    ["two thirds on the left", "left-two-thirds"],
    ["2/3 right", "right-two-thirds"],
    ["66% left", "left-two-thirds"],
    ["left third", "left-third"],
    ["1/3 center", "center-third"],
    ["middle third", "center-third"],
    ["center", "center"],
    ["Center window", "center"],
    ["centre this window", "center"],
    ["almost maximize", "almost-maximize"],
    ["almost maximize window", "almost-maximize"],
    ["90%", "almost-maximize"],
    ["max", "maximize"],
    ["fullscreen", "maximize"],
    ["maximize", "maximize"],
    ["hide", "minimize"],
    ["minimize this", "minimize"],
    ["bigger", "larger"],
    ["make smaller", "smaller"],
    ["undo", "restore"],
    ["next monitor", "next-display"],
    ["move to the other display", "next-display"],
  ])("%s -> %s", (q, key) => {
    expect(topKey(q)).toBe(key);
  });

  it("offers alternatives for an ambiguous direction", () => {
    const keys = search("left", ctx).map((r) => r.key);
    expect(keys[0]).toBe("left-half");
    expect(keys).toContain("left-third");
    expect(keys).toContain("left-two-thirds");
  });

  it("gives nothing for gibberish", () => {
    expect(search("zzqxv", { ...emptyContext }).length).toBe(0);
  });
});

describe("empty query", () => {
  it("lists featured actions and workspaces", () => {
    const keys = search("", ctx).map((r) => r.key);
    expect(keys).toContain("left-half");
    expect(keys.length).toBeLessThanOrEqual(8);
  });

  it("puts an available update first", () => {
    expect(topKey("", { ...ctx, update: "1.2.0" })).toBe("install-update");
  });

  it("shows unlink first when linked", () => {
    expect(topKey("", { ...ctx, linked: true })).toBe("unsplit");
  });
});

describe("workspaces", () => {
  it("save with a name", () => {
    const r = top("save workspace as Client Review");
    expect(r.command).toEqual({ type: "ws-save", name: "Client Review" });
  });

  it("save without a name asks for one", () => {
    expect(top("save workspace").command).toEqual({ type: "complete", text: "save workspace as " });
  });

  it("flags overwriting an existing name", () => {
    expect(top("save workspace as dev").subtitle).toMatch(/Replaces/);
  });

  it("open by name", () => {
    expect(top("open writing").command).toEqual({ type: "ws-open", name: "Writing" });
  });

  it("open with fuzzy name and noise words", () => {
    expect(top("open my dev workspace").command).toEqual({ type: "ws-open", name: "Dev" });
  });

  it("a bare workspace name opens it", () => {
    expect(top("writ").command).toEqual({ type: "ws-open", name: "Writing" });
  });

  it("listing workspaces", () => {
    const rows = search("workspaces", ctx);
    expect(rows.map((r) => r.command)).toEqual([
      { type: "ws-open", name: "Dev" },
      { type: "ws-open", name: "Writing" },
    ]);
  });

  it("update with no name targets the active workspace", () => {
    const rows = search("update workspace", ctx);
    expect(rows[0].command).toEqual({ type: "ws-update" });
    expect(rows[0].title).toContain("Dev");
    expect(rows.some((r) => r.command.type === "ws-update" && r.command.name === "Writing")).toBe(true);
  });

  it("update with a name targets that workspace", () => {
    expect(top("update writing").command).toEqual({ type: "ws-update", name: "Writing" });
  });

  it("update with no workspaces points to saving", () => {
    const r = top("update workspace", { ...ctx, workspaces: [], activeWorkspace: null });
    expect(r.command.type).toBe("complete");
  });

  it("delete requires confirmation", () => {
    const r = top("delete workspace writing");
    expect(r.command).toEqual({ type: "ws-delete", name: "Writing" });
    expect(r.confirm).toBe(true);
  });
});

describe("split", () => {
  it("lists partner windows without the target", () => {
    const rows = search("split", ctx);
    expect(rows.map((r) => r.command)).toEqual([
      { type: "split", windowId: 2 },
      { type: "split", windowId: 3 },
    ]);
  });

  it("filters by app name", () => {
    expect(top("split with slack").command).toEqual({ type: "split", windowId: 2 });
  });

  it("filters by window title", () => {
    expect(top("split mintflow").command).toEqual({ type: "split", windowId: 2 });
  });

  it("a bare app name offers a split", () => {
    expect(top("safari").command).toEqual({ type: "split", windowId: 3 });
  });

  it("unsplit", () => {
    expect(topKey("unsplit", { ...ctx, linked: true })).toBe("unsplit");
  });
});

describe("app actions", () => {
  it("settings", () => expect(topKey("settings")).toBe("settings"));
  it("quit", () => expect(topKey("quit")).toBe("quit"));
  it("check for updates", () => expect(topKey("check for updates")).toBe("check-update"));
  it("update with an update ready offers install", () => {
    const keys = search("update", { ...ctx, update: "2.0.0" }).map((r) => r.key);
    expect(keys).toContain("install-update");
  });
});
