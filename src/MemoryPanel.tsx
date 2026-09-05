import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "./i18n";
export const memoryMessages = {
  en: [
    "Agent Memory OS",
    "Check an official JSONL export before transport. This does not read the live database or import memories.",
    "Choose exported JSONL",
    "Envelope check passed; semantic import validation remains with Agent Memory OS.",
    "Contains deletion or organization records. Applying them requires a trusted import review.",
    "Unsupported or invalid export (maximum 1 MiB, versions 1–3).",
    "Version",
    "Records",
  ],
  "zh-Hant": [
    "Agent Memory OS",
    "傳輸前檢查正式 JSONL 匯出檔。不讀取即時資料庫，也不匯入記憶。",
    "選擇 JSONL 匯出檔",
    "封包外層檢查通過；內容語意仍須由 Agent Memory OS 匯入驗證。",
    "包含刪除或組織資料；套用前須審核受信任匯入。",
    "不支援或無效的匯出檔（最多 1 MiB，版本 1–3）。",
    "版本",
    "紀錄數",
  ],
  "zh-Hans": [
    "Agent Memory OS",
    "传输前检查正式 JSONL 导出文件。不读取实时数据库，也不导入记忆。",
    "选择 JSONL 导出文件",
    "封包外层检查通过；内容语义仍须由 Agent Memory OS 导入验证。",
    "包含删除或组织数据；应用前须审核受信任导入。",
    "不支持或无效的导出文件（最多 1 MiB，版本 1–3）。",
    "版本",
    "记录数",
  ],
  ja: [
    "Agent Memory OS",
    "転送前に公式 JSONL エクスポートを確認します。稼働中の DB の読み取りや記憶のインポートは行いません。",
    "JSONL エクスポートを選択",
    "外部形式を確認しました。内容の意味検証は Agent Memory OS のインポート時に行います。",
    "削除または組織データを含みます。適用前に信頼されたインポートの確認が必要です。",
    "未対応または無効なエクスポート（最大 1 MiB、バージョン 1–3）。",
    "バージョン",
    "レコード数",
  ],
  ko: [
    "Agent Memory OS",
    "전송 전 공식 JSONL 내보내기 파일을 확인합니다. 실행 중인 DB를 읽거나 기억을 가져오지 않습니다.",
    "JSONL 내보내기 선택",
    "외부 형식 확인을 통과했습니다. 내용 의미 검증은 Agent Memory OS 가져오기에서 수행합니다.",
    "삭제 또는 조직 데이터가 포함됩니다. 적용 전 신뢰할 수 있는 가져오기 검토가 필요합니다.",
    "지원되지 않거나 잘못된 내보내기 (최대 1 MiB, 버전 1–3).",
    "버전",
    "레코드 수",
  ],
} as const;
type Inspection = {
  version: number;
  records: number;
  kinds: Record<string, number>;
  containsAuthorityChanges: boolean;
};
export default function MemoryPanel({
  native,
  locale,
}: {
  native: boolean;
  locale: Locale;
}) {
  const t = memoryMessages[locale];
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<Inspection | null>(null);
  const [error, setError] = useState(false);
  async function inspect() {
    setBusy(true);
    setResult(null);
    setError(false);
    try {
      setResult(await invoke<Inspection | null>("inspect_memory_export"));
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="panel">
      <h2>{t[0]}</h2>
      <p>{t[1]}</p>
      <button disabled={!native || busy} onClick={inspect}>
        {t[2]}
      </button>
      {error && <p role="alert">{t[5]}</p>}
      {result && (
        <div role="status">
          <p>{t[3]}</p>
          <p>
            {t[6]}: {result.version} · {t[7]}: {result.records}
          </p>
          {result.containsAuthorityChanges && <p>{t[4]}</p>}
        </div>
      )}
    </section>
  );
}
