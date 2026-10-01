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
export function resumeCommand(
  agent: string,
  path: string,
  session: string,
  cwd: string,
  powershell: boolean,
) {
  if (
    !/^[A-Za-z0-9_-]+$/.test(session) ||
    !path ||
    !cwd ||
    /[\r\n\0]/.test(path + cwd)
  )
    return null;
  const agents: Record<string, [string, string[]]> = {
    codex: ["CODEX_HOME", ["codex", "resume", session]],
    "chatgpt-work": ["CODEX_HOME", ["codex", "resume", session]],
    claude: ["CLAUDE_CONFIG_DIR", ["claude", "--resume", session]],
    "claude-code": ["CLAUDE_CONFIG_DIR", ["claude", "--resume", session]],
    pi: ["PI_CODING_AGENT_DIR", ["pi", "--session", session]],
    grok: ["GROK_HOME", ["grok", "--resume", session]],
  };
  const spec = agents[agent];
  if (!spec) return null;
  const quote = (value: string) =>
    powershell
      ? `'${value.replaceAll("'", "''")}'`
      : `'${value.replaceAll("'", `'"'"'`)}'`;
  const [variable, args] = spec;
  const command = [args[0], ...args.slice(1).map(quote)].join(" ");
  return powershell
    ? `& { $bastetPreviousPath = Get-Location; $bastetPreviousHome = $env:${variable}; try { Set-Location -LiteralPath ${quote(cwd)} -ErrorAction Stop; $env:${variable} = ${quote(path)}; ${command} } finally { Set-Location -LiteralPath $bastetPreviousPath; $env:${variable} = $bastetPreviousHome } }`
    : `(cd -- ${quote(cwd)} && ${variable}=${quote(path)} ${command})`;
}
export default function Continuation({
  locale,
  path,
  session,
  agent = "grok",
  cwd,
}: {
  locale: Locale;
  path: string;
  session: string;
  agent?: string;
  cwd?: string;
}) {
  const t = ops[locale];
  const [powershell, setPowershell] = useState(
    navigator.platform.startsWith("Win"),
  );
  const [failed, setFailed] = useState(false);
  const command = cwd
    ? resumeCommand(agent, path, session, cwd, powershell)
    : agent === "grok"
      ? grokCommand(path, session, powershell)
      : null;
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
