import { describe, expect, it } from "vitest";
// @ts-expect-error plain JS module
import { artifactCandidates, buildManifest } from "./release-json.mjs";

describe("buildManifest", () => {
  const manifest = buildManifest({
    version: "0.2.0",
    notes: "Fixes",
    pubDate: "2026-01-01T00:00:00.000Z",
    baseUrl: "https://example.com/dl/v0.2.0/",
    artifacts: {
      "darwin-aarch64": { file: "SplitShadow_0.2.0_universal.app.tar.gz", signature: "SIG-MAC\n" },
      "darwin-x86_64": { file: "SplitShadow_0.2.0_universal.app.tar.gz", signature: "SIG-MAC\n" },
      "windows-x86_64": { file: "SplitShadow 0.2.0 setup.exe", signature: "SIG-WIN" },
    },
  });

  it("uses Tauri's static manifest shape", () => {
    expect(manifest.version).toBe("0.2.0");
    expect(Object.keys(manifest.platforms)).toEqual(["darwin-aarch64", "darwin-x86_64", "windows-x86_64"]);
  });

  it("trims signatures and joins URLs without double slashes", () => {
    expect(manifest.platforms["darwin-aarch64"].signature).toBe("SIG-MAC");
    expect(manifest.platforms["darwin-aarch64"].url).toBe("https://example.com/dl/v0.2.0/SplitShadow_0.2.0_universal.app.tar.gz");
  });

  it("percent-encodes file names", () => {
    expect(manifest.platforms["windows-x86_64"].url).toBe("https://example.com/dl/v0.2.0/SplitShadow%200.2.0%20setup.exe");
  });
});

describe("artifactCandidates", () => {
  it("prefers the universal macOS build and maps it to both architectures", () => {
    const [first] = artifactCandidates("1.0.0").mac;
    expect(first.keys).toEqual(["darwin-aarch64", "darwin-x86_64"]);
    expect(first.name).toContain("1.0.0");
  });
});
