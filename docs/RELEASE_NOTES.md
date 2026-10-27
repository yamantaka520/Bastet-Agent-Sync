# 🐈 Bastet Agent Sync v@VERSION@

繁體中文 · 简体中文 · English · 日本語 · 한국어

@DOWNLOADS@

## 繁體中文

0.8.2 修補 Google Drive 下載快取的磁碟耗盡風險。相同封包內容共用一份本機快取，遠端檔案 ID 只保留小型索引；快取寫入前會套用容量限制，並清理過期、無引用與舊格式快取。這可防止同一有效密文的多個遠端副本各自占用完整磁碟空間。原有加密、空間驗證、更新簽章與獨立還原保護持續保留。快取是可重建資料，清理不刪除雲端或本機對話歷史。

回歸測試使用合成物件及隔離暫存目錄，不代表實際 Drive 攻擊或雙機交付驗收。此版不宣稱修復 Hermes 就緒標示、macOS 鑰匙圈重複提示或完成 App Store 審核。獨立安裝包不需要 Node.js/Rust；iCloud／OneDrive 資料夾模式仍需對應桌面程式。macOS 需 Developer ID 簽章與公證；Windows 發行者簽章暫緩，更新包仍驗證簽章。

## 简体中文

0.8.2 修补 Google Drive 下载缓存的磁盘耗尽风险。相同封包内容共用一份本地缓存，远端文件 ID 只保留小型索引；缓存写入前应用容量限制，并清理过期、无引用和旧格式缓存，避免同一有效密文的多个远端副本分别占用完整磁盘空间。原有加密、空间验证、更新签名及独立恢复保护保留。清理可重建缓存不会删除云端或本地对话历史。

回归测试使用合成对象和隔离临时目录，不代表实际 Drive 攻击或双机交付验收。此版不宣称修复 Hermes 就绪标记、macOS 钥匙串重复提示或完成 App Store 审核。独立安装包不需要 Node.js/Rust；iCloud／OneDrive 文件夹模式仍需对应桌面程序。macOS 须签名与公证；Windows 发布者签名暂缓，更新包仍验证签名。

## English

0.8.2 fixes disk exhaustion through the Google Drive download cache. Identical validated bundles share one local content object; remote file IDs retain small revision references. Admission enforces a capacity budget before cache writes and removes expired, unreferenced and legacy cache data, preventing copied valid ciphertext from creating a full local copy for every remote ID. Encryption, space validation, updater signatures and isolated restore remain enforced. Cache cleanup removes only rebuildable data, never cloud or local conversation history.

Regression tests use synthetic objects and isolated temporary directories, not a live Drive attack or physical two-device delivery. This release does not claim to resolve Hermes readiness labels, repeated macOS Keychain prompts or App Store review. Standalone installers need no Node.js/Rust; iCloud/OneDrive folder modes require their desktop clients. macOS requires Developer ID signing and notarization; Windows publisher signing remains deferred, while updater signatures remain required.

## 日本語

0.8.2 は Google Drive ダウンロードキャッシュによるディスク枯渇の危険を修正します。同じ検証済みバンドルは一つのローカルデータを共有し、リモートファイル ID は小さな版情報のみを保持します。書き込み前に容量上限を適用し、期限切れ、未参照、旧形式のキャッシュを削除します。同じ有効な暗号文の複製がそれぞれ完全なローカルコピーを作ることを防ぎます。暗号化、スペース検証、更新署名、隔離復元は維持されます。キャッシュ削除でクラウドやローカルの会話履歴は削除しません。

回帰テストは合成データと一時ディレクトリを使用し、実際の Drive 攻撃や実機二台への配信の検証ではありません。Hermes の準備状態、macOS キーチェーンの繰り返し確認、App Store 審査の解決は主張しません。単体版に Node.js/Rust は不要です。iCloud/OneDrive にはデスクトップクライアントが必要です。macOS は Developer ID 署名と公証が必要で、Windows 発行者署名は延期中ですが更新署名は必須です。

## 한국어

0.8.2는 Google Drive 다운로드 캐시를 통한 디스크 고갈 위험을 수정합니다. 동일한 검증된 번들은 하나의 로컬 콘텐츠를 공유하고 원격 파일 ID에는 작은 버전 참조만 보관합니다. 캐시 쓰기 전에 용량 제한을 적용하고 만료되거나 참조되지 않는 데이터 및 이전 형식 캐시를 정리하여, 복제된 유효 암호문마다 전체 로컬 사본이 생기는 일을 막습니다. 암호화, 공간 검증, 업데이트 서명 및 격리 복원 보호는 유지됩니다. 캐시 정리는 클라우드나 로컬 대화 기록을 삭제하지 않습니다.

회귀 테스트는 합성 데이터와 격리 임시 디렉터리를 사용하며 실제 Drive 공격이나 실제 두 기기 전달 검증은 아닙니다. Hermes 준비 상태, 반복되는 macOS 키체인 요청 또는 App Store 심사 해결을 주장하지 않습니다. 독립 설치 파일은 Node.js/Rust가 필요 없고 iCloud/OneDrive 폴더 모드에는 데스크톱 클라이언트가 필요합니다. macOS는 Developer ID 서명과 공증이 필수이며 Windows 게시자 서명은 보류 중이지만 업데이트 서명은 계속 검증합니다.
