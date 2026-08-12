import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import type {
  DashboardStats,
  FolderNode,
  HourCell,
  LeaderboardItem,
  LogFilter,
  OpenLogWithVideo,
  PageResult,
  ScanProgress,
  ScanResult,
  Tag,
  TagGroupInput,
  TagGroupWithTags,
  TagInput,
  TagStat,
  TrendPoint,
  VideoDetail,
  VideoInfo,
  VideoMetaUpdate,
  VideoQuery,
  Workspace,
} from "../types";

/** 将本地路径转为可被 <img>/<video> 使用的 URL */
export function toAssetUrl(path: string | null | undefined): string {
  if (!path) return "";
  return convertFileSrc(path);
}

// ---------- 扫描与工作区 ----------
export const scanApi = {
  scanRootFolder: (path: string, name: string) =>
    invoke<ScanResult>("scan_root_folder", { path, name }),
  getScanProgress: () => invoke<ScanProgress>("get_scan_progress"),
  cancelScan: () => invoke<void>("cancel_scan"),
  getFolderChildren: (parentId: number | null) =>
    invoke<FolderNode[]>("get_folder_children", { parentId }),
  getRootFolders: (workspaceId: number) =>
    invoke<FolderNode[]>("get_root_folders", { workspaceId }),
};

export const workspaceApi = {
  list: () => invoke<Workspace[]>("list_workspaces"),
  remove: (workspaceId: number) => invoke<void>("delete_workspace", { workspaceId }),
};

// ---------- 视频 ----------
export const videoApi = {
  list: (query: VideoQuery) => invoke<PageResult<VideoInfo>>("list_videos", { query }),
  detail: (videoId: number) => invoke<VideoDetail>("get_video_detail", { videoId }),
  updateMeta: (videoId: number, meta: VideoMetaUpdate) =>
    invoke<void>("update_video_meta", { videoId, meta }),
  batchDelete: (videoIds: number[]) => invoke<number>("batch_delete_videos", { videoIds }),
  search: (keyword: string, limit = 50) =>
    invoke<VideoInfo[]>("search_videos", { keyword, limit }),
  openWithPlayer: (videoId: number, playerPath?: string | null) =>
    invoke<number>("open_with_player", { videoId, playerPath }),
  logOpen: (videoId: number) => invoke<number>("log_video_open", { videoId }),
  logClose: (logId: number) => invoke<void>("log_video_close", { logId }),
};

// ---------- 标签 ----------
export const tagApi = {
  getTree: () => invoke<TagGroupWithTags[]>("get_tag_tree"),
  upsert: (tag: TagInput) => invoke<Tag>("upsert_tag", { tag }),
  remove: (tagId: number) => invoke<void>("delete_tag", { tagId }),
  upsertGroup: (group: TagGroupInput) => invoke<void>("upsert_tag_group", { group }),
  removeGroup: (groupId: number) => invoke<void>("delete_tag_group", { groupId }),
  setVideoTags: (videoId: number, tagIds: number[]) =>
    invoke<void>("set_video_tags", { videoId, tagIds }),
  batchAdd: (videoIds: number[], tagIds: number[]) =>
    invoke<void>("batch_add_tags", { videoIds, tagIds }),
  batchRemove: (videoIds: number[], tagIds: number[]) =>
    invoke<void>("batch_remove_tags", { videoIds, tagIds }),
};

// ---------- 日志 ----------
export const logApi = {
  list: (filter: LogFilter, workspaceId: number | null, page: number, pageSize: number) =>
    invoke<PageResult<OpenLogWithVideo>>("list_logs", { filter, workspaceId, page, pageSize }),
  export: (filter: LogFilter, workspaceId: number | null, outputPath: string) =>
    invoke<void>("export_logs", { filter, workspaceId, outputPath }),
};

// ---------- 统计 ----------
export const statsApi = {
  dashboard: (workspaceId: number | null) =>
    invoke<DashboardStats>("get_dashboard_stats", { workspaceId }),
  trend: (workspaceId: number | null, range: "day" | "week" | "month") =>
    invoke<TrendPoint[]>("get_view_trend", { workspaceId, range }),
  leaderboard: (workspaceId: number | null, category: string, limit: number) =>
    invoke<LeaderboardItem[]>("get_leaderboard", { workspaceId, category, limit }),
  tagStats: (workspaceId: number | null) =>
    invoke<TagStat[]>("get_tag_stats", { workspaceId }),
  hourlyHeatmap: (workspaceId: number | null) =>
    invoke<HourCell[]>("get_hourly_heatmap", { workspaceId }),
};

// ---------- 设置 ----------
export const settingsApi = {
  getAll: () => invoke<Record<string, string>>("get_settings"),
  set: (key: string, value: string) => invoke<void>("set_setting", { key, value }),
  detectMediaTools: () =>
    invoke<{ ffprobe: string | null; ffmpeg: string | null }>("detect_media_tools"),
  exportBackup: (outputPath: string) => invoke<void>("export_backup", { outputPath }),
  importBackup: (inputPath: string) => invoke<void>("import_backup", { inputPath }),
};
