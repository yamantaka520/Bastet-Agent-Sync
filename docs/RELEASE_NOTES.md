# 🐈 Bastet Agent Sync v@VERSION@

繁體中文 · 简体中文 · English · 日本語 · 한국어

@DOWNLOADS@

## 繁體中文

0.5.0 新增同步控制中心：逐來源階段、已檢查項目數、目前封包位元組及有足夠樣本時的封包剩餘秒數；保存最近 500 輪同步紀錄；加密裝置回報顯示回報與接收時間。裝置回報不是即時在線狀態。

先暫停，再設定 1–6 組並行（預設 3）、上下傳 KiB/s 上限（0 表示不限）與本機允許時段，儲存後套用。共用資料區仍依序處理。允許時段控制每輪開始時間，支援跨午夜，同起訖表示全天；15 分鐘暫停會在程式持續運行時自動恢復。限速針對本程式 Drive 資料讀取量，不是系統總流量。

暫停後比較快照，可查看本機／接收內容或雜湊，選擇保留本機並標記已檢視，或保留兩版到新資料夾。本機檔案改變後需重新檢視。可按需量測本機／快取與 Drive 同步物件用量；清除下載快取會保留副本、日誌與所有雲端歷史。

設定／技能同步預設關閉。啟用後預覽目前草稿、展開內容、取消不需要的檔案並儲存。只包含允許的單純偏好欄位與文字技能；標準 Codex 的共用使用者技能分開保存。設定中的憑證、hook、MCP／供應商定義與機器路徑不會匯出。已知秘密樣式會被過濾，但任意文字仍須自行檢查。接收包須比較並另存新資料夾，不自動安裝或執行。

已安裝 CLI 驗收：Grok 可匯出經 Bastet 加密、隔離還原後的兩則測試訊息。Agy 在專用原始測試對話中成功接續標記，還原資料庫完整性通過；不宣稱已驗證 Agy 從還原目錄載入模型上下文或兩台實機搬移。Grok 恢復後提供可複製的 POSIX／PowerShell 接續指令。雲端聊天／Work、專案路徑映射、外部附件與完整設定／技能搬移仍不在保證範圍。

安裝包：Windows 會偵測並使用內含離線安裝器補齊 WebView2；macOS 使用系統 WebKit；Linux install-linux.sh 會選擇 apt/dnf、校驗下載並安裝相依套件。不需要 Node.js、Rust 或 Drive 桌面版；Agent CLI 與帳號登入仍須另行準備。更新包有簽章；依本次範圍不處理 Apple 公證與 Windows Authenticode，可能出現 OS 信任提示。使用者已回報 macOS App 內升級成功，這不是本輪重新執行的儀器驗證。

## 简体中文

0.5.0 新增同步控制中心：各来源阶段、已检查项目数、当前数据包字节与样本足够时的剩余秒数；保存最近 500 轮同步记录；加密设备报告显示报告与接收时间。设备报告不是实时在线状态。

先暂停，再设置 1–6 组并行（默认 3）、上传／下载 KiB/s 上限（0 表示不限）及本地允许时段，保存后应用。共享数据区仍依序处理。时段控制每轮开始，支持跨午夜，相同起止表示全天；暂停 15 分钟会在程序持续运行时自动恢复。限速针对本程序 Drive 数据读取量，不是系统总流量。

暂停后比较快照，查看本地／接收内容或哈希，保留本地并标记已检查，或将两版保留到新文件夹。本地文件改变后需重新检查。可按需测量本地／缓存及 Drive 同步对象用量；清除下载缓存保留副本、日志和全部云端历史。

设置／技能同步默认关闭。启用后预览当前草稿、展开内容、取消不需要的文件并保存。仅包括允许的简单偏好字段和文本技能；标准 Codex 的共享用户技能单独保存。配置中的凭据、hook、MCP／提供商定义及机器路径不导出。已知秘密模式会被过滤，但任意文本仍需自行检查。接收包须比较并另存新文件夹，不自动安装或执行。

已安装 CLI 验收：Grok 能导出经 Bastet 加密和隔离恢复后的两条测试消息。Agy 在专用原始测试对话中成功接续标记，恢复数据库完整性通过；不宣称已验证 Agy 从恢复目录载入模型上下文或两台实机搬移。Grok 恢复提供可复制的 POSIX／PowerShell 接续命令。云端聊天／Work、项目路径映射、外部附件和完整设置／技能迁移仍不在保证范围。

安装包：Windows 检测并使用内置离线安装器补齐 WebView2；macOS 使用系统 WebKit；Linux install-linux.sh 自动选择 apt/dnf、校验下载并安装依赖。不需要 Node.js、Rust 或 Drive 桌面版；Agent CLI 和账号登录须另行准备。更新包有签名；本轮不处理 Apple 公证和 Windows Authenticode，可能出现系统信任提示。用户已报告 macOS App 内升级成功，不代表本轮重新执行了仪器验证。

## English

0.5.0 adds the sync control center: source stages, examined item counts, current payload bytes and a sampled payload ETA; up to 500 persistent cycle records; encrypted device reports with report/observation times. Device reports are not live online status.

Pause, set 1–6 concurrent storage groups (default 3), upload/download KiB/s limits (0 = unlimited), and local allowed hours, then save. Shared profiles remain sequential. Allowed hours govern cycle start; equal times mean all day and overnight windows are supported. A 15-minute pause resumes while the app runs. Limits measure this app's Drive payload consumption, not system traffic.

Pause and Compare a snapshot to inspect local/incoming text or hashes. Keep local and mark reviewed, or keep both in a new folder. A changed local file invalidates review. Measure local app/cache or Drive object usage on demand; Clear download cache preserves replicas, journals and all cloud history.

Portable settings/skills default off. Opt in, Preview draft choices, expand content, uncheck individual files and save. Only allowlisted scalar preferences and supported text skills are included; standard Codex shared user skills are kept separately. Credentials, hooks, MCP/provider definitions and machine paths are excluded from config. Known secret patterns are filtered, but arbitrary content still needs human review. Received packages require comparison and new-folder recovery; nothing is automatically installed or executed.

Installed CLI checks: Grok exported both test messages after Bastet's encrypted isolated-profile restore. Agy repeated a marker in its dedicated original test conversation; its restored database passed integrity checks. Restored-profile Agy model continuation and physical two-device acceptance are not claimed. Grok recovery provides copyable POSIX/PowerShell continuation commands. Cloud chats/Work, project mapping, external attachments and complete settings/skill migration remain outside guarantees.

Installers: Windows detects and installs missing WebView2 from the embedded offline installer; macOS uses system WebKit; Linux install-linux.sh chooses apt/dnf, verifies checksums and installs package dependencies. No Node.js, Rust or Drive Desktop is needed. Agent CLIs and account login remain separate. Updates are signed. Apple notarization and Windows Authenticode are excluded by scope; OS trust prompts may appear. macOS in-app upgrade was reported successful by the user; it is not a new instrumented upgrade test.

## 日本語

0.5.0 では同期コントロールセンターを追加しました。ソース別の段階、処理項目数、現在の通信データ量と十分な測定後の残り秒数、最新500回の永続履歴、暗号化された端末報告を表示します。報告時刻と受信時刻はオンライン状態の証明ではありません。

一時停止して並行数1～6（初期値3）、送受信上限KiB/s（0は無制限）、端末の許可時間帯を設定して保存します。共有領域は順次処理します。時間帯は同期開始を制御し、日付をまたぐ指定に対応、開始と終了が同じなら終日です。15分停止はアプリ動作中に自動再開します。帯域制限は本アプリのDriveデータ読み取り量に適用され、システム全体の通信量ではありません。

停止後にスナップショットを比較し、ローカル／受信テキストやハッシュを確認できます。ローカルを保持して確認済みにするか、新規フォルダーに両方を保存します。ローカル変更後は再確認が必要です。ローカル／キャッシュとDriveオブジェクト容量を手動計測できます。キャッシュ削除はレプリカ、記録、クラウド履歴を保持します。

設定／スキル同期は初期状態で無効です。有効化して現在の設定案をプレビューし、内容を展開、不要ファイルを外して保存します。許可された単純な設定値とテキストスキルのみを対象とし、標準Codexの共有ユーザースキルは別に保存します。設定内の認証情報、hook、MCP／プロバイダー定義、端末パスは除外します。既知の秘密情報パターンは除外しますが、任意の文章は利用者の確認が必要です。受信ファイルは比較後に新規フォルダーへ復元し、自動導入・実行しません。

導入済みCLI検証では、GrokがBastetの暗号化・分離復元後に2件のテストメッセージを出力しました。Agyは専用の元テスト会話を再開し、復元DBの整合性も合格しました。復元先Agyのモデル再開や実機2台の移行を検証済みとはしません。Grok復元ではPOSIX／PowerShell再開コマンドをコピーできます。クラウド会話／Work、プロジェクトパス変換、外部添付、設定／スキル全体の移行は保証対象外です。

Windowsは同梱のオフラインインストーラーで不足するWebView2を導入します。macOSはシステムWebKitを使用し、Linuxのinstall-linux.shはapt/dnfの選択、チェックサム検証、依存パッケージ導入を行います。Node.js、Rust、Drive Desktopは不要です。Agent CLIとログインは別途必要です。更新署名はありますが、Apple公証とWindows Authenticodeは今回の対象外で、OSの確認画面が出る場合があります。macOSアプリ内更新成功は利用者からの報告で、新たな計測試験ではありません。

## 한국어

0.5.0은 동기화 제어 센터를 추가합니다. 소스별 단계, 처리 항목 수, 현재 전송 바이트와 충분히 측정된 경우의 남은 초, 최근 500회 영구 기록, 암호화된 장치 보고를 표시합니다. 보고 시각과 수신 시각은 실시간 온라인 상태를 뜻하지 않습니다.

일시 중지 후 동시 그룹 1–6개(기본 3), 업로드／다운로드 KiB/s 제한(0은 무제한), 로컬 허용 시간을 설정하고 저장하세요. 공유 저장소는 순서대로 처리합니다. 허용 시간은 주기 시작을 제어하며 자정을 넘길 수 있고 시작과 끝이 같으면 하루 종일입니다. 15분 중지는 앱이 실행 중일 때 자동으로 재개됩니다. 제한은 이 앱의 Drive 데이터 소비량에 적용되며 시스템 전체 트래픽이 아닙니다.

중지 후 스냅샷을 비교하여 로컬／수신 텍스트 또는 해시를 확인하세요. 로컬을 유지하고 검토 완료로 표시하거나 새 폴더에 두 버전을 보관합니다. 로컬 파일 변경 시 다시 검토해야 합니다. 로컬／캐시와 Drive 객체 용량을 직접 측정할 수 있습니다. 다운로드 캐시 정리는 복제본, 기록과 모든 클라우드 이력을 보존합니다.

설정／스킬 동기화는 기본적으로 꺼져 있습니다. 활성화 후 현재 초안을 미리 보고 내용을 펼쳐 불필요한 파일을 해제한 뒤 저장하세요. 허용된 단순 환경설정과 텍스트 스킬만 포함하며 표준 Codex의 공유 사용자 스킬은 별도로 보관합니다. 설정의 자격 증명, hook, MCP／공급자 정의 및 장치 경로는 제외합니다. 알려진 비밀 패턴은 필터링하지만 임의 텍스트는 직접 검토해야 합니다. 수신 파일은 비교 후 새 폴더에 복원하며 자동 설치나 실행은 하지 않습니다.

설치된 CLI 검증에서 Grok은 Bastet 암호화 및 격리 복원 후 테스트 메시지 두 개를 내보냈습니다. Agy는 전용 원본 테스트 대화를 재개했고 복원 DB 무결성도 통과했습니다. 복원 프로필에서 Agy 모델 재개나 실제 두 컴퓨터 이동까지 검증했다고 주장하지 않습니다. Grok 복원은 복사 가능한 POSIX／PowerShell 재개 명령을 제공합니다. 클라우드 대화／Work, 프로젝트 경로 매핑, 외부 첨부와 전체 설정／스킬 이전은 보장 범위 밖입니다。

Windows는 포함된 오프라인 설치기로 부족한 WebView2를 설치합니다. macOS는 시스템 WebKit을 사용하며 Linux install-linux.sh는 apt/dnf 선택, 체크섬 검증과 의존 패키지 설치를 수행합니다. Node.js, Rust, Drive Desktop은 필요하지 않습니다. Agent CLI와 로그인은 별도로 준비하세요. 업데이트 서명은 있으나 Apple 공증과 Windows Authenticode는 이번 범위에서 제외되어 OS 확인이 나타날 수 있습니다. macOS 앱 내 업그레이드 성공은 사용자 보고이며 새로 수행한 계측 검증은 아닙니다.

[Validation](https://github.com/yamantaka520/Bastet-Agent-Sync/blob/main/docs/VALIDATION.md) · [Sync control contract](https://github.com/yamantaka520/Bastet-Agent-Sync/blob/main/docs/SYNC_CONTROL.md)
