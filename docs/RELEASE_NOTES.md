# 🐈 Bastet Agent Sync v@VERSION@

繁體中文 · 简体中文 · English · 日本語 · 한국어

@DOWNLOADS@

## 繁體中文

0.8.1 是 0.8.0（Hermes Agent）的維護修補，修正大型可攜工作區多段傳輸的就緒狀態。只有當索引宣稱的每個分段都存在且索引一致時，才會顯示「已收到分段 — 還原前會驗證完整性」，不再顯示「就緒」或「已驗證」；完整但遭竄改的傳輸仍會在比對／還原時（重新組裝並逐塊驗證後）被拒絕。分段存在性只採信格式正確的 `PortablePart`：偽造或僅以子字串嵌入傳輸 ID 的內容，無法再造成假的完成狀態。原生崩潰視窗的 profile 列舉上限改為對齊 `MAX_MANIFEST_FILES`，使超過 2048 個檔案的正當 home 可被還原，而非被判為 `session_limit`。0.8.0 的 Hermes 來源、三個互斥目的地選項與恢復套件維持不變。

多段傳輸採緩衝方式而非串流；測試使用記憶體模擬 relay，不等於實際雲端交付。就緒狀態只代表分段存在與索引一致，位元組層級完整性一律由比對／還原驗證。Hermes 合成測試不等於實際雙機交付、模型實機續聊、Windows／Linux 執行期或 App Store 驗收，這些關卡仍未達成。0.7.0 的 macOS 鑰匙圈重複提示問題維持不變。

獨立下載安裝包不需要 Node.js 或 Rust。Google API 模式不需要 Drive Desktop；iCloud／OneDrive 資料夾模式需要對應桌面程式。macOS 發布須通過 Developer ID 簽章與公證；Windows 發行者簽章暫緩；更新包仍驗證簽章。

## 简体中文

0.8.1 是 0.8.0（Hermes Agent）的维护修补，修正大型可携工作区多段传输的就绪状态。只有当索引声明的每个分段都存在且索引一致时，才显示“已收到分段 — 还原前会验证完整性”，不再显示“就绪”或“已验证”；完整但被篡改的传输仍会在比对／还原时（重新组装并逐块验证后）被拒绝。分段存在性只采信格式正确的 `PortablePart`：伪造或仅以子字符串嵌入传输 ID 的内容，无法再造成假的完成状态。原生崩溃窗口的 profile 枚举上限改为对齐 `MAX_MANIFEST_FILES`，使超过 2048 个文件的正当 home 可被还原，而非被判为 `session_limit`。0.8.0 的 Hermes 来源、三个互斥目标选项与恢复套件保持不变。

多段传输采用缓冲方式而非流式；测试使用内存模拟 relay，不等于实际云端交付。就绪状态只代表分段存在与索引一致，字节级完整性一律由比对／还原验证。Hermes 合成测试不等于实际双机交付、模型实机续聊、Windows／Linux 运行期或 App Store 验收，这些关卡仍未达成。0.7.0 的 macOS 钥匙串重复提示问题保持不变。

独立下载安装包不需要 Node.js 或 Rust。Google API 模式不需要 Drive Desktop；iCloud／OneDrive 文件夹模式需要对应桌面程序。macOS 发布须通过 Developer ID 签名与公证；Windows 发布者签名暂缓；更新包仍验证签名。

## English

0.8.1 is a maintenance patch on top of 0.8.0 (Hermes Agent) that fixes the readiness state of large portable-workspace multipart transfers. A transfer shows “parts received — integrity will be verified before restore” only when every part named by the index is present and the index is internally consistent — never “ready” or “verified”; a complete-but-tampered transfer is still refused at compare/restore (which reassembles and verifies every chunk). Part presence is attributed only from a well-formed `PortablePart`: a forged or truncated bundle that merely embeds a transfer id as a substring can no longer inflate presence into a false complete state. The native crash-window profile enumerator now bounds its walk by `MAX_MANIFEST_FILES` instead of a fixed 2048, so a legitimate home with more than 2048 captured files is restorable rather than refused as `session_limit`. The 0.8.0 Hermes source, the three mutually exclusive destinations and the recovery kits are unchanged.

The multipart transport is buffered, not streaming, and the tests use an in-memory mock relay, which is not real cloud delivery. Readiness means part presence plus index consistency only; byte-level integrity is always verified by compare/restore. Hermes fixtures are not physical two-device delivery, live-model continuation, Windows/Linux runtime or App Store acceptance; those gates remain open. The 0.7.0 unresolved repeated macOS Keychain prompt is unchanged.

Standalone installers do not require Node.js or Rust. Google API mode needs no Drive Desktop; iCloud and OneDrive folder modes require their respective desktop sync clients. macOS publication requires Developer ID signing and notarization; Windows publisher signing is deferred; update packages still require signature verification.

## 日本語

0.8.1 は 0.8.0（Hermes Agent）に対するメンテナンス修正で、大規模な可搬ワークスペースのマルチパート転送の準備状態を修正します。索引が示すすべてのパートが存在し索引が内部的に整合している場合にのみ「パートを受信済み — 復元前に完全性を検証します」と表示し、「準備完了」「検証済み」とは表示しません。完全だが改ざんされた転送は、比較／復元時（再組み立てと全チャンク検証を行う）に拒否されます。パートの存在は整形済みの `PortablePart` のみから判定し、転送 ID を部分文字列として埋め込んだだけの偽造／切詰めバンドルは存在を水増しして誤った「完了」状態にできません。ネイティブのクラッシュウィンドウの profile 列挙上限は固定 2048 ではなく `MAX_MANIFEST_FILES` に揃えられ、2048 ファイルを超える正当な home も `session_limit` で拒否されず復元できます。0.8.0 の Hermes ソース、三つの排他的な同期先、復元キットは変更ありません。

マルチパート転送はストリーミングではなくバッファ方式で、テストはインメモリのモック relay を使用するため実クラウド配信ではありません。準備状態はパート存在と索引整合のみを意味し、バイトレベルの完全性は常に比較／復元で検証されます。Hermes の合成テストは実機２台への配信、モデル実機での再開、Windows／Linux 実行時、App Store の受入ではありません。これらのゲートは未達のままです。0.7.0 の未解決の macOS キーチェーン繰り返し確認は変わりません。

単体配布版に Node.js と Rust は不要です。Google API モードに Drive Desktop は不要ですが、iCloud／OneDrive フォルダーモードには対応するデスクトップ同期アプリが必要です。macOS の公開には Developer ID 署名と公証が必要です。Windows 発行者署名は延期中で、更新パッケージの署名検証は継続します。

## 한국어

0.8.1은 0.8.0(Hermes Agent)에 대한 유지보수 패치로, 대용량 휴대형 워크스페이스의 멀티파트 전송 준비 상태를 바로잡습니다. 인덱스가 지정한 모든 파트가 존재하고 인덱스가 내부적으로 일관될 때에만 "파트 수신됨 — 복원 전 무결성을 검증합니다"를 표시하며, "준비됨"이나 "검증됨"은 표시하지 않습니다. 완전하지만 변조된 전송은 비교/복원 시(재조립 및 모든 청크 검증) 거부됩니다. 파트 존재는 형식이 올바른 `PortablePart`에서만 판정하며, 전송 ID를 부분 문자열로 끼워 넣은 위조/절단 번들은 존재를 부풀려 잘못된 "완료" 상태를 만들 수 없습니다. 네이티브 크래시 윈도의 profile 열거 상한은 고정 2048이 아니라 `MAX_MANIFEST_FILES`에 맞춰져, 2048개를 초과하는 정당한 home도 `session_limit`으로 거부되지 않고 복원됩니다. 0.8.0의 Hermes 소스, 세 개의 상호 배타적 대상, 복구 키트는 변경되지 않았습니다.

멀티파트 전송은 스트리밍이 아닌 버퍼 방식이며, 테스트는 인메모리 모의 relay를 사용하므로 실제 클라우드 전달이 아닙니다. 준비 상태는 파트 존재와 인덱스 일관성만을 의미하고, 바이트 수준 무결성은 항상 비교/복원으로 검증됩니다. Hermes 합성 테스트는 실제 두 기기 전달, 모델 실기 재개, Windows／Linux 런타임, App Store 검증이 아니며 이 게이트들은 여전히 미충족입니다. 0.7.0의 미해결 macOS 키체인 반복 요청은 그대로입니다.

독립 설치 파일에는 Node.js와 Rust가 필요하지 않습니다. Google API 모드에는 Drive Desktop이 필요 없지만 iCloud／OneDrive 폴더 모드에는 각 서비스의 데스크톱 동기화 앱이 필요합니다. macOS 공개에는 Developer ID 서명과 공증이 필요합니다. Windows 게시자 서명은 보류 중이며 업데이트 패키지 서명 검증은 계속 적용됩니다.
