import { ops } from "./operations-i18n";
import Continuation from "./Continuation";
import ReviewPanel from "./ReviewPanel";
import { SourceProgress, type Progress } from "./OperationsPanel";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "./i18n";
import { names } from "./model";
import { syncDisplay, issueText, savedTime } from "./sync-display";
import { projectError } from "./project-errors";
export const sessionMessages = {
  "zh-Hant": [
    "本機對話同步",
    "同步已勾選來源的原生對話版本。專案路徑可用時，接收版本會準備為獨立資料目錄；接續後的變更會再同步，各分支分別保留。帳號、金鑰、專案檔案和雲端託管聊天不包含在內。Claude 使用本機 Claude Code 對話；Codex 與 Work 共用本機對話來源。Agy 提供資料庫還原。",
    "查看對話快照",
    "恢復到新資料夾",
    "請先暫停同步，再查看或恢復。",
    "尚無快照",
    "已恢復到",
    "完成",
    "沒有本機對話",
    "部分完成",
    "失敗",
    "讀取對話",
    "可接收版本",
    "逐個來源結果",
    "請在原 Agent 指定下列資料目錄，再用對話 ID 繼續；專案路徑不同時，先切換到這台電腦的專案目錄。此動作不會複製登入資訊。",
    "重新授權接續資料夾",
    "若先前還原的對話失去存取權，請選取當初用來還原的原始上層資料夾，然後再查看快照。",
  ],
  "zh-Hans": [
    "本地对话同步",
    "同步所选来源的原生对话版本。项目路径可用时，接收版本会准备为独立数据目录；接续后的更改会再次同步，各分支分别保留。不包含账号、密钥、项目文件和云端托管聊天。Claude 使用本地 Claude Code 对话；Codex 与 Work 共用本地对话来源。Agy 提供数据库恢复。",
    "查看对话快照",
    "恢复到新文件夹",
    "请先暂停同步，再查看或恢复。",
    "暂无快照",
    "已恢复到",
    "完成",
    "没有本地对话",
    "部分完成",
    "失败",
    "读取对话",
    "可接收版本",
    "各来源结果",
    "请在原 Agent 指定下列数据目录，再用对话 ID 继续；项目路径不同时，先切换到此电脑的项目目录。此操作不复制登录信息。",
    "重新授权接续文件夹",
    "如果先前恢复的对话失去访问权限，请选择当初用于恢复的原始上级文件夹，然后重新查看快照。",
  ],
  en: [
    "Local conversation sync",
    "Sync native conversation versions from selected sources. Received versions are prepared in separate profiles when their project paths are available. Continued edits sync back and branches stay separate. Accounts, keys, project files and cloud-hosted chats are excluded. Claude uses local Claude Code conversations; Codex and Work share local storage. Agy provides database recovery.",
    "View conversation snapshots",
    "Restore to a new folder",
    "Pause sync before viewing or restoring.",
    "No snapshots yet",
    "Restored to",
    "Complete",
    "No local conversations",
    "Partially complete",
    "Failed",
    "Conversations read",
    "Received versions",
    "Results by source",
    "Set the original agent’s data directory below and resume by session ID. Switch to this computer’s project directory when paths differ. Login credentials are not copied.",
    "Reauthorize handoff folder",
    "If a restored conversation loses access, select the same parent folder you originally chose for that restore, then view snapshots again.",
  ],
  ja: [
    "ローカル会話の同期",
    "選択したソースの会話の版を同期します。プロジェクトパスが利用可能な場合、受信した版を個別のプロファイルに準備します。再開後の変更も同期し、分岐は個別に保持します。アカウント、キー、プロジェクトファイル、クラウド会話は対象外です。Claude はローカル Claude Code、Codex と Work は共通のローカル会話を使用します。Agy はデータベース復元に対応します。",
    "会話スナップショットを表示",
    "新規フォルダーに復元",
    "表示・復元前に同期を一時停止してください。",
    "スナップショットなし",
    "復元先",
    "完了",
    "ローカル会話なし",
    "一部完了",
    "失敗",
    "読み取った会話",
    "受信した版",
    "ソース別の結果",
    "元の Agent に以下のデータディレクトリを指定し、会話 ID で再開します。パスが異なる場合は、この端末のプロジェクトに移動してください。ログイン情報はコピーされません。",
    "引き継ぎフォルダーを再認証",
    "復元した会話へのアクセスが失われた場合は、復元時に選んだ元の親フォルダーを選択し、スナップショットを再表示してください。",
  ],
  ko: [
    "로컬 대화 동기화",
    "선택한 소스의 대화 버전을 동기화합니다. 프로젝트 경로를 사용할 수 있으면 수신 버전을 별도 프로필로 준비합니다. 계속한 변경 사항도 동기화하고 분기는 별도로 보존합니다. 계정, 키, 프로젝트 파일 및 클라우드 대화는 제외합니다. Claude는 로컬 Claude Code 대화를, Codex와 Work는 같은 로컬 저장소를 사용합니다. Agy는 데이터베이스 복원을 제공합니다.",
    "대화 스냅샷 보기",
    "새 폴더에 복원",
    "조회 또는 복원 전에 동기화를 일시 중지하세요.",
    "스냅샷 없음",
    "복원 위치",
    "완료",
    "로컬 대화 없음",
    "일부 완료",
    "실패",
    "읽은 대화",
    "수신 버전",
    "소스별 결과",
    "원래 Agent에 아래 데이터 디렉터리를 지정하고 대화 ID로 재개하세요. 경로가 다르면 이 컴퓨터의 프로젝트로 이동하세요. 로그인 정보는 복사하지 않습니다.",
    "인계 폴더 다시 승인",
    "복원된 대화에 접근할 수 없다면 복원할 때 선택했던 원래 상위 폴더를 다시 선택한 후 스냅샷을 확인하세요.",
  ],
} as const;
export type SourceStatus = {
  progress?: Progress | null;
  agent: string;
  state: string;
  captured: number;
  available: number;
  published: number;
  received: number;
  restored: number;
  issues: Record<string, number>;
};
type Snapshot = {
  id: string;
  agent: string;
  session: string;
  cwd: string;
  localSavedAt?: number | null;
  parentIds?: string[];
  originDevice?: string;
  generation?: number;
  branchCount?: number;
  managedProfile?: string | null;
  mappedCwd?: string | null;
};
const handoffText = {
  "zh-Hant": [
    "版本",
    "分支",
    "本機專案",
    "接續已恢復的版本",
    "在此資料目錄繼續的變更，會在重新啟動同步後傳送。各分支分別保留，不合併歷史。",
  ],
  "zh-Hans": [
    "版本",
    "分支",
    "本机项目",
    "继续已恢复的版本",
    "在此数据目录继续的更改，会在重新启动同步后发送。各分支分别保留，不合并历史。",
  ],
  en: [
    "Version",
    "Branches",
    "Local project",
    "Continue restored version",
    "Changes made in this profile are sent after you restart sync. Branches stay separate; histories are not concatenated.",
  ],
  ja: [
    "バージョン",
    "分岐",
    "ローカルプロジェクト",
    "復元済みの版を再開",
    "このプロファイルでの変更は同期再開後に送信されます。分岐は個別に保持し、履歴は連結しません。",
  ],
  ko: [
    "버전",
    "분기",
    "로컬 프로젝트",
    "복원된 버전 계속하기",
    "이 프로필에서 변경한 내용은 동기화를 다시 시작하면 전송됩니다. 분기는 별도로 유지하며 기록을 이어 붙이지 않습니다.",
  ],
} as const;
const env: Record<string, string> = {
  codex: "CODEX_HOME",
  "chatgpt-work": "CODEX_HOME",
  claude: "CLAUDE_CONFIG_DIR",
  "claude-code": "CLAUDE_CONFIG_DIR",
  pi: "PI_CODING_AGENT_DIR",
  grok: "GROK_HOME",
  agy: "",
};
export default function NativeSessions({
  native,
  locale,
  running,
  sources,
  storeChannel = false,
}: {
  native: boolean;
  locale: Locale;
  running: boolean;
  sources?: SourceStatus[];
  storeChannel?: boolean;
}) {
  const t = sessionMessages[locale];
  const d = syncDisplay[locale];
  const h = handoffText[locale];
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});

  const [items, setItems] = useState<Snapshot[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [restored, setRestored] = useState<{
    path: string;
    item: Snapshot;
  } | null>(null);
  const action = async (f: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await f();
    } catch (e) {
      setError(projectError(String(e), locale) ?? String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="native-sessions">
      <h3>🐈 {t[0]}</h3>
      <p>{t[1]}</p>
      {!!sources?.length && (
        <div className="source-cards">
          {sources.map((s) => {
            const state =
              ["syncing", "queued"].includes(s.state) && !running
                ? "paused"
                : s.state;
            const labels: Record<string, [string, string]> = {
              syncing: ["🔄", d[0]],
              queued: ["🕒", d[29]],
              complete: ["✅", d[1]],
              partial: ["⚠️", d[3]],
              error: ["❌", d[4]],
              empty: ["📭", d[5]],
              paused: ["⏸️", d[6]],
            };
            const [icon, label] = labels[state] ?? ["🕒", d[7]];
            return (
              <article className="source-card" key={s.agent}>
                <h4>{names[s.agent] ?? s.agent}</h4>
                <p className="source-state">
                  {icon} {label}
                </p>
                {state === "syncing" && (
                  <SourceProgress locale={locale} progress={s.progress} />
                )}
                {state === "complete" &&
                  !s.published &&
                  !s.received &&
                  !s.restored && <p>{d[2]}</p>}
                {state !== "syncing" && state !== "queued" && (
                  <dl className="sync-counts">
                    <div>
                      <dt>↑ {d[8]}</dt>
                      <dd>{s.published}</dd>
                    </div>
                    <div>
                      <dt>↓ {d[9]}</dt>
                      <dd>{s.received}</dd>
                    </div>
                    <div>
                      <dt>{d[10]}</dt>
                      <dd>{s.restored ?? 0}</dd>
                    </div>
                  </dl>
                )}
                {Array.from(
                  new Set(
                    Object.keys(s.issues).map((code) =>
                      issueText(code, locale),
                    ),
                  ),
                ).map((message) => (
                  <p className="source-issue" key={message}>
                    {message}
                  </p>
                ))}
                <details>
                  <summary>{d[11]}</summary>
                  <p>
                    {t[11]}: {s.captured} · {t[12]}: {s.available}
                  </p>
                  {Object.entries(s.issues).map(([code, count]) => (
                    <p key={code}>
                      <code>{code}</code> × {count}
                    </p>
                  ))}
                </details>
              </article>
            );
          })}
        </div>
      )}
      <button
        disabled={!native || running || busy}
        onClick={() =>
          void action(async () =>
            setItems(await invoke<Snapshot[]>("list_received_sessions")),
          )
        }
      >
        {t[2]}
      </button>
      {storeChannel && native && (
        <div>
          <button
            disabled={running || busy}
            onClick={() =>
              void action(async () => {
                const selected = await invoke<string | null>("choose_folder");
                if (selected)
                  setItems(await invoke<Snapshot[]>("list_received_sessions"));
              })
            }
          >
            {t[15]}
          </button>
          <p>{t[16]}</p>
        </div>
      )}
      {running && <p>{t[4]}</p>}
      {items && (
        <div className="snapshot-groups">
          {!items.length ? (
            <p>{t[5]}</p>
          ) : (
            <>
              <p>{d[28]}</p>
              <div className="cloud-actions">
                <button
                  onClick={() =>
                    setExpanded(
                      Object.fromEntries(
                        items.map((item) => [item.agent, true]),
                      ),
                    )
                  }
                >
                  {d[15]}
                </button>
                <button onClick={() => setExpanded({})}>{d[16]}</button>
              </div>
              {Array.from(new Set(items.map((item) => item.agent))).map(
                (agent) => {
                  const group = items
                    .filter((item) => item.agent === agent)
                    .sort(
                      (a, b) =>
                        (b.localSavedAt ?? -1) - (a.localSavedAt ?? -1) ||
                        a.id.localeCompare(b.id),
                    );
                  return (
                    <section className="snapshot-group" key={agent}>
                      <h4>
                        <button
                          className="snapshot-toggle"
                          aria-expanded={!!expanded[agent]}
                          aria-controls={"snapshots-" + agent}
                          onClick={() =>
                            setExpanded((previous) => ({
                              ...previous,
                              [agent]: !previous[agent],
                            }))
                          }
                        >
                          <span aria-hidden="true">
                            {expanded[agent] ? "▾" : "▸"}
                          </span>{" "}
                          {names[agent] ?? agent} · {group.length} {d[14]}
                        </button>
                      </h4>
                      {expanded[agent] && (
                        <ul id={"snapshots-" + agent}>
                          {group.map((item) => (
                            <li key={item.id}>
                              <strong>
                                {item.cwd.split(/[\/]/).filter(Boolean).pop() ||
                                  item.session}
                              </strong>
                              <p className="snapshot-time">
                                🕒 {d[12]}:{" "}
                                {savedTime(item.localSavedAt, locale)}
                              </p>
                              {item.generation !== undefined && (
                                <p>
                                  {h[0]}: {item.generation + 1} · {h[1]}:{" "}
                                  {item.branchCount ?? 1}
                                </p>
                              )}
                              {item.mappedCwd && (
                                <p>
                                  {h[2]}: <code>{item.mappedCwd}</code>
                                </p>
                              )}
                              {item.managedProfile && (
                                <button
                                  disabled={running || busy}
                                  onClick={() =>
                                    setRestored({
                                      path: item.managedProfile!,
                                      item,
                                    })
                                  }
                                >
                                  {h[3]}
                                </button>
                              )}
                              <details>
                                <summary>{d[11]}</summary>
                                <p>
                                  Session ID: <code>{item.session}</code>
                                </p>
                                <p>
                                  <code>{item.cwd}</code>
                                </p>
                                <p>
                                  <code>{item.id}</code>
                                </p>
                                {item.originDevice && (
                                  <p>
                                    Device: <code>{item.originDevice}</code>
                                  </p>
                                )}
                                {item.parentIds?.map((parent) => (
                                  <p key={parent}>
                                    Parent: <code>{parent}</code>
                                  </p>
                                ))}
                              </details>
                              <ReviewPanel
                                locale={locale}
                                agent={item.agent}
                                id={item.id}
                                disabled={running || busy}
                                onRestore={() =>
                                  void action(async () => {
                                    const path = await invoke<string | null>(
                                      "restore_received_session",
                                      { agent: item.agent, id: item.id },
                                    );
                                    if (path) setRestored({ path, item });
                                  })
                                }
                              />
                              <button
                                disabled={running || busy}
                                onClick={() =>
                                  void action(async () => {
                                    const path = await invoke<string | null>(
                                      "restore_received_session",
                                      { agent: item.agent, id: item.id },
                                    );
                                    if (path) setRestored({ path, item });
                                  })
                                }
                              >
                                {t[3]}
                              </button>
                            </li>
                          ))}
                        </ul>
                      )}
                    </section>
                  );
                },
              )}
            </>
          )}
        </div>
      )}
      {restored && (
        <div>
          <p>
            {t[6]}: <code>{restored.path}</code>
          </p>
          <p>
            {restored.item.agent === "agy" ? ops[locale].agyRecovery : t[14]}
          </p>
          {restored.item.agent !== "agy" && <p>{h[4]}</p>}
          {restored.item.agent !== "agy" && (
            <Continuation
              locale={locale}
              path={restored.path}
              session={restored.item.session}
              agent={restored.item.agent}
              cwd={restored.item.mappedCwd ?? undefined}
            />
          )}
          <pre>
            {env[restored.item.agent]
              ? `${env[restored.item.agent]} = ${restored.path}`
              : restored.path}
            {"\n"}Session ID: {restored.item.session}
          </pre>
        </div>
      )}
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
