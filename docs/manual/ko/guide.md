# 🐈 Bastet Agent Sync — 사용 안내

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

## 0.4.0

0.4.0은 로컬 대화와 Agent Memory OS를 동기화합니다. 소스를 선택하고 저장한 뒤 시작하세요. 수동 JSONL 내보내기는 필요 없습니다. Claude／Claude Code는 로컬 코딩 대화를, Codex／Work는 로컬 기록을 공유합니다. Agy는 SQLite를, Grok과 Pi는 기본 대화 파일을 사용합니다.

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

## 🐈 다운로드 및 설치

[버전 및 다운로드](https://github.com/yamantaka520/Bastet-Agent-Sync/releases/latest)의 버전 번호를 누르면 OS별 직접 다운로드 링크가 표시됩니다. Windows는 누락된 WebView2를 내장 설치기로 설치하며 macOS는 시스템 WebKit을 사용합니다. Linux는 다운로드한 `sh install-linux.sh`로 apt/dnf 감지, 검증 및 의존성 설치를 수행합니다. Node.js와 Rust는 필요하지 않습니다. AMOS CLI와 Agent 로그인은 별도 설정이 필요합니다. macOS 공증 및 Windows Authenticode 서명이 없어 OS 확인이 표시될 수 있습니다.

macOS에서 첫 실행이 차단되면 다운로드 출처를 확인하고 시스템 설정 → 개인정보 보호 및 보안 → 그래도 열기에서 이 앱을 승인하세요. [Apple](https://support.apple.com/en-au/guide/mac-help/mh40616/mac)


### 🔐 0.4.1

동기화 전에 **🔐 자격 증명 접근 준비**를 누르면 저장된 정보를 함께 읽습니다. macOS에서는 Bastet의 각 항목에 “항상 허용”을 선택하세요. 정보는 메모리에 보관되며 앱 종료, 로그인 정보 삭제, 설정 재시작 또는 클라이언트 변경 시 지워집니다. 트레이로 닫으면 유지됩니다. 외부에서 변경했다면 버튼으로 다시 읽으세요. Google 연결 확인이나 동기화 시작은 수행하지 않습니다. 컴퓨터마다 승인이 필요하며 Developer ID 서명이 아직 없어 업데이트 후 다시 요청할 수 있습니다.
