import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "./i18n";
export type Mapping = { source: string; target: string };
export const mappingText = {
  "zh-Hant": [
    "專案路徑對應",
    "將來源電腦的專案位置對應到本機已有的專案。儲存後，恢復到新資料夾時套用；不複製專案程式碼，也不修改歷史訊息中的文字。",
    "來源專案路徑",
    "本機專案路徑",
    "選擇本機專案",
    "新增對應",
    "移除",
    "無法選取資料夾",
  ],
  "zh-Hans": [
    "项目路径映射",
    "将来源电脑的项目位置映射到本机已有的项目。保存后，恢复到新文件夹时应用；不复制项目代码，也不修改历史消息中的文字。",
    "来源项目路径",
    "本机项目路径",
    "选择本机项目",
    "添加映射",
    "移除",
    "无法选择文件夹",
  ],
  en: [
    "Project paths",
    "Map a source computer’s project to an existing local project. Save to apply when restoring a new profile. Project code and text inside historical messages are not changed.",
    "Source project path",
    "Local project path",
    "Choose local project",
    "Add mapping",
    "Remove",
    "Could not choose folder",
  ],
  ja: [
    "プロジェクトパスの対応",
    "送信元のプロジェクトを既存のローカルプロジェクトに対応付けます。保存後、新規プロファイルへの復元時に適用します。コードのコピーや過去のメッセージの変更は行いません。",
    "送信元のプロジェクトパス",
    "ローカルのプロジェクトパス",
    "ローカルプロジェクトを選択",
    "対応を追加",
    "削除",
    "フォルダーを選択できませんでした",
  ],
  ko: [
    "프로젝트 경로 연결",
    "원본 컴퓨터의 프로젝트를 기존 로컬 프로젝트에 연결합니다. 저장 후 새 프로필 복원 시 적용됩니다. 프로젝트 코드와 과거 메시지의 텍스트는 변경하지 않습니다.",
    "원본 프로젝트 경로",
    "로컬 프로젝트 경로",
    "로컬 프로젝트 선택",
    "경로 추가",
    "삭제",
    "폴더를 선택할 수 없습니다",
  ],
} as const;
export default function ProjectMappings({
  locale,
  value = [],
  disabled,
  onChange,
}: {
  locale: Locale;
  value?: Mapping[];
  disabled: boolean;
  onChange: (value: Mapping[]) => void;
}) {
  const t = mappingText[locale];
  const [error, setError] = useState(false);
  const update = (index: number, patch: Partial<Mapping>) =>
    onChange(value.map((m, i) => (i === index ? { ...m, ...patch } : m)));
  return (
    <section className="panel project-mappings">
      <h2>{t[0]}</h2>
      <p>{t[1]}</p>
      <fieldset disabled={disabled}>
        {value.map((mapping, index) => (
          <div className="form-grid" key={index}>
            <label>
              {t[2]}
              <input
                value={mapping.source}
                onChange={(e) => update(index, { source: e.target.value })}
                spellCheck={false}
              />
            </label>
            <label>
              {t[3]}
              <input
                value={mapping.target}
                onChange={(e) => update(index, { target: e.target.value })}
                spellCheck={false}
              />
            </label>
            <button
              type="button"
              onClick={() => {
                void invoke<string | null>("choose_folder")
                  .then((path) => {
                    if (path) update(index, { target: path });
                    setError(false);
                  })
                  .catch(() => setError(true));
              }}
            >
              {t[4]}
            </button>
            <button
              type="button"
              aria-label={`${t[6]} ${index + 1}`}
              onClick={() => onChange(value.filter((_, i) => i !== index))}
            >
              {t[6]}
            </button>
          </div>
        ))}
        <button
          type="button"
          disabled={value.length >= 64}
          onClick={() => onChange([...value, { source: "", target: "" }])}
        >
          {t[5]}
        </button>
      </fieldset>
      {error && <p role="alert">{t[7]}</p>}
    </section>
  );
}
