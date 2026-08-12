use serde::{Deserialize, Serialize};

// ---------- 文件夹 ----------

#[derive(Debug, Clone, Serialize)]
pub struct FolderNode {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub path: String,
    pub video_count: i64,
    pub has_children: bool,
}

// ---------- 视频 ----------

#[derive(Debug, Clone, Serialize)]
pub struct VideoInfo {
    pub id: i64,
    pub folder_id: i64,
    pub file_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub duration: Option<f64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub codec: Option<String>,
    pub fps: Option<f64>,
    pub sample_rate: Option<i64>,
    pub cover_path: Option<String>,
    pub custom_title: Option<String>,
    pub notes: Option<String>,
    pub open_count: i64,
    pub file_hash: Option<String>,
    pub created_at: String,
    pub modified_at: String,
    pub scanned_at: String,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VideoDetail {
    #[serde(flatten)]
    pub info: VideoInfo,
    pub folder_path: String,
    pub open_logs: Vec<OpenLog>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PageResult<T> {
    pub total: i64,
    pub items: Vec<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VideoQuery {
    pub folder_id: Option<i64>,
    pub keyword: Option<String>,
    pub tag_ids: Option<Vec<i64>>,
    pub page: u32,
    pub page_size: u32,
    pub sort_by: Option<String>, // name | size | duration | open_count | modified_at
    pub sort_order: Option<String>, // asc | desc
}

#[derive(Debug, Clone, Deserialize)]
pub struct VideoMetaUpdate {
    pub custom_title: Option<String>,
    pub notes: Option<String>,
}

// ---------- 标签 ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: i64,
    pub group_id: Option<i64>,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagGroup {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagGroupWithTags {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TagInput {
    pub id: Option<i64>,
    pub group_id: Option<i64>,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TagGroupInput {
    pub id: Option<i64>,
    pub name: String,
    pub sort_order: Option<i64>,
}

// ---------- 打开日志 ----------

#[derive(Debug, Clone, Serialize)]
pub struct OpenLog {
    pub id: i64,
    pub video_id: i64,
    pub open_time: String,
    pub close_time: Option<String>,
    pub duration: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenLogWithVideo {
    #[serde(flatten)]
    pub log: OpenLog,
    pub file_name: String,
    pub file_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogFilter {
    pub video_id: Option<i64>,
    pub start_date: Option<String>, // YYYY-MM-DD
    pub end_date: Option<String>,   // YYYY-MM-DD
}

// ---------- 统计 ----------

#[derive(Debug, Clone, Serialize)]
pub struct DashboardStats {
    pub total_videos: i64,
    pub total_folders: i64,
    pub total_open_count: i64,
    pub total_watch_seconds: f64,
    pub total_file_size: i64,
    pub today_watch_seconds: f64,
    pub today_open_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrendPoint {
    pub label: String,
    pub watch_seconds: f64,
    pub open_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LeaderboardItem {
    pub video_id: i64,
    pub file_name: String,
    pub file_path: String,
    pub value: f64,
    pub open_count: i64,
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagStat {
    pub tag_id: i64,
    pub tag_name: String,
    pub color: String,
    pub video_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HourCell {
    pub weekday: i64, // 0=周一 ... 6=周日
    pub hour: i64,    // 0-23
    pub count: i64,
    pub seconds: f64,
}

// ---------- 扫描 ----------

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub is_scanning: bool,
    pub progress: f64, // 0-100
    pub current_path: String,
    pub scanned_files: u64,
    pub total_files: u64,
    pub added: u64,
    pub updated: u64,
    pub unchanged: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    pub added: u64,
    pub updated: u64,
    pub unchanged: u64,
    pub errors: Vec<String>,
}
