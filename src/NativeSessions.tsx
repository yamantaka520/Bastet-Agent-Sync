import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "./i18n";
import { names } from "./model";
import { syncDisplay, issueText, savedTime } from "./sync-display";
export const sessionMessages = {
  "zh-Hant": [
    "本機對話同步",
    "同步已勾選來源的原生對話快照。接收端自動加入不存在的對話；既有對話不同時保留快照供另存，不覆蓋使用中的 Agent；帳號、金鑰、專案檔案和雲端託管聊天不包含在內。Claude 使用本機 Claude Code 對話；Codex 與 Work 共用本機對話來源。",
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
  ],
  "zh-Hans": [
    "本地对话同步",
    "同步所选来源的原生对话快照。接收端自动添加不存在的对话；已有对话不同时保留快照供另存，不覆盖正在使用的 Agent；不包含账号、密钥、项目文件和云端托管聊天。Claude 使用本地 Claude Code 对话；Codex 与 Work 共用本地对话来源。",
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
  ],
  en: [
    "Local conversation sync",
    "Sync native conversation snapshots from selected sources. Missing conversations are added automatically. Different existing versions remain snapshots for separate restoration; active profiles are never overwritten. Accounts, keys, project files and cloud-hosted chats are excluded. Claude uses local Claude Code conversations; Codex and Work share local conversation storage.",
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
  ],
  ja: [
    "ローカル会話の同期",
    "選択したソースのネイティブ会話を同期します。未登録の会話は自動追加します。既存の会話と異なる版は別途復元できるよう保存し、使用中の環境を上書きしません。アカウント、キー、プロジェクトファイル、クラウド会話は対象外です。Claude はローカル Claude Code、Codex と Work は共通のローカル会話を使用します。",
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
  ],
  ko: [
    "로컬 대화 동기화",
    "선택한 소스의 기본 대화 스냅샷을 동기화합니다. 없는 대화는 자동 추가합니다. 기존 대화와 다른 버전은 별도 복원용으로 보관하며 사용 중인 프로필을 덮어쓰지 않습니다. 계정, 키, 프로젝트 파일 및 클라우드 대화는 제외합니다. Claude는 로컬 Claude Code 대화를, Codex와 Work는 같은 로컬 저장소를 사용합니다.",
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
  ],
} as const;
export type SourceStatus = {
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
};
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
}: {
  native: boolean;
  locale: Locale;
  running: boolean;
  sources?: SourceStatus[];
}) {
  const t = sessionMessages[locale];
  const d = syncDisplay[locale];
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
      setError(String(e));
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
                              </details>
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
          <p>{t[14]}</p>
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
