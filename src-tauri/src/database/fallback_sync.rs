//! fork 私有备用数据库的状态与单向同步。
//!
//! 方向固定为 primary → fallback。备用库被使用时会在状态文件里标记 dirty，
//! 自动同步随即暂停，避免把用户在主库不兼容期间写入的数据覆盖掉。

use super::selector::{copy_database, DatabaseProbe};
use super::{Database, FALLBACK_DB_FILENAME};
use crate::config::{atomic_write, get_app_config_dir};
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const STATE_FILENAME: &str = "mod-db-state.json";
const SYNC_INTERVAL: Duration = Duration::from_secs(10 * 60);
const WORKER_TICK: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackState {
    pub fallback_dirty: bool,
    pub last_sync_at: Option<u64>,
    pub last_sync_primary_version: Option<i32>,
}

pub fn state_path() -> PathBuf {
    get_app_config_dir().join(STATE_FILENAME)
}

pub fn read_state() -> Option<FallbackState> {
    let data = std::fs::read(state_path()).ok()?;
    serde_json::from_slice(&data).ok()
}

fn write_state(state: &FallbackState) -> Result<(), AppError> {
    let data = serde_json::to_vec_pretty(state)
        .map_err(|e| AppError::Message(format!("serialize fallback state: {e}")))?;
    atomic_write(&state_path(), &data)
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 自动同步（受 dirty 标记与节流限制）。
pub fn sync_if_allowed(db: &Arc<Database>) -> Result<(), AppError> {
    if db.is_fallback() {
        return Ok(());
    }
    let mut state = read_state().unwrap_or_default();
    if state.fallback_dirty {
        return Ok(());
    }
    if let Some(last) = state.last_sync_at {
        if now_epoch().saturating_sub(last) < SYNC_INTERVAL.as_secs() {
            return Ok(());
        }
    }

    let fallback = Database::fallback_path();
    copy_database(db.path(), &fallback)?;

    state.fallback_dirty = false;
    state.last_sync_at = Some(now_epoch());
    state.last_sync_primary_version = Some(super::SCHEMA_VERSION);
    write_state(&state)?;
    log::info!(
        "Synced primary database to fallback: {}",
        fallback.display()
    );
    Ok(())
}

/// 人工强制同步：允许覆盖 dirty 的备用库，但先用 SQLite Backup API 留一份安全副本。
pub fn force_sync(db: &Arc<Database>) -> Result<PathBuf, AppError> {
    if db.is_fallback() {
        return Err(AppError::Message(
            "active database is already the fallback".to_string(),
        ));
    }

    let fallback = Database::fallback_path();
    if fallback.exists() {
        let backup_dir = fallback
            .parent()
            .ok_or_else(|| AppError::Config("invalid fallback path".to_string()))?
            .join("backups");
        std::fs::create_dir_all(&backup_dir).map_err(|e| AppError::io(&backup_dir, e))?;
        let stamp = now_epoch();
        let backup_path = backup_dir.join(format!("mod_db_backup_{stamp}.db"));
        copy_database(&fallback, &backup_path)?;
        log::info!(
            "Created fallback safety backup before force sync: {}",
            backup_path.display()
        );
    }

    copy_database(db.path(), &fallback)?;
    let mut state = read_state().unwrap_or_default();
    state.fallback_dirty = false;
    state.last_sync_at = Some(now_epoch());
    state.last_sync_primary_version = Some(super::SCHEMA_VERSION);
    write_state(&state)?;
    Ok(fallback)
}

/// 备用库被选中时调用：在打开它之前落盘 dirty 标记。
pub fn mark_dirty_on_selection() {
    let mut state = read_state().unwrap_or_default();
    if !state.fallback_dirty {
        state.fallback_dirty = true;
        if let Err(err) = write_state(&state) {
            log::warn!("failed to persist fallback dirty state: {err}");
        }
    }
}

/// 启动后台同步线程；失败只记录日志，不影响应用启动。
pub fn start_worker(db: Arc<Database>) {
    std::thread::spawn(move || {
        if db.is_fallback() {
            mark_dirty_on_selection();
            log::info!("Fallback database active: {}", db.path().display());
            return;
        }

        // 启动后先尝试一次，让备用库尽快有可用快照。
        if let Err(err) = sync_if_allowed(&db) {
            log::warn!("initial fallback sync failed: {err}");
        }

        loop {
            std::thread::sleep(WORKER_TICK);
            if let Err(err) = sync_if_allowed(&db) {
                log::warn!("periodic fallback sync failed: {err}");
            }
        }
    });
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatus {
    pub active_path: String,
    pub is_fallback: bool,
    pub primary: DatabaseProbe,
    pub fallback: DatabaseProbe,
    pub fallback_dirty: bool,
    pub last_sync_at: Option<u64>,
}

pub fn status(db: &Database) -> DatabaseStatus {
    let app_config_dir = get_app_config_dir();
    let primary = super::selector::probe_database(&app_config_dir.join(super::PRIMARY_DB_FILENAME));
    let fallback = super::selector::probe_database(&app_config_dir.join(FALLBACK_DB_FILENAME));
    let state = read_state().unwrap_or_default();
    DatabaseStatus {
        active_path: db.path().display().to_string(),
        is_fallback: db.is_fallback(),
        primary,
        fallback,
        fallback_dirty: state.fallback_dirty,
        last_sync_at: state.last_sync_at,
    }
}
