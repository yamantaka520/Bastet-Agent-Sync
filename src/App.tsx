import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { detectLocale, languages, messages, type Locale } from "./i18n";
import { defaults, names, type Agent, type Settings } from "./model";
import cat from "../assets/calico.png";

export default function App() {
  const native = isTauri();
  const [settings, setSettings] = useState(() =>
    defaults(detectLocale(navigator.language)),
  );
  const [agents, setAgents] = useState<Agent[]>(
    Object.keys(names).map((id) => ({
      id,
      path: "",
      detected: false,
      custom: false,
    })),
  );
  const [tray, setTray] = useState(false);
  const [busy, setBusy] = useState(false);
  const [loaded, setLoaded] = useState(!native);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [dirty, setDirty] = useState(false);
  const [page, setPage] = useState<"setup" | "roadmap">("setup");
  const t = messages[settings.locale];
  useEffect(() => {
    document.documentElement.lang = settings.locale;
  }, [settings.locale]);
  useEffect(() => {
    if (!native) return;
    invoke<{
      settings: Settings | null;
      agents: Agent[];
      trayAvailable: boolean;
    }>("bootstrap")
      .then((b) => {
        if (b.settings) setSettings(b.settings);
        setAgents(b.agents);
        setTray(b.trayAvailable);
        setLoaded(true);
      })
      .catch((e) => setError(String(e)));
  }, [native]);
  function change<K extends keyof Settings>(key: K, value: Settings[K]) {
    setSettings((s) => ({ ...s, [key]: value }));
    setDirty(true);
    setNotice("");
    setError("");
  }
  async function action(work: () => Promise<void>) {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await work();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function scan(s = settings) {
    setAgents(await invoke<Agent[]>("scan_agents", { settings: s }));
  }
  async function choose(id?: string) {
    await action(async () => {
      const path = await invoke<string | null>("choose_folder");
      if (!path) return;
      if (id) {
        const next = {
          ...settings,
          customPaths: { ...settings.customPaths, [id]: path },
        };
        await scan(next);
        change("customPaths", next.customPaths);
      } else change("folder", path);
    });
  }
  const errorText = error ? (t[error as keyof typeof t] ?? t.error) : "";
  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <img src={cat} alt="" />
          <span>
            BASTET<span className="brand-sub">AGENT SYNC</span>
          </span>
        </div>
        <p className="tagline">{t.tagline}</p>
        <nav>
          <button
            className={page === "setup" ? "active" : ""}
            onClick={() => setPage("setup")}
          >
            <span>◈</span>
            {t.overview}
          </button>
          <button
            className={page === "roadmap" ? "active" : ""}
            onClick={() => setPage("roadmap")}
          >
            <span>↗</span>
            {t.roadmap}
          </button>
        </nav>
        <div className="sidebar-bottom">
          <div className="privacy">
            <span>◇</span>
            <strong>{t.privacy}</strong>
            <p>{t.privacyHint}</p>
          </div>
          <label className="lang">
            {t.language}
            <select
              disabled={busy}
              value={settings.locale}
              onChange={(e) => change("locale", e.target.value as Locale)}
            >
              {Object.entries(languages).map(([code, name]) => (
                <option key={code} value={code}>
                  {name}
                </option>
              ))}
            </select>
          </label>
          <small>0.1.0-dev</small>
        </div>
      </aside>
      <main>
        <header>
          <div>
            <span className="eyebrow">BASTET / AGENT SYNC</span>
            <h1>{page === "setup" ? t.setup : t.roadmap}</h1>
            <p>{t.intro}</p>
          </div>
          <img className="hero-cat" src={cat} alt="" />
        </header>
        <div className="status-strip">
          <span className="pill">
            <i />
            {t.foundation}
          </span>
          <span>
            {t.ready}{" "}
            <b>{settings.selectedAgents.length.toString().padStart(2, "0")}</b>
          </span>
          <span>
            {t.transport} <b>{t.notConnected}</b>
          </span>
        </div>
        {!native && <p className="notice">{t.preview}</p>}
        {error && (
          <p role="alert" className="error">
            {errorText}
          </p>
        )}
        {notice && (
          <p role="status" className="notice">
            {t.saved}
          </p>
        )}
        {page === "roadmap" ? (
          <section className="panel">
            <h2>{t.roadmap}</h2>
            <p className="roadmap">{t.roadmapText}</p>
            <p>{t.pending}</p>
          </section>
        ) : (
          <>
            <section className="panel">
              <div className="section-heading">
                <div>
                  <h2>
                    <span className="step">01</span>
                    {t.source}
                  </h2>
                  <p>{t.sourceHint}</p>
                </div>
                <button
                  disabled={!native || busy || !loaded}
                  onClick={() => action(() => scan())}
                >
                  {t.scan}
                </button>
              </div>
              <div className="selection">
                <button
                  disabled={!native || busy || !loaded}
                  onClick={() =>
                    change(
                      "selectedAgents",
                      agents.filter((a) => a.detected).map((a) => a.id),
                    )
                  }
                >
                  {t.all}
                </button>
                <button
                  disabled={busy}
                  onClick={() => change("selectedAgents", [])}
                >
                  {t.none}
                </button>
              </div>
              <div className="agent-grid">
                {agents.map((a) => (
                  <div
                    key={a.id}
                    className={`agent-card ${settings.selectedAgents.includes(a.id) ? "selected" : ""}`}
                  >
                    <label className="agent-title">
                      <span className={`agent-mark ${a.id}`}>
                        {names[a.id][0]}
                      </span>
                      <strong>{names[a.id]}</strong>
                      <input
                        type="checkbox"
                        aria-label={names[a.id]}
                        disabled={!native || !a.detected || busy || !loaded}
                        checked={settings.selectedAgents.includes(a.id)}
                        onChange={(e) =>
                          change(
                            "selectedAgents",
                            e.target.checked
                              ? [...settings.selectedAgents, a.id]
                              : settings.selectedAgents.filter(
                                  (id) => id !== a.id,
                                ),
                          )
                        }
                      />
                    </label>
                    <div className="agent-status">
                      <span className={a.detected ? "dot found" : "dot"} />
                      {a.detected ? t.found : t.missing}
                    </div>
                    <code title={a.path}>{a.path || "—"}</code>
                    <div className="path-actions">
                      <button
                        disabled={!native || busy || !loaded}
                        onClick={() => choose(a.id)}
                      >
                        {t.custom}
                      </button>
                      {a.custom && (
                        <button
                          disabled={busy}
                          onClick={() =>
                            action(async () => {
                              const customPaths = { ...settings.customPaths };
                              delete customPaths[a.id];
                              await scan({ ...settings, customPaths });
                              change("customPaths", customPaths);
                            })
                          }
                        >
                          {t.defaults}
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </section>
            <section className="panel">
              <h2>
                <span className="step">02</span>
                {t.destination}
              </h2>
              <p>{t.destinationHint}</p>
              <label className="folder-label">{t.folder}</label>
              <div className="folder-row">
                <div className="folder-value">
                  ▱ <span>{settings.folder || t.noFolder}</span>
                </div>
                <button
                  disabled={!native || busy || !loaded}
                  onClick={() => choose()}
                >
                  {t.browse}
                </button>
              </div>
              {settings.folder && <p className="muted">{t.localOnly}</p>}
            </section>
            <section className="panel">
              <h2>
                <span className="step">03</span>
                {t.preferences}
              </h2>
              <div className="fields">
                <label>
                  {t.device}
                  <input
                    maxLength={80}
                    value={settings.deviceName}
                    onChange={(e) => change("deviceName", e.target.value)}
                    placeholder={t.device}
                    disabled={busy}
                  />
                </label>
                <label>
                  {t.direction}
                  <select
                    value={settings.direction}
                    onChange={(e) =>
                      change(
                        "direction",
                        e.target.value as Settings["direction"],
                      )
                    }
                    disabled={busy}
                  >
                    {(["bidirectional", "upload", "download"] as const).map(
                      (v) => (
                        <option key={v} value={v}>
                          {t[v]}
                        </option>
                      ),
                    )}
                  </select>
                </label>
                <label>
                  {t.schedule}
                  <select
                    value={settings.schedule}
                    onChange={(e) =>
                      change("schedule", e.target.value as Settings["schedule"])
                    }
                    disabled={busy}
                  >
                    {(["near-realtime", "interval", "manual"] as const).map(
                      (v) => (
                        <option key={v} value={v}>
                          {t[v]}
                        </option>
                      ),
                    )}
                  </select>
                </label>
                {settings.schedule === "interval" && (
                  <label>
                    {t.seconds}
                    <input
                      type="number"
                      min={15}
                      max={86400}
                      value={settings.intervalSeconds}
                      onChange={(e) =>
                        change("intervalSeconds", Number(e.target.value))
                      }
                      disabled={busy}
                    />
                  </label>
                )}
              </div>
              <label className="tray-option">
                <input
                  type="checkbox"
                  checked={settings.closeToTray}
                  disabled={!native || !tray || busy}
                  onChange={(e) => change("closeToTray", e.target.checked)}
                />
                {t.tray}
              </label>
              {native && !tray && <p className="muted">{t.trayUnavailable}</p>}
            </section>
            <footer>
              <div>
                <strong>{dirty ? t.dirty : t.foundation}</strong>
                <p id="sync-reason">{t.pending}</p>
              </div>
              <div className="footer-actions">
                <button
                  disabled={!native || busy || !loaded}
                  onClick={() =>
                    action(async () => {
                      await invoke("save_settings", { settings });
                      setDirty(false);
                      setNotice("saved");
                    })
                  }
                >
                  {busy ? t.busy : t.save}
                </button>
                <button
                  className="primary"
                  disabled
                  aria-describedby="sync-reason"
                >
                  {t.start} <span>→</span>
                </button>
              </div>
            </footer>
          </>
        )}
      </main>
    </div>
  );
}
