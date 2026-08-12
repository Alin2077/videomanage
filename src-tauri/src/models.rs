use serde::{Deserialize, Serialize};

// ---------- 工作区 ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub created_at: String,
    pub video_count: i64,
    pub folder_count: i64,
}

// ---------- 文件夹 ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct VideoDetail {
    #[serde(flatten)]
    pub info: VideoInfo,
    pub folder_path: String,
    pub open_logs: Vec<OpenLog>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageResult<T> {
    pub total: i64,
    pub items: Vec<T>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoQuery {
    pub workspace_id: Option<i64>,
    pub folder_id: Option<i64>,
    pub keyword: Option<String>,
    pub tag_ids: Option<Vec<i64>>,
    pub page: u32,
    pub page_size: u32,
    pub sort_by: Option<String>,   // name | size | duration | open_count | modified_at
    pub sort_order: Option<String>, // asc | desc
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetaUpdate {
    pub custom_title: Option<String>,
    pub notes: Option<String>,
}

// ---------- 标签 ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: i64,
    pub group_id: Option<i64>,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagGroup {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagGroupWithTags {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagInput {
    pub id: Option<i64>,
    pub group_id: Option<i64>,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagGroupInput {
    pub id: Option<i64>,
    pub name: String,
    pub sort_order: Option<i64>,
}

// ---------- 打开日志 ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenLog {
    pub id: i64,
    pub video_id: i64,
    pub open_time: String,
    pub close_time: Option<String>,
    pub duration: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenLogWithVideo {
    #[serde(flatten)]
    pub log: OpenLog,
    pub file_name: String,
    pub file_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFilter {
    pub video_id: Option<i64>,
    pub start_date: Option<String>, // YYYY-MM-DD
    pub end_date: Option<String>,   // YYYY-MM-DD
}

// ---------- 统计 ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    pub label: String,
    pub watch_seconds: f64,
    pub open_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardItem {
    pub video_id: i64,
    pub file_name: String,
    pub file_path: String,
    pub value: f64,
    pub open_count: i64,
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagStat {
    pub tag_id: i64,
    pub tag_name: String,
    pub color: String,
    pub video_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HourCell {
    pub weekday: i64, // 0=周一 ... 6=周日
    pub hour: i64,    // 0-23
    pub count: i64,
    pub seconds: f64,
}

// ---------- 扫描 ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub workspace_id: i64,
    pub added: u64,
    pub updated: u64,
    pub unchanged: u64,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_video() -> VideoInfo {
        VideoInfo {
            id: 1,
            folder_id: 2,
            file_name: "a.mp4".into(),
            file_path: "C:\\a.mp4".into(),
            file_size: 100,
            duration: Some(10.5),
            width: Some(1920),
            height: Some(1080),
            codec: Some("h264".into()),
            fps: Some(30.0),
            sample_rate: None,
            cover_path: None,
            custom_title: None,
            notes: None,
            open_count: 3,
            file_hash: None,
            created_at: String::new(),
            modified_at: String::new(),
            scanned_at: String::new(),
            tags: vec![Tag { id: 9, group_id: None, name: "电影".into(), color: "#1890ff".into() }],
        }
    }

    #[test]
    fn video_info_serializes_with_camel_case() {
        let val = serde_json::to_value(sample_video()).unwrap();
        assert_eq!(val["fileName"], "a.mp4");
        assert_eq!(val["folderId"], 2);
        assert_eq!(val["openCount"], 3);
        assert_eq!(val["sampleRate"], serde_json::Value::Null);
        assert_eq!(val["tags"][0]["groupId"], serde_json::Value::Null);
        assert!(val.get("file_name").is_none(), "不应输出 snake_case 字段 file_name");
        assert!(val.get("open_count").is_none(), "不应输出 snake_case 字段 open_count");
    }

    #[test]
    fn video_query_deserializes_camel_case() {
        // 模拟前端 invoke('list_videos', { query }) 的参数
        let json = json!({
            "workspaceId": 5,
            "folderId": 3,
            "keyword": "demo",
            "tagIds": [1, 2],
            "page": 1,
            "pageSize": 50,
            "sortBy": "name",
            "sortOrder": "asc"
        });
        let q: VideoQuery = serde_json::from_value(json).unwrap();
        assert_eq!(q.workspace_id, Some(5));
        assert_eq!(q.folder_id, Some(3));
        assert_eq!(q.page_size, 50);
        assert_eq!(q.tag_ids, Some(vec![1, 2]));
    }

    #[test]
    fn scan_result_serializes_with_camel_case() {
        let r = ScanResult {
            workspace_id: 7,
            added: 1,
            updated: 2,
            unchanged: 3,
            errors: vec![],
        };
        let val = serde_json::to_value(r).unwrap();
        assert_eq!(val["workspaceId"], 7);
        assert!(val.get("workspace_id").is_none());
    }
}
