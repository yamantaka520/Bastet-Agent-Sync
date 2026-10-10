import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "./i18n";
import { names, type Settings } from "./model";
import { ops } from "./operations-i18n";
import { portableErrorText, portableReadinessText, isBlockingPortableCode, type PortableReadiness } from "./portable-errors";
import ReviewPanel from "./ReviewPanel";
export type PortableOptions = {
  settings: boolean;
  skills: boolean;
  excludedPaths?: Record<string, string[]>;
};
type Package = {
  agent: string;
  files: Record<string, string>;
  excluded: Record<string, string>;
};
type Received = {
  agent: string;
  id: string;
  files: number;
  savedAt?: number | null;
  // Actual readiness from the backend. Absent on an older backend, in which
  // case the panel falls back to its learn-on-compare behaviour.
  readiness?: PortableReadiness | null;
};
export default function PortablePanel({
  native,
  settings,
  locale,
  value = { settings: false, skills: false },
  disabled,
  onChange,
}: {
  native: boolean;
  settings: Settings;
  locale: Locale;
  value?: PortableOptions;
  disabled: boolean;
  onChange: (v: PortableOptions) => void;
}) {
  const t = ops[locale];
  const [preview, setPreview] = useState<Package[] | null>(null);
  const [items, setItems] = useState<Received[]>([]);
  const [restored, setRestored] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  // Per received item, the last observed multipart transport code. Once a
  // transfer is seen as incomplete/tampered/etc. the item is shown as not ready
  // for the rest of this session — the honest state derived from the backend's
  // own readiness and codes.
  const [pending, setPending] = useState<Record<string, string>>({});
  const note = (id: string, code: string) =>
    setPending((previous) => ({ ...previous, [id]: code }));
  async function act(f: () => Promise<void>) {
    setBusy(true);
    setError("");
    try {
      await f();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function restore(item: Received) {
    setBusy(true);
    try {
      setRestored(
        await invoke("portable_restore", { agent: item.agent, id: item.id }),
      );
    } catch (e) {
      note(item.id, String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="panel portable-panel">
      <h2>📦 {t.portable}</h2>
      <p>{t.portableHint}</p>
      <fieldset disabled={!native || disabled || busy}>
        <label>
          <input
            type="checkbox"
            checked={value.settings}
            onChange={(e) => onChange({ ...value, settings: e.target.checked })}
          />
          {t.portableSettings}
        </label>
        <label>
          <input
            type="checkbox"
            checked={value.skills}
            onChange={(e) => onChange({ ...value, skills: e.target.checked })}
          />
          {t.portableSkills}
        </label>
      </fieldset>
      <div className="inline-actions">
        <button
          disabled={!native || disabled || busy}
          onClick={() =>
            void act(async () =>
              setPreview(await invoke("portable_preview", { settings })),
            )
          }
        >
          {t.preview}
        </button>
        <button
          disabled={!native || disabled || busy}
          onClick={() =>
            void act(async () => setItems(await invoke("portable_list")))
          }
        >
          {t.incoming}
        </button>
      </div>
      {error && (
        <p role="alert">{portableErrorText(error, locale) ?? t.error}</p>
      )}
      {restored && (
        <p>
          {t.restored}: <code>{restored}</code>
        </p>
      )}
      {preview &&
        preview.map((p) => (
          <details key={p.agent}>
            <summary>
              {names[p.agent] ?? p.agent} · {Object.keys(p.files).length}
            </summary>
            {!Object.keys(p.files).length && <p>{t.empty}</p>}
            {Object.entries(p.files).map(([path, text]) => (
              <details key={path}>
                <summary>
                  <input
                    type="checkbox"
                    aria-label={path}
                    disabled={disabled || busy}
                    checked={!value.excludedPaths?.[p.agent]?.includes(path)}
                    onChange={(e) => {
                      const paths = (
                        value.excludedPaths?.[p.agent] ?? []
                      ).filter((v) => v !== path);
                      if (!e.target.checked) paths.push(path);
                      onChange({
                        ...value,
                        excludedPaths: {
                          ...value.excludedPaths,
                          [p.agent]: paths,
                        },
                      });
                    }}
                  />
                  {path}
                </summary>
                <pre>{text.slice(0, 32768)}</pre>
              </details>
            ))}
            {Object.entries(p.excluded).map(([path, reason]) => (
              <p key={path}>
                ⚠️ {path} · <code>{reason}</code>
              </p>
            ))}
          </details>
        ))}
      {items.map((item) => {
        const readiness = item.readiness;
        // A truthful, backend-derived badge: error when the item could not be
        // read at all, "complete" when every part is present (integrity is still
        // verified by compare/restore — never labelled verified here), pending
        // (with the backend's exact counts) otherwise. No badge when an older
        // backend returned no readiness — the item is not mislabelled.
        const state = readiness
          ? readiness.issue
            ? "error"
            : readiness.complete
              ? "complete"
              : "pending"
          : null;
        // Gate compare/restore while the transfer is not locally complete, or
        // once the backend has reported a blocking issue (missing/tampered/…).
        // A complete-but-not-yet-verified transfer is allowed through so compare
        // (and restore) can perform the byte-level verification.
        const gated =
          (readiness ? !readiness.complete : false) ||
          isBlockingPortableCode(pending[item.id], locale);
        return (
          <details key={item.id}>
            <summary>
              {names[item.agent] ?? item.agent} · {item.files} ·{" "}
              {item.savedAt
                ? new Date(item.savedAt * 1000).toLocaleString(locale)
                : "—"}
            </summary>
            {readiness && state && (
              <p
                className={`portable-badge portable-${state}`}
                role={state === "error" ? "alert" : "status"}
              >
                {portableReadinessText(readiness, locale)}
              </p>
            )}
            {pending[item.id] && portableErrorText(pending[item.id], locale) && (
              <p className="portable-pending" role="alert">
                {portableErrorText(pending[item.id], locale)}
              </p>
            )}
            <ReviewPanel
              locale={locale}
              agent={item.agent}
              id={item.id}
              portable
              disabled={disabled || busy || gated}
              onError={(code) => note(item.id, code)}
              onRestore={() => void restore(item)}
            />
          </details>
        );
      })}
    </section>
  );
}
