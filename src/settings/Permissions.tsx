import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button, Row, Section } from "./ui";

export function useAccessibility() {
  const [trusted, setTrusted] = useState<boolean | null>(null);
  useEffect(() => {
    const read = () => void invoke<boolean>("accessibility_status").then(setTrusted);
    read();
    // The user grants it in System Settings, so poll while this window is open.
    const t = setInterval(read, 1500);
    return () => clearInterval(t);
  }, []);
  return trusted;
}

export function Permissions() {
  const trusted = useAccessibility();
  return (
    <Section title="Permissions">
      <Row
        label="Accessibility"
        hint="Required on macOS so SplitShadow can move and resize other apps' windows. Enable SplitShadow in System Settings, Privacy & Security, Accessibility."
      >
        {trusted ? (
          <span className="text-[13px]" style={{ color: "var(--glyph-fill)" }}>
            Granted
          </span>
        ) : (
          <Button kind="primary" onClick={() => void invoke("request_accessibility")}>
            Grant access
          </Button>
        )}
      </Row>
    </Section>
  );
}
