import type { Locale } from "./i18n";

// Machine codes from the bounded multipart portable transport. Sources:
// src-tauri/src/portable_parts.rs (index/part reassembly, tamper, foreign part,
// version, transfer-wide limits) and src-tauri/src/portable.rs (load_workspace
// readiness gate, package/missing). Each maps to one localized, actionable
// instruction shown wherever the code reaches the UI.
//
// HONESTY CONTRACT: the backend returns the *actual* per-transfer readiness
// (`Received.readiness`: complete / legacy / present / expected / missing and an
// explicit `issue` when an item could not be read). `complete` means every part
// is present and the index is internally consistent — it is NOT byte-level
// verification, which only compare/restore perform. So a complete transfer is
// labelled "parts received — integrity will be verified before restore", never
// "ready"/"verified"; an incomplete transfer reads as waiting for its counted
// missing parts. These messages render the backend's own counts verbatim and
// never invent an "N of M" the backend did not return.
export type PortableErrorCode =
  | "portable_incomplete"
  | "portable_tampered"
  | "portable_foreign_part"
  | "portable_version_unsupported"
  | "portable_limit"
  | "portable_format"
  | "portable_unsafe"
  | "portable_missing";

export const PORTABLE_ERROR_CODES: PortableErrorCode[] = [
  "portable_version_unsupported",
  "portable_foreign_part",
  "portable_incomplete",
  "portable_tampered",
  "portable_limit",
  "portable_format",
  "portable_unsafe",
  "portable_missing",
];

const messages: Record<Locale, Record<PortableErrorCode, string>> = {
  en: {
    portable_incomplete:
      "This transfer is not complete: one or more parts have not arrived yet. Keep syncing so the remaining parts are received. Compare and restore stay unavailable until every part is present, so this snapshot is not ready.",
    portable_tampered:
      "A received part failed its integrity check and was refused. Nothing was restored or written. Sync this source again from the sender before trusting it.",
    portable_foreign_part:
      "A received part belongs to a different transfer and was ignored; it is not part of this snapshot. Sync this source again from the sender.",
    portable_version_unsupported:
      "This transfer uses a format newer than this app version supports. Update Bastet, then sync again; the received data is untouched.",
    portable_limit:
      "This transfer exceeds a size or count limit, so it cannot complete as it stands. Reduce the portable selection, then sync again from the sender.",
    portable_format:
      "A received transfer manifest or part could not be read, so it was refused. Sync this source again from the sender.",
    portable_unsafe:
      "This transfer was rejected as unsafe and was not restored. Sync this source again from the sender.",
    portable_missing:
      "No transfer was found for this item. Sync this source again from the sender.",
  },
  "zh-Hant": {
    portable_incomplete:
      "此傳輸尚未完成：仍有部分尚未送達。請繼續同步，讓其餘部分送達。在全部部分到齊前，比對與還原都不會提供，因此此快照尚未就緒。",
    portable_tampered:
      "收到的部分未通過完整性檢查而被拒絕。未還原或寫入任何內容。請從傳送端重新同步此來源後再信任。",
    portable_foreign_part:
      "收到的部分屬於其他傳輸，已忽略；它不屬於此快照。請從傳送端重新同步此來源。",
    portable_version_unsupported:
      "此傳輸使用的格式比目前程式版本支援的更新。請更新 Bastet 後再同步；已收到的資料不會被更動。",
    portable_limit:
      "此傳輸超過大小或數量上限，無法以此狀態完成。請減少可攜選取範圍，再從傳送端重新同步。",
    portable_format:
      "收到的傳輸索引或部分無法讀取而被拒絕。請從傳送端重新同步此來源。",
    portable_unsafe: "此傳輸被判定為不安全而未還原。請從傳送端重新同步此來源。",
    portable_missing: "找不到此項目的傳輸。請從傳送端重新同步此來源。",
  },
  "zh-Hans": {
    portable_incomplete:
      "此传输尚未完成：仍有部分尚未送达。请继续同步，让其余部分送达。在全部部分到齐前，比对和还原都不会提供，因此此快照尚未就绪。",
    portable_tampered:
      "收到的部分未通过完整性检查而被拒绝。未还原或写入任何内容。请从发送端重新同步此来源后再信任。",
    portable_foreign_part:
      "收到的部分属于其他传输，已忽略；它不属于此快照。请从发送端重新同步此来源。",
    portable_version_unsupported:
      "此传输使用的格式比当前程序版本支持的更新。请更新 Bastet 后再同步；已收到的数据不会被改动。",
    portable_limit:
      "此传输超过大小或数量上限，无法以此状态完成。请减少便携选择范围，再从发送端重新同步。",
    portable_format:
      "收到的传输索引或部分无法读取而被拒绝。请从发送端重新同步此来源。",
    portable_unsafe: "此传输被判定为不安全而未还原。请从发送端重新同步此来源。",
    portable_missing: "找不到此项目的传输。请从发送端重新同步此来源。",
  },
  ja: {
    portable_incomplete:
      "この転送は未完了です。まだ届いていない部分があります。同期を続けて残りの部分を受信してください。すべての部分が揃うまで比較と復元は利用できないため、このスナップショットは準備完了ではありません。",
    portable_tampered:
      "受信した部分が整合性チェックに失敗したため拒否されました。復元も書き込みも行っていません。信頼する前に、送信側からこのソースを再同期してください。",
    portable_foreign_part:
      "受信した部分は別の転送に属するため無視しました。このスナップショットの一部ではありません。送信側からこのソースを再同期してください。",
    portable_version_unsupported:
      "この転送は、このアプリのバージョンが対応するより新しい形式です。Bastet を更新してから再同期してください。受信済みのデータは変更されません。",
    portable_limit:
      "この転送はサイズまたは件数の上限を超えており、このままでは完了できません。移行対象を減らして、送信側から再同期してください。",
    portable_format:
      "受信した転送の索引または部分を読み取れず拒否されました。送信側からこのソースを再同期してください。",
    portable_unsafe:
      "この転送は安全でないと判定され、復元されませんでした。送信側からこのソースを再同期してください。",
    portable_missing:
      "この項目の転送が見つかりません。送信側からこのソースを再同期してください。",
  },
  ko: {
    portable_incomplete:
      "이 전송은 완료되지 않았습니다. 아직 도착하지 않은 부분이 있습니다. 동기화를 계속해 나머지 부분을 받으세요. 모든 부분이 도착할 때까지 비교와 복원은 제공되지 않으므로 이 스냅숏은 준비되지 않았습니다.",
    portable_tampered:
      "수신한 부분이 무결성 검사에 실패해 거부되었습니다. 복원하거나 기록한 내용은 없습니다. 신뢰하기 전에 보내는 쪽에서 이 소스를 다시 동기화하세요.",
    portable_foreign_part:
      "수신한 부분이 다른 전송에 속해 무시되었습니다. 이 스냅숏의 일부가 아닙니다. 보내는 쪽에서 이 소스를 다시 동기화하세요.",
    portable_version_unsupported:
      "이 전송은 이 앱 버전이 지원하는 것보다 새로운 형식입니다. Bastet을 업데이트한 뒤 다시 동기화하세요. 수신한 데이터는 변경되지 않습니다.",
    portable_limit:
      "이 전송은 크기 또는 개수 한도를 초과해 현재 상태로는 완료할 수 없습니다. 이동 가능한 선택을 줄인 뒤 보내는 쪽에서 다시 동기화하세요.",
    portable_format:
      "수신한 전송 인덱스나 부분을 읽을 수 없어 거부되었습니다. 보내는 쪽에서 이 소스를 다시 동기화하세요.",
    portable_unsafe:
      "이 전송은 안전하지 않은 것으로 판정되어 복원되지 않았습니다. 보내는 쪽에서 이 소스를 다시 동기화하세요.",
    portable_missing:
      "이 항목의 전송을 찾을 수 없습니다. 보내는 쪽에서 이 소스를 다시 동기화하세요.",
  },
};

// Resolve a multipart transport code to a localized, actionable message. A code
// absent here returns null so the caller keeps its own fallback; the resolver
// never invents a readiness count.
export function portableErrorText(
  error: string,
  locale: Locale,
): string | null {
  const code = PORTABLE_ERROR_CODES.find((candidate) =>
    error.includes(candidate),
  );
  return code ? messages[locale][code] : null;
}

// Serialized per-transfer readiness returned by `portable_list`
// (src-tauri/src/portable.rs `Received.readiness`). Counts are the backend's,
// rendered verbatim. `complete` is presence + index consistency only; it never
// claims the bytes are verified — compare/restore verify them.
export type PortableReadiness = {
  complete: boolean;
  legacy: boolean;
  present: number;
  expected: number;
  missing: number[];
  issue?: string | null;
};

const readinessText: Record<
  Locale,
  { received: string; legacyReceived: string; waiting: string }
> = {
  en: {
    received:
      "Parts received — integrity will be verified before restore. Compare or restore to verify this snapshot.",
    legacyReceived:
      "Single-package snapshot received — compare or restore to verify it.",
    waiting:
      "Not ready — {present} of {expected} parts received; {missing} still missing. Keep syncing; compare and restore stay unavailable until every part arrives.",
  },
  "zh-Hant": {
    received:
      "已收到各部分 — 完整性將於還原前驗證。請比對或還原來驗證此快照。",
    legacyReceived: "已收到單一同步包快照 — 請比對或還原來驗證。",
    waiting:
      "尚未就緒 — 已收到 {present}／{expected} 個部分，仍有 {missing} 個未送達。請繼續同步；全部部分到齊前，比對與還原都不會提供。",
  },
  "zh-Hans": {
    received:
      "已收到各部分 — 完整性将在还原前验证。请比对或还原来验证此快照。",
    legacyReceived: "已收到单一同步包快照 — 请比对或还原来验证。",
    waiting:
      "尚未就绪 — 已收到 {present}／{expected} 个部分，仍有 {missing} 个未送达。请继续同步；全部部分到齐前，比对和还原都不会提供。",
  },
  ja: {
    received:
      "各部分を受信しました — 整合性は復元前に検証されます。比較または復元でこのスナップショットを検証してください。",
    legacyReceived:
      "単一パッケージのスナップショットを受信しました — 比較または復元で検証してください。",
    waiting:
      "未準備 — {expected} 個中 {present} 個の部分を受信、残り {missing} 個。同期を続けてください。すべて揃うまで比較と復元は利用できません。",
  },
  ko: {
    received:
      "모든 부분을 받았습니다 — 무결성은 복원 전에 검증됩니다. 비교 또는 복원으로 이 스냅숏을 검증하세요.",
    legacyReceived:
      "단일 패키지 스냅숏을 받았습니다 — 비교 또는 복원으로 검증하세요.",
    waiting:
      "준비되지 않음 — {expected}개 중 {present}개 부분을 수신했고 {missing}개가 아직 없습니다. 동기화를 계속하세요. 모두 도착할 때까지 비교와 복원은 사용할 수 없습니다.",
  },
};

// Render a received item's readiness. `issue` (a backend code) resolves through
// the same localized table; otherwise the exact present/expected/missing counts
// the backend returned are shown. A complete transfer is described as received
// with integrity still to be verified — never as "ready" or "verified".
export function portableReadinessText(
  readiness: PortableReadiness,
  locale: Locale,
): string {
  if (readiness.issue) {
    return (
      portableErrorText(readiness.issue, locale) ?? messages[locale].portable_format
    );
  }
  const text = readinessText[locale];
  if (readiness.complete) {
    return readiness.legacy ? text.legacyReceived : text.received;
  }
  return text.waiting
    .replace("{present}", String(readiness.present))
    .replace("{expected}", String(readiness.expected))
    .replace("{missing}", String(readiness.missing.length));
}

// A backend issue code that must gate compare/restore for this item.
export function isBlockingPortableCode(
  code: string | undefined,
  locale: Locale,
): boolean {
  return Boolean(code && portableErrorText(code, locale));
}
