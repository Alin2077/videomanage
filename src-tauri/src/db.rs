use rusqlite::Connection;
use std::path::PathBuf;

/// 获取数据库文件路径：<data_dir>/video-manager/videos.db
pub fn db_path() -> Result<PathBuf, String> {
    let dir = dirs::data_dir()
        .or_else(|| dirs::home_dir())
        .ok_or_else(|| "无法确定数据目录".to_string())?
        .join("video-manager");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建数据目录失败: {e}"))?;
    Ok(dir.join("videos.db"))
}

/// 获取封面缓存目录
pub fn cover_dir() -> Result<PathBuf, String> {
    let dir = dirs::data_dir()
        .or_else(|| dirs::home_dir())
        .ok_or_else(|| "无法确定数据目录".to_string())?
        .join("video-manager")
        .join("covers");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建封面目录失败: {e}"))?;
    Ok(dir)
}

/// 应用待导入的备份（存在 .import-pending 时执行），返回是否导入成功
pub fn apply_pending_import() -> Result<bool, String> {
    let db_file = db_path()?;
    let data_dir = db_file.parent().ok_or("无法确定数据目录")?.to_path_buf();
    let pending_dir = data_dir.join(".import-pending");
    if !pending_dir.is_dir() {
        return Ok(false);
    }
    let pending_db = pending_dir.join("videos.db");
    if !pending_db.is_file() {
        let _ = std::fs::remove_dir_all(&pending_dir);
        return Ok(false);
    }
    // 覆盖数据库与封面
    std::fs::copy(&pending_db, &db_file).map_err(|e| format!("应用数据库失败: {e}"))?;
    let covers = data_dir.join("covers");
    let pending_covers = pending_dir.join("covers");
    if covers.is_dir() {
        let _ = std::fs::remove_dir_all(&covers);
    }
    if pending_covers.is_dir() {
        std::fs::create_dir_all(&covers).map_err(|e| format!("创建封面目录失败: {e}"))?;
        for entry in std::fs::read_dir(&pending_covers).map_err(|e| format!("读取封面失败: {e}"))? {
            let entry = entry.map_err(|e| format!("读取封面失败: {e}"))?;
            let name = entry.file_name().to_string_lossy().to_string();
            let _ = std::fs::copy(entry.path(), covers.join(name));
        }
    }
    let _ = std::fs::remove_dir_all(&pending_dir);
    Ok(true)
}

/// 打开并初始化数据库
pub fn init_db() -> Result<Connection, String> {
    let path = db_path()?;
    init_db_at(&path)
}

/// 在指定路径打开并初始化数据库
pub fn init_db_at(path: &std::path::Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建数据目录失败: {e}"))?;
    }
    let conn = Connection::open(path).map_err(|e| format!("打开数据库失败: {e}"))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("设置 WAL 失败: {e}"))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| format!("启用外键失败: {e}"))?;
    conn.pragma_update(None, "busy_timeout", 5000)
        .map_err(|e| format!("设置 busy_timeout 失败: {e}"))?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
-- 文件夹表
CREATE TABLE IF NOT EXISTS folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id INTEGER,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (parent_id) REFERENCES folders(id) ON DELETE CASCADE
);

-- 视频表
CREATE TABLE IF NOT EXISTS videos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_id INTEGER NOT NULL,
    file_name TEXT NOT NULL,
    file_path TEXT NOT NULL UNIQUE,
    file_size INTEGER NOT NULL,
    duration REAL,
    width INTEGER,
    height INTEGER,
    codec TEXT,
    fps REAL,
    sample_rate INTEGER,
    cover_path TEXT,
    custom_title TEXT,
    notes TEXT,
    open_count INTEGER DEFAULT 0,
    file_hash TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    modified_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    scanned_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE CASCADE
);

-- 标签组表
CREATE TABLE IF NOT EXISTS tag_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    sort_order INTEGER DEFAULT 0
);

-- 标签表
CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER,
    name TEXT NOT NULL,
    color TEXT DEFAULT '#1890ff',
    UNIQUE (group_id, name),
    FOREIGN KEY (group_id) REFERENCES tag_groups(id) ON DELETE SET NULL
);

-- 视频-标签关联表
CREATE TABLE IF NOT EXISTS video_tags (
    video_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (video_id, tag_id),
    FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

-- 打开日志表
CREATE TABLE IF NOT EXISTS open_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    video_id INTEGER NOT NULL,
    open_time DATETIME NOT NULL,
    close_time DATETIME,
    duration REAL,
    status TEXT DEFAULT 'active',
    FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE
);

-- 系统设置表
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 扫描错误日志表
CREATE TABLE IF NOT EXISTS scan_errors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT,
    message TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_folders_parent ON folders(parent_id);
CREATE INDEX IF NOT EXISTS idx_videos_path ON videos(file_path);
CREATE INDEX IF NOT EXISTS idx_videos_folder ON videos(folder_id);
CREATE INDEX IF NOT EXISTS idx_open_logs_video ON open_logs(video_id);
CREATE INDEX IF NOT EXISTS idx_open_logs_time ON open_logs(open_time);
CREATE INDEX IF NOT EXISTS idx_video_tags_tag ON video_tags(tag_id);
CREATE INDEX IF NOT EXISTS idx_videos_hash ON videos(file_hash);
CREATE INDEX IF NOT EXISTS idx_videos_modified ON videos(modified_at);
"#,
    )
    .map_err(|e| format!("初始化数据库失败: {e}"))?;
    Ok(())
}
