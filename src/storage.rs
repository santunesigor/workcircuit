use crate::domain::{Core, Session, Settings, Task};
use anyhow::{Context, Result, bail};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct Store {
    connection: Connection,
}

pub fn data_path() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .context("Neither XDG_DATA_HOME nor HOME is available")?;
    Ok(base.join("workcircuit/workcircuit.db"))
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .mode(0o600)
                .open(path)?;
        }
        Self::from_connection(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(mut connection: Connection) -> Result<Self> {
        connection.busy_timeout(std::time::Duration::from_secs(2))?;
        connection.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 1 {
            bail!("Database was created by a newer WorkCircuit version");
        }
        if version == 0 {
            let tx = connection.transaction()?;
            tx.execute_batch(include_str!("../migrations/001_initial.sql"))?;
            tx.commit()?;
        }
        Ok(Self { connection })
    }

    pub fn load(&mut self) -> Result<Core> {
        let recovered = self
            .connection
            .execute("UPDATE sessions SET open=0 WHERE open=1", [])?;
        let mut tasks = vec![];
        let mut stmt = self.connection.prepare(
            "SELECT id,title,description,position,archived,updated_at FROM tasks ORDER BY position",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, bool>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;
        for row in rows {
            let (id, title, description, position, archived, updated_at) = row?;
            tasks.push(Task {
                id: Uuid::parse_str(&id)?,
                title,
                description,
                position,
                archived,
                updated_at: parse_time(&updated_at)?,
            });
        }
        let settings = self
            .connection
            .query_row(
                "SELECT value FROM preferences WHERE key='settings'",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .map(|s| serde_json::from_str::<Settings>(&s))
            .transpose()?
            .unwrap_or_default();
        for rule in &settings.exclusions {
            rule.validate().context("Invalid stored scheduled break")?;
        }
        let selected = self
            .connection
            .query_row(
                "SELECT value FROM preferences WHERE key='selected'",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        let mut core = Core::new(tasks, settings);
        if let Some(id) = selected.and_then(|s| Uuid::parse_str(&s).ok())
            && core.tasks.iter().any(|t| t.id == id && !t.archived)
        {
            core.selected = Some(id);
        }
        if recovered > 0 {
            core.notice = "Recovered interrupted work at the last saved checkpoint".into();
        }
        Ok(core)
    }

    /// Snapshot and session transitions commit together. Pending work clears only after success.
    pub fn save(&mut self, core: &mut Core) -> Result<()> {
        let tx = self.connection.transaction()?;
        {
            let mut stmt = tx.prepare("INSERT INTO tasks(id,title,description,position,archived,updated_at) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET title=excluded.title,description=excluded.description,position=excluded.position,archived=excluded.archived,updated_at=excluded.updated_at WHERE tasks.updated_at != excluded.updated_at")?;
            for task in &core.tasks {
                stmt.execute(params![
                    task.id.to_string(),
                    task.title,
                    task.description,
                    task.position,
                    task.archived,
                    sql_time(task.updated_at)
                ])?;
            }
        }
        tx.execute("INSERT INTO preferences(key,value) VALUES('settings',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value WHERE preferences.value != excluded.value", [serde_json::to_string(&core.settings)?])?;
        tx.execute("INSERT INTO preferences(key,value) VALUES('selected',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value WHERE preferences.value != excluded.value", [core.selected.map(|id| id.to_string()).unwrap_or_default()])?;
        {
            let mut stmt = tx.prepare("INSERT INTO sessions(id,task_id,started_at,ended_at,elapsed_ms,open) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET ended_at=excluded.ended_at,elapsed_ms=excluded.elapsed_ms,open=excluded.open")?;
            for s in core.pending.iter().chain(core.active.iter()) {
                stmt.execute(params![
                    s.id.to_string(),
                    s.task_id.to_string(),
                    sql_time(s.started_at),
                    sql_time(s.ended_at),
                    i64::try_from(s.elapsed_ms)?,
                    s.open
                ])?;
            }
        }
        tx.commit()?;
        core.pending.clear();
        Ok(())
    }

    pub fn sessions(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<Session>> {
        // Fixed-width UTC strings keep the period predicate indexable.
        let mut stmt = self.connection.prepare("SELECT id,task_id,started_at,ended_at,elapsed_ms,open FROM sessions WHERE started_at < ?2 AND (ended_at > ?1 OR (ended_at <= started_at AND started_at >= ?1)) ORDER BY started_at")?;
        let rows = stmt.query_map(params![sql_time(start), sql_time(end)], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, bool>(5)?,
            ))
        })?;
        let mut sessions = vec![];
        for row in rows {
            let (id, task_id, started, ended, elapsed, open) = row?;
            sessions.push(Session {
                id: Uuid::parse_str(&id)?,
                task_id: Uuid::parse_str(&task_id)?,
                started_at: parse_time(&started)?,
                ended_at: parse_time(&ended)?,
                elapsed_ms: u64::try_from(elapsed)?,
                open,
            });
        }
        Ok(sessions)
    }
}

use rusqlite::OptionalExtension;
fn parse_time(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}

fn sql_time(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Nanos, true)
}
