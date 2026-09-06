import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "./i18n";
import { names, type Settings } from "./model";
import { ops } from "./operations-i18n";
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
  const [error, setError] = useState(false);
  async function act(f: () => Promise<void>) {
    setBusy(true);
    setError(false);
    try {
      await f();
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }
  async function restore(item: Received) {
    await act(async () =>
      setRestored(
        await invoke("portable_restore", { agent: item.agent, id: item.id }),
      ),
    );
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
      {error && <p role="alert">{t.error}</p>}
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
      {items.map((item) => (
        <details key={item.id}>
          <summary>
            {names[item.agent] ?? item.agent} · {item.files} ·{" "}
            {item.savedAt
              ? new Date(item.savedAt * 1000).toLocaleString(locale)
              : "—"}
          </summary>
          <ReviewPanel
            locale={locale}
            agent={item.agent}
            id={item.id}
            portable
            disabled={disabled || busy}
            onRestore={() => void restore(item)}
          />
        </details>
      ))}
    </section>
  );
}
