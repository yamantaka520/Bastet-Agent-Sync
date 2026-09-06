import { useState } from "react";
import type { Locale } from "./i18n";
import { ops } from "./operations-i18n";
export function grokCommand(
  path: string,
  session: string,
  powershell: boolean,
) {
  if (!/^[A-Za-z0-9_-]+$/.test(session) || /[\r\n\0]/.test(path)) return null;
  const quoted = powershell
    ? `'${path.replaceAll("'", "''")}'`
    : `'${path.replaceAll("'", `'"'"'`)}'`;
  return powershell
    ? `& { $previousGrokHome = $env:GROK_HOME; try { $env:GROK_HOME = ${quoted}; grok --resume '${session}' } finally { $env:GROK_HOME = $previousGrokHome } }`
    : `GROK_HOME=${quoted} grok --resume '${session}'`;
}
export default function Continuation({
  locale,
  path,
  session,
}: {
  locale: Locale;
  path: string;
  session: string;
}) {
  const t = ops[locale];
  const [powershell, setPowershell] = useState(
    navigator.platform.startsWith("Win"),
  );
  const [failed, setFailed] = useState(false);
  const command = grokCommand(path, session, powershell);
  if (!command) return null;
  return (
    <details className="continuation">
      <summary>{t.continuation}</summary>
      <p>{t.cliHint}</p>
      <select
        aria-label="Shell"
        value={powershell ? "powershell" : "posix"}
        onChange={(e) => setPowershell(e.target.value === "powershell")}
      >
        <option value="posix">macOS / Linux (sh, bash, zsh)</option>
        <option value="powershell">Windows (PowerShell)</option>
      </select>
      <pre>{command}</pre>
      <button
        onClick={() => {
          void navigator.clipboard.writeText(command).then(
            () => setFailed(false),
            () => setFailed(true),
          );
        }}
      >
        {t.copyCommand}
      </button>
      {failed && <p role="alert">{t.error}</p>}
    </details>
  );
}
