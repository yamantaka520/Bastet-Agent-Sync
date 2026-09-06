import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "./i18n";
import { ops } from "./operations-i18n";
import { formatBytes } from "./TrafficStatus";
type FileDiff = {
  path: string;
  state: string;
  localBytes: number | null;
  incomingBytes: number;
  localHash: string | null;
  incomingHash: string;
  localText: string | null;
  incomingText: string | null;
  truncated: boolean;
};
type Comparison = { files: FileDiff[]; fingerprint: string; reviewed: boolean };
export default function ReviewPanel({
  locale,
  agent,
  id,
  portable = false,
  disabled,
  onRestore,
}: {
  locale: Locale;
  agent: string;
  id: string;
  portable?: boolean;
  disabled: boolean;
  onRestore?: () => void;
}) {
  const t = ops[locale];
  const [sourceAgent, setSourceAgent] = useState(agent);
  const [comparison, setComparison] = useState<Comparison | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  async function compare() {
    setBusy(true);
    setError(false);
    try {
      setComparison(
        await invoke(
          portable ? "portable_compare" : "compare_received_session",
          { agent, id, ...(!portable ? { sourceAgent } : {}) },
        ),
      );
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }
  async function mark() {
    if (!comparison) return;
    setBusy(true);
    setError(false);
    try {
      await invoke("review_received_session", {
        agent,
        id,
        fingerprint: comparison.fingerprint,
        sourceAgent,
      });
      setComparison({ ...comparison, reviewed: true });
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="review-panel">
      {!portable && agent === "codex" && (
        <label>
          {t.localVersion}
          <select
            disabled={disabled || busy}
            value={sourceAgent}
            onChange={(e) => {
              setSourceAgent(e.target.value);
              setComparison(null);
            }}
          >
            <option value="codex">Codex</option>
            <option value="chatgpt-work">ChatGPT Work</option>
          </select>
        </label>
      )}
      <button disabled={disabled || busy} onClick={() => void compare()}>
        {t.compare}
      </button>
      {error && <p role="alert">{t.error}</p>}
      {comparison && (
        <div>
          <p>{comparison.reviewed ? t.reviewed : t.compare}</p>
          {comparison.files.map((f) => (
            <details key={f.path}>
              <summary>
                {f.path} ·{" "}
                {(
                  {
                    same: t.same,
                    missing: t.missing,
                    different: t.different,
                  } as Record<string, string>
                )[f.state] ?? f.state}
              </summary>
              <p>
                {t.localVersion}:{" "}
                {f.localBytes == null ? "—" : formatBytes(f.localBytes, locale)}{" "}
                · {t.remoteVersion}: {formatBytes(f.incomingBytes, locale)}
              </p>
              <div className="diff-columns">
                <div>
                  <h5>{t.localVersion}</h5>
                  {f.localText != null ? (
                    <pre>
                      {f.localText}
                      {f.truncated ? "\n…" : ""}
                    </pre>
                  ) : (
                    <code>{f.localHash ?? "—"}</code>
                  )}
                </div>
                <div>
                  <h5>{t.remoteVersion}</h5>
                  {f.incomingText != null ? (
                    <pre>
                      {f.incomingText}
                      {f.truncated ? "\n…" : ""}
                    </pre>
                  ) : (
                    <code>{f.incomingHash}</code>
                  )}
                </div>
              </div>
            </details>
          ))}
          <div className="inline-actions">
            {!portable && (
              <button
                disabled={disabled || busy || comparison.reviewed}
                onClick={() => void mark()}
              >
                {t.acknowledge}
              </button>
            )}
            {onRestore && (
              <button disabled={disabled || busy} onClick={onRestore}>
                {t.keepBoth}
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
