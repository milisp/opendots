use rusqlite::Connection;
use std::path::PathBuf;

fn get_db_path() -> Result<PathBuf, String> {
    let data_dir = dirs::data_dir().ok_or("Could not get application data directory")?;
    let app_dir = data_dir.join("opendots");
    std::fs::create_dir_all(&app_dir)
        .map_err(|e| format!("Failed to create opendots data directory: {e}"))?;
    Ok(app_dir.join("bots.db"))
}

pub(crate) fn get_connection() -> Result<Connection, String> {
    let conn =
        Connection::open(get_db_path()?).map_err(|e| format!("Failed to open database: {e}"))?;
    init_tables(&conn)?;
    Ok(conn)
}

fn init_tables(conn: &Connection) -> Result<(), String> {
    init_bots_table(conn)?;
    init_acp_tables(conn)?;
    init_runs_table(conn)
}

fn init_bots_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS bots (
        id TEXT PRIMARY KEY, name TEXT NOT NULL, title TEXT, avatar TEXT NOT NULL, color TEXT NOT NULL,
        agent_id TEXT NOT NULL, provider TEXT, model TEXT, reasoning_effort TEXT, cwd TEXT NOT NULL,
        system_prompt TEXT, trust_level TEXT NOT NULL, approved_tools TEXT NOT NULL DEFAULT '[]',
        mcp_servers TEXT NOT NULL DEFAULT '[]', pinned INTEGER NOT NULL DEFAULT 0, archived INTEGER NOT NULL DEFAULT 0,
        notifications_enabled INTEGER NOT NULL DEFAULT 1, unread_count INTEGER NOT NULL DEFAULT 0,
        last_viewed_at TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
        ")
        .map_err(|e| e.to_string())?;
    add_column_if_missing(conn, "bots", "title", "TEXT")?;
    add_column_if_missing(conn, "bots", "provider", "TEXT")?;
    add_column_if_missing(conn, "bots", "model", "TEXT")?;
    add_column_if_missing(conn, "bots", "reasoning_effort", "TEXT")?;
    add_column_if_missing(conn, "bots", "system_prompt", "TEXT")?;
    add_column_if_missing(conn, "bots", "approved_tools", "TEXT NOT NULL DEFAULT '[]'")?;
    add_column_if_missing(conn, "bots", "mcp_servers", "TEXT NOT NULL DEFAULT '[]'")?;
    add_column_if_missing(conn, "bots", "pinned", "INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(conn, "bots", "archived", "INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(
        conn,
        "bots",
        "notifications_enabled",
        "INTEGER NOT NULL DEFAULT 1",
    )?;
    add_column_if_missing(conn, "bots", "unread_count", "INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(conn, "bots", "last_viewed_at", "TEXT")?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_bots_pinned_updated ON bots(pinned DESC, updated_at DESC)",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
    if let Err(error) = conn.execute(&sql, []) {
        if !error.to_string().contains("duplicate column name") {
            return Err(error.to_string());
        }
    }
    Ok(())
}

fn init_acp_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS acp_sessions (
        session_id TEXT PRIMARY KEY, agent_id TEXT NOT NULL, agent_title TEXT, cwd TEXT NOT NULL,
        title TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS acp_session_updates (
        id INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL, payload TEXT NOT NULL, created_at TEXT NOT NULL);
        CREATE INDEX IF NOT EXISTS idx_acp_updates_session ON acp_session_updates(session_id, id ASC);")
        .map_err(|e| e.to_string())?;
    // Upgrade databases made before ACP conversations were associated with bots.
    add_column_if_missing(conn, "acp_sessions", "agent_title", "TEXT")?;
    add_column_if_missing(conn, "acp_sessions", "title", "TEXT")?;
    add_column_if_missing(conn, "acp_sessions", "bot_id", "TEXT")?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_acp_sessions_bot_updated ON acp_sessions(bot_id, updated_at DESC)",
        [],
    )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn init_runs_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS automation_runs (
        run_id TEXT PRIMARY KEY, task_id TEXT NOT NULL, task_name TEXT NOT NULL,
        thread_id TEXT NOT NULL UNIQUE, status TEXT NOT NULL, started_at TEXT NOT NULL,
        updated_at TEXT NOT NULL, cwd TEXT);
        CREATE TABLE IF NOT EXISTS automation_run_steps (
        id INTEGER PRIMARY KEY AUTOINCREMENT, run_id TEXT NOT NULL, step_kind TEXT NOT NULL,
        turn_id TEXT, message TEXT, created_at TEXT NOT NULL);
        CREATE INDEX IF NOT EXISTS idx_automation_runs_task_started ON automation_runs(task_id, started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_automation_runs_thread ON automation_runs(thread_id);")
        .map_err(|e| e.to_string())?;
    add_column_if_missing(conn, "automation_runs", "cwd", "TEXT")?;
    Ok(())
}
