# 🐈 Bastet Agent Sync — 利用ガイド

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

## 0.3.0

0.3.0 はローカル会話と Agent Memory OS を同期します。ソースを選択、保存して開始します。手動 JSONL 出力は不要です。Claude／Claude Code はローカル開発会話、Codex／Work はローカル記録を共有します。Agy は SQLite、Grok と Pi はネイティブ会話ファイルを使用します。

## 設定

言語、端末名、Agent とデータフォルダーを選択します。Drive ウィザードは OAuth JSON、Google 認証、フォルダー、復元キー、確認の５段階です。完了済みの段階は自動保存し、続行、再開、手動設定を同じ検証条件で利用できます。他の端末は復元キットで同じ領域に参加します。

## 同期と復元

送信、受信、双方向と、手動、間隔、準リアルタイムを選び、保存して開始します。ソース別にデータなし、処理中、一部完了、失敗を表示します。未登録の会話は自動追加し、異なる既存ファイルは競合として保護します。一時停止後、スナップショットを新規フォルダーへ復元して両方を保持できます。Agent Memory OS は公式インターフェースで自動バックアップ・統合します。

## 制限

Claude のクラウド会話、通常の ChatGPT 会話、クラウド Work は対象外です。プロジェクト、外部添付、設定／スキル全体、Cowork VM、パス変換は未完了です。Agy／Grok のネイティブ再開と実機２台の受入試験は未検証です。復元に認証情報は含みません。

## トレイと更新

トレイ常駐を有効にできます。設定変更前に一時停止してください。ロゴ下に実際のバージョンを表示します。更新を確認してインストールし、明示的に再起動します。署名付き公開リリースが必要です。

## 検証

[Native sessions](../../NATIVE_SESSIONS.md) · [Agent Memory OS](../../AGENT_MEMORY_OS.md) · [Drive wizard](../../SETUP_WIZARD.md) · [Validation](../../VALIDATION.md) · [Plan](../../MASTER_PLAN.md)

```sh
npm ci
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```
