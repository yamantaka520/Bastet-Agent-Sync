import type { Locale } from "./i18n";
const en = {
  title: "Sync control center",
  resources: "Network and schedule",
  parallel: "Concurrent storage groups",
  upload: "Upload limit (KiB/s)",
  download: "Download limit (KiB/s)",
  unlimited:
    "0 = unlimited. Limits cover this app’s Drive payloads; save settings to apply.",
  window: "Only start cycles during these local hours",
  from: "From",
  to: "Until",
  hours: "Equal times mean all day; an end before the start crosses midnight.",
  pause: "Pause for 15 minutes",
  resume: "Resume / sync now",
  history: "Recent sync cycles (up to 500)",
  devices: "Devices — last reported, not live online status",
  refresh: "Refresh",
  storage: "Storage",
  measure: "Measure local storage",
  cloudMeasure: "Measure Drive sync objects",
  local: "Local app data",
  cache: "Rebuildable download cache",
  cloud: "Drive sync objects",
  clean: "Clear download cache",
  cleanHint:
    "Pause sync first. Removes only cached downloads; preserves sessions, journals, settings and cloud history.",
  empty: "No records yet",
  error:
    "Could not complete this action. Pause synchronization if needed and retry.",
  at: "Observed",
  reported: "Reported",
  version: "Version",
  complete: "Complete",
  partial: "Needs attention",
  paused: "Paused",
  failed: "Failed",
  scan: "Examining files",
  list: "Listing cloud objects",
  downloading: "Checking / downloading objects",
  uploading: "Uploading changed objects",
  restore: "Restoring / merging",
  progress: "Items examined / processed",
  payload: "Current HTTP payload",
  windowWait: "Outside allowed hours",
  timedPause: "Paused until scheduled resume",
  resumeAt: "Resume at",
  portable: "Portable settings and skills",
  portableSettings: "Sync supported portable settings",
  portableSkills: "Sync skills and instructions",
  portableHint:
    "Opt-in for selected agents. Credentials, machine paths and execution settings are excluded; received files require review before separate-folder recovery.",
  preview: "Preview",
  incoming: "Received packages",
  compare: "Compare",
  keepBoth: "Keep both: restore to new folder",
  path: "File",
  localVersion: "Local",
  remoteVersion: "Incoming",
  missing: "Missing locally",
  same: "Identical",
  different: "Different — original preserved",
  content: "View content",
  acknowledge: "Keep local and mark reviewed",
  reviewed: "Reviewed",
  continuation: "CLI continuation",
  agyRecovery:
    "Agy has no verified alternate-profile switch. This folder is a recovery copy; do not replace an active profile. Separate-folder model continuation is not verified.",
  hermesRecovery:
    "Hermes Agent restores into an isolated managed home (HERMES_HOME). The snapshot is a consistent copy of its native state.db; do not point HERMES_HOME at an active profile. Cross-device resume acceptance is not claimed.",
  cliHint:
    "Validate the recovered profile before opening a conversation. No project files or credentials are restored automatically.",
  copyCommand: "Copy continuation command",
  restored: "Recovered folder",
  checking: "Working…",
  contReadiness: "Continuation readiness",
  contReady: "Ready to continue",
  contUnverified: "Unverified — not confirmed ready",
  contBlocked: "Blocked",
  contBlockers: "Blockers",
  contWarnings: "Warnings",
  contVerified: "Verified",
  contProject: "Restored project",
  contLegacy:
    "No readiness report was received for this version. Treating it as unverified, not ready.",
  contBlockedAction:
    "The continuation command is withheld until these blockers are resolved. Restore to an isolated folder is still available.",
  contUnverifiedNote:
    "Not verified as ready. Confirm model credentials and dependencies before continuing; the command is shown for reference.",
  contReadyNote: "Dependencies were verified for this version.",
  contUnknown: "Unrecognized requirement",
  recheck: "Recheck continuation",
  recheckHint:
    "Re-run dependency and memory readiness for this restored version. Nothing is applied automatically — after you install the provider plugin or change a binding, recheck re-verifies the received snapshot.",
  setupProvider: "Set up memory provider",
  setupProviderHint:
    "Installs the trusted local Agent Memory OS provider plugin into this restored version's isolated home (agent-memory hermes install) and re-runs the official memory import. It never runs received code and never downloads anything.",
  acknowledgeRequirements: "I configured these prerequisites; recheck",
  acknowledgeHint:
    "Records that you configured the opaque requirements (MCP servers, model provider, credential files, host apps). This is an explicit acknowledgment, not a validation: those items stay as warnings and are never reported verified, and verifiable prerequisites (environment variables, commands, operating system) still block.",
  contAcknowledged: "Acknowledged — still unverified",
  contResolved: "Verified on this computer",
};
type Labels = typeof en;
export const ops: Record<Locale, Labels> = {
  en,
  "zh-Hant": {
    title: "同步控制中心",
    resources: "網路與時段",
    parallel: "同時同步的資料組數",
    upload: "上傳上限（KiB/s）",
    download: "下載上限（KiB/s）",
    unlimited: "0 表示不限速。限制本程式的 Drive 資料傳輸；儲存設定後套用。",
    window: "只在以下本機時段開始同步",
    from: "開始",
    to: "結束",
    hours: "起訖相同表示全天；結束早於開始表示跨午夜。",
    pause: "暫停 15 分鐘",
    resume: "恢復／立即同步",
    history: "最近同步紀錄（最多 500 輪）",
    devices: "裝置清單 — 顯示最後回報，不代表即時在線",
    refresh: "重新整理",
    storage: "儲存空間",
    measure: "統計本機空間",
    cloudMeasure: "統計 Drive 同步物件",
    local: "本機程式資料",
    cache: "可重建的下載快取",
    cloud: "Drive 同步物件",
    clean: "清除下載快取",
    cleanHint: "請先暫停同步。僅清除下載快取，保留對話、紀錄、設定與雲端歷史。",
    empty: "尚無紀錄",
    error: "未能完成操作。必要時先暫停同步，再重試。",
    at: "本機查得時間",
    reported: "裝置回報時間",
    version: "版本",
    complete: "完成",
    partial: "需要處理",
    paused: "已暫停",
    failed: "失敗",
    scan: "檢查檔案",
    list: "列出雲端物件",
    downloading: "檢查／下載物件",
    uploading: "上傳變更物件",
    restore: "還原／合併",
    progress: "已檢查／處理項目",
    payload: "目前 HTTP 資料傳輸",
    windowWait: "等待允許同步的時段",
    timedPause: "定時暫停中",
    resumeAt: "預計恢復",
    portable: "可攜設定與技能",
    portableSettings: "同步支援的可攜設定",
    portableSkills: "同步技能與指令",
    portableHint:
      "僅為勾選的 Agent 啟用。排除憑證、機器路徑及自動執行設定；接收後需檢視，再還原至獨立資料夾。",
    preview: "預覽",
    incoming: "已接收的設定包",
    compare: "比較差異",
    keepBoth: "保留兩份：還原到新資料夾",
    path: "檔案",
    localVersion: "本機版本",
    remoteVersion: "接收版本",
    missing: "本機尚無此檔案",
    same: "內容相同",
    different: "內容不同，原檔已保留",
    content: "查看內容",
    acknowledge: "保留本機並標記已檢視",
    reviewed: "已檢視",
    continuation: "CLI 接續",
    agyRecovery:
      "Agy 尚無已驗證的替代資料目錄選項。此資料夾是恢復副本，請勿替換使用中的資料區；尚未驗證從另存目錄接續模型。",
    hermesRecovery:
      "Hermes Agent 會還原到隔離的受管理主目錄（HERMES_HOME）。此快照是原生 state.db 的一致副本；請勿把 HERMES_HOME 指向使用中的資料區。尚未宣稱跨裝置接續驗收。",
    cliHint: "開啟對話前先驗證還原的資料區。專案檔與憑證不會自動恢復。",
    copyCommand: "複製接續指令",
    restored: "還原資料夾",
    checking: "處理中…",
    contReadiness: "接續就緒狀態",
    contReady: "可以接續",
    contUnverified: "未驗證 — 尚未確認就緒",
    contBlocked: "受阻",
    contBlockers: "阻礙項目",
    contWarnings: "警告",
    contVerified: "已驗證",
    contProject: "已還原專案",
    contLegacy: "此版本未收到就緒報告；視為未驗證，而非就緒。",
    contBlockedAction:
      "在阻礙項目排除前，不提供接續指令。仍可還原到獨立資料夾。",
    contUnverifiedNote:
      "尚未確認就緒。接續前請確認模型憑證與相依項目；指令僅供參考。",
    contReadyNote: "此版本的相依項目已驗證。",
    contUnknown: "未辨識的需求",
    recheck: "重新檢查接續狀態",
    recheckHint:
      "為此還原版本重新檢查相依項目與記憶就緒狀態。不會自動套用任何內容——您安裝提供者外掛或調整綁定後，重新檢查會重新驗證已接收的快照。",
    setupProvider: "設定記憶提供者",
    setupProviderHint:
      "把受信任的本機 Agent Memory OS 提供者外掛安裝到此還原版本的隔離主目錄（agent-memory hermes install），並重新執行官方記憶匯入。不會執行接收的程式碼，也不會下載任何內容。",
    acknowledgeRequirements: "我已設定這些前置需求；重新檢查",
    acknowledgeHint:
      "記錄您已設定不透明需求（MCP 伺服器、模型提供者、憑證檔、主機應用程式）。這是明確的確認，不是驗證：這些項目只會列為警告，永不回報為已驗證；可驗證的前置需求（環境變數、指令、作業系統）仍會阻擋。",
    contAcknowledged: "已確認 — 仍未驗證",
    contResolved: "已在此電腦驗證",
  },
  "zh-Hans": {
    title: "同步控制中心",
    resources: "网络与时段",
    parallel: "同时同步的数据组数",
    upload: "上传上限（KiB/s）",
    download: "下载上限（KiB/s）",
    unlimited: "0 表示不限速。限制本程序的 Drive 数据传输；保存设置后应用。",
    window: "只在以下本地时段开始同步",
    from: "开始",
    to: "结束",
    hours: "起止相同表示全天；结束早于开始表示跨午夜。",
    pause: "暂停 15 分钟",
    resume: "恢复／立即同步",
    history: "最近同步记录（最多 500 轮）",
    devices: "设备列表 — 最后报告不代表实时在线",
    refresh: "刷新",
    storage: "存储空间",
    measure: "统计本地空间",
    cloudMeasure: "统计 Drive 同步对象",
    local: "本地程序数据",
    cache: "可重建的下载缓存",
    cloud: "Drive 同步对象",
    clean: "清除下载缓存",
    cleanHint: "请先暂停同步。只清除下载缓存，保留对话、记录、设置和云端历史。",
    empty: "暂无记录",
    error: "无法完成操作。必要时先暂停同步，再重试。",
    at: "本地获取时间",
    reported: "设备报告时间",
    version: "版本",
    complete: "完成",
    partial: "需要处理",
    paused: "已暂停",
    failed: "失败",
    scan: "检查文件",
    list: "列出云端对象",
    downloading: "检查／下载对象",
    uploading: "上传变更对象",
    restore: "还原／合并",
    progress: "已检查／处理项目",
    payload: "当前 HTTP 数据传输",
    windowWait: "等待允许同步的时段",
    timedPause: "定时暂停中",
    resumeAt: "预计恢复",
    portable: "便携设置与技能",
    portableSettings: "同步支持的便携设置",
    portableSkills: "同步技能与指令",
    portableHint:
      "仅为选中的 Agent 启用。排除凭据、机器路径和自动执行设置；接收后需检查，再还原到独立文件夹。",
    preview: "预览",
    incoming: "已接收的设置包",
    compare: "比较差异",
    keepBoth: "保留两份：还原到新文件夹",
    path: "文件",
    localVersion: "本地版本",
    remoteVersion: "接收版本",
    missing: "本地没有此文件",
    same: "内容相同",
    different: "内容不同，原文件已保留",
    content: "查看内容",
    acknowledge: "保留本地并标记已检查",
    reviewed: "已检查",
    continuation: "CLI 接续",
    agyRecovery:
      "Agy 尚无已验证的替代数据目录选项。此文件夹是恢复副本，请勿替换使用中的数据区；尚未验证从另存目录接续模型。",
    hermesRecovery:
      "Hermes Agent 会还原到隔离的受管主目录（HERMES_HOME）。此快照是原生 state.db 的一致副本；请勿把 HERMES_HOME 指向使用中的数据区。尚未声明跨设备接续验收。",
    cliHint: "打开对话前先验证还原的数据区。项目文件和凭据不会自动恢复。",
    copyCommand: "复制接续命令",
    restored: "还原文件夹",
    checking: "处理中…",
    contReadiness: "接续就绪状态",
    contReady: "可以接续",
    contUnverified: "未验证 — 尚未确认就绪",
    contBlocked: "受阻",
    contBlockers: "阻碍项",
    contWarnings: "警告",
    contVerified: "已验证",
    contProject: "已还原项目",
    contLegacy: "此版本未收到就绪报告；视为未验证，而非就绪。",
    contBlockedAction:
      "在阻碍项排除前，不提供接续命令。仍可还原到独立文件夹。",
    contUnverifiedNote:
      "尚未确认就绪。接续前请确认模型凭据与依赖项；命令仅供参考。",
    contReadyNote: "此版本的依赖项已验证。",
    contUnknown: "未识别的需求",
    recheck: "重新检查接续状态",
    recheckHint:
      "为此还原版本重新检查依赖项和记忆就绪状态。不会自动应用任何内容——您安装提供程序插件或调整绑定后，重新检查会重新验证已接收的快照。",
    setupProvider: "设置记忆提供程序",
    setupProviderHint:
      "把受信任的本地 Agent Memory OS 提供程序插件安装到此还原版本的隔离主目录（agent-memory hermes install），并重新执行官方记忆导入。不会运行接收的代码，也不会下载任何内容。",
    acknowledgeRequirements: "我已设置这些前置要求；重新检查",
    acknowledgeHint:
      "记录您已设置不透明要求（MCP 服务器、模型提供程序、凭据文件、主应用程序）。这是明确的确认，不是验证：这些项目只会列为警告，永不报告为已验证；可验证的前置要求（环境变量、命令、操作系统）仍会阻止。",
    contAcknowledged: "已确认 — 仍未验证",
    contResolved: "已在此电脑验证",
  },
  ja: {
    title: "同期コントロール",
    resources: "通信と時間帯",
    parallel: "同時に同期する保存領域数",
    upload: "送信上限（KiB/s）",
    download: "受信上限（KiB/s）",
    unlimited:
      "0 は無制限。Drive データ転送に適用し、設定保存後に有効になります。",
    window: "このローカル時間帯のみ同期開始",
    from: "開始",
    to: "終了",
    hours: "同時刻は終日、終了が開始より前なら日付をまたぎます。",
    pause: "15分間一時停止",
    resume: "再開／今すぐ同期",
    history: "最近の同期履歴（最大500回）",
    devices: "端末 — 最終報告であり現在のオンライン状態ではありません",
    refresh: "更新",
    storage: "ストレージ",
    measure: "ローカル容量を集計",
    cloudMeasure: "Drive 同期オブジェクトを集計",
    local: "ローカルアプリデータ",
    cache: "再構築可能な受信キャッシュ",
    cloud: "Drive 同期オブジェクト",
    clean: "受信キャッシュを消去",
    cleanHint:
      "先に同期を一時停止。会話、履歴、設定、クラウド履歴は保持します。",
    empty: "記録がありません",
    error:
      "操作を完了できません。必要に応じて同期を一時停止し再試行してください。",
    at: "確認日時",
    reported: "報告日時",
    version: "バージョン",
    complete: "完了",
    partial: "要確認",
    paused: "一時停止",
    failed: "失敗",
    scan: "ファイル確認",
    list: "クラウド一覧取得",
    downloading: "確認／ダウンロード",
    uploading: "変更をアップロード",
    restore: "復元／マージ",
    progress: "確認／処理済み項目",
    payload: "現在の HTTP データ転送",
    windowWait: "許可時間帯を待機中",
    timedPause: "指定時刻まで一時停止",
    resumeAt: "再開予定",
    portable: "移行可能な設定とスキル",
    portableSettings: "対応する移行可能な設定を同期",
    portableSkills: "スキルと指示を同期",
    portableHint:
      "選択した Agent のみ。認証情報、端末パス、自動実行設定を除外し、受信後に確認して別フォルダーへ復元します。",
    preview: "プレビュー",
    incoming: "受信した設定パッケージ",
    compare: "差分を比較",
    keepBoth: "両方保持：新規フォルダーへ復元",
    path: "ファイル",
    localVersion: "ローカル",
    remoteVersion: "受信版",
    missing: "ローカルに存在しません",
    same: "同一内容",
    different: "内容が異なります・元のファイルを保持",
    content: "内容を表示",
    acknowledge: "ローカルを保持し確認済みにする",
    reviewed: "確認済み",
    continuation: "CLI 再開",
    agyRecovery:
      "Agyの別プロファイル指定は未検証です。このフォルダーは復元コピーです。使用中の領域を置き換えないでください。別フォルダーからのモデル再開は未検証です。",
    hermesRecovery:
      "Hermes Agent は隔離された管理ホーム（HERMES_HOME）に復元します。スナップショットはネイティブ state.db の整合コピーです。使用中の領域を HERMES_HOME に指定しないでください。クロスデバイスでの再開受入は未検証です。",
    cliHint:
      "会話を開く前に復元先を検証してください。プロジェクトや認証情報は自動復元されません。",
    copyCommand: "再開コマンドをコピー",
    restored: "復元先",
    checking: "処理中…",
    contReadiness: "再開の準備状況",
    contReady: "再開可能",
    contUnverified: "未検証 — 準備完了は未確認",
    contBlocked: "ブロック中",
    contBlockers: "阻害要因",
    contWarnings: "警告",
    contVerified: "検証済み",
    contProject: "復元したプロジェクト",
    contLegacy:
      "この版の準備状況レポートを受信していません。準備完了ではなく未検証として扱います。",
    contBlockedAction:
      "これらの阻害要因が解消するまで再開コマンドは提供されません。隔離フォルダーへの復元は引き続き可能です。",
    contUnverifiedNote:
      "準備完了は未確認です。再開前にモデルの認証情報と依存関係を確認してください。コマンドは参考表示です。",
    contReadyNote: "この版の依存関係は検証済みです。",
    contUnknown: "不明な要件",
    recheck: "再開の準備状況を再確認",
    recheckHint:
      "この復元済みの版について依存関係とメモリの準備状況を再確認します。自動的に適用されるものはありません。プロバイダープラグインの導入やバインドの変更後に、再確認で受信スナップショットを再検証します。",
    setupProvider: "メモリプロバイダーを設定",
    setupProviderHint:
      "信頼済みのローカル Agent Memory OS プロバイダープラグインをこの復元済みの版の隔離ホームへ導入し（agent-memory hermes install）、公式のメモリ取り込みを再実行します。受信したコードは実行せず、ダウンロードも行いません。",
    acknowledgeRequirements: "これらの前提を設定しました。再確認",
    acknowledgeHint:
      "不透明な要件（MCP サーバー、モデルプロバイダー、認証情報ファイル、ホストアプリ）を設定したことを記録します。これは明示的な確認であり検証ではありません。これらの項目は警告のままで、検証済みとして報告されることはなく、検証可能な前提（環境変数、コマンド、OS）は引き続きブロックします。",
    contAcknowledged: "確認済み — 未検証のまま",
    contResolved: "この端末で検証済み",
  },
  ko: {
    title: "동기화 제어",
    resources: "네트워크와 시간대",
    parallel: "동시 저장소 그룹 수",
    upload: "업로드 제한 (KiB/s)",
    download: "다운로드 제한 (KiB/s)",
    unlimited:
      "0은 무제한입니다. 앱의 Drive 전송에 적용하며 설정 저장 후 반영됩니다.",
    window: "이 로컬 시간대에만 동기화 시작",
    from: "시작",
    to: "종료",
    hours: "같은 시각은 하루 종일, 종료가 시작보다 이르면 자정을 넘깁니다.",
    pause: "15분 일시 중지",
    resume: "재개 / 지금 동기화",
    history: "최근 동기화 기록 (최대 500회)",
    devices: "장치 — 마지막 보고이며 실시간 온라인 상태가 아닙니다",
    refresh: "새로 고침",
    storage: "저장 공간",
    measure: "로컬 공간 집계",
    cloudMeasure: "Drive 동기화 객체 집계",
    local: "로컬 앱 데이터",
    cache: "재생성 가능한 다운로드 캐시",
    cloud: "Drive 동기화 객체",
    clean: "다운로드 캐시 지우기",
    cleanHint:
      "먼저 동기화를 중지하세요. 대화, 기록, 설정, 클라우드 기록은 유지합니다.",
    empty: "기록 없음",
    error:
      "작업을 완료하지 못했습니다. 필요하면 동기화를 중지하고 다시 시도하세요.",
    at: "확인 시각",
    reported: "보고 시각",
    version: "버전",
    complete: "완료",
    partial: "확인 필요",
    paused: "일시 중지",
    failed: "실패",
    scan: "파일 확인",
    list: "클라우드 목록 조회",
    downloading: "확인 / 다운로드",
    uploading: "변경사항 업로드",
    restore: "복원 / 병합",
    progress: "확인 / 처리한 항목",
    payload: "현재 HTTP 데이터 전송",
    windowWait: "허용 시간대 대기",
    timedPause: "예약 재개까지 일시 중지",
    resumeAt: "재개 예정",
    portable: "이동 가능한 설정과 스킬",
    portableSettings: "지원하는 이동 가능 설정 동기화",
    portableSkills: "스킬과 지침 동기화",
    portableHint:
      "선택한 Agent만 적용합니다. 자격 증명, 장치 경로, 자동 실행 설정은 제외하며 수신 후 검토하여 별도 폴더로 복원합니다.",
    preview: "미리 보기",
    incoming: "수신한 설정 패키지",
    compare: "차이 비교",
    keepBoth: "둘 다 보관: 새 폴더로 복원",
    path: "파일",
    localVersion: "로컬",
    remoteVersion: "수신본",
    missing: "로컬에 없음",
    same: "동일한 내용",
    different: "내용이 다름 · 원본 보관",
    content: "내용 보기",
    acknowledge: "로컬 보관 및 검토 완료 표시",
    reviewed: "검토 완료",
    continuation: "CLI 재개",
    agyRecovery:
      "Agy의 대체 프로필 옵션은 검증되지 않았습니다. 이 폴더는 복원 사본이며 사용 중인 프로필을 교체하지 마세요. 별도 폴더에서 모델 재개는 검증되지 않았습니다.",
    hermesRecovery:
      "Hermes Agent는 격리된 관리 홈(HERMES_HOME)에 복원합니다. 스냅숏은 네이티브 state.db의 일관된 사본입니다. 사용 중인 프로필을 HERMES_HOME으로 지정하지 마세요. 교차 기기 재개 검증은 아직 주장하지 않습니다.",
    cliHint:
      "대화를 열기 전에 복원된 저장소를 검증하세요. 프로젝트 파일과 자격 증명은 자동 복원하지 않습니다.",
    copyCommand: "재개 명령 복사",
    restored: "복원 폴더",
    checking: "처리 중…",
    contReadiness: "이어하기 준비 상태",
    contReady: "이어하기 가능",
    contUnverified: "미검증 — 준비 완료 미확인",
    contBlocked: "차단됨",
    contBlockers: "차단 요인",
    contWarnings: "경고",
    contVerified: "검증됨",
    contProject: "복원된 프로젝트",
    contLegacy:
      "이 버전에 대한 준비 상태 보고를 받지 못했습니다. 준비 완료가 아니라 미검증으로 처리합니다.",
    contBlockedAction:
      "이 차단 요인이 해결될 때까지 이어하기 명령을 제공하지 않습니다. 격리 폴더로 복원은 계속 가능합니다.",
    contUnverifiedNote:
      "준비 완료로 확인되지 않았습니다. 이어하기 전에 모델 자격 증명과 종속 항목을 확인하세요. 명령은 참고용입니다.",
    contReadyNote: "이 버전의 종속 항목이 검증되었습니다.",
    contUnknown: "인식되지 않은 요구 사항",
    recheck: "이어하기 재점검",
    recheckHint:
      "이 복원된 버전의 종속 항목과 메모리 준비 상태를 다시 확인합니다. 자동으로 적용되는 것은 없습니다. 공급자 플러그인을 설치하거나 바인딩을 변경한 뒤 재점검하면 수신한 스냅숏을 다시 검증합니다.",
    setupProvider: "메모리 공급자 설정",
    setupProviderHint:
      "신뢰할 수 있는 로컬 Agent Memory OS 공급자 플러그인을 이 복원된 버전의 격리된 홈에 설치하고(agent-memory hermes install) 공식 메모리 가져오기를 다시 실행합니다. 수신한 코드를 실행하지 않으며 아무것도 내려받지 않습니다.",
    acknowledgeRequirements: "이 필수 요건을 설정했습니다. 재점검",
    acknowledgeHint:
      "불투명 요건(MCP 서버, 모델 공급자, 자격 증명 파일, 호스트 앱)을 설정했음을 기록합니다. 이는 명시적 확인이며 검증이 아닙니다. 해당 항목은 경고로만 남고 검증됨으로 보고되지 않으며, 검증 가능한 필수 요건(환경 변수, 명령, 운영 체제)은 계속 차단합니다.",
    contAcknowledged: "확인됨 — 아직 미검증",
    contResolved: "이 컴퓨터에서 검증됨",
  },
};


// Continuation readiness machine codes -> localized, actionable guidance. A code
// absent from a table is never hidden: continuationIssueText reports the raw
// code, so required setup is always visible instead of a blanket "unsupported".
const continuationIssue: Record<Locale, Record<string, string>> = {
  en: {
    project_mapping_required:
      "Map this source project to a local folder in Project mappings, then sync and restore again. Project files are not copied without that authorization.",
    project_mapping_target_missing:
      "The mapped local project folder does not exist. Choose an existing folder, then restore again.",
    project_mapping_invalid:
      "Project mappings must use absolute paths, with one destination per source.",
    project_mapping_conflict:
      "The same source project is mapped to more than one destination. Keep a single mapping.",
    project_mapping_format_unsupported:
      "This project mapping format has no verified migration yet. Restore the snapshot to a new folder instead.",
    project_no_commit:
      "The source repository has no commit yet. Create an initial commit on the sender, then sync again.",
    project_git_unavailable:
      "Git is unavailable on the sender, so the working tree could not be captured. Install git and sync again.",
    project_submodule:
      "A git submodule cannot be packaged. Initialize the submodule on this computer after restore.",
    project_selected_empty:
      "No project files were selected, so the working tree is incomplete. Select the files to include, then sync again.",
    project_limit:
      "The project exceeds the capture size limit. Reduce the selected files or exclude large paths, then sync again.",
    project_missing_files:
      "Some project files were missing when captured. Recreate or restore them before continuing.",
    project_missing_lfs:
      "A Git LFS object was unavailable when captured. Fetch the LFS files before continuing.",
    project_baseline_required:
      "This snapshot is a partial working tree. Reconstruct the remaining baseline before continuing.",
    memory_provider_unknown:
      "The source uses an unrecognized memory provider. Select the Agent Memory OS provider explicitly, then sync again.",
    memory_provider_unresolved:
      "The memory provider could not be resolved. Set memory provider agent-memory-os in config.yaml on the sender, then sync again.",
    memory_identity_missing:
      "No external memory identity was found. Sign in to Agent Memory OS on the sender, then sync again.",
    memory_export_unauthorized:
      "The memory export was not authorized. Opt in to the Agent Memory OS source and approve the export on the sender.",
    memory_cli_missing:
      "The Agent Memory OS CLI was not found. Install it on this computer, then restore the memory again.",
    memory_cli_failed:
      "The Agent Memory OS CLI failed during import. Check its output, then retry.",
    memory_store_missing:
      "The external memory store was not found at the bound home. Verify the Agent Memory OS home, then sync again.",
    memory_unavailable_store:
      "External memory is unavailable in this build. Use an authorized Agent Memory OS installation.",
    workspace_format:
      "The portable settings and skills payload could not be parsed. Sync again from the sender.",
    workspace_limit:
      "The portable settings and skills payload exceeds the limit. Reduce the selection, then sync again.",
    workspace_unsafe:
      "The portable settings and skills payload was rejected as unsafe. Sync again from the sender.",
    hermes_attachment_missing:
      "An attached file was missing when captured. Re-supply it on the sender, then sync again.",
    hermes_attachment_credential:
      "An attachment looked like a credential and was excluded. Provide it separately; it is never transferred.",
    hermes_attachment_outside_roots:
      "An attachment referenced a path outside the authorized roots. Add an authorized root, then sync again.",
    hermes_attachment_unsupported:
      "An attachment type is not supported for transport. Provide it separately.",
    hermes_parent_missing:
      "A required native parent conversation is missing. Re-sync the parent from the sender, then restore again.",
    model_credentials_required:
      "Model credentials are not transferred. Sign in or set the provider credentials on this computer's Hermes install before continuing.",
    model_provider_unknown:
      "The configured model provider is unrecognized. Set model.provider and its credentials on this computer before continuing.",
    mcp_binding_required:
      "MCP servers need a recipient binding. Re-bind the required MCP environment variables on this computer before continuing.",
    required_environment_variables:
      "Required environment variables were not transferred. Set them on this computer before continuing.",

    // Workspace aggregate (`workspace_{reason}` / `workspace_dependency_{kind}`)
    // emitted by native_sessions::continuation_for.
    workspace_file_count_limit:
      "The portable settings and skills selection hit the file-count limit, so some files were excluded. Reduce the selection, then sync again.",
    workspace_oversized:
      "A settings or skill file exceeded the size limit and was excluded. Trim that file, then sync again.",
    workspace_total_limit:
      "The portable settings and skills payload exceeded the total size limit. Reduce the selection, then sync again.",
    workspace_suspected_credential:
      "A file looked like a credential and was excluded. Add it separately; it is never transferred.",
    workspace_missing:
      "A referenced settings or skill file was missing, or the payload was absent. Sync the source workspace again.",
    workspace_depth_limit:
      "A skill directory nested deeper than allowed and was excluded. Flatten it, then sync again.",
    workspace_symlink:
      "A symlinked path was excluded. Copy the real file into the workspace, then sync again.",
    workspace_excluded_directory:
      "A version-control or cache directory was excluded. This is expected unless it held a required file.",
    workspace_unsupported_or_sensitive_path:
      "An unsupported or sensitive path was excluded. Provide it separately.",
    workspace_unsafe_store:
      "A symlinked or unsafe skills store was excluded. Use a real directory, then sync again.",
    workspace_missing_required_skill_asset:
      "A skill referenced a support file that was not captured, so the restored skill is incomplete. Sync the source skill again with all its required assets.",
    workspace_unparseable_config:
      "config.yaml could not be sanitized. Fix its YAML, then sync again from the sender.",
    workspace_non_text:
      "config.yaml was not valid UTF-8 text and was excluded. Save it as text, then sync again.",
    workspace_invalid:
      "The portable settings and skills payload could not be read. Sync again from the sender.",
    workspace_dependency_mcp:
      "The restored config names an MCP server whose command, arguments and secrets were not transferred. Install and bind it on this computer before continuing.",
    workspace_dependency_model:
      "A custom model provider endpoint or login material was dropped. Configure the provider and model credentials on this computer before continuing.",
    workspace_dependency_memory:
      "An external memory provider is selected whose bound store and identity are not portable. Bind them on this computer before continuing.",
    workspace_dependency_os:
      "A skill requires a supported operating system. Continue only on a listed platform.",
    workspace_dependency_env:
      "A skill requires environment variables that are not transferred. Set them on this computer before continuing.",
    workspace_dependency_runtime:
      "A skill requires an interpreter or tool package that is not transferred. Install it on this computer before continuing.",
    workspace_dependency_credential:
      "A skill requires credential files. They are never transferred; configure them locally and verify access before continuing.",
    workspace_dependency_app:
      "A skill requires host applications or plugins. Review its prerequisites and enable them on this computer before continuing.",
    // Attachment namespace aggregated by continuation_for.
    attachment_missing:
      "An attached file was missing when captured. Re-supply it on the sender, then sync again.",
    attachment_symlink:
      "An attachment was a symlink and was excluded. Reference the real file instead.",
    attachment_credential:
      "An attachment looked like a credential and was excluded. Provide it separately; it is never transferred.",
    attachment_outside_roots:
      "An attachment referenced a path outside the authorized roots. Add an authorized root, then sync again.",
    attachment_oversized:
      "An attachment exceeded the size limit. Provide it separately.",
    attachment_limit:
      "An attachment hit the transport limit. Provide it separately.",
    attachment_unreadable:
      "An attachment could not be read on the sender. Fix it, then sync again.",
    attachment_unsupported:
      "An attachment type is not supported for transport. Provide it separately.",
    // Project continuity codes added by hermes_project / project_mapping.
    project_non_utf8_path:
      "The project contains a path that is not valid UTF-8 and cannot be packaged. Rename it, then sync again.",
    project_hash_mismatch:
      "A project file did not match its recorded hash and was refused. Re-sync the project from the sender.",
    project_unreadable:
      "The project could not be read on the sender. Fix its permissions or location, then sync again.",
    project_absent:
      "The snapshot carries no project working tree. Map and select the project on the sender, then sync again.",
    project_invalid:
      "The project namespace could not be read. Re-sync the project from the sender.",
    // External memory codes emitted by the continuation memory namespace.
    memory_provider_not_installed:
      "The captured memory needs its provider plugin, which was not received. Review and approve installing the trusted provider plugin on this computer, then retry the memory restore.",
    memory_store_diverged:
      "The existing memory store differs from the captured snapshot, so it was not overwritten. Your changed store is preserved; recapture a fresh version on the sender, then sync again.",
    memory_export_failed:
      "The Agent Memory OS export failed on the sender. Check its output, then sync again.",
    memory_export_missing:
      "The snapshot carries no exported memory to import. Enable the memory source on the sender, then sync again.",
    memory_invalid:
      "The memory payload could not be read. Sync again from the sender.",
    memory_unverified:
      "No verified external-memory binding was found. Bind Agent Memory OS on this computer before continuing.",
  },
  "zh-Hant": {
    project_mapping_required:
      "請在「專案對應」把此來源專案對應到本機資料夾，再重新同步與還原。未授權時不會複製專案檔。",
    project_mapping_target_missing:
      "對應的本機專案資料夾不存在；請選擇已有的資料夾，再重新還原。",
    project_mapping_invalid:
      "專案對應必須使用絕對路徑，且每個來源只能有一個目的地。",
    project_mapping_conflict:
      "同一來源專案對應到多個目的地；請只保留一個對應。",
    project_mapping_format_unsupported:
      "此外掛專案格式尚無可驗證的移植方式；請改把快照還原到新的資料夾。",
    project_no_commit:
      "來源儲存庫尚無提交；請先在傳送端建立首次提交，再重新同步。",
    project_git_unavailable:
      "傳送端無法使用 git，工作區未能擷取；請安裝 git 後重新同步。",
    project_submodule:
      "git 子模組無法封裝；還原後請在此電腦初始化子模組。",
    project_selected_empty:
      "未選取任何專案檔，工作區不完整；請選取要包含的檔案後重新同步。",
    project_limit:
      "專案超過擷取容量上限；請減少選取的檔案或排除大型路徑後重新同步。",
    project_missing_files:
      "擷取時部分專案檔缺少；請在接續前重建或還原這些檔案。",
    project_missing_lfs:
      "擷取時 Git LFS 物件無法取得；請先取得 LFS 檔案再接續。",
    project_baseline_required:
      "此快照是部分工作區；請在接續前重建其餘基準。",
    memory_provider_unknown:
      "來源使用無法辨識的記憶提供者；請明確選用 Agent Memory OS 提供者後重新同步。",
    memory_provider_unresolved:
      "無法解析記憶提供者；請在傳送端的 config.yaml 設定 memory provider 為 agent-memory-os，再重新同步。",
    memory_identity_missing:
      "找不到外部記憶身分；請在傳送端登入 Agent Memory OS 後重新同步。",
    memory_export_unauthorized:
      "記憶匯出未獲授權；請在傳送端啟用 Agent Memory OS 來源並核准匯出。",
    memory_cli_missing:
      "找不到 Agent Memory OS CLI；請在此電腦安裝後再還原記憶。",
    memory_cli_failed:
      "Agent Memory OS CLI 匯入失敗；請檢查其輸出後重試。",
    memory_store_missing:
      "在綁定的主目錄找不到外部記憶庫；請確認 Agent Memory OS 主目錄後重新同步。",
    memory_unavailable_store:
      "此版本無法使用外部記憶；請使用已授權的 Agent Memory OS 安裝。",
    workspace_format:
      "可攜設定與技能內容無法解析；請從傳送端重新同步。",
    workspace_limit:
      "可攜設定與技能內容超過上限；請減少選取後重新同步。",
    workspace_unsafe:
      "可攜設定與技能內容被判定為不安全；請從傳送端重新同步。",
    hermes_attachment_missing:
      "擷取時缺少附件檔；請在傳送端重新提供後同步。",
    hermes_attachment_credential:
      "某附件疑似憑證而被排除；請另行提供，附件永不傳輸。",
    hermes_attachment_outside_roots:
      "某附件指向授權根目錄之外的路徑；請加入授權根目錄後重新同步。",
    hermes_attachment_unsupported:
      "不支援傳輸此附件類型；請另行提供。",
    hermes_parent_missing:
      "缺少必要的原生父對話；請從傳送端重新同步父對話後再還原。",
    model_credentials_required:
      "模型憑證不會傳輸；請在此電腦的 Hermes 安裝登入或設定提供者憑證後再接續。",
    model_provider_unknown:
      "設定的模型提供者無法辨識；請在此電腦設定 model.provider 及其憑證後再接續。",
    mcp_binding_required:
      "MCP 伺服器需要收件端綁定；請在此電腦重新綁定必要的 MCP 環境變數後再接續。",
    required_environment_variables:
      "必要的環境變數不會傳輸；請在此電腦設定後再接續。",

    workspace_file_count_limit:
      "可攜設定與技能的選取達到檔案數上限，部分檔案未包含；請減少選取後重新同步。",
    workspace_oversized:
      "某設定或技能檔超過大小上限而被排除；請精簡該檔後重新同步。",
    workspace_total_limit:
      "可攜設定與技能內容超過總量上限；請減少選取後重新同步。",
    workspace_suspected_credential:
      "某檔案疑似憑證而被排除；請另行提供，該檔永不傳輸。",
    workspace_missing:
      "缺少被引用的設定或技能檔，或內容不存在；請從傳送端重新同步工作區。",
    workspace_depth_limit:
      "某技能目錄層級過深而被排除；請攤平後重新同步。",
    workspace_symlink:
      "符號連結路徑被排除；請把實體檔複製進工作區後重新同步。",
    workspace_excluded_directory:
      "版本控制或快取目錄被排除；除非其中含必要檔案，否則屬預期行為。",
    workspace_unsupported_or_sensitive_path:
      "不支援或敏感的路徑被排除；請另行提供。",
    workspace_unsafe_store:
      "符號連結或不安全的技能儲存區被排除；請改用實體目錄後重新同步。",
    workspace_missing_required_skill_asset:
      "某技能引用的支援檔未被擷取，還原後的技能不完整；請帶齊所有必要資產後重新同步來源技能。",
    workspace_unparseable_config:
      "config.yaml 無法淨化；請修正其 YAML 後從傳送端重新同步。",
    workspace_non_text:
      "config.yaml 不是有效的 UTF-8 文字而被排除；請存成文字後重新同步。",
    workspace_invalid:
      "可攜設定與技能內容無法讀取；請從傳送端重新同步。",
    workspace_dependency_mcp:
      "還原的設定指定了一個 MCP 伺服器，但其指令、參數與密鑰未傳輸；請在此電腦安裝並綁定後再接續。",
    workspace_dependency_model:
      "自訂模型提供者的端點或登入資訊已被捨棄；請在此電腦設定提供者與模型憑證後再接續。",
    workspace_dependency_memory:
      "來源選用了外部記憶提供者，其綁定的儲存與身分無法可攜；請在此電腦綁定後再接續。",
    workspace_dependency_os:
      "某技能需要受支援的作業系統；請只在列出的平台上接續。",
    workspace_dependency_env:
      "某技能需要未傳輸的環境變數；請在此電腦設定後再接續。",
    workspace_dependency_runtime:
      "某技能需要未傳輸的解譯器或工具套件；請在此電腦安裝後再接續。",
    workspace_dependency_credential:
      "某技能需要憑證檔案；憑證永不傳輸。請在此電腦另行設定並確認可存取後再接續。",
    workspace_dependency_app:
      "某技能需要主機上的應用程式或外掛；請檢查技能需求並在此電腦啟用後再接續。",
    attachment_missing:
      "擷取時缺少附件檔；請在傳送端重新提供後同步。",
    attachment_symlink:
      "某附件是符號連結而被排除；請改為引用實體檔。",
    attachment_credential:
      "某附件疑似憑證而被排除；請另行提供，附件永不傳輸。",
    attachment_outside_roots:
      "某附件指向授權根目錄之外的路徑；請加入授權根目錄後重新同步。",
    attachment_oversized:
      "某附件超過大小上限；請另行提供。",
    attachment_limit:
      "某附件達到傳輸上限；請另行提供。",
    attachment_unreadable:
      "某附件在傳送端無法讀取；請修正後重新同步。",
    attachment_unsupported:
      "不支援傳輸此附件類型；請另行提供。",
    project_non_utf8_path:
      "專案含非有效 UTF-8 的路徑而無法封裝；請改名後重新同步。",
    project_hash_mismatch:
      "某專案檔與記錄的雜湊不符而被拒絕；請從傳送端重新同步該專案。",
    project_unreadable:
      "傳送端無法讀取專案；請修正權限或位置後重新同步。",
    project_absent:
      "此快照不含專案工作區；請在傳送端對應並選取專案後重新同步。",
    project_invalid:
      "無法讀取專案命名空間；請從傳送端重新同步專案。",
    memory_provider_not_installed:
      "擷取的記憶需要其提供者外掛，但外掛未隨附抵達。請檢視並同意在此電腦安裝受信任的提供者外掛，再重試記憶還原。",
    memory_store_diverged:
      "現有記憶庫與擷取的快照不同，因此未覆寫；您變更過的記憶庫已保留。請在傳送端重新擷取新版本後再同步。",
    memory_export_failed:
      "傳送端的 Agent Memory OS 匯出失敗；請檢查其輸出後重新同步。",
    memory_export_missing:
      "此快照不含可匯入的記憶匯出；請在傳送端啟用記憶來源後重新同步。",
    memory_invalid:
      "記憶內容無法讀取；請從傳送端重新同步。",
    memory_unverified:
      "找不到已驗證的外部記憶綁定；請在此電腦綁定 Agent Memory OS 後再接續。",
  },
  "zh-Hans": {
    project_mapping_required:
      "请在“项目映射”中把此来源项目映射到本机文件夹，再重新同步和还原。未授权时不会复制项目文件。",
    project_mapping_target_missing:
      "映射的本机项目文件夹不存在；请选择已有的文件夹，再重新还原。",
    project_mapping_invalid:
      "项目映射必须使用绝对路径，且每个来源只能有一个目标。",
    project_mapping_conflict:
      "同一来源项目映射到多个目标；请只保留一个映射。",
    project_mapping_format_unsupported:
      "此外挂项目格式尚无可验证的迁移方式；请改将快照还原到新文件夹。",
    project_no_commit:
      "来源仓库尚无提交；请先在发送端建立首次提交，再重新同步。",
    project_git_unavailable:
      "发送端无法使用 git，工作区未能捕获；请安装 git 后重新同步。",
    project_submodule:
      "git 子模块无法打包；还原后请在此电脑初始化子模块。",
    project_selected_empty:
      "未选择任何项目文件，工作区不完整；请选择要包含的文件后重新同步。",
    project_limit:
      "项目超过捕获容量上限；请减少选择的文件或排除大型路径后重新同步。",
    project_missing_files:
      "捕获时部分项目文件缺失；请在接续前重建或还原这些文件。",
    project_missing_lfs:
      "捕获时 Git LFS 对象不可用；请先获取 LFS 文件后再接续。",
    project_baseline_required:
      "此快照是部分工作区；请在接续前重建其余基线。",
    memory_provider_unknown:
      "来源使用了无法识别的记忆提供程序；请明确选用 Agent Memory OS 提供程序后重新同步。",
    memory_provider_unresolved:
      "无法解析记忆提供程序；请在发送端的 config.yaml 将 memory provider 设为 agent-memory-os，再重新同步。",
    memory_identity_missing:
      "找不到外部记忆身份；请在发送端登录 Agent Memory OS 后重新同步。",
    memory_export_unauthorized:
      "记忆导出未获授权；请在发送端启用 Agent Memory OS 来源并批准导出。",
    memory_cli_missing:
      "找不到 Agent Memory OS CLI；请在此电脑安装后再还原记忆。",
    memory_cli_failed:
      "Agent Memory OS CLI 导入失败；请检查其输出后重试。",
    memory_store_missing:
      "在绑定的主目录找不到外部记忆库；请确认 Agent Memory OS 主目录后重新同步。",
    memory_unavailable_store:
      "此版本无法使用外部记忆；请使用已授权的 Agent Memory OS 安装。",
    workspace_format:
      "便携设置与技能内容无法解析；请从发送端重新同步。",
    workspace_limit:
      "便携设置与技能内容超过上限；请减少选择后重新同步。",
    workspace_unsafe:
      "便携设置与技能内容被判定为不安全；请从发送端重新同步。",
    hermes_attachment_missing:
      "捕获时缺少附件文件；请在发送端重新提供后同步。",
    hermes_attachment_credential:
      "某个附件疑似凭据而被排除；请另行提供，附件永不传输。",
    hermes_attachment_outside_roots:
      "某个附件指向授权根目录之外的路径；请加入授权根目录后重新同步。",
    hermes_attachment_unsupported:
      "不支持传输此附件类型；请另行提供。",
    hermes_parent_missing:
      "缺少必要的原生父对话；请从发送端重新同步父对话后再还原。",
    model_credentials_required:
      "模型凭据不会传输；请在此电脑的 Hermes 安装登录或设置提供程序凭据后再接续。",
    model_provider_unknown:
      "配置的模型提供程序无法识别；请在此电脑设置 model.provider 及其凭据后再接续。",
    mcp_binding_required:
      "MCP 服务器需要收件端绑定；请在此电脑重新绑定必要的 MCP 环境变量后再接续。",
    required_environment_variables:
      "必要的环境变量不会传输；请在此电脑设置后再接续。",

    workspace_file_count_limit:
      "便携设置与技能的选取达到文件数上限，部分文件未包含；请减少选择后重新同步。",
    workspace_oversized:
      "某个设置或技能文件超过大小上限而被排除；请精简该文件后重新同步。",
    workspace_total_limit:
      "便携设置与技能内容超过总量上限；请减少选择后重新同步。",
    workspace_suspected_credential:
      "某个文件疑似凭据而被排除；请另行提供，该文件永不传输。",
    workspace_missing:
      "缺少被引用的设置或技能文件，或内容不存在；请从发送端重新同步工作区。",
    workspace_depth_limit:
      "某个技能目录层级过深而被排除；请压平后重新同步。",
    workspace_symlink:
      "符号链接路径被排除；请把实体文件复制进工作区后重新同步。",
    workspace_excluded_directory:
      "版本控制或缓存目录被排除；除非其中含必需文件，否则属预期行为。",
    workspace_unsupported_or_sensitive_path:
      "不受支持或敏感的路径被排除；请另行提供。",
    workspace_unsafe_store:
      "符号链接或不安全的技能存储区被排除；请改用实体目录后重新同步。",
    workspace_missing_required_skill_asset:
      "某个技能引用的支持文件未被捕获，还原后的技能不完整；请带齐所有必需资产后重新同步来源技能。",
    workspace_unparseable_config:
      "config.yaml 无法净化；请修正其 YAML 后从发送端重新同步。",
    workspace_non_text:
      "config.yaml 不是有效的 UTF-8 文本而被排除；请存为文本后重新同步。",
    workspace_invalid:
      "便携设置与技能内容无法读取；请从发送端重新同步。",
    workspace_dependency_mcp:
      "还原的配置指定了一个 MCP 服务器，但其命令、参数和密钥未传输；请在此电脑安装并绑定后再接续。",
    workspace_dependency_model:
      "自定义模型提供程序的端点或登录信息已被丢弃；请在此电脑设置提供程序与模型凭据后再接续。",
    workspace_dependency_memory:
      "来源选用了外部记忆提供程序，其绑定的存储与身份不可便携；请在此电脑绑定后再接续。",
    workspace_dependency_os:
      "某个技能需要受支持的操作系统；请只在列出的平台上接续。",
    workspace_dependency_env:
      "某个技能需要未传输的环境变量；请在此电脑设置后再接续。",
    workspace_dependency_runtime:
      "某个技能需要未传输的解释器或工具包；请在此电脑安装后再接续。",
    workspace_dependency_credential:
      "某个技能需要凭据文件；凭据永不传输。请在此电脑另行设置并确认可访问后再接续。",
    workspace_dependency_app:
      "某个技能需要主机上的应用程序或插件；请检查技能要求并在此电脑启用后再接续。",
    attachment_missing:
      "捕获时缺少附件文件；请在发送端重新提供后同步。",
    attachment_symlink:
      "某个附件是符号链接而被排除；请改为引用实体文件。",
    attachment_credential:
      "某个附件疑似凭据而被排除；请另行提供，附件永不传输。",
    attachment_outside_roots:
      "某个附件指向授权根目录之外的路径；请加入授权根目录后重新同步。",
    attachment_oversized:
      "某个附件超过大小上限；请另行提供。",
    attachment_limit:
      "某个附件达到传输上限；请另行提供。",
    attachment_unreadable:
      "某个附件在发送端无法读取；请修正后重新同步。",
    attachment_unsupported:
      "不支持传输此附件类型；请另行提供。",
    project_non_utf8_path:
      "项目含非有效 UTF-8 的路径而无法打包；请改名后重新同步。",
    project_hash_mismatch:
      "某个项目文件与记录的哈希不符而被拒绝；请从发送端重新同步该项目。",
    project_unreadable:
      "发送端无法读取项目；请修正权限或位置后重新同步。",
    project_absent:
      "此快照不含项目工作区；请在发送端映射并选择项目后重新同步。",
    project_invalid:
      "无法读取项目命名空间；请从发送端重新同步项目。",
    memory_provider_not_installed:
      "捕获的记忆需要其提供程序插件，但插件未随附到达。请检查并同意在此电脑安装受信任的提供程序插件，再重试记忆还原。",
    memory_store_diverged:
      "现有记忆库与捕获的快照不同，因此未覆盖；您更改过的记忆库已保留。请在发送端重新捕获新版本后再同步。",
    memory_export_failed:
      "发送端的 Agent Memory OS 导出失败；请检查其输出后重新同步。",
    memory_export_missing:
      "此快照不含可导入的记忆导出；请在发送端启用记忆来源后重新同步。",
    memory_invalid:
      "记忆内容无法读取；请从发送端重新同步。",
    memory_unverified:
      "找不到已验证的外部记忆绑定；请在此电脑绑定 Agent Memory OS 后再接续。",
  },
  ja: {
    project_mapping_required:
      "このソースプロジェクトを「プロジェクト対応付け」でローカルフォルダーに対応付けしてから、再同期して復元してください。許可がない限りプロジェクトファイルはコピーされません。",
    project_mapping_target_missing:
      "対応付けたローカルプロジェクトフォルダーが存在しません。既存のフォルダーを選んで再度復元してください。",
    project_mapping_invalid:
      "プロジェクト対応付けは絶対パスで、各ソースにつき一つの保存先にしてください。",
    project_mapping_conflict:
      "同じソースプロジェクトが複数の保存先に対応付けられています。対応付けを一つにしてください。",
    project_mapping_format_unsupported:
      "このプロジェクト対応付け形式には検証済みの移行方法がありません。代わりにスナップショットを新規フォルダーへ復元してください。",
    project_no_commit:
      "ソースリポジトリにコミットがありません。送信側で初回コミットを作成してから再同期してください。",
    project_git_unavailable:
      "送信側で git を利用できないため作業ツリーを取得できませんでした。git を導入して再同期してください。",
    project_submodule:
      "git サブモジュールは梱包できません。復元後にこの端末でサブモジュールを初期化してください。",
    project_selected_empty:
      "プロジェクトファイルが未選択のため作業ツリーが不完全です。含めるファイルを選択して再同期してください。",
    project_limit:
      "プロジェクトが取得サイズ上限を超えています。選択ファイルを減らすか大きなパスを除外して再同期してください。",
    project_missing_files:
      "取得時に一部のプロジェクトファイルが欠落していました。再開前に再作成または復元してください。",
    project_missing_lfs:
      "取得時に Git LFS オブジェクトを利用できませんでした。再開前に LFS ファイルを取得してください。",
    project_baseline_required:
      "このスナップショットは部分的な作業ツリーです。再開前に残りのベースラインを再構築してください。",
    memory_provider_unknown:
      "ソースが未知のメモリプロバイダーを使用しています。Agent Memory OS プロバイダーを明示的に選んで再同期してください。",
    memory_provider_unresolved:
      "メモリプロバイダーを解決できません。送信側の config.yaml で memory provider を agent-memory-os に設定して再同期してください。",
    memory_identity_missing:
      "外部メモリの身元が見つかりません。送信側で Agent Memory OS にサインインして再同期してください。",
    memory_export_unauthorized:
      "メモリのエクスポートが許可されていません。送信側で Agent Memory OS ソースを有効化しエクスポートを承認してください。",
    memory_cli_missing:
      "Agent Memory OS CLI が見つかりません。この端末に導入してからメモリを再度復元してください。",
    memory_cli_failed:
      "Agent Memory OS CLI の取り込みに失敗しました。出力を確認して再試行してください。",
    memory_store_missing:
      "バインドしたホームに外部メモリストアが見つかりません。Agent Memory OS ホームを確認して再同期してください。",
    memory_unavailable_store:
      "このビルドでは外部メモリを利用できません。認可済みの Agent Memory OS を使用してください。",
    workspace_format:
      "移行可能な設定とスキルの内容を解析できませんでした。送信側から再同期してください。",
    workspace_limit:
      "移行可能な設定とスキルの内容が上限を超えています。選択を減らして再同期してください。",
    workspace_unsafe:
      "移行可能な設定とスキルの内容が安全でないと判定されました。送信側から再同期してください。",
    hermes_attachment_missing:
      "取得時に添付ファイルが欠落していました。送信側で再提供して同期してください。",
    hermes_attachment_credential:
      "添付ファイルが認証情報らしく除外されました。別途提供してください。添付は転送されません。",
    hermes_attachment_outside_roots:
      "添付ファイルが許可ルート外のパスを参照しています。許可ルートを追加して再同期してください。",
    hermes_attachment_unsupported:
      "この添付種別は転送に対応していません。別途提供してください。",
    hermes_parent_missing:
      "必要なネイティブ親会話がありません。送信側から親を再同期してから再度復元してください。",
    model_credentials_required:
      "モデルの認証情報は転送されません。再開前にこの端末の Hermes でサインインまたはプロバイダー認証情報を設定してください。",
    model_provider_unknown:
      "設定されたモデルプロバイダーを認識できません。再開前にこの端末で model.provider と認証情報を設定してください。",
    mcp_binding_required:
      "MCP サーバーには受信側のバインドが必要です。再開前にこの端末で必要な MCP 環境変数を再バインドしてください。",
    required_environment_variables:
      "必要な環境変数は転送されません。再開前にこの端末で設定してください。",

    workspace_file_count_limit:
      "移行可能な設定とスキルの選択がファイル数上限に達し、一部が除外されました。選択を減らして再同期してください。",
    workspace_oversized:
      "設定またはスキルのファイルがサイズ上限を超え除外されました。そのファイルを縮小して再同期してください。",
    workspace_total_limit:
      "移行可能な設定とスキルの内容が合計上限を超えました。選択を減らして再同期してください。",
    workspace_suspected_credential:
      "認証情報らしいファイルが除外されました。別途提供してください。転送はされません。",
    workspace_missing:
      "参照された設定またはスキルのファイルが欠落、または内容が存在しません。送信側からワークスペースを再同期してください。",
    workspace_depth_limit:
      "スキルディレクトリの階層が深すぎて除外されました。平坦にして再同期してください。",
    workspace_symlink:
      "シンボリックリンクのパスが除外されました。実体ファイルをワークスペースへコピーして再同期してください。",
    workspace_excluded_directory:
      "バージョン管理やキャッシュのディレクトリが除外されました。必要なファイルを含まない限り想定どおりです。",
    workspace_unsupported_or_sensitive_path:
      "未対応または機微なパスが除外されました。別途提供してください。",
    workspace_unsafe_store:
      "シンボリックリンクまたは安全でないスキルストアが除外されました。実ディレクトリを使って再同期してください。",
    workspace_missing_required_skill_asset:
      "スキルが参照する支援ファイルが取得されておらず、復元後のスキルは不完全です。必要な資産をすべて含めてソーススキルを再同期してください。",
    workspace_unparseable_config:
      "config.yaml を無害化できませんでした。YAML を修正し、送信側から再同期してください。",
    workspace_non_text:
      "config.yaml が有効な UTF-8 テキストでないため除外されました。テキストとして保存して再同期してください。",
    workspace_invalid:
      "移行可能な設定とスキルの内容を読み取れませんでした。送信側から再同期してください。",
    workspace_dependency_mcp:
      "復元した設定は MCP サーバーを指定していますが、コマンド・引数・秘密は転送されていません。再開前にこの端末へ導入してバインドしてください。",
    workspace_dependency_model:
      "カスタムのモデルプロバイダー定義（エンドポイントやログイン情報）は破棄されました。再開前にこの端末でプロバイダーとモデルの認証情報を設定してください。",
    workspace_dependency_memory:
      "外部メモリプロバイダーが選択されていますが、そのストアと身元は移行できません。再開前にこの端末でバインドしてください。",
    workspace_dependency_os:
      "スキルが対応オペレーティングシステムを要求しています。対応対象のプラットフォームでのみ再開してください。",
    workspace_dependency_env:
      "スキルが転送されない環境変数を要求しています。再開前にこの端末で設定してください。",
    workspace_dependency_runtime:
      "スキルが転送されないインタープリターまたはツールパッケージを要求しています。再開前にこの端末へ導入してください。",
    workspace_dependency_credential:
      "スキルには認証情報ファイルが必要です。転送されないため、この端末で別途設定し、アクセスを確認してから再開してください。",
    workspace_dependency_app:
      "スキルには端末上のアプリまたはプラグインが必要です。要件を確認し、この端末で有効にしてから再開してください。",
    attachment_missing:
      "取得時に添付ファイルが欠落していました。送信側で再提供して同期してください。",
    attachment_symlink:
      "添付がシンボリックリンクのため除外されました。実体ファイルを参照してください。",
    attachment_credential:
      "添付が認証情報らしく除外されました。別途提供してください。添付は転送されません。",
    attachment_outside_roots:
      "添付が許可ルート外のパスを参照しています。許可ルートを追加して再同期してください。",
    attachment_oversized:
      "添付がサイズ上限を超えています。別途提供してください。",
    attachment_limit:
      "添付が転送上限に達しました。別途提供してください。",
    attachment_unreadable:
      "添付を送信側で読み取れませんでした。修正して再同期してください。",
    attachment_unsupported:
      "この添付種別は転送に対応していません。別途提供してください。",
    project_non_utf8_path:
      "プロジェクトに有効な UTF-8 でないパスがあり梱包できません。名前を変更して再同期してください。",
    project_hash_mismatch:
      "プロジェクトファイルが記録されたハッシュと一致せず拒否されました。送信側からプロジェクトを再同期してください。",
    project_unreadable:
      "送信側でプロジェクトを読み取れませんでした。権限や場所を修正して再同期してください。",
    project_absent:
      "このスナップショットにプロジェクト作業ツリーはありません。送信側で対応付けと選択を行って再同期してください。",
    project_invalid:
      "プロジェクト名前空間を読み取れませんでした。送信側からプロジェクトを再同期してください。",
    memory_provider_not_installed:
      "取得したメモリにはプロバイダープラグインが必要ですが、プラグインは受信していません。この端末へ信頼済みプロバイダープラグインを導入してよいか確認・承認してから、メモリの復元を再試行してください。",
    memory_store_diverged:
      "既存のメモリストアが取得したスナップショットと異なるため上書きしませんでした。変更済みのストアは保持されています。送信側で新しい版を再取得してから再同期してください。",
    memory_export_failed:
      "送信側で Agent Memory OS のエクスポートに失敗しました。出力を確認して再同期してください。",
    memory_export_missing:
      "このスナップショットに取り込めるメモリのエクスポートがありません。送信側でメモリソースを有効化して再同期してください。",
    memory_invalid:
      "メモリの内容を読み取れませんでした。送信側から再同期してください。",
    memory_unverified:
      "検証済みの外部メモリのバインドが見つかりません。再開前にこの端末で Agent Memory OS をバインドしてください。",
  },
  ko: {
    project_mapping_required:
      "이 소스 프로젝트를 '프로젝트 매핑'에서 로컬 폴더에 매핑한 뒤 다시 동기화하고 복원하세요. 승인 없이는 프로젝트 파일이 복사되지 않습니다.",
    project_mapping_target_missing:
      "매핑한 로컬 프로젝트 폴더가 없습니다. 기존 폴더를 선택한 뒤 다시 복원하세요.",
    project_mapping_invalid:
      "프로젝트 매핑은 절대 경로를 사용하고 소스마다 대상은 하나여야 합니다.",
    project_mapping_conflict:
      "같은 소스 프로젝트가 둘 이상의 대상에 매핑되어 있습니다. 매핑을 하나만 유지하세요.",
    project_mapping_format_unsupported:
      "이 프로젝트 매핑 형식은 검증된 이전 방법이 없습니다. 대신 스냅숏을 새 폴더로 복원하세요.",
    project_no_commit:
      "소스 저장소에 아직 커밋이 없습니다. 보내는 쪽에서 최초 커밋을 만든 뒤 다시 동기화하세요.",
    project_git_unavailable:
      "보내는 쪽에서 git을 사용할 수 없어 작업 트리를 캡처하지 못했습니다. git을 설치한 뒤 다시 동기화하세요.",
    project_submodule:
      "git 서브모듈은 패키징할 수 없습니다. 복원 후 이 컴퓨터에서 서브모듈을 초기화하세요.",
    project_selected_empty:
      "프로젝트 파일을 선택하지 않아 작업 트리가 불완전합니다. 포함할 파일을 선택한 뒤 다시 동기화하세요.",
    project_limit:
      "프로젝트가 캡처 용량 한도를 초과합니다. 선택한 파일을 줄이거나 큰 경로를 제외한 뒤 다시 동기화하세요.",
    project_missing_files:
      "캡처할 때 일부 프로젝트 파일이 없었습니다. 계속하기 전에 다시 만들거나 복원하세요.",
    project_missing_lfs:
      "캡처할 때 Git LFS 객체를 사용할 수 없었습니다. 계속하기 전에 LFS 파일을 가져오세요.",
    project_baseline_required:
      "이 스냅숏은 일부 작업 트리입니다. 계속하기 전에 나머지 기준을 재구성하세요.",
    memory_provider_unknown:
      "소스가 알 수 없는 메모리 공급자를 사용합니다. Agent Memory OS 공급자를 명시적으로 선택한 뒤 다시 동기화하세요.",
    memory_provider_unresolved:
      "메모리 공급자를 확인할 수 없습니다. 보내는 쪽 config.yaml에서 memory provider를 agent-memory-os로 설정한 뒤 다시 동기화하세요.",
    memory_identity_missing:
      "외부 메모리 신원을 찾을 수 없습니다. 보내는 쪽에서 Agent Memory OS에 로그인한 뒤 다시 동기화하세요.",
    memory_export_unauthorized:
      "메모리 내보내기가 승인되지 않았습니다. 보내는 쪽에서 Agent Memory OS 소스를 선택하고 내보내기를 승인하세요.",
    memory_cli_missing:
      "Agent Memory OS CLI를 찾을 수 없습니다. 이 컴퓨터에 설치한 뒤 메모리를 다시 복원하세요.",
    memory_cli_failed:
      "Agent Memory OS CLI 가져오기가 실패했습니다. 출력을 확인한 뒤 다시 시도하세요.",
    memory_store_missing:
      "바인딩된 홈에서 외부 메모리 저장소를 찾을 수 없습니다. Agent Memory OS 홈을 확인한 뒤 다시 동기화하세요.",
    memory_unavailable_store:
      "이 빌드에서는 외부 메모리를 사용할 수 없습니다. 승인된 Agent Memory OS 설치를 사용하세요.",
    workspace_format:
      "이동 가능한 설정과 스킬 내용을 해석할 수 없습니다. 보내는 쪽에서 다시 동기화하세요.",
    workspace_limit:
      "이동 가능한 설정과 스킬 내용이 한도를 초과합니다. 선택을 줄인 뒤 다시 동기화하세요.",
    workspace_unsafe:
      "이동 가능한 설정과 스킬 내용이 안전하지 않은 것으로 판정되었습니다. 보내는 쪽에서 다시 동기화하세요.",
    hermes_attachment_missing:
      "캡처할 때 첨부 파일이 없었습니다. 보내는 쪽에서 다시 제공한 뒤 동기화하세요.",
    hermes_attachment_credential:
      "첨부 파일이 자격 증명으로 보여 제외되었습니다. 별도로 제공하세요. 첨부는 전송되지 않습니다.",
    hermes_attachment_outside_roots:
      "첨부 파일이 승인된 루트 밖 경로를 참조합니다. 승인된 루트를 추가한 뒤 다시 동기화하세요.",
    hermes_attachment_unsupported:
      "이 첨부 형식은 전송을 지원하지 않습니다. 별도로 제공하세요.",
    hermes_parent_missing:
      "필요한 네이티브 부모 대화가 없습니다. 보내는 쪽에서 부모를 다시 동기화한 뒤 다시 복원하세요.",
    model_credentials_required:
      "모델 자격 증명은 전송되지 않습니다. 계속하기 전에 이 컴퓨터의 Hermes에서 로그인하거나 공급자 자격 증명을 설정하세요.",
    model_provider_unknown:
      "설정된 모델 공급자를 알 수 없습니다. 계속하기 전에 이 컴퓨터에서 model.provider와 자격 증명을 설정하세요.",
    mcp_binding_required:
      "MCP 서버에는 수신자 바인딩이 필요합니다. 계속하기 전에 이 컴퓨터에서 필요한 MCP 환경 변수를 다시 바인딩하세요.",
    required_environment_variables:
      "필요한 환경 변수는 전송되지 않습니다. 계속하기 전에 이 컴퓨터에서 설정하세요.",

    workspace_file_count_limit:
      "이동 가능한 설정과 스킬 선택이 파일 수 한도에 도달해 일부가 제외되었습니다. 선택을 줄인 뒤 다시 동기화하세요.",
    workspace_oversized:
      "설정 또는 스킬 파일이 크기 한도를 초과해 제외되었습니다. 해당 파일을 줄인 뒤 다시 동기화하세요.",
    workspace_total_limit:
      "이동 가능한 설정과 스킬 내용이 총량 한도를 초과했습니다. 선택을 줄인 뒤 다시 동기화하세요.",
    workspace_suspected_credential:
      "자격 증명으로 보이는 파일이 제외되었습니다. 별도로 제공하세요. 전송되지 않습니다.",
    workspace_missing:
      "참조된 설정 또는 스킬 파일이 없거나 내용이 존재하지 않습니다. 보내는 쪽에서 작업 영역을 다시 동기화하세요.",
    workspace_depth_limit:
      "스킬 디렉터리 깊이가 너무 깊어 제외되었습니다. 평탄화한 뒤 다시 동기화하세요.",
    workspace_symlink:
      "심볼릭 링크 경로가 제외되었습니다. 실제 파일을 작업 영역으로 복사한 뒤 다시 동기화하세요.",
    workspace_excluded_directory:
      "버전 관리 또는 캐시 디렉터리가 제외되었습니다. 필요한 파일이 없으면 정상 동작입니다.",
    workspace_unsupported_or_sensitive_path:
      "지원되지 않거나 민감한 경로가 제외되었습니다. 별도로 제공하세요.",
    workspace_unsafe_store:
      "심볼릭 링크 또는 안전하지 않은 스킬 저장소가 제외되었습니다. 실제 디렉터리를 사용한 뒤 다시 동기화하세요.",
    workspace_missing_required_skill_asset:
      "스킬이 참조하는 지원 파일이 캡처되지 않아 복원된 스킬이 불완전합니다. 필요한 자산을 모두 포함해 원본 스킬을 다시 동기화하세요.",
    workspace_unparseable_config:
      "config.yaml을 정리할 수 없습니다. YAML을 수정한 뒤 보내는 쪽에서 다시 동기화하세요.",
    workspace_non_text:
      "config.yaml이 유효한 UTF-8 텍스트가 아니어서 제외되었습니다. 텍스트로 저장한 뒤 다시 동기화하세요.",
    workspace_invalid:
      "이동 가능한 설정과 스킬 내용을 읽을 수 없습니다. 보내는 쪽에서 다시 동기화하세요.",
    workspace_dependency_mcp:
      "복원된 설정이 MCP 서버를 지정하지만 명령, 인수, 비밀은 전송되지 않았습니다. 계속하기 전에 이 컴퓨터에 설치하고 바인딩하세요.",
    workspace_dependency_model:
      "사용자 지정 모델 공급자 정의(엔드포인트 또는 로그인 정보)가 버려졌습니다. 계속하기 전에 이 컴퓨터에서 공급자와 모델 자격 증명을 설정하세요.",
    workspace_dependency_memory:
      "외부 메모리 공급자가 선택되었지만 그 저장소와 신원은 이전할 수 없습니다. 계속하기 전에 이 컴퓨터에서 바인딩하세요.",
    workspace_dependency_os:
      "스킬이 지원 운영 체제를 요구합니다. 나열된 플랫폼에서만 계속하세요.",
    workspace_dependency_env:
      "스킬이 전송되지 않은 환경 변수를 요구합니다. 계속하기 전에 이 컴퓨터에서 설정하세요.",
    workspace_dependency_runtime:
      "스킬이 전송되지 않은 인터프리터 또는 도구 패키지를 요구합니다. 계속하기 전에 이 컴퓨터에 설치하세요.",
    workspace_dependency_credential:
      "스킬에 자격 증명 파일이 필요합니다. 이 파일은 전송되지 않으므로 이 컴퓨터에서 별도로 설정하고 접근을 확인하세요.",
    workspace_dependency_app:
      "스킬에 호스트 앱 또는 플러그인이 필요합니다. 요구 사항을 검토하고 이 컴퓨터에서 활성화한 후 계속하세요.",
    attachment_missing:
      "캡처할 때 첨부 파일이 없었습니다. 보내는 쪽에서 다시 제공한 뒤 동기화하세요.",
    attachment_symlink:
      "첨부가 심볼릭 링크여서 제외되었습니다. 실제 파일을 참조하세요.",
    attachment_credential:
      "첨부가 자격 증명으로 보여 제외되었습니다. 별도로 제공하세요. 첨부는 전송되지 않습니다.",
    attachment_outside_roots:
      "첨부가 승인된 루트 밖 경로를 참조합니다. 승인된 루트를 추가한 뒤 다시 동기화하세요.",
    attachment_oversized:
      "첨부가 크기 한도를 초과했습니다. 별도로 제공하세요.",
    attachment_limit:
      "첨부가 전송 한도에 도달했습니다. 별도로 제공하세요.",
    attachment_unreadable:
      "첨부를 보내는 쪽에서 읽을 수 없습니다. 수정한 뒤 다시 동기화하세요.",
    attachment_unsupported:
      "이 첨부 형식은 전송을 지원하지 않습니다. 별도로 제공하세요.",
    project_non_utf8_path:
      "프로젝트에 유효한 UTF-8이 아닌 경로가 있어 패키징할 수 없습니다. 이름을 바꾼 뒤 다시 동기화하세요.",
    project_hash_mismatch:
      "프로젝트 파일이 기록된 해시와 일치하지 않아 거부되었습니다. 보내는 쪽에서 프로젝트를 다시 동기화하세요.",
    project_unreadable:
      "보내는 쪽에서 프로젝트를 읽을 수 없습니다. 권한이나 위치를 수정한 뒤 다시 동기화하세요.",
    project_absent:
      "이 스냅숏에는 프로젝트 작업 트리가 없습니다. 보내는 쪽에서 매핑하고 선택한 뒤 다시 동기화하세요.",
    project_invalid:
      "프로젝트 네임스페이스를 읽을 수 없습니다. 보내는 쪽에서 프로젝트를 다시 동기화하세요.",
    memory_provider_not_installed:
      "캡처한 메모리에는 공급자 플러그인이 필요하지만 플러그인은 수신되지 않았습니다. 이 컴퓨터에 신뢰할 수 있는 공급자 플러그인을 설치해도 되는지 검토하고 승인한 뒤 메모리 복원을 다시 시도하세요.",
    memory_store_diverged:
      "기존 메모리 저장소가 캡처한 스냅숏과 달라 덮어쓰지 않았습니다. 변경된 저장소는 보존되었습니다. 보내는 쪽에서 새 버전을 다시 캡처한 뒤 동기화하세요.",
    memory_export_failed:
      "보내는 쪽에서 Agent Memory OS 내보내기가 실패했습니다. 출력을 확인한 뒤 다시 동기화하세요.",
    memory_export_missing:
      "이 스냅숏에는 가져올 메모리 내보내기가 없습니다. 보내는 쪽에서 메모리 소스를 활성화한 뒤 다시 동기화하세요.",
    memory_invalid:
      "메모리 내용을 읽을 수 없습니다. 보내는 쪽에서 다시 동기화하세요.",
    memory_unverified:
      "검증된 외부 메모리 바인딩을 찾지 못했습니다. 계속하기 전에 이 컴퓨터에서 Agent Memory OS를 바인딩하세요.",
  },
};

// Verified continuation facts -> localized positive labels.
const continuationFacts: Record<Locale, Record<string, string>> = {
  en: {
    workspace: "Portable workspace payload checked",
    attachments: "Captured attachment payload checked",
    project: "Project snapshot checked",
    memory: "Memory continuation check passed",
    conversation_graph_preserved: "Parent and branch lineage preserved",
    session_store_isolated: "Native state.db copied into an isolated home",
    project_working_tree_restored: "Project working tree restored to a new folder",
    project_commit_verified: "Source commit verified",
    external_memory_bound: "External memory bound to an isolated store",
    workspace_files_restored: "Portable settings and skills restored",
    attachments_verified: "Referenced attachments verified",
    cwd_remapped: "Session working directory remapped",
    provider_identity_verified: "Provider identity verified",
  },
  "zh-Hant": {
    conversation_graph_preserved: "保留父層與分支血緣",
    workspace: "已檢查可攜工作環境資料",
    attachments: "已檢查擷取的附件資料",
    project: "已檢查專案快照",
    memory: "記憶接續檢查通過",
    session_store_isolated: "原生 state.db 已複製到隔離主目錄",
    project_working_tree_restored: "專案工作區已還原到新資料夾",
    project_commit_verified: "已驗證來源提交",
    external_memory_bound: "外部記憶已綁定到隔離儲存",
    workspace_files_restored: "已還原可攜設定與技能",
    attachments_verified: "已驗證引用的附件",
    cwd_remapped: "已重新對應對話工作目錄",
    provider_identity_verified: "已驗證提供者身分",
  },
  "zh-Hans": {
    conversation_graph_preserved: "保留父级与分支血缘",
    workspace: "已检查便携工作环境数据",
    attachments: "已检查捕获的附件数据",
    project: "已检查项目快照",
    memory: "记忆接续检查通过",
    session_store_isolated: "原生 state.db 已复制到隔离主目录",
    project_working_tree_restored: "项目工作区已还原到新文件夹",
    project_commit_verified: "已验证来源提交",
    external_memory_bound: "外部记忆已绑定到隔离存储",
    workspace_files_restored: "已还原便携设置与技能",
    attachments_verified: "已验证引用的附件",
    cwd_remapped: "已重新映射对话工作目录",
    provider_identity_verified: "已验证提供者身份",
  },
  ja: {
    conversation_graph_preserved: "親と分岐の系統を保持",
    workspace: "移行可能な作業環境データを確認済み",
    attachments: "取得済みの添付データを確認済み",
    project: "プロジェクトのスナップショットを確認済み",
    memory: "メモリ再開チェックに合格",
    session_store_isolated: "ネイティブ state.db を隔離ホームへコピー",
    project_working_tree_restored: "プロジェクト作業ツリーを新規フォルダーへ復元",
    project_commit_verified: "ソースのコミットを検証",
    external_memory_bound: "外部メモリを隔離ストアへバインド",
    workspace_files_restored: "移行可能な設定とスキルを復元",
    attachments_verified: "参照された添付を検証",
    cwd_remapped: "セッションの作業ディレクトリを再マッピング",
    provider_identity_verified: "プロバイダー身元を検証",
  },
  ko: {
    conversation_graph_preserved: "부모 및 분기 계보 보존",
    workspace: "이동 가능한 작업 환경 데이터 확인됨",
    attachments: "캡처된 첨부 데이터 확인됨",
    project: "프로젝트 스냅샷 확인됨",
    memory: "메모리 이어하기 확인 통과",
    session_store_isolated: "네이티브 state.db를 격리된 홈으로 복사",
    project_working_tree_restored: "프로젝트 작업 트리를 새 폴더로 복원",
    project_commit_verified: "소스 커밋 검증",
    external_memory_bound: "외부 메모리를 격리 저장소에 바인딩",
    workspace_files_restored: "이동 가능한 설정과 스킬 복원",
    attachments_verified: "참조된 첨부 검증",
    cwd_remapped: "세션 작업 디렉터리 재매핑",
    provider_identity_verified: "공급자 신원 검증",
  },
};

export function continuationIssueText(code: string, locale: Locale): string {
  const hit = continuationIssue[locale][code];
  if (hit) return hit;
  // A recipient-acknowledged dependency keeps its base setup guidance, clearly
  // marked as an unverified acknowledgment (never a pass).
  if (code.endsWith("_acknowledged")) {
    const base = continuationIssue[locale][code.slice(0, -"_acknowledged".length)];
    if (base) return `${ops[locale].contAcknowledged}: ${base}`;
  }
  return hit ? hit : `${ops[locale].contUnknown}: ${code}`;
}
export function continuationFactText(fact: string, locale: Locale): string {
  const hit = continuationFacts[locale][fact];
  if (hit) return hit;
  // A locally-probed requirement that resolved keeps its base guidance, marked
  // as verified on this computer.
  if (fact.endsWith("_resolved")) {
    const base = continuationIssue[locale][fact.slice(0, -"_resolved".length)];
    if (base) return `${ops[locale].contResolved}: ${base}`;
  }
  return hit ? hit : `${ops[locale].contUnknown}: ${fact}`;
}
export { continuationIssue, continuationFacts };
