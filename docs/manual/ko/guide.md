# 🐈 Bastet Agent Sync — 사용 안내

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

## 0.3.2

0.3.2은 로컬 대화와 Agent Memory OS를 동기화합니다. 소스를 선택하고 저장한 뒤 시작하세요. 수동 JSONL 내보내기는 필요 없습니다. Claude／Claude Code는 로컬 코딩 대화를, Codex／Work는 로컬 기록을 공유합니다. Agy는 SQLite를, Grok과 Pi는 기본 대화 파일을 사용합니다.

## 설정

언어, 장치 이름, Agent와 데이터 폴더를 선택합니다. Drive 마법사는 OAuth JSON, Google 인증, 폴더, 복구 키, 확인의 5단계입니다. 완료 단계는 자동 저장되며 이어서 설정, 다시 시작 또는 수동 설정에 같은 검증을 적용합니다. 다른 컴퓨터는 복구 키트로 같은 공간에 참여합니다.

## 동기화와 복원

업로드, 다운로드, 양방향 및 수동, 간격, 준실시간을 선택하고 저장 후 시작합니다. 소스마다 데이터 없음, 처리 중, 일부 완료, 실패를 표시합니다. 없는 대화는 자동 추가하며 다른 기존 파일은 충돌로 보호합니다. 일시 중지 후 스냅샷을 새 폴더에 복원하여 두 버전을 유지하세요. Agent Memory OS는 공식 인터페이스로 자동 백업 및 병합합니다.

## 제한

Claude 클라우드 대화, 일반 ChatGPT 대화 및 클라우드 Work는 제외합니다. 프로젝트 파일, 외부 첨부, 전체 설정／스킬, Cowork VM, 경로 매핑은 아직 완성되지 않았습니다. Agy／Grok 기본 재개 및 실제 두 컴퓨터 검증은 미완료입니다. 복원에는 로그인 정보가 포함되지 않습니다.

## 트레이와 업데이트

트레이에 상주하도록 설정할 수 있습니다. 설정 변경 전에 일시 중지하세요. 로고 아래 실제 버전이 표시됩니다. 업데이트 확인, 설치 후 명시적으로 재시작하며 서명된 공개 릴리스가 필요합니다.

## 검증

[Native sessions](../../NATIVE_SESSIONS.md) · [Agent Memory OS](../../AGENT_MEMORY_OS.md) · [Drive wizard](../../SETUP_WIZARD.md) · [Validation](../../VALIDATION.md) · [Plan](../../MASTER_PLAN.md)

```sh
npm ci
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```
