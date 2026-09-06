# 🐈 Bastet Agent Sync — 使用指南

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

## 0.4.1

0.4.1 支援本機對話與 Agent Memory OS 同步。勾選來源、儲存後按啟動，不需要手動匯出 JSONL。Claude／Claude Code 共用本機程式工作對話；Codex／Work 共用本機紀錄。Agy 使用 SQLite 快照；Grok、Pi 使用原生對話檔案。

## 設定

選擇語言、裝置名稱、Agent 與資料目錄。Google Drive 精靈分為桌面 OAuth JSON、Google 授權、資料夾、恢復金鑰、確認五步。完成步驟自動保存，可接續、重新開始或完全手動設定；驗證條件相同。另一台電腦使用恢復套件加入同一空間。

## 同步與恢復

選擇上傳、下載或雙向，以及手動、自訂間隔或接近即時。儲存並啟動後，各來源分別顯示沒有資料、處理中、部分完成或失敗。接收端自動加入尚不存在的對話；既有檔案不同時保留衝突。先暫停，再查看快照並另存新資料夾，可保留兩個版本。Agent Memory OS 自動透過正式介面備份與合併。

## 目前界線

不匯出 Claude 雲端聊天、一般 ChatGPT 聊天或雲端 Work。專案檔案、外部附件、完整設定／技能、Cowork VM 及路徑映射尚未完成。Agy／Grok 原生續接與兩台實體電腦驗收仍待驗證。新資料夾恢復不會安裝登入憑證。

## 常駐與更新

啟用關閉至系統匣可保留程式。修改設定前先暫停。Logo 下顯示實際版本。線上更新依序檢查、檢視、安裝並明確重啟；需要已發布且具簽章的版本。

## 驗證

[Native sessions](../../NATIVE_SESSIONS.md) · [Agent Memory OS](../../AGENT_MEMORY_OS.md) · [Drive wizard](../../SETUP_WIZARD.md) · [Validation](../../VALIDATION.md) · [Plan](../../MASTER_PLAN.md)

```sh
npm ci
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```

## 🐈 下載與安裝

[版本與下載](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/latest)：點選版本號，在 Release notes 依作業系統下載。Windows 安裝器內含 WebView2，偵測缺少時補裝；macOS 使用系統 WebKit；Linux 可執行下載的 `sh install-linux.sh`，自動選擇 apt/dnf、校驗並安裝相依套件。無須 Node.js 或 Rust。AMOS CLI 與各 Agent 帳號需自行設定。macOS 未公證、Windows 無 Authenticode，可能顯示系統信任提示。

macOS 首次開啟若被阻擋，確認下載來源後，到「系統設定 → 隱私權與安全性 → 仍要打開」確認此 App。 [Apple](https://support.apple.com/en-au/guide/mac-help/mh40616/mac)


### 🔐 0.4.1

同步前可按 **🔐 準備憑證存取**，集中讀取已儲存的憑證。macOS 提示時，針對 Bastet 各項憑證選擇「永遠允許」。成功讀取後暫存於記憶體，結束程式、忘記登入、重新開始設定或更換用戶端時清除；關閉至系統匣會保留。若在程式外修改憑證，可再按按鈕重新讀取。此操作不驗證 Google 連線，也不啟動同步。每台電腦需各自授權；尚未配置 Developer ID 簽章，更新後仍可能再次詢問。


同步包依 Agent 分組，預設收合，可逐組或全部展開／收合；展開後依本機保存時間由新到舊排列。各 Agent 以狀態卡顯示結果、上傳／下載同步包及加入本機的數量。常見問題提供處理建議，技術代碼可展開查看。保存時間不是原始對話建立時間。


### 0.4.2

0.4.2 可在同步期間切換語言並自動儲存，同時更新系統匣文字；其他未儲存設定不受影響。保存失敗時保留原語言。0.4.1 暫時需先暫停同步、切換語言，再按儲存設定。
