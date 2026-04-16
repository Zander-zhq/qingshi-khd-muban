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

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )
    .map_err(|e| format!("建表失败: {}", e))?;

    Ok(DbState(Mutex::new(conn)))
}
