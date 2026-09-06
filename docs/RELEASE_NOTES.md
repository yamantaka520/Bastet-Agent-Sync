# 🐈 Bastet Agent Sync v@VERSION@

繁體中文 · 简体中文 · English · 日本語 · 한국어

@DOWNLOADS@

## 繁體中文

本版改善憑證存取與同步結果介面，保留同步預覽的支援界線。新增「準備憑證存取」及執行期間快取，減少重複讀取鑰匙圈；退出、忘記登入或重新設定會清除快取。同步包可依 Agent 展開／收合，顯示本機保存時間並由新到舊排序；各 Agent 改用清楚的狀態卡、傳輸數量與問題處理建議。時間不是原始對話建立時間，更新後仍可能需要 OS 授權。

提供五語 GUI、系統匣、可接續的 Google Drive 設定精靈、本機對話加密同步、AMOS 自動備份合併、逐來源狀態與更新按鈕。

Windows 安裝器會偵測並使用內含的離線安裝器補齊 WebView2；C/C++ 執行庫靜態連結。macOS 使用系統 WebKit，拖曳至 Applications 即可。Linux 建議下載並執行 `sh install-linux.sh`，自動辨識 apt/dnf、校驗下載檔並補齊 WebKitGTK、GTK、系統匣與鑰匙圈相依套件；需要網路與管理員權限。也可用軟體安裝中心開啟 DEB/RPM。無須安裝 Node.js、Rust 或 Google Drive 桌面版。

更新封包有簽章，但 macOS 僅 ad-hoc 簽章、未經 Apple 公證；Windows 沒有 Authenticode，初次開啟可能出現系統信任提示。不要停用整體系統防護。AMOS 需另有可用的正式 CLI；Agent 安裝、帳號登入、OS 升級及桌面工作階段的鑰匙圈解鎖不會被安裝器代辦。

雲端 ChatGPT／Work、Claude 雲端聊天、完整設定／技能、外部附件與跨機專案路徑不在本版保證範圍。Agy／Grok 原生續接及所有來源的兩台實機驗收仍待完成。

macOS 首次開啟若被阻擋，確認下載來源後，到「系統設定 → 隱私權與安全性 → 仍要打開」確認此 App。 [Apple](https://support.apple.com/en-au/guide/mac-help/mh40616/mac)

## 简体中文

本版新增凭据访问准备与内存缓存，减少重复读取钥匙串；退出、忘记登录或重新设置会清除缓存。同步包按 Agent 展开／收起，显示本地保存时间并从新到旧排列；状态卡提供明确结果、传输数量与处理建议。保存时间不是原始对话时间，更新后仍可能需要系统授权。

提供五语言界面、系统托盘、可恢复的 Drive 向导、本地对话加密同步、AMOS 自动备份合并和在线更新。Windows 自动检测并补装内置 WebView2；macOS 使用系统 WebKit；Linux 执行 `sh install-linux.sh`，自动选择 apt/dnf、校验下载并安装缺少的依赖，需要网络和管理员权限。无需 Node.js、Rust 或 Drive 桌面版。

更新包有签名，但 macOS 未公证、Windows 无 Authenticode，可能出现系统信任提示。AMOS CLI 和 Agent 登录需自行配置。云端聊天／Work、完整设置／技能、外部附件、路径映射和完整双机验收仍不在保证范围。

macOS 首次打开若被阻止，确认下载来源后，在「系统设置 → 隐私与安全性 → 仍要打开」确认此 App。 [Apple](https://support.apple.com/en-au/guide/mac-help/mh40616/mac)

## English

This update adds explicit credential preparation and a process-memory cache to reduce repeated keychain reads; exit, forgetting login and setup reset clear the cache. Snapshots collapse by agent and sort by local save time, newest first. Source cards provide clear results, labelled transfer counts and actionable issue summaries. Save times are not original conversation times, and OS authorization may still be needed after updates.

Includes five-language GUI/tray, resumable Drive wizard, encrypted local conversation exchange, automatic AMOS backup/merge, per-source status and signed updates. Windows detects and installs missing WebView2 using the embedded offline installer and statically links the C runtime. macOS uses system WebKit. On Linux run `sh install-linux.sh`: it detects apt/dnf, verifies the download checksum and installs missing package dependencies. Network access and administrator permission are required. Node.js, Rust and Drive Desktop are unnecessary.

Update signatures are present; macOS is ad-hoc signed, not notarized, and Windows has no Authenticode certificate. OS trust prompts may appear. AMOS requires its separately installed official CLI; agents, account login, OS upgrades and desktop keyring unlocking are not provisioned by the installer. Cloud chats/Work, full settings/skills, external attachments and project mapping remain outside this release's guarantees. Agy/Grok native continuation and complete physical two-device acceptance remain unverified.

If macOS blocks the first launch, confirm the download source, then use System Settings → Privacy & Security → Open Anyway for this app. [Apple](https://support.apple.com/en-au/guide/mac-help/mh40616/mac)

## 日本語

認証情報の事前準備とメモリキャッシュにより、キーチェーンの反復読み取りを減らします。終了、ログイン情報削除、設定再開始で消去します。スナップショットは Agent ごとに展開でき、端末への保存日時が新しい順に表示します。状態カードに結果、転送数、対処方法を示します。日時は元の会話日時ではなく、更新後にOSの許可が必要な場合があります。

5言語のGUI／トレイ、再開可能なDrive設定、本機会話の暗号化同期、AMOS自動バックアップ／マージ、状態表示と署名付き更新に対応します。Windowsは同梱のオフラインインストーラーで不足するWebView2を補い、macOSはシステムWebKitを使用します。Linuxは `sh install-linux.sh` でapt/dnfを判定し、ダウンロードを検証して依存パッケージを補います。ネット接続と管理者権限が必要です。Node.js、Rust、Driveデスクトップは不要です。

macOSは未公証、WindowsはAuthenticode未署名のため、OSの確認が表示される場合があります。AMOS CLIとAgentのログインは別途設定が必要です。クラウド会話／Work、全設定／スキル、外部添付、パス変換、全Agentの実機2台での検証は保証対象外です。

macOSで初回起動が阻止された場合は配布元を確認し、「システム設定 → プライバシーとセキュリティ → このまま開く」でこのAppを確認してください。 [Apple](https://support.apple.com/en-au/guide/mac-help/mh40616/mac)

## 한국어

자격 증명 접근 준비와 메모리 캐시로 키체인 반복 읽기를 줄였습니다. 종료, 로그인 정보 삭제 또는 설정 재시작 시 캐시를 지웁니다. 스냅샷은 Agent별로 접거나 펼치며 로컬 저장 시간순으로 표시합니다. 상태 카드에 결과, 전송 수 및 해결 방법을 표시합니다. 저장 시간은 원래 대화 시간이 아니며 업데이트 후 OS 승인이 필요할 수 있습니다.

 5개 언어 GUI/트레이, 재개 가능한 Drive 설정, 로컬 대화 암호화 동기화, AMOS 자동 백업/병합, 상태 표시 및 서명된 업데이트를 제공합니다. Windows는 포함된 오프라인 설치기로 누락된 WebView2를 설치하며 macOS는 시스템 WebKit을 사용합니다. Linux에서 `sh install-linux.sh`를 실행하면 apt/dnf 감지, 다운로드 검증 및 의존성 설치를 수행합니다. 네트워크와 관리자 권한이 필요합니다. Node.js, Rust, Drive 데스크톱은 필요하지 않습니다.

macOS 공증과 Windows Authenticode 서명은 없어 OS 신뢰 확인이 표시될 수 있습니다. AMOS CLI와 Agent 로그인은 별도로 설정해야 합니다. 클라우드 대화/Work, 전체 설정/스킬, 외부 첨부, 경로 매핑 및 모든 Agent의 실제 두 컴퓨터 검증은 보장 범위에 포함되지 않습니다.

macOS에서 첫 실행이 차단되면 다운로드 출처를 확인하고 시스템 설정 → 개인정보 보호 및 보안 → 그래도 열기에서 이 앱을 승인하세요. [Apple](https://support.apple.com/en-au/guide/mac-help/mh40616/mac)

[Support boundaries](https://github.com/yamantaka520/Bastet-Agent-Sync/blob/v@VERSION@/docs/NATIVE_SESSIONS.md) · [Validation](https://github.com/yamantaka520/Bastet-Agent-Sync/blob/main/docs/VALIDATION.md)
