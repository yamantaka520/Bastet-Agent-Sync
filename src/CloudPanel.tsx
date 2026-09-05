import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "./i18n";

export const cloudMessages = {
  en: {
    title: "Private cloud connection · M3 preview",
    hint: "Encrypted transport is in development. Connecting lists app-accessible folders; it does not sync agents.",
    unavailable:
      "Google sign-in is unavailable: this build has no OAuth client configured.",
    connect: "Connect Google Drive",
    disconnect: "Forget local login",
    check: "Check encryption and recovery",
    passed:
      "Encryption and recovery passed with synthetic data. No cloud transfer occurred.",
    failed:
      "The operation failed. Check your connection or credential store and try again.",
    folders: "Accessible folders",
    empty:
      "No app-accessible folders found. Existing shared folders may require a future Picker grant.",
    create: "Create folder",
    folderName: "New folder name",
    selected:
      "Folder inventory only; binding a sync space and saving its recovery key are not enabled yet.",
    forget:
      "Forgetting removes the local login. Revoke app access in your Google account if needed.",
  },
  "zh-Hant": {
    title: "私密雲端連線 · M3 預覽",
    hint: "加密傳輸開發中。登入只列出此應用程式可存取的資料夾，不會同步 Agent。",
    unavailable: "此版本尚未配置 OAuth Client，Google 登入暫不可用。",
    connect: "連接 Google Drive",
    disconnect: "移除本機登入",
    check: "檢查加密與金鑰恢復",
    passed: "合成資料的加密與金鑰恢復檢查通過，未進行雲端傳輸。",
    failed: "操作失敗，請檢查網路或系統憑證庫後重試。",
    folders: "可存取的資料夾",
    empty:
      "找不到此應用程式可存取的資料夾；既有共用資料夾可能需要後續的 Picker 授權。",
    create: "建立資料夾",
    folderName: "新資料夾名稱",
    selected: "目前只盤點資料夾，尚未開放綁定同步空間與保存恢復金鑰。",
    forget:
      "此操作只移除本機登入；如有需要，請在 Google 帳號撤銷應用程式存取權。",
  },
  "zh-Hans": {
    title: "私密云端连接 · M3 预览",
    hint: "加密传输开发中。登录仅列出此应用可访问的文件夹，不会同步 Agent。",
    unavailable: "此版本尚未配置 OAuth Client，Google 登录暂不可用。",
    connect: "连接 Google Drive",
    disconnect: "移除本机登录",
    check: "检查加密与密钥恢复",
    passed: "合成数据的加密与密钥恢复检查通过，未进行云端传输。",
    failed: "操作失败，请检查网络或系统凭据库后重试。",
    folders: "可访问的文件夹",
    empty:
      "未找到此应用可访问的文件夹；现有共享文件夹可能需要后续的 Picker 授权。",
    create: "创建文件夹",
    folderName: "新文件夹名称",
    selected: "目前仅列出文件夹，尚未开放绑定同步空间和保存恢复密钥。",
    forget:
      "此操作仅移除本机登录；如有需要，请在 Google 账号撤销应用访问权限。",
  },
  ja: {
    title: "プライベートなクラウド接続 · M3 プレビュー",
    hint: "暗号化転送は開発中です。接続ではアプリがアクセスできるフォルダーを表示し、Agent の同期は行いません。",
    unavailable: "このビルドには OAuth クライアントが設定されていません。",
    connect: "Google Drive に接続",
    disconnect: "ローカルログインを削除",
    check: "暗号化と鍵の復元を確認",
    passed:
      "サンプルデータの暗号化と鍵の復元を確認しました。クラウド転送は行っていません。",
    failed:
      "操作に失敗しました。ネットワークまたは資格情報ストアを確認してください。",
    folders: "アクセス可能なフォルダー",
    empty:
      "フォルダーがありません。既存の共有フォルダーには今後の Picker による許可が必要な場合があります。",
    create: "フォルダーを作成",
    folderName: "新しいフォルダー名",
    selected:
      "一覧表示のみです。同期スペースの関連付けと復元鍵の保存はまだ利用できません。",
    forget:
      "ローカルログインのみ削除します。必要に応じて Google アカウントでアクセスを取り消してください。",
  },
  ko: {
    title: "비공개 클라우드 연결 · M3 미리보기",
    hint: "암호화 전송을 개발 중입니다. 연결하면 앱이 접근할 수 있는 폴더만 표시하며 Agent는 동기화하지 않습니다.",
    unavailable: "이 빌드에는 OAuth 클라이언트가 설정되지 않았습니다.",
    connect: "Google Drive 연결",
    disconnect: "로컬 로그인 삭제",
    check: "암호화 및 키 복구 확인",
    passed:
      "샘플 데이터의 암호화와 키 복구를 확인했습니다. 클라우드 전송은 없었습니다.",
    failed: "작업에 실패했습니다. 네트워크 또는 자격 증명 저장소를 확인하세요.",
    folders: "접근 가능한 폴더",
    empty:
      "폴더가 없습니다. 기존 공유 폴더는 향후 Picker 권한 부여가 필요할 수 있습니다.",
    create: "폴더 만들기",
    folderName: "새 폴더 이름",
    selected:
      "폴더 목록만 표시합니다. 동기화 공간 연결과 복구 키 저장은 아직 지원하지 않습니다.",
    forget:
      "로컬 로그인만 삭제합니다. 필요하면 Google 계정에서 앱 접근 권한을 취소하세요.",
  },
} satisfies Record<Locale, Record<string, string>>;
type Folder = { id: string; name: string };
export default function CloudPanel({
  native,
  locale,
}: {
  native: boolean;
  locale: Locale;
}) {
  const t = cloudMessages[locale];
  const [status, setStatus] = useState({ configured: false, connected: false });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const [passed, setPassed] = useState(false);
  const [folders, setFolders] = useState<Folder[]>([]);
  const [name, setName] = useState("Bastet Agent Sync");
  useEffect(() => {
    if (native)
      invoke<typeof status>("cloud_status")
        .then((s) => {
          if (s) setStatus(s);
        })
        .catch(() => setError(true));
  }, [native]);
  async function run(work: () => Promise<void>) {
    setBusy(true);
    setError(false);
    setPassed(false);
    try {
      await work();
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="panel cloud-panel">
      <h2>{t.title}</h2>
      <p>{t.hint}</p>
      {!status.configured && <p>{t.unavailable}</p>}
      <div className="cloud-actions">
        <button
          disabled={!native || busy || !status.configured}
          onClick={() =>
            run(async () => {
              const list = await invoke<Folder[]>("connect_google");
              setFolders(list);
              setStatus((s) => ({ ...s, connected: true }));
            })
          }
        >
          {t.connect}
        </button>
        <button
          disabled={!native || busy}
          onClick={() =>
            run(async () => {
              const result = await invoke<{
                verified: boolean;
                recoveryVerified: boolean;
                tamperRejected: boolean;
              }>("run_crypto_diagnostic");
              if (
                !result?.verified ||
                !result.recoveryVerified ||
                !result.tamperRejected
              )
                throw new Error();
              setPassed(true);
            })
          }
        >
          {t.check}
        </button>
      </div>
      {status.connected && (
        <>
          <h3>{t.folders}</h3>
          <p>{t.selected}</p>
          {folders.length ? (
            <ul>
              {folders.map((f) => (
                <li key={f.id}>
                  {f.name} <small>({f.id})</small>
                </li>
              ))}
            </ul>
          ) : (
            <p>{t.empty}</p>
          )}
          <label>
            {t.folderName}
            <input
              value={name}
              maxLength={128}
              disabled={busy}
              onChange={(e) => setName(e.target.value)}
            />
          </label>
          <button
            disabled={busy || !name.trim()}
            onClick={() =>
              run(async () => {
                const folder = await invoke<Folder>("create_google_folder", {
                  name,
                });
                setFolders((f) => [...f, folder]);
              })
            }
          >
            {t.create}
          </button>
          <p>{t.forget}</p>
          <button
            disabled={busy}
            onClick={() =>
              run(async () => {
                await invoke("disconnect_google");
                setStatus((s) => ({ ...s, connected: false }));
                setFolders([]);
              })
            }
          >
            {t.disconnect}
          </button>
        </>
      )}
      {passed && <p role="status">{t.passed}</p>}
      {error && <p role="alert">{t.failed}</p>}
    </section>
  );
}
