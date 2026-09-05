import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runtimeMessages } from "./runtime-i18n";
import type { Locale } from "./i18n";
type Status = {
  phase: string;
  version: string | null;
  downloaded: number;
  total: number | null;
};
export default function UpdatePanel({
  native,
  locale,
  dirty,
}: {
  native: boolean;
  locale: Locale;
  dirty: boolean;
}) {
  const t = runtimeMessages[locale];
  const [status, setStatus] = useState<Status>({
    phase: "",
    version: null,
    downloaded: 0,
    total: null,
  });
  const [busy, setBusy] = useState(false);
  async function run(command: string) {
    setBusy(true);
    setStatus((s) => ({
      ...s,
      phase: command === "check_update" ? "checking" : "installing",
    }));
    try {
      setStatus(await invoke<Status>(command));
    } catch {
      setStatus((s) => ({ ...s, phase: "failed" }));
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    if (status.phase !== "installing" || !busy) return;
    let active = true;
    const timer = setInterval(() => {
      void invoke<Status>("update_status")
        .then((s) => {
          if (active) setStatus(s);
        })
        .catch(() => {});
    }, 1000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, [status.phase, busy]);
  const labels: Record<string, string> = {
    checking: t[14],
    installing: t[16],
    installed: t[17],
    current: t[19],
    unpublished: t[20],
    unsupported: t[21],
    failed: t[22],
  };
  return (
    <div className="update-panel">
      <strong>{t[12]}</strong>
      <button
        disabled={!native || busy || status.phase === "installed"}
        onClick={() => run("check_update")}
      >
        {t[13]}
      </button>
      <p role="status">
        {labels[status.phase]}
        {status.phase === "available" && `v${status.version}`}
      </p>
      {status.phase === "installing" && (
        <p>
          {Math.round(status.downloaded / 1024)} KiB
          {status.total ? ` / ${Math.round(status.total / 1024)} KiB` : ""}
        </p>
      )}
      {status.phase === "available" && (
        <>
          <button
            disabled={busy || dirty}
            onClick={() => run("install_update")}
          >
            {t[15]} v{status.version}
          </button>
          {dirty && <p>{t[23]}</p>}
        </>
      )}
      {status.phase === "installed" && (
        <button
          disabled={dirty}
          onClick={() => {
            void invoke("restart_after_update").catch(() =>
              setStatus((s) => ({ ...s, phase: "failed" })),
            );
          }}
        >
          {t[18]}
        </button>
      )}
    </div>
  );
}
