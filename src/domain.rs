use anyhow::{Result, bail};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub position: i64,
    pub archived: bool,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub temporary: bool,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn is_active(&self) -> bool {
        !self.archived && self.completed_at.is_none()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Exclusion {
    pub id: Uuid,
    /// Monday = 0, Sunday = 6. A range belongs to the day it starts on.
    pub days: Vec<u32>,
    pub start: NaiveTime,
    pub end: NaiveTime,
}

impl Exclusion {
    pub fn validate(&self) -> Result<()> {
        if self.days.is_empty() || self.days.iter().any(|day| *day > 6) {
            bail!("Select at least one valid weekday");
        }
        if self.start == self.end {
            bail!("Break start and end must differ");
        }
        Ok(())
    }

    fn range<T: TimeZone>(&self, date: NaiveDate, tz: &T) -> Option<(DateTime<T>, DateTime<T>)> {
        if !self.days.contains(&date.weekday().num_days_from_monday()) {
            return None;
        }
        let end_date = if self.end < self.start {
            date.succ_opt()?
        } else {
            date
        };
        let resolve = |date: NaiveDate, time: NaiveTime| {
            let mut naive = date.and_time(time);
            // At DST gaps, use the first valid local minute after the requested time.
            for _ in 0..=180 {
                if let Some(value) = tz.from_local_datetime(&naive).earliest() {
                    return Some(value);
                }
                naive += Duration::minutes(1);
            }
            None
        };
        Some((resolve(date, self.start)?, resolve(end_date, self.end)?))
    }

    pub fn contains<T: TimeZone>(&self, at: &DateTime<T>) -> bool {
        let today = at.date_naive();
        [Some(today), today.pred_opt()]
            .into_iter()
            .flatten()
            .any(|date| {
                self.range(date, &at.timezone())
                    .is_some_and(|(start, end)| *at >= start && *at < end)
            })
    }

    pub fn first_start<T: TimeZone>(
        &self,
        after: &DateTime<T>,
        until: &DateTime<T>,
    ) -> Option<DateTime<T>> {
        if until <= after {
            return None;
        }
        let mut date = after.date_naive();
        let mut best = None;
        // One recurring week is enough to find the first upcoming start.
        for _ in 0..=8 {
            if let Some((start, _)) = self.range(date, &after.timezone())
                && start > *after
                && start <= *until
            {
                best = Some(start);
                break;
            }
            date = date.succ_opt()?;
        }
        best
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    pub exclusions: Vec<Exclusion>,
    pub reminder_minutes: u32,
    #[serde(default)]
    pub language: crate::i18n::Language,
}

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub wall: DateTime<Utc>,
    pub mono_ms: u64,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub id: Uuid,
    pub task_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub elapsed_ms: u64,
    pub open: bool,
}

#[derive(Clone, Debug)]
pub struct Core {
    pub tasks: Vec<Task>,
    pub settings: Settings,
    pub selected: Option<Uuid>,
    pub active: Option<Session>,
    pub pending: Vec<Session>,
    pub visit_ms: u64,
    pub reminder_sent: bool,
    pub notice: String,
    pub last: Option<Sample>,
}

impl Core {
    pub fn new(tasks: Vec<Task>, settings: Settings) -> Self {
        let selected = tasks
            .iter()
            .filter(|t| t.is_active())
            .min_by_key(|t| t.position)
            .map(|t| t.id);
        Self {
            tasks,
            settings,
            selected,
            active: None,
            pending: vec![],
            visit_ms: 0,
            reminder_sent: false,
            notice: "Ready when you are".into(),
            last: None,
        }
    }

    pub fn queue(&self) -> Vec<&Task> {
        let mut tasks: Vec<_> = self.tasks.iter().filter(|t| t.is_active()).collect();
        tasks.sort_by_key(|t| t.position);
        tasks
    }

    pub fn current(&self) -> Option<&Task> {
        self.tasks
            .iter()
            .find(|t| Some(t.id) == self.selected && t.is_active())
    }

    pub fn next_id(&self) -> Option<Uuid> {
        let queue = self.queue();
        if queue.is_empty() {
            return None;
        }
        let index = queue.iter().position(|t| Some(t.id) == self.selected);
        Some(queue[index.map_or(0, |i| (i + 1) % queue.len())].id)
    }

    pub fn blocked(&self, at: DateTime<Utc>) -> bool {
        let local = at.with_timezone(&Local);
        self.settings
            .exclusions
            .iter()
            .any(|rule| rule.contains(&local))
    }

    pub fn add_task(&mut self, title: &str, description: &str, at: DateTime<Utc>) -> Result<Uuid> {
        let title = title.trim();
        if title.is_empty() {
            bail!("A task title is required");
        }
        let id = Uuid::new_v4();
        let position = self.tasks.iter().map(|t| t.position).max().unwrap_or(-1) + 1;
        self.tasks.push(Task {
            id,
            title: title.into(),
            description: description.trim().into(),
            position,
            archived: false,
            completed_at: None,
            temporary: false,
            updated_at: at,
        });
        if self.selected.is_none() {
            self.selected = Some(id);
        }
        Ok(id)
    }

    pub fn add_temporary_task(&mut self, title: &str, now: Sample) -> Result<Uuid> {
        if self.blocked(now.wall) {
            bail!("Scheduled break — resume after it ends");
        }
        let position = self.current().map(|task| task.position);
        let id = self.add_task(title, "", now.wall)?;
        if let Some(position) = position {
            for task in self.tasks.iter_mut().filter(|task| task.id != id) {
                if task.position >= position {
                    task.position += 1;
                    task.updated_at = now.wall;
                }
            }
        }
        let task = self.tasks.iter_mut().find(|task| task.id == id).unwrap();
        task.temporary = true;
        if let Some(position) = position {
            task.position = position;
        }
        self.select(id, now)?;
        self.start(now)?;
        Ok(id)
    }

    pub fn edit_task(
        &mut self,
        id: Uuid,
        title: &str,
        description: &str,
        at: DateTime<Utc>,
    ) -> Result<()> {
        if title.trim().is_empty() {
            bail!("A task title is required");
        }
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
        task.title = title.trim().into();
        task.description = description.trim().into();
        task.updated_at = at;
        Ok(())
    }

    pub fn reorder(&mut self, id: Uuid, direction: i32, at: DateTime<Utc>) {
        let ids: Vec<_> = self.queue().iter().map(|t| t.id).collect();
        if let Some(i) = ids.iter().position(|value| *value == id) {
            let j = i as i32 + direction;
            if j >= 0 && (j as usize) < ids.len() {
                let a = self.tasks.iter().position(|t| t.id == id).unwrap();
                let b = self
                    .tasks
                    .iter()
                    .position(|t| t.id == ids[j as usize])
                    .unwrap();
                let position = self.tasks[a].position;
                self.tasks[a].position = self.tasks[b].position;
                self.tasks[b].position = position;
                self.tasks[a].updated_at = at;
                self.tasks[b].updated_at = at;
            }
        }
    }

    pub fn complete(&mut self, id: Uuid, now: Sample) -> Result<()> {
        let index = self
            .tasks
            .iter()
            .position(|t| t.id == id && t.is_active())
            .ok_or_else(|| anyhow::anyhow!("Task is not in the active queue"))?;
        self.tick(now);
        let current = self.selected == Some(id);
        let running = self.active.is_some();
        let next = self.next_id().filter(|next| *next != id);
        self.tasks[index].completed_at = Some(now.wall);
        self.tasks[index].updated_at = now.wall;
        if current {
            self.pause("Task completed");
            self.selected = next;
            self.reset_visit();
            if running && next.is_some() {
                self.start(now)?;
            }
        }
        Ok(())
    }

    pub fn archive(&mut self, id: Uuid, now: Sample) {
        if !self.tasks.iter().any(|t| t.id == id && t.is_active()) {
            return;
        }
        self.tick(now);
        if self.selected == Some(id) {
            self.pause("Task archived");
            self.selected = self.queue().iter().find(|t| t.id != id).map(|t| t.id);
            self.reset_visit();
        }
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.archived = true;
            task.updated_at = now.wall;
        }
    }

    pub fn restore(&mut self, id: Uuid, at: DateTime<Utc>) {
        let position = self.tasks.iter().map(|t| t.position).max().unwrap_or(-1) + 1;
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.archived = false;
            task.completed_at = None;
            task.position = position;
            task.updated_at = at;
            if self.selected.is_none() {
                self.selected = Some(id);
            }
        }
    }

    fn reset_visit(&mut self) {
        self.visit_ms = 0;
        self.reminder_sent = false;
    }

    pub fn select(&mut self, id: Uuid, now: Sample) -> Result<()> {
        self.tick(now);
        if !self.tasks.iter().any(|t| t.id == id && t.is_active()) {
            bail!("Task is not in the active queue");
        }
        if self.selected == Some(id) {
            return Ok(());
        }
        let running = self.active.is_some();
        let temporary = self
            .current()
            .filter(|task| task.temporary)
            .map(|task| task.id);
        self.finish();
        if let Some(id) = temporary {
            let task = self.tasks.iter_mut().find(|task| task.id == id).unwrap();
            task.completed_at = Some(now.wall);
            task.updated_at = now.wall;
        }
        self.selected = Some(id);
        self.reset_visit();
        if running {
            self.start(now)?;
        }
        Ok(())
    }

    pub fn advance(&mut self, now: Sample) -> Result<()> {
        if let Some(task) = self.current().filter(|task| task.temporary) {
            return self.complete(task.id, now);
        }
        let id = self
            .next_id()
            .ok_or_else(|| anyhow::anyhow!("Add a task first"))?;
        self.select(id, now)
    }

    pub fn start(&mut self, now: Sample) -> Result<()> {
        self.tick(now);
        if self.active.is_some() {
            return Ok(());
        }
        if self.blocked(now.wall) {
            bail!("Scheduled break — resume after it ends");
        }
        let task_id = self
            .current()
            .ok_or_else(|| anyhow::anyhow!("Add or select a task first"))?
            .id;
        self.active = Some(Session {
            id: Uuid::new_v4(),
            task_id,
            started_at: now.wall,
            ended_at: now.wall,
            elapsed_ms: 0,
            open: true,
        });
        self.last = Some(now);
        self.notice = "Tracking".into();
        Ok(())
    }

    fn finish(&mut self) {
        if let Some(mut session) = self.active.take() {
            session.open = false;
            self.pending.push(session);
        }
    }

    pub fn pause(&mut self, reason: &str) {
        self.finish();
        self.notice = reason.into();
    }

    /// Returns true exactly once per visit when the reminder threshold is reached.
    pub fn tick(&mut self, now: Sample) -> bool {
        let previous = self.last.replace(now);
        if let (Some(previous), Some(_)) = (previous, self.active.as_ref()) {
            let mono_delta = now.mono_ms.saturating_sub(previous.mono_ms);
            let wall_delta = (now.wall - previous.wall).num_milliseconds();
            if wall_delta > mono_delta as i64 + 3000 || mono_delta > 15_000 {
                self.pause("Paused after sleep or an interrupted timer");
                return false;
            }
            let from = previous.wall.with_timezone(&Local);
            let to = now.wall.with_timezone(&Local);
            let boundary = self
                .settings
                .exclusions
                .iter()
                .filter_map(|rule| rule.first_start(&from, &to))
                .min()
                .map(|at| at.with_timezone(&Utc));
            let excluded_now = self.blocked(now.wall);
            let elapsed = boundary.map_or(mono_delta, |at| {
                let until_boundary = (at - previous.wall).num_milliseconds().max(0) as u64;
                mono_delta.min(until_boundary)
            });
            if let Some(session) = &mut self.active {
                session.elapsed_ms += elapsed;
                session.ended_at = boundary.unwrap_or(now.wall);
            }
            self.visit_ms += elapsed;
            if boundary.is_some() || excluded_now {
                self.pause("Scheduled break — resume manually afterwards");
            }
        }
        let threshold = u64::from(self.settings.reminder_minutes) * 60_000;
        if self.active.is_some()
            && threshold > 0
            && !self.reminder_sent
            && self.visit_ms >= threshold
        {
            self.reminder_sent = true;
            self.notice = "Time to check your next task".into();
            return true;
        }
        false
    }

    pub fn set_settings(&mut self, settings: Settings, now: Sample) -> Result<()> {
        for rule in &settings.exclusions {
            rule.validate()?;
        }
        if settings.reminder_minutes > 1440 {
            bail!("Reminder must be between 0 and 1440 minutes");
        }
        self.tick(now);
        self.settings = settings;
        if self.blocked(now.wall) {
            self.pause("Scheduled break — resume manually afterwards");
        }
        Ok(())
    }
}

pub fn format_duration(ms: u64) -> String {
    let seconds = ms / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}
