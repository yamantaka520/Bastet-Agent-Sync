import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "./i18n";
import type { Settings } from "./model";
import { folderMessages } from "./folder-i18n";

export type FolderProvider = Exclude<Settings["cloudProvider"], "google-drive">;
export type FolderStatus = {
  provider: FolderProvider;
  path: string | null;
  space: string | null;
  complete: boolean;
  handoff: "local-folder";
};
function folderErrorText(error: string, t: (typeof folderMessages)[Locale]) {
  if (/folder_pending/.test(error)) return t.pendingDownload;
  if (/recovery_inside_sync_folder/.test(error)) return t.recoveryInside;
  if (/folder_has_sync_objects|folder_object_exists/.test(error))
    return t.occupied;
  if (
    /recovery_wrong_provider|invalid_recovery_kit|space_mismatch|invalid_space_proof|decrypt_failed|wizard_restart_required/.test(
      error,
    )
  )
    return t.kitMismatch;
  if (/sync_running|sync_busy/.test(error)) return t.locked;
  if (
    /folder_unavailable|folder_object_missing|sandbox_reauthorize/.test(error)
  )
    return t.unavailable;
  if (
    /store_unavailable|credential|recovery_export_failed|unsafe_store|folder_setup_invalid/.test(
      error,
    )
  )
    return t.storage;
  return t.error;
}

export default function FolderPanel({
  native,
  locale,
  provider,
  disabled = false,
  onChange,
}: {
  native: boolean;
  locale: Locale;
  provider: FolderProvider;
  disabled?: boolean;
  onChange?: (status: FolderStatus | null) => void;
}) {
  const t = folderMessages[locale];
  const [status, setStatus] = useState<FolderStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    setStatus(null);
    setError("");
    onChange?.(null);
    if (!native) return;
    let active = true;
    void invoke<FolderStatus>("folder_status", { provider })
      .then((result) => {
        if (active) setStatus(result);
      })
      .catch((reason) => {
        if (active) setError(String(reason));
      });
    return () => {
      active = false;
    };
  }, [native, provider, onChange]);
  useEffect(() => {
    onChange?.(status);
  }, [status, onChange]);
  async function run(
    command:
      | "folder_pick"
      | "folder_prepare"
      | "folder_join"
      | "folder_export_recovery",
  ) {
    setBusy(true);
    setError("");
    try {
      await invoke<FolderStatus | boolean | null>(command, { provider });
    } catch (reason) {
      setError(String(reason));
    } finally {
      // The returned action result is not proof that a saved space is still present.
      // This also refreshes pending state after a cancelled native dialog.
      try {
        setStatus(await invoke<FolderStatus>("folder_status", { provider }));
      } catch (reason) {
        setStatus(null);
        setError((previous) => previous || String(reason));
      }
      setBusy(false);
    }
  }
  return (
    <section className="panel folder-panel" aria-busy={busy}>
      <h2 id="folder-panel-title" tabIndex={-1}>
        {t.title}
      </h2>
      <p>{t.intro}</p>
      <p className="wizard-note">{t.recovery}</p>
      <dl className="wizard-review">
        <dt>{t.selected}</dt>
        <dd className="folder-path">{status?.path ?? t.noFolder}</dd>
        <dt>{t.space}</dt>
        <dd>{status?.space ?? "—"}</dd>
      </dl>
      <div className="cloud-actions">
        <button
          disabled={!native || disabled || busy}
          onClick={() => run("folder_pick")}
        >
          {t.pick}
        </button>
        <button
          disabled={
            !native || disabled || busy || !status?.path || status.complete
          }
          onClick={() => run("folder_prepare")}
        >
          {t.prepare}
        </button>
        <button
          disabled={
            !native || disabled || busy || !status?.path || status.complete
          }
          onClick={() => run("folder_join")}
        >
          {t.join}
        </button>
        <button
          disabled={!native || disabled || busy || !status?.complete}
          onClick={() => run("folder_export_recovery")}
        >
          {t.export}
        </button>
      </div>
      <p role="status">
        {busy ? t.working : status?.complete ? t.complete : t.pending}
      </p>
      <p className="muted">{t.handoff}</p>
      {error && <p role="alert">{folderErrorText(error, t)}</p>}
    </section>
  );
}
