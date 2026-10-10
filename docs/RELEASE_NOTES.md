# 🐈 Bastet Agent Sync v@VERSION@

繁體中文 · 简体中文 · English · 日本語 · 한국어

@DOWNLOADS@

## 繁體中文

0.8.0 在既有的 Google Drive API、iCloud Drive 資料夾與 OneDrive 資料夾之外，新增 Hermes Agent 來源。選擇 **Hermes Agent**、儲存後明確啟動同步。來源為 `HERMES_HOME` 或 `~/.hermes`；具名 profile 以自訂路徑指定其 home，一次只選取一個 Hermes home。原生對話與必要血緣（已完成的非同步結果以惰性方式攜帶；進行中的工作為 blocker）會還原至獨立受管理目錄，專案工作狀態與引用的附件僅在授權的專案對應下讀取。另會攜帶白名單化的可攜工作區——技能與資產、專案規則、內建記憶檔及去敏的非機密 `config.yaml`。外部 Agent Memory OS 記憶只在顯式綁定 home 時透過官方 CLI 匯出（同身分但範圍廣泛，不保證內容層級機密）。每個還原版本顯示五語接續報告；憑證、模型／供應商登入與機密設定值永不轉移。

安裝包經三平台 CI、已簽章更新來源與安裝檢查驗證。Hermes 合成測試不等於實際雙機交付、模型實機續聊、Windows／Linux 執行期或 App Store 驗收，這些關卡仍未達成。0.7.0 的目的地選項、恢復套件與尚未解決的 macOS 鑰匙圈重複提示維持不變。

獨立下載安裝包不需要 Node.js 或 Rust。Google API 模式不需要 Drive Desktop；iCloud／OneDrive 資料夾模式需要對應桌面程式。macOS 發布須通過 Developer ID 簽章與公證；Windows 發行者簽章暫緩；更新包仍驗證簽章。

## 简体中文

0.8.0 在既有 Google Drive API、iCloud Drive 文件夹与 OneDrive 文件夹之外，新增 Hermes Agent 来源。勾选 **Hermes Agent**、保存后明确启动同步。来源为 `HERMES_HOME` 或 `~/.hermes`；具名 profile 以自定义路径指定其 home，一次只选取一个 Hermes home。原生对话与必要血缘（已完成的异步结果以惰性方式携带；进行中的任务为 blocker）会恢复到独立受管目录，项目工作状态与引用的附件仅在授权的项目映射下读取。另会携带白名单化的可携工作区——技能与资产、项目规则、内置记忆文件及脱敏的非机密 `config.yaml`。外部 Agent Memory OS 记忆只在显式绑定 home 时通过官方 CLI 导出（同身份但范围广泛，不保证内容级机密）。每个恢复版本显示五语接续报告；凭据、模型／供应商登录与机密配置值永不转移。

安装包经三平台 CI、已签名更新源与安装检查验证。Hermes 合成测试不等于实际双机交付、模型实机续聊、Windows／Linux 运行期或 App Store 验收，这些关卡仍未达成。0.7.0 的目标选项、恢复套件与尚未解决的 macOS 钥匙串重复提示保持不变。

独立下载安装包不需要 Node.js 或 Rust。Google API 模式不需要 Drive Desktop；iCloud／OneDrive 文件夹模式需要对应桌面程序。macOS 发布须通过 Developer ID 签名与公证；Windows 发布者签名暂缓；更新包仍验证签名。

## English

0.8.0 adds a Hermes Agent source alongside the existing Google Drive API, iCloud Drive folder and OneDrive folder destinations. Select **Hermes Agent**, save and explicitly start synchronization. The source is `HERMES_HOME` or `~/.hermes`; a named profile's home is a custom path, and only one Hermes home is selected at a time. Native conversations and their required lineage (a finished async result travels inert; an in-flight job is a blocker) are restored into a separate managed home, and project working state and referenced attachments are read only under an authorized project mapping. An allowlisted portable workspace travels too — skills with assets, project rules, built-in memory files and a sanitized non-secret `config.yaml`. External Agent Memory OS memory is exported through its official CLI only under an explicit home binding (same identity but broad, with no content-level secret guarantee). Each restored version shows a five-language continuation report; credentials, model/provider login and secret config values are never transferred.

Installers are verified with three-platform CI, signed updater feeds and installer checks. Hermes fixtures are not physical two-device delivery, live-model continuation, Windows/Linux runtime or App Store acceptance; those gates remain open. The 0.7.0 destination options, recovery kits and the unresolved repeated macOS Keychain prompt are unchanged.

Standalone installers do not require Node.js or Rust. Google API mode needs no Drive Desktop; iCloud and OneDrive folder modes require their respective desktop sync clients. macOS publication requires Developer ID signing and notarization; Windows publisher signing is deferred; update packages still require signature verification.

## 日本語

0.8.0 は既存の Google Drive API、iCloud Drive フォルダー、OneDrive フォルダーに加えて Hermes Agent ソースを追加します。**Hermes Agent** を選択・保存して明示的に同期を開始してください。ソースは `HERMES_HOME` または `~/.hermes`。名前付き profile はホームをカスタムパスで指定し、同時に選択できる Hermes ホームは一つです。会話と必要な系譜（完了済みの非同期結果は不活性な形で伝送、実行中ジョブは blocker）は独立した管理ホームへ復元し、プロジェクト作業状態と参照添付は認可されたプロジェクト対応付けの下でのみ読み取ります。許可リスト化された可搬ワークスペース（アセット付きスキル、プロジェクト規則、内蔵メモリファイル、サニタイズ済みの非機密 `config.yaml`）も伝送します。外部 Agent Memory OS メモリは home を明示的にバインドした場合のみ公式 CLI でエクスポートします（同一アイデンティティですが範囲は広く、内容レベルの機密保証はありません）。各復元版は五言語の再開レポートを表示し、認証情報・モデル／プロバイダーのログイン・機密設定値は伝送されません。

インストーラーは三プラットフォーム CI、署名済み更新フィード、インストール検証で確認されます。Hermes の合成テストは実機２台への配信、モデル実機での再開、Windows／Linux 実行時、App Store の受入ではありません。これらのゲートは未達のままです。0.7.0 の同期先オプション、復元キット、未解決の macOS キーチェーン繰り返し確認は変わりません。

単体配布版に Node.js と Rust は不要です。Google API モードに Drive Desktop は不要ですが、iCloud／OneDrive フォルダーモードには対応するデスクトップ同期アプリが必要です。macOS の公開には Developer ID 署名と公証が必要です。Windows 発行者署名は延期中で、更新パッケージの署名検証は継続します。

## 한국어

0.8.0은 기존 Google Drive API, iCloud Drive 폴더, OneDrive 폴더에 더해 Hermes Agent 소스를 추가합니다. **Hermes Agent**를 선택하고 저장한 뒤 명시적으로 동기화를 시작하세요. 소스는 `HERMES_HOME` 또는 `~/.hermes`이며 이름이 있는 profile은 홈을 사용자 지정 경로로 지정하고, 한 번에 하나의 Hermes 홈만 선택합니다. 대화와 필요한 계보(완료된 비동기 결과는 불활성으로 전달, 진행 중 작업은 blocker)는 별도의 관리 홈에 복원하고, 프로젝트 작업 상태와 참조 첨부는 승인된 프로젝트 매핑 아래에서만 읽습니다. 허용 목록화된 휴대형 워크스페이스(에셋 포함 스킬, 프로젝트 규칙, 내장 메모리 파일, 정제된 비밀 아닌 `config.yaml`)도 함께 전송합니다. 외부 Agent Memory OS 메모리는 home을 명시적으로 바인딩한 경우에만 공식 CLI로 내보냅니다(동일 신원이지만 범위가 넓으며 내용 수준 비밀 보장은 없습니다). 각 복원 버전은 5개 언어 재개 보고서를 표시하고, 자격 증명·모델／공급자 로그인·비밀 설정 값은 전송되지 않습니다.

설치 프로그램은 3개 플랫폼 CI, 서명된 업데이트 피드, 설치 검증으로 확인됩니다. Hermes 합성 테스트는 실제 두 기기 전달, 모델 실기 재개, Windows／Linux 런타임, App Store 검증이 아니며 이 게이트들은 여전히 미충족입니다. 0.7.0의 대상 옵션, 복구 키트, 미해결 macOS 키체인 반복 요청은 그대로입니다.

독립 설치 파일에는 Node.js와 Rust가 필요하지 않습니다. Google API 모드에는 Drive Desktop이 필요 없지만 iCloud／OneDrive 폴더 모드에는 각 서비스의 데스크톱 동기화 앱이 필요합니다. macOS 공개에는 Developer ID 서명과 공증이 필요합니다. Windows 게시자 서명은 보류 중이며 업데이트 패키지 서명 검증은 계속 적용됩니다.
