# 🐈 Bastet Agent Sync — 使い方と計画

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

エージェントと会話を、別のコンピューターでも。

> 開発を開始しました。M1 はデスクトップの基盤です。実際の同期と別端末での会話再開はまだ利用できません。

## 予定している設定手順

1. 言語と端末名を選びます。
2. Claude、Claude Code、Codex、Google Agy CLI、Grok Build CLI、Pi Agent を検出して選択します。パスも指定できます。
3. Google Drive に接続し、共有フォルダーを選択または作成します。Linux では今後実装する API 接続を使います。
4. 双方向、アップロードのみ、ダウンロードのみと、手動、間隔指定、ほぼリアルタイムの頻度を選び、設定後に開始します。
5. 有効にするとウィンドウをトレイに格納できます。トレイから一時停止・再開もできます。

## 現在の基盤機能

会話内容を読まずに候補ディレクトリを検出し、ローカルフォルダーを選択して設定を保存します。同期エンジンの実装までは開始ボタンを無効にします。ブラウザープレビューではネイティブ機能を使えません。

## 会話の再開

ネイティブ復元には検証済みのアダプターが必要です。コンテキストを渡す方式は新しい会話として明示します。エクスポート可能な添付ファイル、ツール結果、プロジェクトパスを保持します。同時変更は分岐として保持し、認証情報は端末内に残します。

## ロードマップ

M0：リポジトリ、計画、5言語、猫のアイコン。M1：デスクトップ、検出、設定、トレイ。M2：スナップショット、競合、ローカル転送。M3：Google Drive と暗号化。M4：エージェントのネイティブ復元。M5：スケジュールと3 OS 向け配布。

## 開発方法

Node.js、Rust、Tauri の OS 別前提条件をインストールし、リポジトリのルートで次のコマンドを実行します。

```sh
npm ci
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```

## 検証と制限

Windows／Linux のビルドとトレイの動作は検証が必要です。M1 はクラウドへのログイン、アップロード、会話のインポートを行いません。会話履歴の移動は実行中のプロセスを移動しません。

## 参照

[Master plan](../../MASTER_PLAN.md) · [Validation](../../VALIDATION.md) · [Requirements](../../../REQUIREMENTS.md) · [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
