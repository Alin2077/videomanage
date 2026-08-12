use super::with_db;
use super::TauriState;
use crate::models::{Tag, TagGroup, TagGroupInput, TagGroupWithTags, TagInput};
use rusqlite::params;

/// 获取所有标签组及标签
#[tauri::command]
pub fn get_tag_tree(state: TauriState<'_>) -> Result<Vec<TagGroupWithTags>, String> {
    with_db(&state, |conn| {
        let mut stmt = conn
            .prepare("SELECT id, name, sort_order FROM tag_groups ORDER BY sort_order, id")
            .map_err(|e| format!("查询失败: {e}"))?;
        let groups: Vec<TagGroup> = stmt
            .query_map([], |r| {
                Ok(TagGroup { id: r.get(0)?, name: r.get(1)?, sort_order: r.get(2)? })
            })
            .map_err(|e| format!("查询失败: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("读取失败: {e}"))?;

        let mut tag_stmt = conn
            .prepare("SELECT id, group_id, name, color FROM tags ORDER BY group_id, id")
            .map_err(|e| format!("查询失败: {e}"))?;
        let tags: Vec<Tag> = tag_stmt
            .query_map([], |r| {
                Ok(Tag { id: r.get(0)?, group_id: r.get(1)?, name: r.get(2)?, color: r.get(3)? })
            })
            .map_err(|e| format!("查询失败: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("读取失败: {e}"))?;

        let mut result = groups
            .into_iter()
            .map(|g| TagGroupWithTags { id: g.id, name: g.name, sort_order: g.sort_order, tags: Vec::new() })
            .collect::<Vec<_>>();

        // 无组的标签归入默认组
        let mut default_group = TagGroupWithTags { id: 0, name: "默认".to_string(), sort_order: 999, tags: Vec::new() };
        for t in tags {
            match t.group_id {
                Some(gid) => {
                    if let Some(g) = result.iter_mut().find(|g| g.id == gid) {
                        g.tags.push(t);
                    } else {
                        default_group.tags.push(t);
                    }
                }
                None => default_group.tags.push(t),
            }
        }
        if !default_group.tags.is_empty() {
            result.push(default_group);
        }
        Ok(result)
    })
}

/// 创建/更新标签
#[tauri::command]
pub fn upsert_tag(state: TauriState<'_>, tag: TagInput) -> Result<Tag, String> {
    let name = tag.name.trim().to_string();
    if name.is_empty() {
        return Err("标签名不能为空".to_string());
    }
    with_db(&state, |conn| {
        let color = tag.color.unwrap_or_else(|| "#1890ff".to_string());
        match tag.id {
            Some(id) => {
                conn.execute(
                    "UPDATE tags SET name = ?1, color = ?2, group_id = ?3 WHERE id = ?4",
                    params![name, color, tag.group_id, id],
                )
                .map_err(|e| format!("更新标签失败: {e}"))?;
                Ok(Tag { id, group_id: tag.group_id, name, color })
            }
            None => {
                conn.execute(
                    "INSERT INTO tags (group_id, name, color) VALUES (?1, ?2, ?3)",
                    params![tag.group_id, name, color],
                )
                .map_err(|e| format!("创建标签失败: {e}"))?;
                Ok(Tag { id: conn.last_insert_rowid(), group_id: tag.group_id, name, color })
            }
        }
    })
}

/// 删除标签
#[tauri::command]
pub fn delete_tag(state: TauriState<'_>, tag_id: i64) -> Result<(), String> {
    with_db(&state, |conn| {
        conn.execute("DELETE FROM tags WHERE id = ?1", params![tag_id])
            .map_err(|e| format!("删除标签失败: {e}"))?;
        Ok(())
    })
}

/// 创建/更新标签组
#[tauri::command]
pub fn upsert_tag_group(state: TauriState<'_>, group: TagGroupInput) -> Result<TagGroup, String> {
    let name = group.name.trim().to_string();
    if name.is_empty() {
        return Err("标签组名不能为空".to_string());
    }
    with_db(&state, |conn| {
        let sort_order = group.sort_order.unwrap_or(0);
        match group.id {
            Some(id) => {
                conn.execute(
                    "UPDATE tag_groups SET name = ?1, sort_order = ?2 WHERE id = ?3",
                    params![name, sort_order, id],
                )
                .map_err(|e| format!("更新标签组失败: {e}"))?;
                Ok(TagGroup { id, name, sort_order })
            }
            None => {
                conn.execute(
                    "INSERT INTO tag_groups (name, sort_order) VALUES (?1, ?2)",
                    params![name, sort_order],
                )
                .map_err(|e| format!("创建标签组失败: {e}"))?;
                Ok(TagGroup { id: conn.last_insert_rowid(), name, sort_order })
            }
        }
    })
}

/// 删除标签组（组内标签移入默认组）
#[tauri::command]
pub fn delete_tag_group(state: TauriState<'_>, group_id: i64) -> Result<(), String> {
    with_db(&state, |conn| {
        let tx = conn.unchecked_transaction().map_err(|e| format!("事务失败: {e}"))?;
        tx.execute("UPDATE tags SET group_id = NULL WHERE group_id = ?1", params![group_id])
            .map_err(|e| format!("更新标签失败: {e}"))?;
        tx.execute("DELETE FROM tag_groups WHERE id = ?1", params![group_id])
            .map_err(|e| format!("删除标签组失败: {e}"))?;
        tx.commit().map_err(|e| format!("提交失败: {e}"))?;
        Ok(())
    })
}

/// 为视频设置标签（全量覆盖）
#[tauri::command]
pub fn set_video_tags(state: TauriState<'_>, video_id: i64, tag_ids: Vec<i64>) -> Result<(), String> {
    with_db(&state, |conn| {
        let tx = conn.unchecked_transaction().map_err(|e| format!("事务失败: {e}"))?;
        tx.execute("DELETE FROM video_tags WHERE video_id = ?1", params![video_id])
            .map_err(|e| format!("更新标签失败: {e}"))?;
        for tid in &tag_ids {
            tx.execute(
                "INSERT OR IGNORE INTO video_tags (video_id, tag_id) VALUES (?1, ?2)",
                params![video_id, tid],
            )
            .map_err(|e| format!("更新标签失败: {e}"))?;
        }
        tx.commit().map_err(|e| format!("提交失败: {e}"))?;
        Ok(())
    })
}

/// 批量打标签（追加，不去重）
#[tauri::command]
pub fn batch_add_tags(
    state: TauriState<'_>,
    video_ids: Vec<i64>,
    tag_ids: Vec<i64>,
) -> Result<(), String> {
    if tag_ids.is_empty() || video_ids.is_empty() {
        return Ok(());
    }
    with_db(&state, |conn| {
        let tx = conn.unchecked_transaction().map_err(|e| format!("事务失败: {e}"))?;
        for vid in &video_ids {
            for tid in &tag_ids {
                tx.execute(
                    "INSERT OR IGNORE INTO video_tags (video_id, tag_id) VALUES (?1, ?2)",
                    params![vid, tid],
                )
                .map_err(|e| format!("批量打标签失败: {e}"))?;
            }
        }
        tx.commit().map_err(|e| format!("提交失败: {e}"))?;
        Ok(())
    })
}

/// 批量移除标签
#[tauri::command]
pub fn batch_remove_tags(
    state: TauriState<'_>,
    video_ids: Vec<i64>,
    tag_ids: Vec<i64>,
) -> Result<(), String> {
    if tag_ids.is_empty() || video_ids.is_empty() {
        return Ok(());
    }
    with_db(&state, |conn| {
        let tx = conn.unchecked_transaction().map_err(|e| format!("事务失败: {e}"))?;
        for vid in &video_ids {
            for tid in &tag_ids {
                tx.execute(
                    "DELETE FROM video_tags WHERE video_id = ?1 AND tag_id = ?2",
                    params![vid, tid],
                )
                .map_err(|e| format!("批量移除标签失败: {e}"))?;
            }
        }
        tx.commit().map_err(|e| format!("提交失败: {e}"))?;
        Ok(())
    })
}
