# 🐈 Bastet Agent Sync — 使用指南

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

## 0.3.0

0.3.0 支援本機對話與 Agent Memory OS 同步。勾選來源、儲存後按啟動，不需要手動匯出 JSONL。Claude／Claude Code 共用本機程式工作對話；Codex／Work 共用本機紀錄。Agy 使用 SQLite 快照；Grok、Pi 使用原生對話檔案。

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
