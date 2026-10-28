import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Button, Row, Section } from "./ui";

interface Status {
  current: string;
  available: boolean;
  version: string | null;
  notes: string | null;
  error: string | null;
}

export function Updates() {
  const [status, setStatus] = useState<Status | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<{ downloaded: number; total: number | null } | null>(null);
  const [installError, setInstallError] = useState<string | null>(null);

  const check = useCallback(async () => {
    setBusy(true);
    try {
      setStatus(await invoke<Status>("check_for_updates"));
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void check();
    const un = listen<{ downloaded: number; total: number | null }>("updater://progress", (e) => setProgress(e.payload));
    return () => void un.then((f) => f());
  }, [check]);

  const install = async () => {
    setInstallError(null);
    setProgress({ downloaded: 0, total: null });
    try {
      await invoke("install_update");
    } catch (e) {
      setProgress(null);
      setInstallError(String(e));
    }
  };

  const pct = progress?.total ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100)) : null;

  return (
    <Section title="Updates">
      <Row label={`Version ${status?.current ?? ""}`} hint={status?.error ? status.error : status?.available ? `Version ${status.version} is available.` : busy ? "Checking..." : "You are up to date."}>
        {status?.available ? (
          <Button kind="primary" onClick={() => void install()} disabled={!!progress}>
            {progress ? (pct !== null ? `Downloading ${pct}%` : "Downloading...") : "Install and restart"}
          </Button>
        ) : (
          <Button onClick={() => void check()} disabled={busy}>
            Check now
          </Button>
        )}
      </Row>
      {status?.available && status.notes && (
        <div className="whitespace-pre-wrap px-4 py-3 text-[13px]" style={{ color: "var(--muted)" }}>
          {status.notes}
        </div>
      )}
      {installError && (
        <div className="px-4 py-3 text-[13px]" style={{ background: "rgba(255,69,58,0.15)", color: "#ff6b61" }}>
          {installError}
        </div>
      )}
    </Section>
  );
}
