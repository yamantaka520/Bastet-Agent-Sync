import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "./i18n";
import { ops } from "./operations-i18n";
import { formatBytes } from "./TrafficStatus";
import { portableErrorText } from "./portable-errors";
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
  onError,
}: {
  locale: Locale;
  agent: string;
  id: string;
  portable?: boolean;
  disabled: boolean;
  onRestore?: () => void;
  onError?: (code: string) => void;
}) {
  const t = ops[locale];
  const [sourceAgent, setSourceAgent] = useState(agent);
  const [comparison, setComparison] = useState<Comparison | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function compare() {
    setBusy(true);
    setError("");
    try {
      setComparison(
        await invoke(
          portable ? "portable_compare" : "compare_received_session",
          { agent, id, ...(!portable ? { sourceAgent } : {}) },
        ),
      );
    } catch (e) {
      const code = String(e);
      setError(code);
      onError?.(code);
    } finally {
      setBusy(false);
    }
  }
  async function mark() {
    if (!comparison) return;
    setBusy(true);
    setError("");
    try {
      await invoke("review_received_session", {
        agent,
        id,
        fingerprint: comparison.fingerprint,
        sourceAgent,
      });
      setComparison({ ...comparison, reviewed: true });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  // A multipart transport code gets its own actionable, localized message (an
  // incomplete transfer reads as waiting for its missing parts, never ready);
  // any other failure keeps the shared generic text. When a parent owns the
  // per-item state (onError), it renders the message and this stays silent so
  // the same alert is never duplicated.
  const errorText =
    (portable ? portableErrorText(error, locale) : null) ?? t.error;
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
      {error && !onError && <p role="alert">{errorText}</p>}
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
