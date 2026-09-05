# 🐈 Bastet Agent Sync — 使用指南与规划

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

让 Agent 和对话随你切换电脑的桌面助手。

> 快照核心已实现。在桌面应用中点击 **运行隔离检查**，会用两个临时设备和共享文件夹传输合成文本、保留两个分支、确认重复执行不增加传输，并验证重新打开后的恢复。检查不会访问所选 Agent 或 Drive 文件夹。

## 计划中的设置流程

1. 选择语言和设备名称。
2. 自动检测并选择 Claude、Claude Code、Codex、Google Agy CLI、Grok Build CLI、Pi Agent；支持自定义路径。
3. 连接 Google Drive，选择或创建共享文件夹。Linux 将通过计划中的 API 直接连接。
4. 选择双向、仅上传或仅下载，以及手动、自定义间隔或近实时模式；设置完成后点击启动。
5. 启用后可关闭到系统托盘，也可从托盘暂停／恢复。

## 当前基础功能

检测候选数据目录而不读取对话内容，选择本地文件夹并保存偏好设置。云端连接和适配器验证前，不开放 Agent 同步启动按钮。浏览器预览无法使用原生功能。

## 对话续接

原生恢复需要经过验证的适配器。上下文续接会创建新对话并单独标注。根据导出能力保留附件、工具结果和项目路径；并发修改的对话保留分支，凭据保留在本地。

## 开发路线

M0：仓库、规划、五种语言和猫咪标识。M1：桌面、检测、设置和托盘。M2：快照、冲突和本地传输。M3：Google Drive 和加密。M4：Agent 原生恢复。M5：调度和三平台发布。

## 开发方式

先安装 Node.js、Rust 和 Tauri 平台依赖，然后在仓库根目录运行以下命令。

```sh
npm ci
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```

## 验证与限制

Windows／Linux 构建和实际托盘行为需要验证。M1 不进行云端登录、上传或对话导入。迁移对话历史不会迁移正在运行的 Agent 进程。

## 参考文档

[Master plan](../../MASTER_PLAN.md) · [Validation](../../VALIDATION.md) · [Requirements](../../../REQUIREMENTS.md) · [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

## M2：隔离核心检查

快照核心已实现。在桌面应用中点击 **运行隔离检查**，会用两个临时设备和共享文件夹传输合成文本、保留两个分支、确认重复执行不增加传输，并验证重新打开后的恢复。检查不会访问所选 Agent 或 Drive 文件夹。

这不是 Agent 原生同步。Google Drive 登录、加密传输、原生对话恢复和调度仍待完成。v1 核心只接收预先整理的文本文件，保留所有版本，不传播删除。

[M2 详细规范](../../SNAPSHOT_PROTOCOL.md)

## 🐈 M3 云端预览

已实现 OAuth／凭据库代码、加密 Drive API 和合成数据加密／恢复检查。默认版本没有 OAuth Client，登录停用。本机 HTTP 测试不代表真实 Google 或两台电脑验证。加密队列已能交换两个隔离副本、保留分支和重试 ID；空间／密钥引导现已实现（见下方）；Picker 和 GUI 同步控制仍待完成。

[Technical contract / 技術文件](../../CLOUD_SECURITY.md)

## 🐈 引导向导、接续和手动设置

Drive 向导分为登录设置、Google 授权、文件夹、加密／恢复和最后检查五步。完成的步骤和模式自动保存，重新打开后可继续；重新开始会归档旧进度，不删除 Drive 数据或密钥。完全手动设置展开所有区域，仍使用相同验证。无内置 Client 时可导入自己的桌面 OAuth JSON，也可直接输入文件夹 ID／链接。新空间先保存恢复文件再验证；加入另一台电脑的空间则导入其恢复文件。完成设置不等于启用 Agent 同步。

[Setup contract / 設定文件](../../SETUP_WIZARD.md)

## 🐈 账号核对与 ChatGPT 整合版

向导显示当前连接或已保存的 Google 账号，拒绝以不同账号继续。可移除本机登录后使用原账号重新连接，或重新开始设置以更换账号。旧进度仍可读取，明确重新连接后补上身份。

整合版 ChatGPT 桌面的本机 Codex／Worktree 纳入适配器目标；ChatGPT Work 本机任务须验证格式与继续能力。一般 ChatGPT 聊天与云端任务另行研究正式整合方式。目前尚未验证原生对话跨机恢复。

[OpenAI documentation](https://learn.chatgpt.com/docs/environments/modes) · [ChatGPT / Codex](https://learn.chatgpt.com/docs/use-chatgpt)

## 🐈 取消 Google 授权等待

连接时可取消等待浏览器授权，保留已保存步骤后再次连接。仅关闭浏览器会等待三分钟超时。Token 交换与账号检查须等待完成；无法取消的阶段会明确提示。
