import type { Locale } from "./i18n";
export const wizardMessages = {
  en: {
    cancelLogin: "Cancel browser authorization wait",
    cancelHint:
      "Cancellation is available while waiting for the browser. Token exchange and account checks must finish before retrying.",
    cancelUnavailable:
      "Browser authorization is not currently waiting. Retry cancellation once the browser opens, or wait for the connection check to finish.",
    cancelled:
      "Authorization wait cancelled. Your saved setup is unchanged; connect again when ready.",
    forgetLogin: "Forget local login",
    forgetHint:
      "This removes the saved login token only. Setup progress and space keys stay on this computer. Google permissions can be revoked in your account.",
    accountCurrent: "Connected Google account",
    accountSaved: "Saved Google account (not connected)",
    accountMismatch:
      "This setup belongs to another Google account. Remove the local login and reconnect with the saved account, or restart setup to use a different account.",
    connectionPending:
      "Google connection has not been checked in this session. Saved steps are preserved; reconnect or run the final check when ready.",
    title: "Google Drive setup",
    intro: "Set up your shared space. Completed steps are saved automatically.",
    guided: "Step-by-step wizard",
    manual: "Manual setup",
    saved: "Progress saved on this computer",
    resume: "Continue where you left off",
    restart: "Start again",
    restartHint:
      "Your previous setup record is kept. Restarting does not delete Drive files, keys or Google permissions.",
    confirmRestart: "Restart setup",
    cancel: "Cancel",
    loading: "Loading saved progress…",
    working: "Working…",
    clientHint:
      "Use the app’s built-in login configuration, or import your own Desktop OAuth client file. Changing this configuration resets later steps.",
    noClient:
      "No built-in OAuth client is available. You can import your own configuration or return later.",
    useBuild: "Use built-in configuration",
    importClient: "Import Desktop OAuth JSON",
    clientLabel: "OAuth client",
    help: "Google setup guide",
    helpSteps: [
      "For a custom client, create a Google Cloud project and enable the Google Drive API.",
      "Configure the Google Auth consent screen and test users as appropriate for your account.",
      "Create an OAuth client of type Desktop app and download its JSON file.",
      "Import that file here. Client credentials stay in the system credential store.",
    ],
    connect: "Authorize Google Drive",
    authHint:
      "Open Google in your browser and grant the requested app-file access. This does not upload agent data.",
    reconnect: "Reconnect Google",
    folderHint:
      "Choose an accessible folder, create one, or enter a folder ID/link. A link alone does not grant access.",
    folderName: "New folder name",
    createFolder: "Create and use folder",
    folderId: "Folder ID or Google Drive link",
    selectFolder: "Verify and use folder",
    refreshFolders: "Refresh folder list",
    empty:
      "No accessible folders listed. Create a folder or verify an existing folder ID.",
    keyHint:
      "Create a new encrypted space, or join an existing space using a recovery kit from your other computer.",
    prepareKey: "Create space key",
    exportRecovery: "Save recovery kit",
    publishProof: "Verify encrypted space",
    importRecovery: "Import recovery kit",
    backupHint:
      "Keep the recovery kit outside the shared Drive folder. It contains the key needed to decrypt your data.",
    keyPrepared: "Space key",
    backupSaved: "Recovery kit backup",
    proofDone: "Encrypted space verification",
    reviewHint:
      "Review the selected folder and space. The final check revalidates access and the saved key.",
    finish: "Verify and finish setup",
    complete: "Setup completed",
    completeHint:
      "This wizard configures Drive only. Agent synchronization remains unavailable until adapters are ready.",
    back: "Back",
    next: "Next",
    done: "Completed",
    pending: "Not completed",
    error:
      "The operation failed. Your saved progress is preserved. Retry when the problem is resolved.",
    authError: "Reconnect to Google and retry.",
    configError:
      "Import a valid Google Desktop OAuth JSON file, or use the configured built-in client.",
    folderError:
      "The folder could not be verified. Check its ID and the app’s access permission.",
    keyError:
      "The recovery file or key could not be verified. Use the original kit for this folder.",
    restartError:
      "This space is already bound. Start again to change its folder or key.",
    storageError:
      "The local file or credential store is unavailable. Check access and retry.",
    stepError: "Complete the required earlier settings first.",
    diagnostic: "Check encryption with sample data",
    diagnosticPassed:
      "Sample encryption and recovery passed. No cloud transfer occurred.",
    diagnosticFailed: "The sample check failed. No agent data was accessed.",
    steps: [
      "Login configuration",
      "Google authorization",
      "Sync folder",
      "Encryption & recovery",
      "Review & finish",
    ],
  },
  "zh-Hant": {
    cancelLogin: "取消等待瀏覽器授權",
    cancelHint:
      "等待瀏覽器授權時可以取消。Token 交換與帳號檢查須完成後才能重試。",
    cancelUnavailable:
      "目前未在等待瀏覽器授權。瀏覽器開啟後可再取消，或等待連線檢查完成。",
    cancelled: "已取消等待授權。已儲存的設定保留，可再次連線。",
    forgetLogin: "移除本機登入",
    forgetHint:
      "只移除已保存的登入 token，保留精靈進度與空間金鑰。Google 權限可在帳號中撤銷。",
    accountCurrent: "目前連線的 Google 帳號",
    accountSaved: "已儲存的 Google 帳號（未連線）",
    accountMismatch:
      "此設定屬於另一個 Google 帳號。請移除本機登入後使用原帳號重新連線；若要更換帳號，請重新開始設定。",
    connectionPending:
      "本次開啟尚未檢查 Google 連線。已完成的步驟仍保留，可重新連接或執行最後檢查。",
    title: "Google Drive 設定",
    intro: "建立你的共用空間。每個完成的步驟都會自動保存。",
    guided: "逐步引導精靈",
    manual: "完全手動設定",
    saved: "進度已保存在這台電腦",
    resume: "接續上次進度",
    restart: "重新開始",
    restartHint:
      "會保留上一份設定紀錄；重新開始不會刪除 Drive 檔案、金鑰或 Google 權限。",
    confirmRestart: "重新開始設定",
    cancel: "取消",
    loading: "正在載入已保存的進度…",
    working: "處理中…",
    clientHint:
      "使用程式內建的登入設定，或匯入自己的桌面 OAuth 設定檔。更換登入設定會重設後續步驟。",
    noClient: "此版本尚無內建 OAuth Client。可匯入自己的設定，或稍後回來接續。",
    useBuild: "使用內建登入設定",
    importClient: "匯入桌面 OAuth JSON",
    clientLabel: "OAuth Client",
    help: "Google 設定說明",
    helpSteps: [
      "若使用自己的 Client，先建立 Google Cloud 專案並啟用 Google Drive API。",
      "依帳號需求設定 Google Auth 同意畫面與測試使用者。",
      "建立「桌面應用程式」類型的 OAuth Client，下載 JSON 檔。",
      "在這裡匯入檔案。Client 憑證保存在系統憑證庫。",
    ],
    connect: "授權 Google Drive",
    authHint:
      "在瀏覽器登入 Google，授予此應用程式所需的檔案存取權。本步驟不會上傳 Agent 資料。",
    reconnect: "重新連接 Google",
    folderHint:
      "選取可存取的資料夾、建立新資料夾，或輸入資料夾 ID／連結。連結本身不會授予存取權。",
    folderName: "新資料夾名稱",
    createFolder: "建立並使用資料夾",
    folderId: "資料夾 ID 或 Google Drive 連結",
    selectFolder: "驗證並使用資料夾",
    refreshFolders: "重新載入資料夾",
    empty: "尚無可列出的資料夾。可建立新資料夾，或輸入既有資料夾 ID 驗證。",
    keyHint: "建立新的加密空間，或匯入另一台電腦的恢復檔以加入既有空間。",
    prepareKey: "建立空間金鑰",
    exportRecovery: "保存恢復檔",
    publishProof: "驗證加密空間",
    importRecovery: "匯入恢復檔",
    backupHint:
      "請把恢復檔保存在共用 Drive 資料夾之外。檔案包含解密資料所需的金鑰。",
    keyPrepared: "空間金鑰",
    backupSaved: "恢復檔備份",
    proofDone: "加密空間驗證",
    reviewHint:
      "確認選定的資料夾與空間。最後檢查會重新驗證存取權與已保存的金鑰。",
    finish: "驗證並完成設定",
    complete: "設定已完成",
    completeHint:
      "精靈僅負責 Drive 設定；Agent 同步仍須等待適配器完成後才能啟用。",
    back: "上一步",
    next: "下一步",
    done: "已完成",
    pending: "尚未完成",
    error: "操作失敗，已保存的進度仍保留。排除問題後可重試。",
    authError: "請重新連接 Google 後重試。",
    configError:
      "請匯入有效的 Google 桌面 OAuth JSON，或使用已配置的內建 Client。",
    folderError: "無法驗證資料夾，請確認 ID 與此應用程式的存取權。",
    keyError: "無法驗證恢復檔或金鑰，請使用此資料夾原本的恢復檔。",
    restartError: "此空間已綁定，若要更換資料夾或金鑰，請重新開始。",
    storageError: "無法存取本機檔案或憑證庫，請檢查權限後重試。",
    stepError: "請先完成必要的前置設定。",
    diagnostic: "以範例資料檢查加密",
    diagnosticPassed: "範例資料的加密與恢復檢查通過，未進行雲端傳輸。",
    diagnosticFailed: "範例檢查失敗，未存取 Agent 資料。",
    steps: [
      "登入設定",
      "Google 授權",
      "同步資料夾",
      "加密與恢復",
      "檢查與完成",
    ],
  },
  "zh-Hans": {
    cancelLogin: "取消等待浏览器授权",
    cancelHint:
      "等待浏览器授权时可以取消。Token 交换与账号检查须完成后才能重试。",
    cancelUnavailable:
      "当前未在等待浏览器授权。浏览器打开后可再取消，或等待连接检查完成。",
    cancelled: "已取消等待授权。已保存的设置保留，可再次连接。",
    forgetLogin: "移除本机登录",
    forgetHint:
      "仅移除已保存的登录 token，保留向导进度和空间密钥。Google 权限可在账号中撤销。",
    accountCurrent: "当前连接的 Google 账号",
    accountSaved: "已保存的 Google 账号（未连接）",
    accountMismatch:
      "此设置属于另一个 Google 账号。请移除本机登录后使用原账号重新连接；若要更换账号，请重新开始设置。",
    connectionPending:
      "本次打开尚未检查 Google 连接。已完成的步骤仍保留，可以重新连接或执行最后检查。",
    title: "Google Drive 设置",
    intro: "建立你的共享空间。每个完成的步骤都会自动保存。",
    guided: "逐步引导向导",
    manual: "完全手动设置",
    saved: "进度已保存在这台电脑",
    resume: "接续上次进度",
    restart: "重新开始",
    restartHint:
      "保留上一份设置记录；重新开始不会删除 Drive 文件、密钥或 Google 权限。",
    confirmRestart: "重新开始设置",
    cancel: "取消",
    loading: "正在加载已保存的进度…",
    working: "处理中…",
    clientHint:
      "使用程序内置的登录设置，或导入自己的桌面 OAuth 配置文件。更换登录设置会重置后续步骤。",
    noClient:
      "此版本尚无内置 OAuth Client。可以导入自己的设置，或稍后回来继续。",
    useBuild: "使用内置登录设置",
    importClient: "导入桌面 OAuth JSON",
    clientLabel: "OAuth Client",
    help: "Google 设置说明",
    helpSteps: [
      "如使用自己的 Client，先创建 Google Cloud 项目并启用 Google Drive API。",
      "按账号需要配置 Google Auth 同意页面和测试用户。",
      "创建“桌面应用”类型的 OAuth Client，下载 JSON 文件。",
      "在这里导入文件。Client 凭据保存在系统凭据库。",
    ],
    connect: "授权 Google Drive",
    authHint:
      "在浏览器登录 Google，授予此应用所需的文件访问权限。本步骤不会上传 Agent 数据。",
    reconnect: "重新连接 Google",
    folderHint:
      "选择可访问的文件夹、创建新文件夹，或输入文件夹 ID／链接。链接本身不会授予访问权限。",
    folderName: "新文件夹名称",
    createFolder: "创建并使用文件夹",
    folderId: "文件夹 ID 或 Google Drive 链接",
    selectFolder: "验证并使用文件夹",
    refreshFolders: "重新加载文件夹",
    empty: "尚无可列出的文件夹。可以创建新文件夹，或输入现有文件夹 ID 验证。",
    keyHint: "创建新的加密空间，或导入另一台电脑的恢复文件以加入现有空间。",
    prepareKey: "创建空间密钥",
    exportRecovery: "保存恢复文件",
    publishProof: "验证加密空间",
    importRecovery: "导入恢复文件",
    backupHint:
      "请把恢复文件保存在共享 Drive 文件夹之外。文件包含解密数据所需的密钥。",
    keyPrepared: "空间密钥",
    backupSaved: "恢复文件备份",
    proofDone: "加密空间验证",
    reviewHint:
      "确认选择的文件夹和空间。最后检查会重新验证访问权限和已保存的密钥。",
    finish: "验证并完成设置",
    complete: "设置已完成",
    completeHint:
      "向导仅负责 Drive 设置；Agent 同步仍须等待适配器完成后才能启用。",
    back: "上一步",
    next: "下一步",
    done: "已完成",
    pending: "尚未完成",
    error: "操作失败，已保存的进度仍保留。排除问题后可以重试。",
    authError: "请重新连接 Google 后重试。",
    configError:
      "请导入有效的 Google 桌面 OAuth JSON，或使用已配置的内置 Client。",
    folderError: "无法验证文件夹，请确认 ID 和此应用的访问权限。",
    keyError: "无法验证恢复文件或密钥，请使用此文件夹原本的恢复文件。",
    restartError: "此空间已绑定，如需更换文件夹或密钥，请重新开始。",
    storageError: "无法访问本机文件或凭据库，请检查权限后重试。",
    stepError: "请先完成必要的前置设置。",
    diagnostic: "用示例数据检查加密",
    diagnosticPassed: "示例数据的加密与恢复检查通过，未进行云端传输。",
    diagnosticFailed: "示例检查失败，未访问 Agent 数据。",
    steps: [
      "登录设置",
      "Google 授权",
      "同步文件夹",
      "加密与恢复",
      "检查与完成",
    ],
  },
  ja: {
    cancelLogin: "ブラウザー認証待ちをキャンセル",
    cancelHint:
      "ブラウザー待機中にキャンセルできます。トークン交換とアカウント確認は完了するまでお待ちください。",
    cancelUnavailable:
      "現在ブラウザー認証を待機していません。ブラウザーが開いた後に再試行するか、接続確認の完了をお待ちください。",
    cancelled:
      "認証待ちをキャンセルしました。保存済み設定は保持され、再接続できます。",
    forgetLogin: "ローカルログインを削除",
    forgetHint:
      "保存されたログイントークンのみ削除します。進行状況とスペース鍵は保持します。Google 権限はアカウントで取り消せます。",
    accountCurrent: "接続中の Google アカウント",
    accountSaved: "保存済み Google アカウント（未接続）",
    accountMismatch:
      "別の Google アカウントの設定です。ローカルのログインを削除し元のアカウントで再接続するか、設定を最初からやり直してください。",
    connectionPending:
      "この起動では Google 接続をまだ確認していません。完了した手順は保持されており、再接続または最終確認ができます。",
    title: "Google Drive の設定",
    intro: "共有スペースを設定します。完了した手順は自動保存されます。",
    guided: "設定ウィザード",
    manual: "手動設定",
    saved: "このコンピューターに進行状況を保存しました",
    resume: "前回の続きから再開",
    restart: "最初からやり直す",
    restartHint:
      "以前の設定記録を保存します。Drive のファイル、鍵、Google の権限は削除しません。",
    confirmRestart: "設定をやり直す",
    cancel: "キャンセル",
    loading: "保存された進行状況を読み込み中…",
    working: "処理中…",
    clientHint:
      "組み込みのログイン設定を使用するか、自分のデスクトップ OAuth 設定ファイルをインポートします。変更すると以降の手順はリセットされます。",
    noClient:
      "組み込み OAuth クライアントは未設定です。自分の設定をインポートするか、後で再開できます。",
    useBuild: "組み込み設定を使用",
    importClient: "デスクトップ OAuth JSON をインポート",
    clientLabel: "OAuth クライアント",
    help: "Google 設定ガイド",
    helpSteps: [
      "独自のクライアントを使う場合は Google Cloud プロジェクトを作成し、Google Drive API を有効にします。",
      "アカウントに合わせて Google Auth の同意画面とテストユーザーを設定します。",
      "種類が「デスクトップアプリ」の OAuth クライアントを作成し、JSON をダウンロードします。",
      "ここでファイルをインポートします。資格情報はシステムの資格情報ストアに保存されます。",
    ],
    connect: "Google Drive を認証",
    authHint:
      "ブラウザーで Google にログインし、アプリに必要なファイルへのアクセスを許可します。Agent データはアップロードしません。",
    reconnect: "Google に再接続",
    folderHint:
      "アクセス可能なフォルダーを選択・作成するか、ID またはリンクを入力します。リンクだけでは権限は付与されません。",
    folderName: "新しいフォルダー名",
    createFolder: "フォルダーを作成して使用",
    folderId: "フォルダー ID または Google Drive リンク",
    selectFolder: "検証してフォルダーを使用",
    refreshFolders: "フォルダーを再読み込み",
    empty:
      "表示できるフォルダーがありません。作成するか、既存の ID を入力して検証してください。",
    keyHint:
      "新しい暗号化スペースを作成するか、別のコンピューターの復元ファイルを使って既存のスペースに参加します。",
    prepareKey: "スペース鍵を作成",
    exportRecovery: "復元ファイルを保存",
    publishProof: "暗号化スペースを検証",
    importRecovery: "復元ファイルをインポート",
    backupHint:
      "復元ファイルは共有 Drive フォルダーの外に保管してください。データの復号に必要な鍵を含みます。",
    keyPrepared: "スペース鍵",
    backupSaved: "復元ファイルのバックアップ",
    proofDone: "暗号化スペースの検証",
    reviewHint:
      "フォルダーとスペースを確認します。最後にアクセス権と保存された鍵を再検証します。",
    finish: "検証して設定を完了",
    complete: "設定が完了しました",
    completeHint:
      "このウィザードは Drive を設定します。Agent 同期はアダプターの完成後に利用可能になります。",
    back: "戻る",
    next: "次へ",
    done: "完了",
    pending: "未完了",
    error:
      "操作に失敗しました。保存済みの進行状況は保持されています。問題を解決して再試行してください。",
    authError: "Google に再接続して再試行してください。",
    configError:
      "有効な Google デスクトップ OAuth JSON をインポートするか、組み込み設定を使用してください。",
    folderError:
      "フォルダーを検証できません。ID とアプリのアクセス権を確認してください。",
    keyError:
      "復元ファイルまたは鍵を検証できません。このフォルダーの元の復元ファイルを使用してください。",
    restartError:
      "このスペースは関連付け済みです。フォルダーや鍵を変更するには最初からやり直してください。",
    storageError:
      "ローカルファイルまたは資格情報ストアにアクセスできません。権限を確認してください。",
    stepError: "必要な前の設定を完了してください。",
    diagnostic: "サンプルデータで暗号化を確認",
    diagnosticPassed:
      "サンプルの暗号化と復元を確認しました。クラウド転送は行っていません。",
    diagnosticFailed:
      "サンプルの確認に失敗しました。Agent データにはアクセスしていません。",
    steps: [
      "ログイン設定",
      "Google 認証",
      "同期フォルダー",
      "暗号化と復元",
      "確認と完了",
    ],
  },
  ko: {
    cancelLogin: "브라우저 인증 대기 취소",
    cancelHint:
      "브라우저 대기 중에 취소할 수 있습니다. 토큰 교환과 계정 확인이 끝나면 다시 시도하세요.",
    cancelUnavailable:
      "현재 브라우저 인증 대기 중이 아닙니다. 브라우저가 열리면 취소를 다시 시도하거나 연결 확인 완료를 기다리세요.",
    cancelled:
      "인증 대기를 취소했습니다. 저장된 설정이 유지되며 다시 연결할 수 있습니다.",
    forgetLogin: "로컬 로그인 삭제",
    forgetHint:
      "저장된 로그인 토큰만 삭제합니다. 진행 상황과 공간 키는 유지됩니다. Google 권한은 계정에서 취소할 수 있습니다.",
    accountCurrent: "연결된 Google 계정",
    accountSaved: "저장된 Google 계정 (연결 안 됨)",
    accountMismatch:
      "다른 Google 계정의 설정입니다. 로컬 로그인을 제거하고 원래 계정으로 다시 연결하거나 설정을 처음부터 시작하세요.",
    connectionPending:
      "이번 실행에서는 Google 연결을 아직 확인하지 않았습니다. 완료한 단계는 유지되며 다시 연결하거나 마지막 검사를 실행할 수 있습니다.",
    title: "Google Drive 설정",
    intro: "공유 공간을 설정합니다. 완료한 단계는 자동으로 저장됩니다.",
    guided: "단계별 설정 마법사",
    manual: "수동 설정",
    saved: "이 컴퓨터에 진행 상황을 저장했습니다",
    resume: "이전 진행 상황에서 계속",
    restart: "처음부터 다시 시작",
    restartHint:
      "이전 설정 기록을 보존합니다. Drive 파일, 키 또는 Google 권한은 삭제하지 않습니다.",
    confirmRestart: "설정 다시 시작",
    cancel: "취소",
    loading: "저장된 진행 상황을 불러오는 중…",
    working: "처리 중…",
    clientHint:
      "앱에 포함된 로그인 설정을 사용하거나 자신의 데스크톱 OAuth 설정 파일을 가져오세요. 변경하면 이후 단계가 초기화됩니다.",
    noClient:
      "기본 OAuth 클라이언트가 없습니다. 자신의 설정을 가져오거나 나중에 계속할 수 있습니다.",
    useBuild: "기본 로그인 설정 사용",
    importClient: "데스크톱 OAuth JSON 가져오기",
    clientLabel: "OAuth 클라이언트",
    help: "Google 설정 안내",
    helpSteps: [
      "자체 클라이언트를 사용하려면 Google Cloud 프로젝트를 만들고 Google Drive API를 활성화하세요.",
      "계정에 맞게 Google Auth 동의 화면과 테스트 사용자를 설정하세요.",
      "유형이 데스크톱 앱인 OAuth 클라이언트를 만들고 JSON 파일을 다운로드하세요.",
      "여기서 파일을 가져오세요. 자격 증명은 시스템 자격 증명 저장소에 보관됩니다.",
    ],
    connect: "Google Drive 권한 부여",
    authHint:
      "브라우저에서 Google에 로그인하고 필요한 앱 파일 접근 권한을 부여하세요. Agent 데이터는 업로드하지 않습니다.",
    reconnect: "Google 다시 연결",
    folderHint:
      "접근 가능한 폴더를 선택하거나 새로 만들고, 또는 ID나 링크를 입력하세요. 링크만으로 권한이 부여되지는 않습니다.",
    folderName: "새 폴더 이름",
    createFolder: "폴더 생성 및 사용",
    folderId: "폴더 ID 또는 Google Drive 링크",
    selectFolder: "폴더 검증 및 사용",
    refreshFolders: "폴더 목록 새로 고침",
    empty:
      "표시할 폴더가 없습니다. 폴더를 만들거나 기존 ID를 입력해 검증하세요.",
    keyHint:
      "새 암호화 공간을 만들거나 다른 컴퓨터의 복구 파일로 기존 공간에 참여하세요.",
    prepareKey: "공간 키 생성",
    exportRecovery: "복구 파일 저장",
    publishProof: "암호화 공간 검증",
    importRecovery: "복구 파일 가져오기",
    backupHint:
      "복구 파일은 공유 Drive 폴더 외부에 보관하세요. 데이터를 복호화하는 데 필요한 키가 포함됩니다.",
    keyPrepared: "공간 키",
    backupSaved: "복구 파일 백업",
    proofDone: "암호화 공간 검증",
    reviewHint:
      "선택한 폴더와 공간을 확인하세요. 마지막 검사에서 접근 권한과 저장된 키를 다시 검증합니다.",
    finish: "검증 후 설정 완료",
    complete: "설정 완료",
    completeHint:
      "이 마법사는 Drive만 설정합니다. Agent 동기화는 어댑터가 준비된 후 사용할 수 있습니다.",
    back: "이전",
    next: "다음",
    done: "완료",
    pending: "미완료",
    error:
      "작업에 실패했습니다. 저장된 진행 상황은 유지됩니다. 문제를 해결한 후 다시 시도하세요.",
    authError: "Google에 다시 연결한 후 시도하세요.",
    configError:
      "유효한 Google 데스크톱 OAuth JSON을 가져오거나 기본 설정을 사용하세요.",
    folderError: "폴더를 검증할 수 없습니다. ID와 앱 접근 권한을 확인하세요.",
    keyError:
      "복구 파일이나 키를 검증할 수 없습니다. 이 폴더의 원래 복구 파일을 사용하세요.",
    restartError:
      "이미 연결된 공간입니다. 폴더나 키를 변경하려면 처음부터 다시 시작하세요.",
    storageError:
      "로컬 파일 또는 자격 증명 저장소에 접근할 수 없습니다. 권한을 확인하세요.",
    stepError: "필요한 이전 설정을 먼저 완료하세요.",
    diagnostic: "샘플 데이터로 암호화 확인",
    diagnosticPassed:
      "샘플 암호화와 복구 확인을 마쳤습니다. 클라우드 전송은 없었습니다.",
    diagnosticFailed:
      "샘플 검사에 실패했습니다. Agent 데이터에는 접근하지 않았습니다.",
    steps: [
      "로그인 설정",
      "Google 권한",
      "동기화 폴더",
      "암호화 및 복구",
      "검토 및 완료",
    ],
  },
} satisfies Record<Locale, Record<string, string | string[]>>;
