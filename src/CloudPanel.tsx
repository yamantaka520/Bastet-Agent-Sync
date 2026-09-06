import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "./i18n";
import { wizardMessages } from "./wizard-i18n";
export type Wizard = {
  schema: number;
  session: string;
  mode: "guided" | "manual";
  page: number;
  clientId: string | null;
  clientSource: string | null;
  authorized: boolean;
  account?: {
    permissionId: string;
    displayName: string | null;
    emailAddress: string | null;
  } | null;
  folderId: string | null;
  folderName: string | null;
  binding: { folder: string; space: string; proof: string } | null;
  recoverySaved: boolean;
  proofVerified: boolean;
  complete: boolean;
};
export type WizardView = {
  wizard: Wizard;
  buildConfigured: boolean;
  connected: boolean;
  folders: { id: string; name: string }[];
};
export function nextStep(w: Wizard) {
  return !w.clientId
    ? 0
    : !w.authorized
      ? 1
      : !w.folderId
        ? 2
        : !w.proofVerified || !w.recoverySaved
          ? 3
          : 4;
}
export default function CloudPanel({
  native,
  locale,
  onChange,
}: {
  native: boolean;
  locale: Locale;
  onChange?: (view: WizardView | null) => void;
}) {
  const t = wizardMessages[locale];
  const [view, setView] = useState<WizardView | null>(null);
  useEffect(() => {
    onChange?.(view);
  }, [view, onChange]);
  const [credentialsReady, setCredentialsReady] = useState(false);
  const [busy, setBusy] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [cancelNote, setCancelNote] = useState(false);
  const [error, setError] = useState("");
  const [restart, setRestart] = useState(false);
  const [folderName, setFolderName] = useState("Bastet Agent Sync");
  const [folderId, setFolderId] = useState("");
  const [samplePassed, setSamplePassed] = useState(false);
  useEffect(() => {
    if (native)
      invoke<WizardView>("wizard_get")
        .then((v) => {
          if (v) {
            setView(v);
            setFolderId(v.wizard.folderId ?? "");
          }
        })
        .catch((e) => setError(String(e)));
  }, [native]);
  useEffect(() => {
    if (!native || busy) return;
    let active = true;
    const timer = setInterval(() => {
      void invoke<WizardView>("wizard_get")
        .then((next) => {
          if (active && next)
            setView((previous) => ({
              ...next,
              folders: previous?.folders ?? [],
            }));
        })
        .catch(() => {});
    }, 15000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, [native, busy]);
  async function run(work: () => Promise<WizardView | void>) {
    setBusy(true);
    setCredentialsReady(false);
    setError("");
    setSamplePassed(false);
    try {
      const updated = await work();
      if (updated) setView(updated);
    } catch (e) {
      setError(String(e));
      if (native) {
        try {
          const latest = await invoke<WizardView>("wizard_get");
          if (latest) setView(latest);
        } catch {
          /* Preserve original failure. */
        }
      }
    } finally {
      setBusy(false);
    }
  }
  async function execute(action: string, input = "") {
    setConnecting(action === "connect");
    setCancelNote(false);
    try {
      await run(() =>
        invoke<WizardView>("wizard_execute", { action, input, locale }),
      );
    } finally {
      setConnecting(false);
      setCancelNote(false);
    }
  }
  async function cancelLogin() {
    try {
      if (!(await invoke<boolean>("wizard_cancel_login"))) setCancelNote(true);
    } catch {
      setCancelNote(true);
    }
  }
  function navigate(mode: "guided" | "manual", page: number) {
    return run(() => invoke<WizardView>("wizard_navigate", { mode, page }));
  }
  const w = view?.wizard;
  const frontier = w ? nextStep(w) : 0;
  const completed = w
    ? [
        !!w.clientId,
        w.authorized,
        !!w.folderId,
        w.recoverySaved && w.proofVerified,
        w.complete,
      ]
    : [false, false, false, false, false];
  const unavailable = !native || busy || !w;
  const errorText = !error
    ? ""
    : /client_credentials_unavailable/.test(error)
      ? t.clientStoreError
      : /login_credentials_unavailable/.test(error)
        ? t.loginStoreError
        : /browser_open_failed/.test(error)
          ? t.browserError
          : /oauth_cancelled/.test(error)
            ? t.cancelled
            : /account_mismatch/.test(error)
              ? t.accountMismatch
              : /reauth|oauth_timeout|oauth_denied/.test(error)
                ? t.authError
                : /oauth|client/.test(error)
                  ? t.configError
                  : /restart_required/.test(error)
                    ? t.restartError
                    : /step_required|backup_required/.test(error)
                      ? t.stepError
                      : /drive_|folder/.test(error)
                        ? t.folderError
                        : /recovery|decrypt|space_key|proof/.test(error)
                          ? t.keyError
                          : /store|credential|wizard_corrupt|unsafe|immutable/.test(
                                error,
                              )
                            ? t.storageError
                            : t.error;
  return (
    <section className="panel cloud-panel wizard-panel" aria-busy={busy}>
      <div className="section-heading">
        <div>
          <h2>{t.title}</h2>
          <p>{t.intro}</p>
        </div>
      </div>
      <div className="wizard-note">
        <p>{t.credentialHint}</p>
        <button
          disabled={unavailable || !w?.clientId}
          onClick={() =>
            run(async () => {
              const result = await invoke<WizardView>("wizard_execute", {
                action: "unlock_credentials",
                input: "",
                locale,
              });
              setCredentialsReady(true);
              return result;
            })
          }
        >
          {t.unlockCredentials}
        </button>
        {credentialsReady && <p role="status">{t.credentialsReady}</p>}
      </div>
      {connecting && (
        <div role="status">
          <button onClick={cancelLogin}>{t.cancelLogin}</button>
          <p>{t.cancelHint}</p>
          {cancelNote && <p>{t.cancelUnavailable}</p>}
        </div>
      )}
      <div className="wizard-toolbar">
        <div role="group" aria-label={t.title}>
          <button
            aria-pressed={w?.mode === "guided"}
            disabled={unavailable}
            onClick={() => navigate("guided", w?.page ?? 0)}
          >
            {t.guided}
          </button>
          <button
            aria-pressed={w?.mode === "manual"}
            disabled={unavailable}
            onClick={() => navigate("manual", w?.page ?? 0)}
          >
            {t.manual}
          </button>
        </div>
        <button disabled={!native || busy} onClick={() => setRestart(true)}>
          {t.restart}
        </button>
      </div>
      {restart && (
        <div
          className="wizard-restart"
          role="group"
          aria-label={t.confirmRestart}
        >
          <p>{t.restartHint}</p>
          <button
            disabled={busy}
            onClick={() =>
              run(async () => {
                const result = await invoke<WizardView>("wizard_restart");
                setRestart(false);
                setFolderId("");
                return result;
              })
            }
          >
            {t.confirmRestart}
          </button>
          <button disabled={busy} onClick={() => setRestart(false)}>
            {t.cancel}
          </button>
        </div>
      )}
      {native && !w && !error && <p>{t.loading}</p>}
      {w && (
        <>
          <div className="wizard-save" role="status">
            {busy ? t.working : t.saved}
          </div>
          {w.account && (
            <p className="wizard-note">
              {view?.connected ? t.accountCurrent : t.accountSaved}:{" "}
              {w.account.emailAddress ||
                w.account.displayName ||
                w.account.permissionId}
            </p>
          )}
          {w.authorized && !view?.connected && (
            <p className="wizard-note">{t.connectionPending}</p>
          )}
          <progress
            aria-label={t.title}
            value={completed.filter(Boolean).length}
            max={5}
          />
          <ol className="wizard-steps">
            {t.steps.map((step, index) => (
              <li key={step}>
                <button
                  disabled={unavailable || index > frontier}
                  aria-current={
                    w.mode === "guided" && w.page === index ? "step" : undefined
                  }
                  onClick={() => navigate("guided", index)}
                >
                  <span aria-hidden="true">
                    {completed[index] ? "✓" : index + 1}
                  </span>
                  {step}
                  <small>{completed[index] ? t.done : t.pending}</small>
                </button>
              </li>
            ))}
          </ol>
          {w.mode === "guided" && w.page !== frontier && (
            <button
              disabled={busy}
              onClick={() => navigate("guided", frontier)}
            >
              {t.resume}
            </button>
          )}
          {t.steps.map(
            (step, index) =>
              (w.mode === "manual" || w.page === index) && (
                <div className="wizard-step" key={step}>
                  <h3>
                    {index + 1}. {step}
                  </h3>
                  {index === 0 && (
                    <>
                      <p>{t.clientHint}</p>
                      {w.clientId && (
                        <p>
                          {t.clientLabel}: <code>{w.clientId}</code>
                        </p>
                      )}
                      {!view?.buildConfigured && (
                        <div className="wizard-note">
                          <p>{t.noClient}</p>
                          <ol>
                            {t.helpSteps.map((s) => (
                              <li key={s}>{s}</li>
                            ))}
                          </ol>
                        </div>
                      )}
                      <div className="cloud-actions">
                        <button
                          disabled={unavailable || !view?.buildConfigured}
                          onClick={() => execute("use_build")}
                        >
                          {t.useBuild}
                        </button>
                        <button
                          disabled={unavailable}
                          onClick={() => execute("import_client")}
                        >
                          {t.importClient}
                        </button>
                        <button
                          disabled={unavailable}
                          onClick={() => execute("open_help")}
                        >
                          {t.help}
                        </button>
                      </div>
                    </>
                  )}
                  {index === 1 && (
                    <>
                      <p>{t.authHint}</p>
                      <button
                        disabled={unavailable || !w.clientId}
                        onClick={() => execute("connect")}
                      >
                        {w.authorized ? t.reconnect : t.connect}
                      </button>
                      {w.authorized && (
                        <>
                          <p>{t.forgetHint}</p>
                          <button
                            disabled={unavailable}
                            onClick={() => execute("forget_login")}
                          >
                            {t.forgetLogin}
                          </button>
                        </>
                      )}
                    </>
                  )}
                  {index === 2 && (
                    <>
                      <p>{t.folderHint}</p>
                      {w.folderId && (
                        <p className="wizard-selected">
                          ✓ {w.folderName} <code>{w.folderId}</code>
                        </p>
                      )}
                      <button
                        disabled={unavailable || !w.authorized}
                        onClick={() => execute("list_folders")}
                      >
                        {t.refreshFolders}
                      </button>
                      {view?.folders.length ? (
                        <ul className="wizard-folders">
                          {view.folders.map((f) => (
                            <li key={f.id}>
                              <button
                                disabled={unavailable || !!w.binding}
                                onClick={() => {
                                  setFolderId(f.id);
                                  execute("select_folder", f.id);
                                }}
                              >
                                {f.name} <small>{f.id}</small>
                              </button>
                            </li>
                          ))}
                        </ul>
                      ) : (
                        <p>{t.empty}</p>
                      )}
                      <label>
                        {t.folderId}
                        <input
                          value={folderId}
                          disabled={unavailable || !!w.binding}
                          onChange={(e) => setFolderId(e.target.value)}
                          placeholder="https://drive.google.com/drive/folders/…"
                        />
                      </label>
                      <button
                        disabled={
                          unavailable ||
                          !w.authorized ||
                          !folderId.trim() ||
                          !!w.binding
                        }
                        onClick={() => execute("select_folder", folderId)}
                      >
                        {t.selectFolder}
                      </button>
                      <label>
                        {t.folderName}
                        <input
                          value={folderName}
                          maxLength={128}
                          disabled={unavailable || !!w.binding}
                          onChange={(e) => setFolderName(e.target.value)}
                        />
                      </label>
                      <button
                        disabled={
                          unavailable ||
                          !w.authorized ||
                          !folderName.trim() ||
                          !!w.binding
                        }
                        onClick={() => execute("create_folder", folderName)}
                      >
                        {t.createFolder}
                      </button>
                    </>
                  )}
                  {index === 3 && (
                    <>
                      <p>{t.keyHint}</p>
                      <p className="wizard-note">{t.backupHint}</p>
                      <ul className="wizard-checks">
                        <li>
                          {w.binding ? "✓" : "○"} {t.keyPrepared}
                        </li>
                        <li>
                          {w.recoverySaved ? "✓" : "○"} {t.backupSaved}
                        </li>
                        <li>
                          {w.proofVerified ? "✓" : "○"} {t.proofDone}
                        </li>
                      </ul>
                      <div className="cloud-actions">
                        <button
                          disabled={unavailable || !w.folderId || !!w.binding}
                          onClick={() => execute("prepare_key")}
                        >
                          {t.prepareKey}
                        </button>
                        <button
                          disabled={unavailable || !w.binding}
                          onClick={() => execute("export_recovery")}
                        >
                          {t.exportRecovery}
                        </button>
                        <button
                          disabled={unavailable || !w.recoverySaved}
                          onClick={() => execute("publish_proof")}
                        >
                          {t.publishProof}
                        </button>
                        <button
                          disabled={unavailable || !w.folderId}
                          onClick={() => execute("import_recovery")}
                        >
                          {t.importRecovery}
                        </button>
                      </div>
                    </>
                  )}
                  {index === 4 && (
                    <>
                      <p>{t.reviewHint}</p>
                      <dl className="wizard-review">
                        <dt>{t.clientLabel}</dt>
                        <dd>{w.clientId ?? "—"}</dd>
                        <dt>{t.steps[2]}</dt>
                        <dd>
                          {w.folderName ?? "—"}
                          <br />
                          {w.folderId}
                        </dd>
                        <dt>{t.steps[3]}</dt>
                        <dd>{w.binding?.space ?? "—"}</dd>
                      </dl>
                      {w.complete && (
                        <p className="wizard-complete">✓ {t.complete}</p>
                      )}
                      <p>{t.completeHint}</p>
                      <button
                        disabled={
                          unavailable || !w.recoverySaved || !w.proofVerified
                        }
                        onClick={() => execute("finish")}
                      >
                        {t.finish}
                      </button>
                    </>
                  )}
                </div>
              ),
          )}
          {w.mode === "guided" && (
            <div className="wizard-navigation">
              <button
                disabled={unavailable || w.page === 0}
                onClick={() => navigate("guided", w.page - 1)}
              >
                {t.back}
              </button>
              <button
                disabled={unavailable || w.page >= frontier}
                onClick={() => navigate("guided", w.page + 1)}
              >
                {t.next}
              </button>
            </div>
          )}
        </>
      )}
      {error && <p role="alert">{errorText}</p>}
      <div className="wizard-diagnostic">
        <button
          disabled={!native || busy}
          onClick={() =>
            run(async () => {
              const r = await invoke<{
                verified: boolean;
                recoveryVerified: boolean;
                tamperRejected: boolean;
              }>("run_crypto_diagnostic");
              if (!r?.verified || !r.recoveryVerified || !r.tamperRejected)
                throw new Error("crypto_check_failed");
              setSamplePassed(true);
            })
          }
        >
          {t.diagnostic}
        </button>
        {samplePassed && <p>{t.diagnosticPassed}</p>}
      </div>
    </section>
  );
}
