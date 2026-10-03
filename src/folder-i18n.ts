import type { Locale } from "./i18n";

const en = {
  provider: "Sync destination",
  google: "Google Drive",
  icloud: "iCloud Drive folder",
  onedrive: "OneDrive folder",
  title: "Synced folder setup",
  readyBadge: "✅ Local space ready",
  pendingBadge: "⚪ Local space pending",
  clientStatus: "🟡 Sync client delivery unverified",
  intro:
    "Choose a folder inside your provider's synced drive. Bastet stores encrypted sync objects there; the provider's desktop client moves them between computers.",
  pick: "Choose synced folder",
  selected: "Selected folder",
  noFolder: "No folder selected",
  prepare: "Create new space and save recovery kit",
  join: "Join existing space with recovery kit",
  export: "Save another recovery kit",
  space: "Space ID",
  complete: "Local space ready. Save setup, then press Start sync.",
  pending: "Choose a folder, then create or join a space.",
  recovery:
    "Keep the recovery kit outside the synced folder. It contains the key needed to read your data. Cancelling its save leaves the new space unprepared.",
  handoff:
    "Bastet can hand encrypted files to this local folder. Delivery to another computer depends on the provider's client and has not been verified here.",
  saveFirst: "Save setup before starting sync.",
  error: "Folder setup failed. Check access and try again.",
  working: "Working…",
  localLimits:
    "Speed limits apply to Bastet's Google API transfers only. Your sync client controls network speed for this folder.",
  localMeasure: "Measure local sync objects",
  localObjects: "Local sync objects (not cloud quota)",
  handedOff:
    "📁 Encrypted files handed to the local sync folder; delivery by the sync client is unverified",
  written: "Written to local folder",
  read: "Read from local folder",
  unavailable:
    "The selected folder is unavailable. Check that the sync drive is mounted, then choose the folder again.",
  pendingDownload:
    "A cloud placeholder has not finished downloading. Wait for the sync client, then retry.",
  recoveryInside:
    "Save the recovery kit outside the synced folder and try again.",
  kitMismatch:
    "This kit does not match the selected provider or space. Choose the matching folder and original recovery kit.",
  occupied:
    "This folder already has sync objects. Join with its original recovery kit or choose an empty folder.",
  locked: "Pause sync before changing this folder setup.",
  storage:
    "Bastet cannot read or save this folder setup. Check folder and system credential access, then retry.",
};
type FolderMessages = { [K in keyof typeof en]: string };
export const folderMessages: Record<Locale, FolderMessages> = {
  en,
  "zh-Hant": {
    provider: "同步目標",
    google: "Google Drive",
    icloud: "iCloud Drive 資料夾",
    onedrive: "OneDrive 資料夾",
    title: "同步資料夾設定",
    readyBadge: "✅ 本機空間已備妥",
    pendingBadge: "⚪ 本機空間待設定",
    clientStatus: "🟡 同步程式送達狀態未驗證",
    intro:
      "選擇雲端服務同步磁碟內的資料夾。Bastet 將加密同步物件寫入此處，由服務的桌面程式在電腦間傳送。",
    pick: "選擇同步資料夾",
    selected: "已選資料夾",
    noFolder: "尚未選擇資料夾",
    prepare: "建立新空間並儲存恢復檔",
    join: "使用恢復檔加入現有空間",
    export: "再次儲存恢復檔",
    space: "空間 ID",
    complete: "本機空間已備妥。請先儲存設定，再按啟動同步。",
    pending: "選擇資料夾，然後建立或加入空間。",
    recovery:
      "請將恢復檔保存在同步資料夾之外。它含有讀取資料所需的金鑰。取消儲存時，新空間不會標為完成。",
    handoff:
      "Bastet 可以將加密檔案交給此本機資料夾；是否送達其他電腦取決於雲端服務程式，這裡尚未驗證。",
    saveFirst: "啟動同步前請先儲存設定。",
    error: "資料夾設定失敗，請檢查存取權限後重試。",
    working: "處理中…",
    localLimits:
      "速度上限只套用 Bastet 的 Google API 傳輸。此資料夾的網路速度由同步服務程式控制。",
    localMeasure: "統計本機同步物件",
    localObjects: "本機同步物件（不是雲端配額）",
    handedOff: "📁 加密檔案已交給本機同步資料夾；同步程式是否送達未驗證",
    written: "寫入本機資料夾",
    read: "從本機資料夾讀取",
    unavailable: "所選資料夾無法使用。請確認同步磁碟已掛載，再重新選擇資料夾。",
    pendingDownload: "雲端佔位檔尚未下載完成。請等待同步程式，再重試。",
    recoveryInside: "請將恢復檔儲存在同步資料夾之外，再重試。",
    kitMismatch: "恢復檔與所選服務或空間不符。請選擇對應的資料夾與原始恢復檔。",
    occupied: "此資料夾已有同步物件。請使用原始恢復檔加入，或選擇空白資料夾。",
    locked: "變更資料夾設定前，請先暫停同步。",
    storage:
      "Bastet 無法讀取或儲存資料夾設定。請檢查資料夾與系統憑證存取權限。",
  },
  "zh-Hans": {
    provider: "同步目标",
    google: "Google Drive",
    icloud: "iCloud Drive 文件夹",
    onedrive: "OneDrive 文件夹",
    title: "同步文件夹设置",
    readyBadge: "✅ 本地空间已就绪",
    pendingBadge: "⚪ 本地空间待设置",
    clientStatus: "🟡 同步客户端送达状态未验证",
    intro:
      "选择云服务同步磁盘内的文件夹。Bastet 将加密同步对象写入此处，由服务的桌面程序在电脑间传送。",
    pick: "选择同步文件夹",
    selected: "已选文件夹",
    noFolder: "尚未选择文件夹",
    prepare: "创建新空间并保存恢复文件",
    join: "使用恢复文件加入现有空间",
    export: "再次保存恢复文件",
    space: "空间 ID",
    complete: "本地空间已就绪。请先保存设置，再点击启动同步。",
    pending: "选择文件夹，然后创建或加入空间。",
    recovery:
      "请将恢复文件保存在同步文件夹之外。它含有读取数据所需的密钥。取消保存时，新空间不会标记为完成。",
    handoff:
      "Bastet 可以将加密文件交给此本地文件夹；是否送达其他电脑取决于云服务客户端，这里尚未验证。",
    saveFirst: "启动同步前请先保存设置。",
    error: "文件夹设置失败，请检查访问权限后重试。",
    working: "处理中…",
    localLimits:
      "速度上限仅适用于 Bastet 的 Google API 传输。此文件夹的网络速度由同步客户端控制。",
    localMeasure: "统计本地同步对象",
    localObjects: "本地同步对象（非云配额）",
    handedOff: "📁 加密文件已交给本地同步文件夹；同步客户端是否送达未验证",
    written: "写入本地文件夹",
    read: "从本地文件夹读取",
    unavailable: "所选文件夹不可用。请确认同步磁盘已挂载，再重新选择文件夹。",
    pendingDownload: "云端占位文件尚未下载完成。请等待同步客户端，然后重试。",
    recoveryInside: "请将恢复文件保存在同步文件夹之外，然后重试。",
    kitMismatch:
      "恢复文件与所选服务或空间不匹配。请选择对应的文件夹和原始恢复文件。",
    occupied: "此文件夹已有同步对象。请用原始恢复文件加入，或选择空文件夹。",
    locked: "更改文件夹设置前，请先暂停同步。",
    storage:
      "Bastet 无法读取或保存文件夹设置。请检查文件夹和系统凭据访问权限。",
  },
  ja: {
    provider: "同期先",
    google: "Google Drive",
    icloud: "iCloud Drive フォルダー",
    onedrive: "OneDrive フォルダー",
    title: "同期フォルダーの設定",
    readyBadge: "✅ ローカルスペース準備完了",
    pendingBadge: "⚪ ローカルスペース未設定",
    clientStatus: "🟡 同期アプリによる到着は未確認",
    intro:
      "クラウドサービスが同期するドライブ内のフォルダーを選択します。Bastet は暗号化した同期データを書き込み、サービスのデスクトップアプリが端末間で転送します。",
    pick: "同期フォルダーを選択",
    selected: "選択したフォルダー",
    noFolder: "フォルダー未選択",
    prepare: "新しいスペースを作成し復元キットを保存",
    join: "復元キットで既存のスペースに参加",
    export: "復元キットを再保存",
    space: "スペース ID",
    complete:
      "ローカルスペースの準備ができました。設定を保存してから同期を開始してください。",
    pending: "フォルダーを選び、スペースを作成するか参加してください。",
    recovery:
      "復元キットは同期フォルダーの外に保管してください。データを読むための鍵が含まれます。保存をキャンセルすると、新しいスペースは未完了のままです。",
    handoff:
      "Bastet は暗号化ファイルをこのローカルフォルダーに渡せます。他の端末への到着はクラウドサービスのアプリに依存し、ここでは確認していません。",
    saveFirst: "同期開始前に設定を保存してください。",
    error:
      "フォルダー設定に失敗しました。アクセス権を確認して再試行してください。",
    working: "処理中…",
    localLimits:
      "速度制限は Bastet の Google API 転送のみに適用されます。このフォルダーの通信速度は同期アプリが制御します。",
    localMeasure: "ローカル同期オブジェクトを集計",
    localObjects: "ローカル同期オブジェクト（クラウド容量ではありません）",
    handedOff:
      "📁 暗号化ファイルをローカル同期フォルダーに渡しました。同期アプリによる到着は未確認です",
    written: "ローカルフォルダーへ書き込み",
    read: "ローカルフォルダーから読み取り",
    unavailable:
      "選択したフォルダーを使用できません。同期ドライブを確認し、フォルダーを選び直してください。",
    pendingDownload:
      "クラウドのプレースホルダーはまだダウンロード中です。同期アプリを待って再試行してください。",
    recoveryInside:
      "復元キットを同期フォルダーの外に保存して再試行してください。",
    kitMismatch:
      "復元キットが選択したサービスまたはスペースと一致しません。対応するフォルダーと元のキットを選んでください。",
    occupied:
      "このフォルダーには同期データがあります。元の復元キットで参加するか、空のフォルダーを選んでください。",
    locked: "フォルダー設定を変更する前に同期を一時停止してください。",
    storage:
      "Bastet はフォルダー設定を読み書きできません。フォルダーとシステム認証情報へのアクセスを確認してください。",
  },
  ko: {
    provider: "동기화 대상",
    google: "Google Drive",
    icloud: "iCloud Drive 폴더",
    onedrive: "OneDrive 폴더",
    title: "동기화 폴더 설정",
    readyBadge: "✅ 로컬 공간 준비 완료",
    pendingBadge: "⚪ 로컬 공간 설정 대기",
    clientStatus: "🟡 동기화 앱 전송 결과 미확인",
    intro:
      "클라우드 서비스가 동기화하는 드라이브 안의 폴더를 선택하세요. Bastet은 암호화된 동기화 데이터를 저장하고 서비스의 데스크톱 앱이 컴퓨터 간에 전송합니다.",
    pick: "동기화 폴더 선택",
    selected: "선택한 폴더",
    noFolder: "선택한 폴더 없음",
    prepare: "새 공간을 만들고 복구 키트 저장",
    join: "복구 키트로 기존 공간 참여",
    export: "복구 키트 다시 저장",
    space: "공간 ID",
    complete:
      "로컬 공간이 준비되었습니다. 설정을 저장한 뒤 동기화를 시작하세요.",
    pending: "폴더를 선택한 뒤 공간을 만들거나 참여하세요.",
    recovery:
      "복구 키트는 동기화 폴더 밖에 보관하세요. 데이터를 읽는 데 필요한 키가 포함됩니다. 저장을 취소하면 새 공간은 준비되지 않은 상태로 남습니다.",
    handoff:
      "Bastet은 암호화된 파일을 이 로컬 폴더에 전달할 수 있습니다. 다른 컴퓨터에 도착했는지는 클라우드 서비스 앱에 달려 있으며 여기서는 확인하지 않았습니다.",
    saveFirst: "동기화를 시작하기 전에 설정을 저장하세요.",
    error: "폴더 설정에 실패했습니다. 접근 권한을 확인하고 다시 시도하세요.",
    working: "처리 중…",
    localLimits:
      "속도 제한은 Bastet의 Google API 전송에만 적용됩니다. 이 폴더의 네트워크 속도는 동기화 앱에서 제어합니다.",
    localMeasure: "로컬 동기화 객체 측정",
    localObjects: "로컬 동기화 객체 (클라우드 할당량 아님)",
    handedOff:
      "📁 암호화 파일을 로컬 동기화 폴더에 전달했습니다. 동기화 앱을 통한 도착은 확인하지 않았습니다",
    written: "로컬 폴더에 기록",
    read: "로컬 폴더에서 읽기",
    unavailable:
      "선택한 폴더를 사용할 수 없습니다. 동기화 드라이브를 확인한 뒤 폴더를 다시 선택하세요.",
    pendingDownload:
      "클라우드 자리표시자 파일 다운로드가 끝나지 않았습니다. 동기화 앱을 기다린 뒤 다시 시도하세요.",
    recoveryInside: "복구 키트를 동기화 폴더 밖에 저장한 뒤 다시 시도하세요.",
    kitMismatch:
      "복구 키트가 선택한 서비스나 공간과 일치하지 않습니다. 해당 폴더와 원래 키트를 선택하세요.",
    occupied:
      "이 폴더에는 이미 동기화 객체가 있습니다. 원래 복구 키트로 참여하거나 빈 폴더를 선택하세요.",
    locked: "폴더 설정을 변경하기 전에 동기화를 일시 중지하세요.",
    storage:
      "Bastet에서 폴더 설정을 읽거나 저장할 수 없습니다. 폴더와 시스템 자격 증명 접근을 확인하세요.",
  },
};
