# 🐈 Bastet Agent Sync — 使用指南

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

## 0.4.1

0.4.1 支持本地对话与 Agent Memory OS 同步。勾选来源、保存后启动，无需手动导出 JSONL。Claude／Claude Code 共用本地编程对话；Codex／Work 共用本地记录。Agy 使用 SQLite 快照；Grok、Pi 使用原生对话文件。

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

## 🐈 下载与安装

[版本与下载](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/latest)：点击版本号，在 Release notes 下载对应系统安装包。Windows 内置 WebView2 并检测补装；macOS 使用系统 WebKit；Linux 运行下载的 `sh install-linux.sh` 自动选择 apt/dnf、校验并补齐依赖。无需 Node.js 或 Rust。AMOS CLI 和 Agent 账号需另行设置。macOS 未公证、Windows 无 Authenticode，可能显示系统信任提示。

macOS 首次打开若被阻止，确认下载来源后，在「系统设置 → 隐私与安全性 → 仍要打开」确认此 App。 [Apple](https://support.apple.com/en-au/guide/mac-help/mh40616/mac)


### 🔐 0.4.1

同步前可点击 **🔐 准备凭据访问**，集中读取已保存的凭据。macOS 提示时，为 Bastet 的各项凭据选择“始终允许”。成功读取后暂存于内存，退出程序、忘记登录、重新开始设置或更换客户端时清除；关闭到系统托盘会保留。在程序外修改凭据后，可再次点击按钮重新读取。此操作不验证 Google 连接，也不启动同步。每台电脑需分别授权；尚未配置 Developer ID 签名，更新后仍可能再次询问。


同步包按 Agent 分组，默认收起，可逐组或全部展开／收起；展开后按本地保存时间从新到旧排列。各 Agent 用状态卡显示结果、上传／下载同步包及加入本地的数量。常见问题提供处理建议，技术代码可展开查看。保存时间不是原始对话创建时间。
