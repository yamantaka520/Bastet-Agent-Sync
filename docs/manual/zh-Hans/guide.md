# 🐈 Bastet Agent Sync — 使用指南

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

## 0.3.0

0.3.0 支持本地对话与 Agent Memory OS 同步。勾选来源、保存后启动，无需手动导出 JSONL。Claude／Claude Code 共用本地编程对话；Codex／Work 共用本地记录。Agy 使用 SQLite 快照；Grok、Pi 使用原生对话文件。

## 设置

选择语言、设备名称、Agent 和数据目录。Drive 向导包含桌面 OAuth JSON、Google 授权、文件夹、恢复密钥、确认五步。已完成步骤自动保存，可继续、重新开始或完全手动设置，验证条件相同。另一台电脑用恢复套件加入同一空间。

## 同步与恢复

选择上传、下载或双向，以及手动、自定义间隔或接近实时。保存并启动后，各来源分别显示无数据、处理中、部分完成或失败。接收端自动添加不存在的对话；已有文件不同时保留冲突。暂停后查看快照并恢复到新文件夹，保留两个版本。Agent Memory OS 自动使用正式接口备份和合并。

## 当前边界

不导出 Claude 云端聊天、普通 ChatGPT 聊天或云端 Work。项目文件、外部附件、完整设置／技能、Cowork VM 及路径映射尚未完成。Agy／Grok 原生续接与两台实体电脑验收仍待验证。恢复到新文件夹不安装登录凭证。

## 托盘与更新

启用关闭至托盘可保留程序。修改设置前先暂停。Logo 下显示实际版本。在线更新依次检查、查看、安装并明确重启，需要已发布且有签名的版本。

## 验证

[Native sessions](../../NATIVE_SESSIONS.md) · [Agent Memory OS](../../AGENT_MEMORY_OS.md) · [Drive wizard](../../SETUP_WIZARD.md) · [Validation](../../VALIDATION.md) · [Plan](../../MASTER_PLAN.md)

```sh
npm ci
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```
