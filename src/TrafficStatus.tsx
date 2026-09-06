import type { Locale } from "./i18n";
export type Traffic = {
  uploaded: number;
  downloaded: number;
  uploadRate: number;
  downloadRate: number;
};
const labels: Record<Locale, string[]> = {
  "zh-Hant": [
    "Drive 流量",
    "上傳",
    "下載",
    "累計",
    "啟動以來的 Drive HTTP 資料量；速率為最近約 3 秒平均，不含連線加密開銷及其他程式流量。",
    "等待流量資料",
  ],
  "zh-Hans": [
    "Drive 流量",
    "上传",
    "下载",
    "累计",
    "启动以来的 Drive HTTP 数据量；速率为最近约 3 秒平均，不含连接加密开销及其他程序流量。",
    "等待流量数据",
  ],
  en: [
    "Drive traffic",
    "Upload",
    "Download",
    "Total",
    "Drive HTTP payload since app launch; rates average approximately 3 seconds. Excludes transport overhead and other apps.",
    "Waiting for traffic data",
  ],
  ja: [
    "Drive 通信量",
    "送信",
    "受信",
    "累計",
    "起動後の Drive HTTP データ量。速度は直近約3秒の平均です。通信の付加データや他のアプリは含みません。",
    "通信データを待機中",
  ],
  ko: [
    "Drive 트래픽",
    "업로드",
    "다운로드",
    "누적",
    "앱 시작 이후 Drive HTTP 데이터양이며 속도는 최근 약 3초 평균입니다. 전송 오버헤드와 다른 앱은 제외됩니다.",
    "트래픽 데이터 대기 중",
  ],
};
export function formatBytes(value: number, locale: Locale): string {
  const bytes = Number.isFinite(value) ? Math.max(0, value) : 0;
  const index = Math.min(3, Math.floor(Math.log2(Math.max(1, bytes)) / 10));
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: index ? 1 : 0 }).format(bytes / 1024 ** index)} ${["B", "KiB", "MiB", "GiB"][index]}`;
}
export default function TrafficStatus({
  locale,
  traffic,
}: {
  locale: Locale;
  traffic?: Traffic;
}) {
  const t = labels[locale];
  return (
    <div className="traffic-strip" title={t[4]}>
      <span>{t[0]}</span>
      {traffic ? (
        <>
          <span>
            ↑ {t[1]} <b>{formatBytes(traffic.uploadRate, locale)}/s</b>{" "}
            <small>
              ({t[3]} {formatBytes(traffic.uploaded, locale)})
            </small>
          </span>
          <span>
            ↓ {t[2]} <b>{formatBytes(traffic.downloadRate, locale)}/s</b>{" "}
            <small>
              ({t[3]} {formatBytes(traffic.downloaded, locale)})
            </small>
          </span>
        </>
      ) : (
        <span>{t[5]} —</span>
      )}
      <details>
        <summary aria-label={t[4]}>ⓘ</summary>
        <p>{t[4]}</p>
      </details>
    </div>
  );
}
