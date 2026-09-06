import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "./i18n";
import { names } from "./model";
import { ops } from "./operations-i18n";
import { formatBytes } from "./TrafficStatus";
import type { SyncStatus } from "./WorkerStatus";
import type { SourceStatus } from "./NativeSessions";
export type Resources = {
  parallel: number;
  uploadKib: number;
  downloadKib: number;
  windowEnabled: boolean;
  startMinute: number;
  endMinute: number;
};
export const resourceDefaults: Resources = {
  parallel: 3,
  uploadKib: 0,
  downloadKib: 0,
  windowEnabled: false,
  startMinute: 0,
  endMinute: 0,
};
export function ResourceControls({
  locale,
  value = resourceDefaults,
  disabled,
  onChange,
}: {
  locale: Locale;
  value?: Resources;
  disabled: boolean;
  onChange: (v: Resources) => void;
}) {
  const t = ops[locale];
  const time = (n: number) =>
    `${String(Math.floor(n / 60)).padStart(2, "0")}:${String(n % 60).padStart(2, "0")}`;
  const minute = (s: string) => {
    const [h, m] = s.split(":").map(Number);
    return h * 60 + m;
  };
  return (
    <fieldset className="panel resource-controls" disabled={disabled}>
      <legend>{t.resources}</legend>
      <label>
        {t.parallel}
        <input
          type="number"
          min={1}
          max={6}
          value={value.parallel}
          onChange={(e) =>
            onChange({ ...value, parallel: Number(e.target.value) })
          }
        />
      </label>
      <label>
        {t.upload}
        <input
          type="number"
          min={0}
          max={1048576}
          value={value.uploadKib}
          onChange={(e) =>
            onChange({ ...value, uploadKib: Number(e.target.value) })
          }
        />
      </label>
      <label>
        {t.download}
        <input
          type="number"
          min={0}
          max={1048576}
          value={value.downloadKib}
          onChange={(e) =>
            onChange({ ...value, downloadKib: Number(e.target.value) })
          }
        />
      </label>
      <p>{t.unlimited}</p>
      <label>
        <input
          type="checkbox"
          checked={value.windowEnabled}
          onChange={(e) =>
            onChange({ ...value, windowEnabled: e.target.checked })
          }
        />
        {t.window}
      </label>
      <label>
        {t.from}
        <input
          type="time"
          value={time(value.startMinute)}
          onChange={(e) =>
            e.target.value &&
            onChange({ ...value, startMinute: minute(e.target.value) })
          }
        />
      </label>
      <label>
        {t.to}
        <input
          type="time"
          value={time(value.endMinute)}
          onChange={(e) =>
            e.target.value &&
            onChange({ ...value, endMinute: minute(e.target.value) })
          }
        />
      </label>
      <p>{t.hours}</p>
    </fieldset>
  );
}
export type Progress = {
  stage: string;
  completed: number;
  total?: number | null;
  bytesDone: number;
  bytesTotal?: number | null;
  etaSeconds?: number | null;
};
export function SourceProgress({
  locale,
  progress: p,
}: {
  locale: Locale;
  progress?: Progress | null;
}) {
  if (!p) return null;
  const t = ops[locale];
  const stage =
    (
      {
        scan: t.scan,
        list: t.list,
        download: t.downloading,
        upload: t.uploading,
        restore: t.restore,
      } as Record<string, string>
    )[p.stage] ?? t.checking;
  return (
    <div className="source-progress">
      <strong>{stage}</strong>
      <p>
        {t.progress}: {p.completed}
        {p.total != null ? ` / ${p.total}` : ""}
      </p>
      {p.total != null && p.total > 0 && (
        <progress
          aria-label={t.progress}
          max={p.total}
          value={Math.min(p.completed, p.total)}
        />
      )}
      {(p.bytesDone > 0 || p.bytesTotal != null) && (
        <p>
          {t.payload}: {formatBytes(p.bytesDone, locale)}
          {p.bytesTotal != null
            ? ` / ${formatBytes(p.bytesTotal, locale)}`
            : ""}
        </p>
      )}
      {p.etaSeconds != null && <p>≈ {p.etaSeconds}s</p>}
      {p.bytesTotal != null && p.bytesTotal > 0 && (
        <progress
          aria-label={t.payload}
          max={p.bytesTotal}
          value={Math.min(p.bytesDone, p.bytesTotal)}
        />
      )}
    </div>
  );
}
type History = {
  started: number;
  finished: number;
  outcome: string;
  error?: string | null;
  sources: SourceStatus[];
};
type Device = {
  id: string;
  name: string;
  os: string;
  version: string;
  reportedAt: number;
  observedAt: number;
  outcome: string;
  agents: string[];
};
export default function OperationsPanel({
  native,
  locale,
  runtime,
}: {
  native: boolean;
  locale: Locale;
  runtime: SyncStatus | null;
}) {
  const t = ops[locale];
  const [data, setData] = useState<{ history: History[]; devices: Device[] }>({
    history: [],
    devices: [],
  });
  const [usage, setUsage] = useState<{
    localBytes: number;
    cacheBytes: number;
  } | null>(null);
  const [cloud, setCloud] = useState<{
    bytes: number;
    objects: number;
    measuredAt: number;
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const date = (n: number) => new Date(n * 1000).toLocaleString(locale);
  const state = (s: string) =>
    ({
      complete: t.complete,
      partial: t.partial,
      paused: t.paused,
      error: t.failed,
    })[s] ?? s;
  async function act(fn: () => Promise<void>) {
    setBusy(true);
    setError(false);
    try {
      await fn();
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    if (!native) return;
    let live = true;
    invoke<typeof data>("operations_view")
      .then((v) => {
        if (live && v && Array.isArray(v.history) && Array.isArray(v.devices))
          setData(v);
      })
      .catch(() => {
        if (live) setError(true);
      });
    return () => {
      live = false;
    };
  }, [native, runtime?.phase]);
  return (
    <section className="panel operations-panel">
      <h2>🐈 {t.title}</h2>
      {native && runtime?.running && (
        <div className="inline-actions">
          <button
            disabled={busy}
            onClick={() =>
              void act(async () => {
                await invoke("sync_pause_for", { seconds: 900 });
              })
            }
          >
            {t.pause}
          </button>
          <button
            disabled={busy}
            onClick={() =>
              void act(async () => {
                await invoke("sync_now");
              })
            }
          >
            {t.resume}
          </button>
        </div>
      )}
      {runtime?.resumeAt && (
        <p>
          {t.resumeAt}: {date(runtime.resumeAt)}
        </p>
      )}
      {(error || runtime?.observerError) && <p role="alert">{t.error}</p>}
      <details>
        <summary>{t.history}</summary>
        <button
          disabled={!native || busy}
          onClick={() =>
            void act(async () => setData(await invoke("operations_view")))
          }
        >
          {t.refresh}
        </button>
        {!data.history.length && <p>{t.empty}</p>}
        {[...data.history].reverse().map((h, i) => (
          <details key={`${h.started}-${i}`}>
            <summary>
              {date(h.started)} · {state(h.outcome)} ·{" "}
              {Math.max(0, h.finished - h.started)}s
            </summary>
            {h.sources.map((s) => (
              <p key={s.agent}>
                {names[s.agent] ?? s.agent}: ↑ {s.published} / ↓ {s.received} /
                ✓ {s.restored}
              </p>
            ))}
            {h.error && <code>{h.error}</code>}
          </details>
        ))}
      </details>
      <details>
        <summary>{t.devices}</summary>
        {!data.devices.length && <p>{t.empty}</p>}
        {data.devices.map((d) => (
          <article key={d.id}>
            <h3>{d.name || d.id}</h3>
            <p>
              {d.os} · {t.version} {d.version} · {state(d.outcome)}
            </p>
            <p>
              {t.reported}: {date(d.reportedAt)} / {t.at}: {date(d.observedAt)}
            </p>
            <p>{d.agents.map((a) => names[a] ?? a).join(", ")}</p>
          </article>
        ))}
      </details>
      <details>
        <summary>{t.storage}</summary>
        <div className="inline-actions">
          <button
            disabled={!native || busy}
            onClick={() =>
              void act(async () => setUsage(await invoke("storage_usage")))
            }
          >
            {t.measure}
          </button>
          <button
            disabled={!native || busy || runtime?.running}
            onClick={() =>
              void act(async () =>
                setCloud(await invoke("cloud_storage_usage")),
              )
            }
          >
            {t.cloudMeasure}
          </button>
          <button
            disabled={!native || busy || runtime?.running}
            onClick={() =>
              void act(async () => {
                await invoke("clear_download_cache");
                setUsage(await invoke("storage_usage"));
              })
            }
          >
            {t.clean}
          </button>
        </div>
        <p>{t.cleanHint}</p>
        {usage && (
          <p>
            {t.local}: {formatBytes(usage.localBytes, locale)} · {t.cache}:{" "}
            {formatBytes(usage.cacheBytes, locale)}
          </p>
        )}
        {cloud && (
          <p>
            {t.cloud}: {formatBytes(cloud.bytes, locale)} ({cloud.objects}) ·{" "}
            {date(cloud.measuredAt)}
          </p>
        )}
      </details>
    </section>
  );
}
