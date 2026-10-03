# 🐈 Bastet Agent Sync v@VERSION@

繁體中文 · 简体中文 · English · 日本語 · 한국어

@DOWNLOADS@

## 繁體中文

0.7.0 可選擇一個同步目的地：Google Drive API、iCloud Drive 資料夾或 OneDrive 資料夾。後兩者使用已安裝的雲端桌面同步程式，不提供 Apple／Microsoft 直接登入。選擇已下載的專用資料夾、建立加密空間，並將恢復套件存於同步資料夾外；另一台電腦用同一服務的套件加入。切換目的地不會搬移舊空間資料。憑證入口現在提供恢復套件匯出及儲存方式說明。

本機同步完成只代表 Bastet 將加密封包交給資料夾；雲端上傳、下載及送達另一台電腦由服務的桌面程式負責。iCloud／OneDrive 尚無兩台實機交付驗證。macOS 鑰匙圈選擇「永遠允許」後仍重複提示的問題尚未解決。對話接續與 Agent 限制見 VALIDATION.md；不會覆寫正在使用的 Agent 資料。

獨立下載安裝包不需要 Node.js 或 Rust。Google API 模式不需要 Drive Desktop；iCloud／OneDrive 資料夾模式需要對應桌面程式。macOS 發布須通過 Developer ID 簽章、公證及安裝驗證。Windows 發行者簽章暫緩；更新包仍驗證簽章。

## 简体中文

0.7.0 可选择一个同步目标：Google Drive API、iCloud Drive 文件夹或 OneDrive 文件夹。后两者使用已安装的云端桌面同步程序，不提供 Apple／Microsoft 直接登录。选择已下载的专用文件夹、创建加密空间，并把恢复套件保存在同步文件夹之外；另一台电脑用同一服务的套件加入。切换目标不会迁移旧空间数据。凭据入口现在提供恢复套件导出及存储方式说明。

本地同步完成只表示 Bastet 已将加密数据交给文件夹；上传、下载及送达另一台电脑由对应的桌面程序负责。iCloud／OneDrive 尚未完成两台实体电脑交付验证。macOS 钥匙串选择“始终允许”后仍反复提示的问题尚未解决。对话续接和 Agent 限制见 VALIDATION.md；不会覆盖正在使用的 Agent 数据。

独立下载安装包不需要 Node.js 或 Rust。Google API 模式不需要 Drive Desktop；iCloud／OneDrive 文件夹模式需要对应桌面程序。macOS 发布须通过 Developer ID 签名、公证和安装验证。Windows 发布者签名暂缓；更新包仍验证签名。

## English

0.7.0 lets you select one sync destination: the Google Drive API, an iCloud Drive folder, or a OneDrive folder. Folder modes use the provider's installed desktop sync client; they do not sign in to Apple or Microsoft directly. Choose a dedicated folder kept downloaded, create an encrypted space, and save its recovery kit outside that folder. Join on another computer with a kit for the same provider. Switching destinations does not migrate the old space. The credentials entry now explains storage and exports another recovery kit.

A completed local cycle means Bastet handed encrypted objects to the selected folder. The provider's client handles upload, download, and delivery to another computer. Physical two-device delivery through iCloud or OneDrive has not been verified. Repeated macOS Keychain prompts after “Always Allow” remain unresolved. See VALIDATION.md for conversation continuation and agent limits; active agent stores are not overwritten.

Standalone installers do not require Node.js or Rust. Google API mode needs no Drive Desktop; iCloud and OneDrive folder modes require their respective desktop sync clients. macOS publication requires Developer ID signing, notarization, and installer verification. Windows publisher signing is deferred; update packages still require signature verification.

## 日本語

0.7.0 では同期先を Google Drive API、iCloud Drive フォルダー、OneDrive フォルダーから一つ選べます。フォルダーモードは導入済みの各社デスクトップ同期アプリを使用し、Apple／Microsoft への直接ログインは行いません。ダウンロード済みの専用フォルダーを選び、暗号化スペースを作成して復元キットを同期フォルダーの外に保存します。別端末では同じサービス用のキットで参加します。同期先を変更しても旧スペースのデータは移行されません。認証情報画面から復元キットを再出力できます。

ローカル同期の完了は、Bastet が暗号化データを選択フォルダーに渡したことを示します。クラウドへの送受信と別端末への配信は各社の同期アプリが担います。iCloud／OneDrive を使った実機２台間の配信は未検証です。macOS キーチェーンで「常に許可」を選んだ後の繰り返し確認は未解決です。会話再開と Agent の制限は VALIDATION.md を参照してください。使用中の Agent データは上書きしません。

単体配布版に Node.js と Rust は不要です。Google API モードに Drive Desktop は不要ですが、iCloud／OneDrive フォルダーモードには対応するデスクトップ同期アプリが必要です。macOS の公開には Developer ID 署名、公証、インストーラー検証が必要です。Windows 発行者署名は延期中で、更新パッケージの署名検証は継続します。

## 한국어

0.7.0에서는 Google Drive API, iCloud Drive 폴더, OneDrive 폴더 중 하나를 동기화 대상으로 선택할 수 있습니다. 폴더 모드는 설치된 제공업체의 데스크톱 동기화 앱을 사용하며 Apple／Microsoft 계정에 직접 로그인하지 않습니다. 다운로드된 전용 폴더를 선택하고 암호화 공간을 만든 뒤 복구 키트를 동기화 폴더 밖에 보관하세요. 다른 컴퓨터에서는 같은 서비스용 키트로 참여합니다. 대상을 바꿔도 이전 공간의 데이터는 자동으로 옮겨지지 않습니다. 자격 증명 메뉴에서 보관 방법을 확인하고 복구 키트를 다시 내보낼 수 있습니다.

로컬 동기화 완료는 Bastet이 암호화된 데이터를 선택한 폴더에 전달했음을 뜻합니다. 클라우드 업로드, 다운로드, 다른 컴퓨터로의 전달은 제공업체의 앱이 담당합니다. iCloud／OneDrive를 통한 실제 두 컴퓨터 간 전달은 아직 검증하지 않았습니다. macOS 키체인에서 “항상 허용”을 선택한 뒤에도 반복되는 요청은 해결되지 않았습니다. 대화 이어가기와 Agent 지원 범위는 VALIDATION.md를 확인하세요. 사용 중인 Agent 저장소는 덮어쓰지 않습니다.

독립 설치 파일에는 Node.js와 Rust가 필요하지 않습니다. Google API 모드에는 Drive Desktop이 필요 없지만 iCloud／OneDrive 폴더 모드에는 각 서비스의 데스크톱 동기화 앱이 필요합니다. macOS 공개에는 Developer ID 서명, 공증, 설치 검증이 필요합니다. Windows 게시자 서명은 보류 중이며 업데이트 패키지 서명 검증은 계속 적용됩니다.
