use rusqlite::Connection;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct DbState(pub Mutex<Connection>);

impl DbState {
    pub fn with_conn<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let conn = self.0.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;
        f(&conn).map_err(|e| format!("数据库操作失败: {}", e))
    }
}

pub fn init_database(app: &AppHandle) -> Result<DbState, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;
    std::fs::create_dir_all(&data_dir).map_err(|e| format!("创建数据目录失败: {}", e))?;

    let db_path = data_dir.join("app_data.db");
    let conn = Connection::open(&db_path).map_err(|e| format!("打开数据库失败: {}", e))?;

    // 通用表：
    //   - app_settings：通用 KV 配置存储
    //   - platform_accounts：多平台账号存储（百家号 / 小红书 / 央视频 / 七猫 / ...）
    //     字段尽量通用，`platform` 值由产品定义，cookies 是 JSON 字符串或 raw cookie header
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS platform_accounts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            platform TEXT NOT NULL,
            name TEXT NOT NULL DEFAULT '',
            avatar TEXT NOT NULL DEFAULT '',
            cookies TEXT,
            status TEXT NOT NULL DEFAULT 'inactive',
            remark TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
        );
        CREATE INDEX IF NOT EXISTS idx_platform_accounts_platform ON platform_accounts(platform);",
    )
    .map_err(|e| format!("建表失败: {}", e))?;

    Ok(DbState(Mutex::new(conn)))
}
