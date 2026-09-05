# 🐈 Bastet Agent Sync — 使用指南與規劃

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

讓 Agent 與對話隨你切換電腦的桌面伴侶。

> 快照核心已實作。在桌面程式按 **執行隔離檢查**，會用兩個暫存裝置與共用資料夾傳输合成文字、保留兩個分支、確認重複執行不增加傳輸，並驗證重開後恢復。檢查不會存取所選 Agent 或 Drive 資料夾。

## 規劃中的設定流程

1. 選擇語言與裝置名稱。
2. 自動偵測並選取 Claude、Claude Code、Codex、Google Agy CLI、Grok Build CLI、Pi Agent；可自訂路徑。
3. 連接 Google Drive，選擇或建立共用資料夾。Linux 將使用規劃中的 API 直接連接。
4. 選擇雙向、僅上傳或僅下載，以及手動、自訂間隔或接近即時模式；完成設定後按啟動。
5. 啟用後可關閉至系統匣，也可從系統匣暫停／恢復。

## 目前的基礎功能

偵測候選資料目錄而不讀取對話內容，選擇本機資料夾並儲存偏好。雲端連線與適配器驗證前，不開放 Agent 同步啟動按鈕。瀏覽器預覽不能操作本機功能。

## 對話接續

原生恢復需要經過驗證的適配器。上下文接續會建立新對話並另行標示。依匯出能力保留附件、工具結果與專案路徑；同時修改的對話保留分支，憑證留在本機。

## 開發路線

M0：儲存庫、規劃、五語系與貓咪識別。M1：桌面、偵測、設定與系統匣。M2：快照、衝突與本機傳輸。M3：Google Drive 與加密。M4：Agent 原生恢復。M5：排程與三平台發布。

## 開發方式

先安裝 Node.js、Rust 與 Tauri 平台相依套件，再於儲存庫根目錄執行以下指令。

```sh
npm ci
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```

## 驗證與限制

Windows／Linux 建置與實際系統匣行為需驗證。M1 不進行雲端登入、上傳或對話匯入。移動對話歷史不會遷移執行中的 Agent 程序。

## 參考文件

[Master plan](../../MASTER_PLAN.md) · [Validation](../../VALIDATION.md) · [Requirements](../../../REQUIREMENTS.md) · [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

## M2：隔離核心檢查

快照核心已實作。在桌面程式按 **執行隔離檢查**，會用兩個暫存裝置與共用資料夾傳输合成文字、保留兩個分支、確認重複執行不增加傳輸，並驗證重開後恢復。檢查不會存取所選 Agent 或 Drive 資料夾。

這不是 Agent 原生同步。Google Drive 登入、加密傳輸、原生對話恢復與排程仍待完成。v1 核心只接收事先整理的文字檔，保留所有版本，不傳播刪除。

[M2 詳細規格](../../SNAPSHOT_PROTOCOL.md)

## 🐈 M3 雲端預覽

已實作 OAuth／憑證庫程式、加密 Drive API 與合成資料加密／恢復檢查。預設版本尚無 OAuth Client，登入停用。本機 HTTP 測試不代表真實 Google 或兩台電腦驗證。加密佇列已能交換兩個隔離副本、保留分支與重試 ID；空間／金鑰引導現已實作（見下方）；Picker 與 GUI 同步控制仍待完成。

[Technical contract / 技術文件](../../CLOUD_SECURITY.md)

## 🐈 引導精靈、接續與手動設定

Drive 精靈分為登入設定、Google 授權、資料夾、加密／恢復及最後檢查五步。完成的步驟與模式會自動保存，重開後可接續；重新開始會封存舊進度，不刪除 Drive 資料或金鑰。完全手動設定展開所有區段，仍使用相同驗證。無內建 Client 時可匯入自己的桌面 OAuth JSON，也可直接輸入資料夾 ID／連結。新空間先保存恢復檔再驗證；加入另一台電腦的空間則匯入其恢復檔。完成設定不等於啟用 Agent 同步。

[Setup contract / 設定文件](../../SETUP_WIZARD.md)

## 🐈 帳號核對與 ChatGPT 整合版

精靈顯示目前連線或已儲存的 Google 帳號，拒絕以不同帳號接續。可移除本機登入後使用原帳號重新連線，或重新開始設定以更換帳號。舊進度仍可讀取，明確重新連線後補上身分。

整合版 ChatGPT 桌面的本機 Codex／Worktree 納入適配器目標；ChatGPT Work 本機任務須驗證格式與接續。一般 ChatGPT 聊天與雲端任務另行研究正式整合方式。目前尚未驗證原生對話跨機恢復。

[OpenAI documentation](https://learn.chatgpt.com/docs/environments/modes) · [ChatGPT / Codex](https://learn.chatgpt.com/docs/use-chatgpt)

## 🐈 取消 Google 授權等待

連線時可取消等待瀏覽器授權，保留已儲存步驟後再次連線。單純關閉瀏覽器會等待三分鐘逾時。Token 交換與帳號檢查須等待完成；無法取消的階段會明確提示。
