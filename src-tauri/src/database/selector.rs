//! 主数据库 / fork 私有备用数据库的选择逻辑。
//!
//! 兼容性采用严格相等：只有 `PRAGMA user_version == SCHEMA_VERSION` 才读写主库。
//! 主库偏旧时不原地迁移，而是复制一份到 `mod_cc-switch.db` 后迁移副本，避免
//! 把仍在被官方版使用的共享数据库升级掉。

use super::{Database, FALLBACK_DB_FILENAME, PRIMARY_DB_FILENAME, SCHEMA_VERSION};
use crate::error::AppError;
use rusqlite::backup::{Backup, StepResult};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DatabaseProbeState {
    Missing,
    Compatible,
    Older,
    TooNew,
    Unusable,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseProbe {
    pub path: String,
    pub exists: bool,
    pub state: DatabaseProbeState,
    pub version: Option<i32>,
    pub error: Option<String>,
}

impl DatabaseProbe {
    fn new(
        path: &Path,
        state: DatabaseProbeState,
        version: Option<i32>,
        error: Option<String>,
    ) -> Self {
        Self {
            path: path.display().to_string(),
            exists: path.exists(),
            state,
            version,
            error,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActiveDatabase {
    pub path: PathBuf,
    pub is_fallback: bool,
    pub reason: String,
    pub primary: DatabaseProbe,
    pub fallback: DatabaseProbe,
}

#[derive(Debug, Clone)]
pub struct SelectionFailure {
    pub reason: String,
    pub primary: DatabaseProbe,
    pub fallback: DatabaseProbe,
}

impl SelectionFailure {
    /// 失败时最值得展示的版本号（用于升级界面）
    pub fn observed_version(&self) -> Option<i32> {
        [self.primary.version, self.fallback.version]
            .into_iter()
            .flatten()
            .max()
    }

    pub fn has_too_new(&self) -> bool {
        matches!(self.primary.state, DatabaseProbeState::TooNew)
            || matches!(self.fallback.state, DatabaseProbeState::TooNew)
    }
}

/// 只读探测数据库；绝不执行 DDL / create_tables / 迁移。
pub fn probe_database(path: &Path) -> DatabaseProbe {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return DatabaseProbe::new(path, DatabaseProbeState::Missing, None, None);
        }
        Err(err) => {
            return DatabaseProbe::new(
                path,
                DatabaseProbeState::Unusable,
                None,
                Some(err.to_string()),
            );
        }
    };

    // 测试和旧版升级逻辑会用零字节文件做哨兵；按“可初始化”处理。
    if metadata.len() == 0 {
        return DatabaseProbe::new(path, DatabaseProbeState::Compatible, None, None);
    }

    let conn = match Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(conn) => conn,
        Err(err) => {
            return DatabaseProbe::new(
                path,
                DatabaseProbeState::Unusable,
                None,
                Some(err.to_string()),
            );
        }
    };

    let version = match Database::get_user_version(&conn) {
        Ok(version) => version,
        Err(err) => {
            return DatabaseProbe::new(
                path,
                DatabaseProbeState::Unusable,
                None,
                Some(err.to_string()),
            );
        }
    };

    if version > SCHEMA_VERSION {
        return DatabaseProbe::new(path, DatabaseProbeState::TooNew, Some(version), None);
    }

    if let Err(err) = quick_check(&conn) {
        return DatabaseProbe::new(path, DatabaseProbeState::Unusable, Some(version), Some(err));
    }

    let state = if version == SCHEMA_VERSION {
        DatabaseProbeState::Compatible
    } else {
        DatabaseProbeState::Older
    };
    DatabaseProbe::new(path, state, Some(version), None)
}

fn quick_check(conn: &Connection) -> Result<(), String> {
    let result: String = conn
        .query_row("PRAGMA quick_check;", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if result.eq_ignore_ascii_case("ok") {
        Ok(())
    } else {
        Err(format!("quick_check failed: {result}"))
    }
}

/// 选择本次启动使用的数据库。
///
/// 主库偏旧且备用库不存在时，会把主库复制到备用库路径；随后
/// `Database::init_with_path` 会在副本上执行迁移，主库本身保持不变。
#[allow(clippy::result_large_err)]
pub fn select_active_database(app_config_dir: &Path) -> Result<ActiveDatabase, SelectionFailure> {
    let primary_path = app_config_dir.join(PRIMARY_DB_FILENAME);
    let fallback_path = app_config_dir.join(FALLBACK_DB_FILENAME);

    let primary = probe_database(&primary_path);
    let fallback = probe_database(&fallback_path);

    let active = |path: PathBuf, is_fallback: bool, reason: &str| ActiveDatabase {
        path,
        is_fallback,
        reason: reason.to_string(),
        primary: primary.clone(),
        fallback: fallback.clone(),
    };

    match primary.state {
        DatabaseProbeState::Compatible => {
            return Ok(active(primary_path, false, "primary schema matches"));
        }
        DatabaseProbeState::Missing => {
            return match fallback.state {
                DatabaseProbeState::Compatible | DatabaseProbeState::Older => Ok(active(
                    fallback_path,
                    true,
                    "primary missing; using existing fallback",
                )),
                _ => Ok(active(primary_path, false, "fresh install")),
            };
        }
        DatabaseProbeState::TooNew | DatabaseProbeState::Unusable | DatabaseProbeState::Older => {}
    }

    match fallback.state {
        DatabaseProbeState::Compatible | DatabaseProbeState::Older => Ok(active(
            fallback_path,
            true,
            "primary is not exact-compatible; using fallback",
        )),
        DatabaseProbeState::Missing => {
            // 主库偏旧时先复制，再让 init 迁移副本；主库偏新/损坏时只能新建空备用库。
            if matches!(primary.state, DatabaseProbeState::Older) {
                if let Err(err) = copy_database(&primary_path, &fallback_path) {
                    return Err(SelectionFailure {
                        reason: format!("failed to seed fallback from primary: {err}"),
                        primary,
                        fallback,
                    });
                }
                log::info!(
                    "Copied older primary database to fallback before migrating: {}",
                    fallback_path.display()
                );
            }
            Ok(active(
                fallback_path,
                true,
                "primary is not exact-compatible; created fallback",
            ))
        }
        DatabaseProbeState::TooNew | DatabaseProbeState::Unusable => Err(SelectionFailure {
            reason: "primary and fallback are both unusable".to_string(),
            primary,
            fallback,
        }),
    }
}

/// 用 SQLite Backup API 复制数据库文件。
pub fn copy_database(source: &Path, destination: &Path) -> Result<(), AppError> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let source_conn = Connection::open_with_flags(
        source,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| AppError::Database(format!("open source database: {e}")))?;

    let temp_path = destination.with_extension("tmp-copy");
    let _ = std::fs::remove_file(&temp_path);
    let mut dest_conn = Connection::open(&temp_path)
        .map_err(|e| AppError::Database(format!("open destination database: {e}")))?;

    {
        let backup = Backup::new(&source_conn, &mut dest_conn)
            .map_err(|e| AppError::Database(format!("start database copy: {e}")))?;
        match backup
            .step(-1)
            .map_err(|e| AppError::Database(format!("database copy failed: {e}")))?
        {
            StepResult::Done => {}
            other => {
                return Err(AppError::Database(format!(
                    "database copy incomplete: {other:?}"
                )));
            }
        }
    }

    dest_conn
        .close()
        .map_err(|(_, e)| AppError::Database(format!("close copied database: {e}")))?;
    std::fs::rename(&temp_path, destination).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        AppError::io(destination, e)
    })
}
