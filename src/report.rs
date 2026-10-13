use crate::{
    domain::{Session, Task},
    storage::Store,
};
use anyhow::{Result, bail};
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use std::{collections::BTreeMap, io::Write};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct ReportSession {
    pub session: Session,
    pub title: String,
    pub description: String,
    pub contribution_ms: u64,
}

#[derive(Clone, Debug)]
pub struct Total {
    pub task_id: Uuid,
    pub title: String,
    pub description: String,
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug)]
pub struct Report {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub sessions: Vec<ReportSession>,
    pub totals: Vec<Total>,
    pub total_ms: u64,
}

pub fn date_range<T: TimeZone>(
    from: NaiveDate,
    through: NaiveDate,
    tz: &T,
) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    if through < from {
        bail!("End date must not precede start date");
    }
    let midnight = |date: NaiveDate| -> Result<DateTime<Utc>> {
        let mut naive = date.and_hms_opt(0, 0, 0).unwrap();
        for _ in 0..=180 {
            if let Some(at) = tz.from_local_datetime(&naive).earliest() {
                return Ok(at.with_timezone(&Utc));
            }
            naive += Duration::minutes(1);
        }
        bail!("Date does not have a valid local midnight")
    };
    Ok((
        midnight(from)?,
        midnight(
            through
                .succ_opt()
                .ok_or_else(|| anyhow::anyhow!("End date is out of range"))?,
        )?,
    ))
}

pub fn load(
    store: &Store,
    tasks: &[Task],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Report> {
    Ok(aggregate(store.sessions(start, end)?, tasks, start, end))
}

pub fn aggregate(
    sessions: Vec<Session>,
    tasks: &[Task],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Report {
    let mut result = Report {
        start,
        end,
        sessions: vec![],
        totals: vec![],
        total_ms: 0,
    };
    let mut totals: BTreeMap<Uuid, Total> = BTreeMap::new();
    for session in sessions {
        let elapsed = contribution(&session, start, end);
        if elapsed == 0 {
            continue;
        }
        let task = tasks.iter().find(|t| t.id == session.task_id);
        let title = task
            .map_or("Archived task", |t| t.title.as_str())
            .to_owned();
        let description = task.map_or("", |t| t.description.as_str()).to_owned();
        totals
            .entry(session.task_id)
            .or_insert_with(|| Total {
                task_id: session.task_id,
                title: title.clone(),
                description: description.clone(),
                elapsed_ms: 0,
            })
            .elapsed_ms += elapsed;
        result.total_ms += elapsed;
        result.sessions.push(ReportSession {
            session,
            title,
            description,
            contribution_ms: elapsed,
        });
    }
    result.totals = totals.into_values().collect();
    result
        .totals
        .sort_by(|a, b| b.elapsed_ms.cmp(&a.elapsed_ms).then(a.title.cmp(&b.title)));
    result
}

pub fn contribution(session: &Session, start: DateTime<Utc>, end: DateTime<Utc>) -> u64 {
    let wall_ms = (session.ended_at - session.started_at).num_milliseconds();
    if wall_ms <= 0 {
        return if session.started_at >= start && session.started_at < end {
            session.elapsed_ms
        } else {
            0
        };
    }
    let overlap = (session.ended_at.min(end) - session.started_at.max(start))
        .num_milliseconds()
        .max(0) as u64;
    ((u128::from(session.elapsed_ms) * u128::from(overlap)) / wall_ms as u128) as u64
}

#[derive(Clone, Copy)]
pub enum Export {
    Totals,
    Sessions,
}

fn safe_cell(value: &str) -> String {
    if value.trim_start().starts_with(['=', '+', '-', '@']) || value.starts_with(['\t', '\r']) {
        format!("'{value}")
    } else {
        value.into()
    }
}

pub fn write_csv(report: &Report, kind: Export, output: impl Write) -> Result<()> {
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(output);
    match kind {
        Export::Totals => {
            writer.write_record([
                "task_id",
                "title",
                "description",
                "seconds",
                "period_start_utc",
                "period_end_utc_exclusive",
            ])?;
            for total in &report.totals {
                writer.write_record([
                    total.task_id.to_string(),
                    safe_cell(&total.title),
                    safe_cell(&total.description),
                    format!("{:.3}", total.elapsed_ms as f64 / 1000.0),
                    report.start.to_rfc3339(),
                    report.end.to_rfc3339(),
                ])?;
            }
        }
        Export::Sessions => {
            writer.write_record([
                "session_id",
                "task_id",
                "title",
                "description",
                "start_utc",
                "end_utc",
                "session_seconds",
                "period_seconds",
            ])?;
            for row in &report.sessions {
                let s = &row.session;
                writer.write_record([
                    s.id.to_string(),
                    s.task_id.to_string(),
                    safe_cell(&row.title),
                    safe_cell(&row.description),
                    s.started_at.to_rfc3339(),
                    s.ended_at.to_rfc3339(),
                    format!("{:.3}", s.elapsed_ms as f64 / 1000.0),
                    format!("{:.3}", row.contribution_ms as f64 / 1000.0),
                ])?;
            }
        }
    }
    writer.flush()?;
    Ok(())
}
