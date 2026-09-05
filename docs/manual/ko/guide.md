# 🐈 Bastet Agent Sync — 사용 안내 및 계획

[繁體中文](../zh-Hant/guide.md) · [简体中文](../zh-Hans/guide.md) · [English](../en/guide.md) · [日本語](../ja/guide.md) · [한국어](../ko/guide.md)

다른 컴퓨터에서도 에이전트와 대화를 이어 가세요.

> 개발을 시작했습니다. M1은 데스크톱 기반 단계이며 실제 동기화와 다른 컴퓨터에서의 대화 재개는 아직 제공하지 않습니다.

## 예정된 설정 절차

1. 언어와 기기 이름을 선택합니다.
2. Claude, Claude Code, Codex, Google Agy CLI, Grok Build CLI, Pi Agent를 감지하고 선택합니다. 경로를 직접 지정할 수도 있습니다.
3. Google Drive에 연결하고 공유 폴더를 선택하거나 만듭니다. Linux는 추후 구현할 API 직접 연결을 사용합니다.
4. 양방향, 업로드만, 다운로드만 중 하나와 수동, 사용자 지정 간격, 실시간에 가까운 주기를 선택한 뒤 시작합니다.
5. 설정을 켜면 창을 트레이로 닫을 수 있으며 트레이에서 일시 정지하거나 재개할 수 있습니다.

## 현재 기반 기능

대화 내용을 읽지 않고 후보 데이터 디렉터리를 감지하며 로컬 폴더를 선택하고 설정을 저장합니다. 동기화 엔진 구현 전에는 시작 버튼이 비활성화됩니다. 브라우저 미리 보기에서는 네이티브 기능을 사용할 수 없습니다.

## 대화 재개

네이티브 복원에는 검증된 어댑터가 필요합니다. 문맥을 전달하는 방식은 새 대화로 명확히 표시합니다. 내보낼 수 있는 첨부 파일, 도구 결과, 프로젝트 경로를 보존합니다. 동시 변경은 분기로 유지하고 자격 증명은 로컬에 보관합니다.

## 로드맵

M0: 저장소, 계획, 5개 언어, 고양이 아이콘. M1: 데스크톱, 감지, 설정, 트레이. M2: 스냅샷, 충돌, 로컬 전송. M3: Google Drive 및 암호화. M4: 에이전트 네이티브 복원. M5: 예약 및 3개 플랫폼 배포.

## 개발 방법

Node.js, Rust 및 Tauri 플랫폼 필수 구성 요소를 설치하고 저장소 루트에서 다음 명령을 실행합니다.

```sh
npm ci
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```

## 검증 및 제한

Windows／Linux 빌드와 실제 트레이 동작은 검증이 필요합니다. M1은 클라우드 로그인, 업로드 또는 대화 가져오기를 수행하지 않습니다. 대화 기록을 이동해도 실행 중인 프로세스는 이동하지 않습니다.

## 참고 문서

[Master plan](../../MASTER_PLAN.md) · [Validation](../../VALIDATION.md) · [Requirements](../../../REQUIREMENTS.md) · [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
