import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "./i18n";
import { names } from "./model";
export type SyncStatus = {
  running: boolean;
  phase: string;
  published: number;
  received: number;
  applied: number;
  lastSuccess: number | null;
  error: string | null;
  skipped: string[];
};
export const workerMessages = {
  "zh-Hant": [
    "Agent Memory OS：勾選後按啟動即可自動同步，不需手動匯出或匯入。僅與你信任且持有同一恢復金鑰的裝置共用同步空間；包含私人記憶、刪除與權限資料。",
    "ChatGPT Work：本機工作與雲端工作分開處理。目前僅偵測候選目錄，尚無已驗證的原生工作搬移介面。",
    "⏳ 正在啟動",
    "🔄 Agent Memory OS 同步中",
    "🕒 本輪完成，等待下一次同步",
    "⏳ 正在安全暫停",
    "⏸️ 已暫停",
    "⚠️ 同步失敗／等待重試",
    "暫停同步",
    "立即同步",
    "以下來源本輪略過，不影響 Agent Memory OS：",
    "上傳封包",
    "接收封包",
    "合併封包",
    "最後完成",
    "找不到 Agent Memory OS CLI。請指定 agent-memory 執行檔。",
    "選擇 Agent Memory OS CLI",
    "尚未選到可用來源；目前自動同步支援 Agent Memory OS。",
    "請先暫停同步再修改設定。",
    "進階：手動封包檢查（同步不需要此步驟）",
  ],
  "zh-Hans": [
    "Agent Memory OS：勾选后按启动即可自动同步，无需手动导入导出。仅与可信且持有同一恢复密钥的设备共享空间；包含私有记忆、删除与权限数据。",
    "ChatGPT Work：区分本地和云端任务。目前仅检测候选目录，尚无已验证的原生任务迁移接口。",
    "⏳ 正在启动",
    "🔄 Agent Memory OS 同步中",
    "🕒 本轮完成，等待下次同步",
    "⏳ 正在安全暂停",
    "⏸️ 已暂停",
    "⚠️ 同步失败／等待重试",
    "暂停同步",
    "立即同步",
    "本轮跳过以下来源，不影响 Agent Memory OS：",
    "上传封包",
    "接收封包",
    "合并封包",
    "最后完成",
    "找不到 Agent Memory OS CLI。请选择 agent-memory 可执行文件。",
    "选择 Agent Memory OS CLI",
    "没有可用来源；目前自动同步支持 Agent Memory OS。",
    "请先暂停同步再修改设置。",
    "高级：手动封包检查（同步不需要此步骤）",
  ],
  en: [
    "Agent Memory OS syncs automatically after selection and Start. No manual export/import. Share this space only with trusted devices holding the same recovery key; it includes private memories, deletions and permissions.",
    "ChatGPT Work: local and cloud tasks are distinct. Candidate discovery only; native task migration has no verified adapter yet.",
    "⏳ Starting",
    "🔄 Syncing Agent Memory OS",
    "🕒 Cycle complete; waiting",
    "⏳ Pausing safely",
    "⏸️ Paused",
    "⚠️ Sync failed / awaiting retry",
    "Pause sync",
    "Sync now",
    "Skipped this cycle; Agent Memory OS continues:",
    "Uploaded bundles",
    "Received bundles",
    "Merged bundles",
    "Last completed",
    "Agent Memory OS CLI was not found. Choose the agent-memory executable.",
    "Choose Agent Memory OS CLI",
    "No ready sources selected. Automatic sync currently supports Agent Memory OS.",
    "Pause synchronization before changing settings.",
    "Advanced: manual bundle check (not needed for sync)",
  ],
  ja: [
    "Agent Memory OS は選択して開始すると自動同期します。手動の入出力は不要です。同じ復元キーを持つ信頼できる端末とのみ共有してください。非公開の記憶、削除、権限も含まれます。",
    "ChatGPT Work：ローカルとクラウドのタスクは別です。候補フォルダーの検出のみで、ネイティブ移行は未検証です。",
    "⏳ 開始中",
    "🔄 Agent Memory OS を同期中",
    "🕒 今回の同期完了・待機中",
    "⏳ 安全に一時停止中",
    "⏸️ 一時停止",
    "⚠️ 同期失敗・再試行待ち",
    "同期を一時停止",
    "今すぐ同期",
    "以下はスキップ。Agent Memory OS は続行：",
    "送信バンドル",
    "受信バンドル",
    "統合バンドル",
    "最終完了",
    "Agent Memory OS CLI がありません。agent-memory 実行ファイルを選択してください。",
    "Agent Memory OS CLI を選択",
    "利用可能なソースが未選択です。自動同期は現在 Agent Memory OS に対応しています。",
    "設定変更前に同期を一時停止してください。",
    "詳細：手動バンドル確認（同期には不要）",
  ],
  ko: [
    "Agent Memory OS는 선택 후 시작하면 자동 동기화합니다. 수동 내보내기/가져오기는 필요 없습니다. 같은 복구 키를 가진 신뢰하는 장치와만 공유하세요. 비공개 기억, 삭제 및 권한도 포함됩니다.",
    "ChatGPT Work: 로컬 작업과 클라우드 작업은 다릅니다. 후보 폴더만 감지하며 기본 작업 이전은 아직 검증되지 않았습니다.",
    "⏳ 시작 중",
    "🔄 Agent Memory OS 동기화 중",
    "🕒 이번 동기화 완료·대기 중",
    "⏳ 안전하게 일시 중지 중",
    "⏸️ 일시 중지됨",
    "⚠️ 동기화 실패·재시도 대기",
    "동기화 일시 중지",
    "지금 동기화",
    "다음 소스는 건너뛰며 Agent Memory OS는 계속합니다:",
    "업로드 묶음",
    "수신 묶음",
    "병합 묶음",
    "마지막 완료",
    "Agent Memory OS CLI를 찾을 수 없습니다. agent-memory 실행 파일을 선택하세요.",
    "Agent Memory OS CLI 선택",
    "사용 가능한 소스가 선택되지 않았습니다. 현재 자동 동기화는 Agent Memory OS를 지원합니다.",
    "설정 변경 전에 동기화를 일시 중지하세요.",
    "고급: 수동 묶음 검사 (동기화에 필요 없음)",
  ],
} as const;
export function phaseText(s: SyncStatus, locale: Locale) {
  const t = workerMessages[locale];
  return (
    (
      {
        starting: t[2],
        syncing: t[3],
        waiting: t[4],
        pausing: t[5],
        paused: t[6],
        error: t[7],
      } as Record<string, string>
    )[s.phase] ?? ""
  );
}
export function workerError(error: string, locale: Locale) {
  const t = workerMessages[locale];
  return (
    (
      {
        memory_cli_missing: t[15],
        no_ready_sources: t[17],
        sync_running: t[18],
      } as Record<string, string>
    )[error] ?? `${t[7]} (${error})`
  );
}
export default function WorkerStatus({
  native,
  locale,
  status,
  onStatus,
}: {
  native: boolean;
  locale: Locale;
  status: SyncStatus | null;
  onStatus: (s: SyncStatus) => void;
}) {
  const t = workerMessages[locale];
  const [pickerError, setPickerError] = useState<string | null>(null);
  useEffect(() => {
    if (!native) return;
    let live = true;
    const refresh = () => {
      void invoke<SyncStatus>("sync_status")
        .then((s) => {
          if (live && s) onStatus(s);
        })
        .catch(() => {});
    };
    refresh();
    const id = setInterval(refresh, 1000);
    return () => {
      live = false;
      clearInterval(id);
    };
  }, [native, onStatus]);
  return (
    <div className="worker-status" aria-live="polite">
      <p>{t[0]}</p>
      <button
        disabled={!native || !!status?.running}
        onClick={() => {
          setPickerError(null);
          void invoke("choose_memory_cli").catch((e) =>
            setPickerError(typeof e === "string" ? e : "memory_cli_missing"),
          );
        }}
      >
        {t[16]}
      </button>
      {pickerError && <p role="alert">{workerError(pickerError, locale)}</p>}
      {status && status.phase && (
        <>
          <strong>{phaseText(status, locale)}</strong>
          <p>
            {t[11]}: {status.published} · {t[12]}: {status.received} · {t[13]}:{" "}
            {status.applied}
          </p>
          {status.lastSuccess && (
            <p>
              {t[14]}:{" "}
              {new Date(status.lastSuccess * 1000).toLocaleString(locale)}
            </p>
          )}
          {status.error && (
            <p role="alert">{workerError(status.error, locale)}</p>
          )}
          {status.skipped.length > 0 && (
            <p>
              {t[10]} {status.skipped.map((id) => names[id] ?? id).join(", ")}
            </p>
          )}
          {status.running && (
            <button onClick={() => void invoke("sync_now").catch(() => {})}>
              {t[9]}
            </button>
          )}
        </>
      )}
    </div>
  );
}
