# 🐈 Bastet Agent Sync — 使用指南与规划

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

让 Agent 和对话随你切换电脑的桌面助手。

> 已开始开发。M1 提供桌面基础功能；实际同步和跨电脑续接暂未开放。

## 计划中的设置流程

1. 选择语言和设备名称。
2. 自动检测并选择 Claude、Claude Code、Codex、Google Agy CLI、Grok Build CLI、Pi Agent；支持自定义路径。
3. 连接 Google Drive，选择或创建共享文件夹。Linux 将通过计划中的 API 直接连接。
4. 选择双向、仅上传或仅下载，以及手动、自定义间隔或近实时模式；设置完成后点击启动。
5. 启用后可关闭到系统托盘，也可从托盘暂停／恢复。

## 当前基础功能

检测候选数据目录而不读取对话内容，选择本地文件夹并保存偏好设置。同步引擎实现前不开放启动按钮。浏览器预览无法使用原生功能。

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
