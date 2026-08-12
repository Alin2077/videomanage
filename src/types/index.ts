// ---------- 工作区 ----------
export interface Workspace {
  id: number;
  name: string;
  path: string;
  createdAt: string;
  videoCount: number;
  folderCount: number;
}

// ---------- 文件夹 ----------
export interface FolderNode {
  id: number;
  parentId: number | null;
  name: string;
  path: string;
  videoCount: number;
  hasChildren: boolean;
}

// ---------- 标签 ----------
export interface Tag {
  id: number;
  groupId: number | null;
  name: string;
  color: string;
}

export interface TagGroupWithTags {
  id: number;
  name: string;
  sortOrder: number;
  tags: Tag[];
}

export interface TagInput {
  id?: number;
  groupId?: number | null;
  name: string;
  color?: string;
}

export interface TagGroupInput {
  id?: number;
  name: string;
  sortOrder?: number;
}

// ---------- 视频 ----------
export interface VideoInfo {
  id: number;
  folderId: number;
  fileName: string;
  filePath: string;
  fileSize: number;
  duration: number | null;
  width: number | null;
  height: number | null;
  codec: string | null;
  fps: number | null;
  sampleRate: number | null;
  coverPath: string | null;
  customTitle: string | null;
  notes: string | null;
  openCount: number;
  fileHash: string | null;
  createdAt: string;
  modifiedAt: string;
  scannedAt: string;
  tags: Tag[];
}

export interface VideoDetail {
  id: number;
  folderId: number;
  fileName: string;
  filePath: string;
  fileSize: number;
  duration: number | null;
  width: number | null;
  height: number | null;
  codec: string | null;
  fps: number | null;
  sampleRate: number | null;
  coverPath: string | null;
  customTitle: string | null;
  notes: string | null;
  openCount: number;
  fileHash: string | null;
  createdAt: string;
  modifiedAt: string;
  scannedAt: string;
  tags: Tag[];
  folderPath: string;
  openLogs: OpenLog[];
}

export interface VideoQuery {
  workspaceId?: number | null;
  folderId?: number | null;
  keyword?: string | null;
  tagIds?: number[] | null;
  page: number;
  pageSize: number;
  sortBy?: "name" | "size" | "duration" | "openCount" | "modifiedAt";
  sortOrder?: "asc" | "desc";
}

export interface VideoMetaUpdate {
  customTitle?: string | null;
  notes?: string | null;
}

export interface PageResult<T> {
  total: number;
  items: T[];
}

// ---------- 打开日志 ----------
export interface OpenLog {
  id: number;
  videoId: number;
  openTime: string;
  closeTime: string | null;
  duration: number | null;
  status: string;
}

export interface OpenLogWithVideo extends OpenLog {
  fileName: string;
  filePath: string;
}

export interface LogFilter {
  videoId?: number | null;
  startDate?: string | null;
  endDate?: string | null;
}

// ---------- 统计 ----------
export interface DashboardStats {
  totalVideos: number;
  totalFolders: number;
  totalOpenCount: number;
  totalWatchSeconds: number;
  totalFileSize: number;
  todayWatchSeconds: number;
  todayOpenCount: number;
}

export interface TrendPoint {
  label: string;
  watchSeconds: number;
  openCount: number;
}

export interface LeaderboardItem {
  videoId: number;
  fileName: string;
  filePath: string;
  value: number;
  openCount: number;
  duration: number | null;
}

export interface TagStat {
  tagId: number;
  tagName: string;
  color: string;
  videoCount: number;
}

export interface HourCell {
  weekday: number;
  hour: number;
  count: number;
  seconds: number;
}

// ---------- 扫描 ----------
export interface ScanProgress {
  isScanning: boolean;
  progress: number;
  currentPath: string;
  scannedFiles: number;
  totalFiles: number;
  added: number;
  updated: number;
  unchanged: number;
  errors: string[];
}

export interface ScanResult {
  workspaceId: number;
  added: number;
  updated: number;
  unchanged: number;
  errors: string[];
}
