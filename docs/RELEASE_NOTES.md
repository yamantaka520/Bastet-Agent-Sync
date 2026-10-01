# 🐈 Bastet Agent Sync v@VERSION@

繁體中文 · 简体中文 · English · 日本語 · 한국어

@DOWNLOADS@

## 繁體中文

0.6.0 加入跨系統專案路徑對應與獨立對話接續目錄。收到的版本不會覆寫原本 Agent 資料；在還原目錄繼續對話後，可同步回傳子版本，同時修改則保留分支。請將參與同步的電腦全部升級，並設定每台電腦的本機專案路徑。

同步會略過未變動內容的重複壓縮、合併重疊雲端查詢，並降低進度回報成本。仍驗證完整內容、子對話及資料庫 WAL，沒有以快取略過空間金鑰驗證。

目前支援範圍與實測證據見儲存庫的 VALIDATION.md 與 CROSS_OS_HANDOFF.md。官方讀取器能開啟資料不等於模型已成功續聊；Agy 仍以資料庫還原為限。專案程式碼、附件、外部依賴與登入憑證不會自動搬移，還原環境可能需要重新登入。

安裝包使用內建／系統元件，不需要 Node.js、Rust 或 Drive Desktop。macOS 發布需通過 Developer ID、公證及 Gatekeeper 驗證。Windows Authenticode 暫緩，可能出現系統信任提示；更新包簽章仍須驗證，與作業系統發行者簽章不同。

## 简体中文

0.6.0 加入跨系统项目路径映射与独立对话续接目录。收到的版本不会覆盖原有 Agent 数据；在还原目录继续对话后，可同步回传子版本，同时修改则保留分支。请升级所有参与同步的电脑，并配置各自的本地项目路径。

同步会跳过未变动内容的重复压缩、合并重叠云端查询，并降低进度回报开销。仍验证完整内容、子对话及数据库 WAL，不会用缓存跳过空间密钥验证。

支持范围与实测证据见仓库的 VALIDATION.md 和 CROSS_OS_HANDOFF.md。官方读取器能打开数据不代表模型已成功续聊；Agy 仍限于数据库恢复。项目代码、附件、外部依赖和登录凭据不会自动迁移，还原环境可能需要重新登录。

安装包使用内置／系统组件，不需要 Node.js、Rust 或 Drive Desktop。macOS 发布须通过 Developer ID、公证和 Gatekeeper 验证。Windows Authenticode 暂缓，可能出现系统信任提示；更新包签名仍须验证，与系统发布者签名不同。

## English

0.6.0 adds cross-platform project path mappings and isolated conversation profiles. Received versions never overwrite your default agent store. Continuing a restored profile publishes a child version back to the other computer; concurrent edits retain separate branches. Upgrade every participating computer and configure its local project paths.

Synchronization avoids recompressing unchanged captures, shares overlapping cloud listings and reduces progress-report overhead. Complete content, child conversations and SQLite WAL snapshots are still checked. Caching never substitutes for space-key proof verification.

See VALIDATION.md and CROSS_OS_HANDOFF.md in the repository for tested scope and evidence. A successful official-reader check does not establish model continuation. Agy remains database recovery. Project code, attachments, external dependencies and credentials are not automatically transferred; restored profiles may need local sign-in.

Installers use bundled or system components; Node.js, Rust and Drive Desktop are not required. macOS publication requires Developer ID signing, notarization and Gatekeeper checks. Windows Authenticode is deferred, so OS trust prompts may appear. Update-package signatures remain mandatory and are separate from OS publisher signatures.

## 日本語

0.6.0 では、OS 間のプロジェクトパス対応と、会話再開用の独立したプロファイルを追加しました。受信したバージョンは既存の Agent データを上書きしません。復元先で会話を続けると子バージョンを同期し、同時編集では両方の分岐を保持します。同期するすべての端末を更新し、各端末のプロジェクトパスを設定してください。

変更のない内容の再圧縮を省き、重複するクラウド一覧取得を共有し、進捗通知の負荷を軽減します。内容、子会話、SQLite WAL の検証と同期空間の鍵の確認は引き続き行います。

検証範囲と結果はリポジトリの VALIDATION.md と CROSS_OS_HANDOFF.md を参照してください。公式リーダーで読み込めることは、モデルによる会話再開の成功を意味しません。Agy はデータベース復元に限定されます。コード、添付ファイル、外部依存関係、認証情報は自動転送されず、復元先で再ログインが必要な場合があります。

Node.js、Rust、Drive Desktop の別途導入は不要です。macOS の公開には Developer ID 署名、公証、Gatekeeper 検証が必要です。Windows Authenticode は延期しているため、OS の信頼確認が表示される場合があります。更新パッケージの署名検証は必須であり、OS の発行者署名とは別です。

## 한국어

0.6.0은 운영체제 간 프로젝트 경로 매핑과 대화 재개용 독립 프로필을 추가합니다. 수신한 버전은 기존 Agent 저장소를 덮어쓰지 않습니다. 복원 프로필에서 대화를 이어가면 하위 버전을 다시 동기화하며, 동시 수정은 별도 분기로 보존합니다. 참여하는 모든 컴퓨터를 업데이트하고 각 컴퓨터의 프로젝트 경로를 설정하세요.

변경되지 않은 내용의 반복 압축을 생략하고, 겹치는 클라우드 목록 요청을 공유하며, 진행 알림 비용을 줄입니다. 전체 내용, 하위 대화, SQLite WAL과 동기화 공간 키 검증은 계속 수행합니다.

검증 범위와 결과는 저장소의 VALIDATION.md 및 CROSS_OS_HANDOFF.md를 확인하세요. 공식 리더가 데이터를 읽는다는 사실만으로 모델의 대화 재개 성공을 의미하지 않습니다. Agy는 데이터베이스 복원으로 제한됩니다. 코드, 첨부 파일, 외부 의존성 및 인증 정보는 자동 전송되지 않으며, 복원 환경에서 다시 로그인해야 할 수 있습니다.

Node.js, Rust, Drive Desktop을 별도로 설치할 필요가 없습니다. macOS 공개에는 Developer ID 서명, 공증 및 Gatekeeper 검증이 필요합니다. Windows Authenticode는 보류되어 OS 신뢰 경고가 표시될 수 있습니다. 업데이트 패키지 서명 검증은 필수이며 OS 게시자 서명과는 별개입니다.
