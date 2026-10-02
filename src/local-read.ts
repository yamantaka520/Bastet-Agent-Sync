import type { Locale } from "./i18n";

const messages: Record<
  Locale,
  Record<"sync_busy" | "wizard_step_required", string>
> = {
  "zh-Hant": {
    sync_busy: "本機同步資料暫時忙碌，請稍後再試。",
    wizard_step_required:
      "請先完成 Google Drive 同步空間設定，再查看對話快照。",
  },
  "zh-Hans": {
    sync_busy: "本地同步数据暂时忙碌，请稍后重试。",
    wizard_step_required:
      "请先完成 Google Drive 同步空间设置，再查看对话快照。",
  },
  en: {
    sync_busy: "Local sync data is temporarily busy. Try again shortly.",
    wizard_step_required:
      "Complete Google Drive sync space setup before viewing conversation snapshots.",
  },
  ja: {
    sync_busy:
      "ローカル同期データが一時的に使用中です。少し待ってから再試行してください。",
    wizard_step_required:
      "会話スナップショットを表示する前に、Google Drive の同期スペースを設定してください。",
  },
  ko: {
    sync_busy:
      "로컬 동기화 데이터가 일시적으로 사용 중입니다. 잠시 후 다시 시도하세요.",
    wizard_step_required:
      "대화 스냅샷을 보기 전에 Google Drive 동기화 공간 설정을 완료하세요.",
  },
};

export function localReadError(error: unknown, locale: Locale): string | null {
  const code = String(error);
  return code === "sync_busy" || code === "wizard_step_required"
    ? messages[locale][code]
    : null;
}

// Only retry read commands. A failed wizard file lock can outlive one IPC call.
export async function retryBusyRead<T>(read: () => Promise<T>): Promise<T> {
  for (let attempt = 0; ; attempt++) {
    try {
      return await read();
    } catch (error) {
      if (String(error) !== "sync_busy" || attempt >= 2) throw error;
      await new Promise((resolve) => setTimeout(resolve, 100 * (attempt + 1)));
    }
  }
}
