import type { Locale } from "./i18n";

type SpaceError =
  | "foreign_space"
  | "unsupported_encryption_version"
  | "encrypted_space_mismatch"
  | "folder_has_sync_objects"
  | "foreign_space_objects"
  | "wrong_space_or_version";

const messages: Record<Locale, Record<SpaceError, string>> = {
  en: {
    foreign_space:
      "An encrypted object belongs to a different space. Data was preserved. Verify the recovery kit from the original device and reimport it only if this computer has the wrong kit.",
    unsupported_encryption_version:
      "This encryption version is not supported by this app release. Keep the data and check whether a compatible Bastet release is available; an update is not guaranteed to resolve it.",
    encrypted_space_mismatch:
      "The authenticated encrypted object is inconsistent with the expected space. Stop applying this object, preserve the data, and report the error. Importing a recovery kit will not repair this inconsistency.",
    folder_has_sync_objects:
      "This folder already contains sync objects, so a new space cannot be created here. Import the original recovery kit or choose an empty folder. An OAuth-client JSON only configures Google sign-in and cannot fix this.",
    foreign_space_objects:
      "Objects from another encrypted space were skipped. Own-space objects can continue, but this cycle is partial; do not treat other devices as synchronized. If expected data is missing, verify the recovery kit from the original device.",
    wrong_space_or_version:
      "Bastet cannot determine whether this encrypted object belongs to another space or uses an unsupported version. The exact cause is unknown. Preserve the data, then verify the original recovery kit and app versions.",
  },
  "zh-Hant": {
    foreign_space:
      "有加密物件屬於不同空間。資料已保留。請確認原始裝置的恢復檔，只有這台電腦使用錯誤恢復檔時才重新匯入。",
    unsupported_encryption_version:
      "此加密版本不受目前的程式版本支援。請保留資料並確認是否有相容的 Bastet 版本；更新不保證能解決問題。",
    encrypted_space_mismatch:
      "已驗證的加密物件與預期空間不一致。請停止套用此物件、保留資料並回報錯誤；重新匯入恢復檔無法修復此不一致。",
    folder_has_sync_objects:
      "此資料夾已有同步物件，不能在此建立新空間。請匯入原本的恢復檔，或改選空白資料夾。OAuth client JSON 只設定 Google 登入，無法解決此問題。",
    foreign_space_objects:
      "已略過其他加密空間的物件。自己的空間物件可繼續處理，但本輪只有部分完成，不能視為其他裝置已同步。若缺少預期資料，請確認原始裝置的恢復檔。",
    wrong_space_or_version:
      "Bastet 無法判斷此加密物件屬於其他空間或使用不支援的版本。確切原因未知。請保留資料，再確認原本的恢復檔與程式版本。",
  },
  "zh-Hans": {
    foreign_space:
      "有加密对象属于不同空间。数据已保留。请确认原始设备的恢复文件，只有这台电脑使用错误恢复文件时才重新导入。",
    unsupported_encryption_version:
      "此加密版本不受当前程序版本支持。请保留数据并确认是否有兼容的 Bastet 版本；更新不保证能解决问题。",
    encrypted_space_mismatch:
      "已验证的加密对象与预期空间不一致。请停止应用此对象、保留数据并报告错误；重新导入恢复文件无法修复此不一致。",
    folder_has_sync_objects:
      "此文件夹已有同步对象，不能在此创建新空间。请导入原来的恢复文件，或改选空文件夹。OAuth client JSON 只配置 Google 登录，无法解决此问题。",
    foreign_space_objects:
      "已跳过其他加密空间的对象。自己空间的对象可继续处理，但本轮只完成一部分，不能视为其他设备已同步。若缺少预期数据，请确认原始设备的恢复文件。",
    wrong_space_or_version:
      "Bastet 无法判断此加密对象属于其他空间或使用不支持的版本。确切原因未知。请保留数据，再确认原来的恢复文件和程序版本。",
  },
  ja: {
    foreign_space:
      "暗号化オブジェクトが別のスペースに属しています。データは保持されています。元の端末のリカバリーキットを確認し、このコンピューターのキットが誤っている場合だけ再インポートしてください。",
    unsupported_encryption_version:
      "この暗号化バージョンは現在のアプリのリリースで未対応です。データを保持し、互換性のある Bastet リリースがあるか確認してください。更新しても解決するとは限りません。",
    encrypted_space_mismatch:
      "認証済みの暗号化オブジェクトが想定したスペースと一致しません。このオブジェクトの適用を止め、データを保持してエラーを報告してください。リカバリーキットの再インポートでは修復できません。",
    folder_has_sync_objects:
      "このフォルダーには既に同期オブジェクトがあるため、新しいスペースを作成できません。元のリカバリーキットをインポートするか、空のフォルダーを選んでください。OAuth クライアント JSON は Google ログインの設定だけで、この問題は解決しません。",
    foreign_space_objects:
      "別の暗号化スペースのオブジェクトをスキップしました。自分のスペースのオブジェクトは続行できますが、今回は一部のみ完了しており、他の端末が同期済みとは扱えません。必要なデータがない場合は、元の端末のリカバリーキットを確認してください。",
    wrong_space_or_version:
      "Bastet はこの暗号化オブジェクトが別のスペースのものか、未対応バージョンかを判断できません。正確な原因は不明です。データを保持し、元のリカバリーキットとアプリのバージョンを確認してください。",
  },
  ko: {
    foreign_space:
      "암호화 객체가 다른 공간에 속합니다. 데이터는 보존되었습니다. 원래 장치의 복구 키트를 확인하고 이 컴퓨터에 잘못된 키트가 있을 때만 다시 가져오세요.",
    unsupported_encryption_version:
      "이 암호화 버전은 현재 앱 릴리스에서 지원되지 않습니다. 데이터를 보존하고 호환되는 Bastet 릴리스가 있는지 확인하세요. 업데이트가 해결을 보장하지는 않습니다.",
    encrypted_space_mismatch:
      "인증된 암호화 객체가 예상 공간과 일치하지 않습니다. 이 객체의 적용을 중지하고 데이터를 보존한 뒤 오류를 보고하세요. 복구 키트를 다시 가져와도 이 불일치는 복구되지 않습니다.",
    folder_has_sync_objects:
      "이 폴더에는 이미 동기화 객체가 있어 새 공간을 만들 수 없습니다. 원래 복구 키트를 가져오거나 빈 폴더를 선택하세요. OAuth 클라이언트 JSON은 Google 로그인만 설정하며 이 문제를 해결하지 않습니다.",
    foreign_space_objects:
      "다른 암호화 공간의 객체를 건너뛰었습니다. 자신의 공간 객체는 계속 처리할 수 있지만 이번 주기는 부분 완료이며 다른 장치가 동기화되었다고 볼 수 없습니다. 필요한 데이터가 없으면 원래 장치의 복구 키트를 확인하세요.",
    wrong_space_or_version:
      "Bastet은 이 암호화 객체가 다른 공간의 것인지 또는 지원되지 않는 버전인지 판단할 수 없습니다. 정확한 원인은 알 수 없습니다. 데이터를 보존하고 원래 복구 키트와 앱 버전을 확인하세요.",
  },
};

const codes: SpaceError[] = [
  "foreign_space_objects",
  "unsupported_encryption_version",
  "encrypted_space_mismatch",
  "folder_has_sync_objects",
  "wrong_space_or_version",
  "foreign_space",
];

export function spaceErrorText(error: string, locale: Locale) {
  const code = codes.find((candidate) => error.includes(candidate));
  return code ? messages[locale][code] : null;
}
