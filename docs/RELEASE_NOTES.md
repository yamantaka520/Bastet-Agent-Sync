# 🐈 Bastet Agent Sync v@VERSION@

繁體中文 · 简体中文 · English · 日本語 · 한국어

@DOWNLOADS@

## 繁體中文

0.5.1 修正同一雲端資料夾混有不同加密空間時的同步中斷。驗證目前空間後，程式會略過其他空間的資料，繼續處理本空間資料，並顯示部分完成。錯誤金鑰、損壞資料與不支援的版本仍會停止處理。

已有同步資料的資料夾不能直接建立新空間；加入時請匯入原始裝置的恢復檔，或為新空間選擇空資料夾。OAuth client JSON 只用於 Google 登入設定。同一份恢復檔已驗證正確時，不需要重設或重建空間；缺少預期資料時再核對恢復檔。隔離測試已通過，兩台實機同步仍待驗證。

安裝包：Windows 以內含離線安裝器補齊 WebView2；macOS 使用系統 WebKit；Linux `install-linux.sh` 選擇 apt/dnf、校驗下載並安裝相依套件。不需要 Node.js、Rust 或 Drive Desktop。更新包有簽章；Apple 公證與 Windows Authenticode 不在本次範圍，可能出現 OS 信任提示。


## 简体中文

0.5.1 修复同一个云端文件夹混有不同加密空间时的同步中断。验证当前空间后，程序会跳过其他空间的数据，继续处理本空间数据，并显示部分完成。错误密钥、损坏数据和不支持的版本仍会停止处理。

已有同步数据的文件夹不能直接创建新空间；加入时请导入原始设备的恢复文件，或为新空间选择空文件夹。OAuth client JSON 仅用于 Google 登录设置。同一份恢复文件已验证正确时，无需重置或重建空间；缺少预期数据时再核对恢复文件。隔离测试已通过，两台实机同步仍待验证。

安装包：Windows 使用内置离线安装器补齐 WebView2；macOS 使用系统 WebKit；Linux `install-linux.sh` 自动选择 apt/dnf、校验下载并安装依赖。不需要 Node.js、Rust 或 Drive Desktop。更新包有签名；Apple 公证和 Windows Authenticode 不在本次范围，可能出现系统信任提示。


## English

0.5.1 fixes interrupted synchronization when one cloud folder contains multiple encrypted spaces. After verifying the current space, Bastet skips data from other spaces, continues processing its own data, and shows a partial result. Wrong keys, malformed data, and unsupported versions still stop processing.

A folder containing sync data cannot be used to create a new space. Join with the recovery kit from the original device, or choose an empty folder for a new space. An OAuth client JSON only configures Google sign-in. If the same recovery kit already verifies correctly, no reset or recreation is needed; check it again if expected data is missing. Isolated tests passed; physical two-computer sync remains unverified.

Installers: Windows uses the embedded offline installer for missing WebView2; macOS uses system WebKit; Linux `install-linux.sh` selects apt/dnf, verifies downloads, and installs dependencies. Node.js, Rust, and Drive Desktop are not required. Update packages are signed. Apple notarization and Windows Authenticode are outside this release scope, so OS trust prompts may appear.


## 日本語

0.5.1 は、同じクラウドフォルダーに複数の暗号化スペースが混在する場合の同期中断を修正します。現在のスペースを検証した後、別スペースのデータをスキップし、自分のスペースの処理を続け、部分完了を表示します。誤った鍵、破損データ、不対応バージョンでは処理を停止します。

同期データがあるフォルダーに新しいスペースは作成できません。参加には元の端末のリカバリーキットを使い、新規作成には空のフォルダーを選んでください。OAuth client JSON は Google ログインの設定用です。同じキットが正しく検証済みならリセットや再作成は不要です。必要なデータがない場合にキットを確認してください。隔離テストは成功しましたが、実機2台の同期は未検証です。

インストーラー：Windows は同梱のオフラインインストーラーで不足する WebView2 を導入し、macOS はシステム WebKit、Linux の `install-linux.sh` は apt/dnf の選択、ダウンロード検証、依存パッケージ導入を行います。Node.js、Rust、Drive Desktop は不要です。更新パッケージには署名があります。Apple 公証と Windows Authenticode は対象外のため、OS の確認画面が出る場合があります。


## 한국어

0.5.1은 하나의 클라우드 폴더에 여러 암호화 공간이 섞여 있을 때 동기화가 중단되는 문제를 수정합니다. 현재 공간을 확인한 후 다른 공간의 데이터를 건너뛰고 현재 공간의 처리를 계속하며 부분 완료를 표시합니다. 잘못된 키, 손상된 데이터와 지원되지 않는 버전은 처리를 중단합니다.

동기화 데이터가 있는 폴더에는 새 공간을 만들 수 없습니다. 참여하려면 원래 장치의 복구 키트를 가져오고, 새 공간을 만들려면 빈 폴더를 선택하세요. OAuth client JSON은 Google 로그인 설정용입니다. 같은 키트가 이미 올바르게 확인되었다면 재설정이나 재생성이 필요 없습니다. 예상한 데이터가 없을 때 키트를 확인하세요. 격리 테스트는 통과했지만 실제 컴퓨터 두 대의 동기화는 아직 검증되지 않았습니다.

설치 프로그램: Windows는 포함된 오프라인 설치기로 누락된 WebView2를 설치하고, macOS는 시스템 WebKit을 사용하며, Linux의 `install-linux.sh`는 apt/dnf 선택, 다운로드 검증과 의존성 설치를 수행합니다. Node.js, Rust, Drive Desktop은 필요하지 않습니다. 업데이트 패키지에는 서명이 있습니다. Apple 공증과 Windows Authenticode는 이번 범위에서 제외되어 OS 확인이 나타날 수 있습니다.

[Validation](https://github.com/yamantaka520/Bastet-Agent-Sync/blob/main/docs/VALIDATION.md) · [Sync control contract](https://github.com/yamantaka520/Bastet-Agent-Sync/blob/main/docs/SYNC_CONTROL.md)
