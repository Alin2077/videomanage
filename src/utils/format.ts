/** 格式化文件大小 */
export function formatSize(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || bytes < 0) return "-";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 100 ? 0 : 1)} ${units[i]}`;
}

/** 格式化时长（秒 → HH:MM:SS / MM:SS） */
export function formatDuration(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || !isFinite(seconds) || seconds <= 0) return "-";
  const s = Math.round(seconds);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) {
    return `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
  }
  return `${m}:${String(sec).padStart(2, "0")}`;
}

/** 格式化观看时长（秒 → X 小时 Y 分） */
export function formatWatchTime(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || seconds <= 0) return "0 分钟";
  const totalMin = Math.round(seconds / 60);
  if (totalMin < 60) return `${totalMin} 分钟`;
  const h = Math.floor(totalMin / 60);
  const m = totalMin % 60;
  return m > 0 ? `${h} 小时 ${m} 分钟` : `${h} 小时`;
}

/** 格式化日期时间 */
export function formatDateTime(s: string | null | undefined): string {
  if (!s) return "-";
  return s.replace("T", " ").slice(0, 19);
}

/** 格式化日期 */
export function formatDate(s: string | null | undefined): string {
  if (!s) return "-";
  return s.slice(0, 10);
}

/** 分辨率 */
export function formatResolution(width: number | null, height: number | null): string {
  if (!width || !height) return "-";
  return `${width}×${height}`;
}
