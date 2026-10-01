import type { Locale } from "./i18n";
const text = {
  "zh-Hant": [
    "請先設定來源專案與本機專案的路徑對應，再重新同步或恢復。",
    "本機專案資料夾不存在；請選擇已有的專案。",
    "專案對應必須使用絕對路徑，且同一來源不能對應到不同位置。",
    "此 Agent 格式尚無可驗證的專案移植方式；快照仍已保留。",
    "接續資料夾無法讀取或已移動；請恢復快照到新的資料夾。",
  ],
  "zh-Hans": [
    "请先设置来源项目与本机项目的路径映射，再重新同步或恢复。",
    "本机项目文件夹不存在；请选择已有的项目。",
    "项目映射必须使用绝对路径，且同一来源不能映射到不同位置。",
    "此 Agent 格式尚无可验证的项目迁移方式；快照仍已保留。",
    "接续文件夹无法读取或已移动；请将快照恢复到新文件夹。",
  ],
  en: [
    "Map the source project to a local project, then sync or restore again.",
    "The local project folder is missing. Choose an existing project.",
    "Use absolute project paths and one destination per source path.",
    "This agent format has no verified project migration yet. The snapshot is retained.",
    "The continuation profile is unreadable or has moved. Restore the snapshot to a new folder.",
  ],
  ja: [
    "送信元のプロジェクトをローカルプロジェクトに対応付けてから、同期または復元してください。",
    "ローカルプロジェクトが見つかりません。既存のフォルダーを選択してください。",
    "絶対パスを使用し、各送信元には一つの復元先を指定してください。",
    "この Agent 形式のプロジェクト移行は未検証です。スナップショットは保持されています。",
    "再開用プロファイルが移動されたか読み取れません。新規フォルダーに復元してください。",
  ],
  ko: [
    "원본 프로젝트를 로컬 프로젝트에 연결한 후 동기화하거나 복원하세요.",
    "로컬 프로젝트 폴더가 없습니다. 기존 프로젝트를 선택하세요.",
    "절대 경로를 사용하고 원본 경로마다 하나의 대상만 지정하세요.",
    "이 Agent 형식의 프로젝트 이전은 아직 검증되지 않았습니다. 스냅샷은 보존됩니다.",
    "계속할 프로필을 읽을 수 없거나 이동되었습니다. 스냅샷을 새 폴더에 복원하세요.",
  ],
} as const;
export function projectError(code: string, locale: Locale): string | null {
  const index =
    code === "project_mapping_required"
      ? 0
      : code === "project_mapping_target_missing"
        ? 1
        : ["project_mapping_invalid", "project_mapping_conflict"].includes(code)
          ? 2
          : code === "project_mapping_format_unsupported"
            ? 3
            : ["handoff_profile_missing", "handoff_registry_invalid"].includes(
                  code,
                )
              ? 4
              : -1;
  return index === -1 ? null : text[locale][index];
}
