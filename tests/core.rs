use chrono::{Duration, FixedOffset, Local, NaiveDate, NaiveTime, TimeZone, Utc};
use uuid::Uuid;
use workcircuit::{
    domain::{Core, Exclusion, Sample, Session, Settings},
    report::{self, Export},
    storage::Store,
};

fn at(seconds: i64) -> Sample {
    Sample {
        wall: Utc.with_ymd_and_hms(2026, 10, 13, 8, 0, 0).unwrap() + Duration::seconds(seconds),
        mono_ms: (seconds.max(0) as u64) * 1000,
    }
}
fn core() -> Core {
    Core::new(vec![], Settings::default())
}
fn tasks(c: &mut Core) -> Vec<Uuid> {
    ["A", "B", "C", "D"]
        .iter()
        .map(|title| c.add_task(title, "", at(0).wall).unwrap())
        .collect()
}

#[test]
fn rotation_jump_and_pause_have_no_overlap() {
    let mut c = core();
    let ids = tasks(&mut c);
    c.start(at(0)).unwrap();
    c.select(ids[2], at(5)).unwrap();
    assert_eq!(c.pending.len(), 1);
    assert_eq!(c.pending[0].elapsed_ms, 5000);
    assert_eq!(c.pending[0].ended_at, c.active.as_ref().unwrap().started_at);
    c.advance(at(10)).unwrap();
    assert_eq!(c.selected, Some(ids[3]));
    c.advance(at(12)).unwrap();
    assert_eq!(c.selected, Some(ids[0]));
    c.tick(at(13));
    c.pause("Bathroom");
    c.advance(at(14)).unwrap();
    assert!(c.active.is_none());
    c.start(at(15)).unwrap();
    c.tick(at(20));
    assert_eq!(c.active.unwrap().elapsed_ms, 5000);
    assert!(!c.pending.iter().any(|s| s.task_id == ids[1]));
}

#[test]
fn blank_titles_empty_queue_and_archiving() {
    let mut c = core();
    assert!(c.add_task("  ", "", at(0).wall).is_err());
    assert!(c.start(at(0)).is_err());
    assert!(c.advance(at(0)).is_err());
    let ids = tasks(&mut c);
    c.start(at(0)).unwrap();
    c.archive(ids[0], at(5));
    assert!(c.active.is_none());
    assert_eq!(c.pending[0].elapsed_ms, 5000);
    assert_eq!(c.selected, Some(ids[1]));
    c.reorder(ids[3], -1, at(5).wall);
    assert_eq!(
        c.queue().iter().map(|t| t.id).collect::<Vec<_>>(),
        [ids[1], ids[3], ids[2]]
    );
}

#[test]
fn single_task_next_is_noop() {
    let mut c = core();
    c.add_task("A", "", at(0).wall).unwrap();
    c.start(at(0)).unwrap();
    c.advance(at(5)).unwrap();
    assert!(c.pending.is_empty());
    assert_eq!(c.active.unwrap().elapsed_ms, 5000);
}

#[test]
fn temporary_task_interrupts_and_returns_to_the_same_rotation_position() {
    let mut c = core();
    let ids = tasks(&mut c);
    c.select(ids[1], at(0)).unwrap();
    c.start(at(0)).unwrap();
    let temporary = c
        .add_temporary_task("  Unexpected meeting  ", at(5))
        .unwrap();
    assert_eq!(c.current().unwrap().title, "Unexpected meeting");
    assert!(c.current().unwrap().temporary);
    assert_eq!(c.active.as_ref().unwrap().task_id, temporary);
    assert_eq!(c.pending[0].task_id, ids[1]);
    assert_eq!(c.pending[0].elapsed_ms, 5000);
    assert_eq!(c.pending[0].ended_at, c.active.as_ref().unwrap().started_at);
    assert_eq!(c.next_id(), Some(ids[1]));
    c.advance(at(10)).unwrap();
    assert_eq!(
        c.tasks
            .iter()
            .find(|t| t.id == temporary)
            .unwrap()
            .completed_at,
        Some(at(10).wall)
    );
    assert_eq!(c.active.as_ref().unwrap().task_id, ids[1]);
    assert_eq!(c.pending[1].elapsed_ms, 5000);
    assert_eq!(c.pending[1].ended_at, c.active.as_ref().unwrap().started_at);
    assert_eq!(c.queue().iter().map(|t| t.id).collect::<Vec<_>>(), ids);
    c.advance(at(11)).unwrap();
    assert_eq!(c.selected, Some(ids[2]));
}

#[test]
fn temporary_task_starts_while_paused_and_completes_on_direct_switch() {
    let mut c = core();
    let ids = tasks(&mut c);
    let temporary = c.add_temporary_task("Meeting", at(0)).unwrap();
    assert!(c.active.is_some());
    c.tick(at(3));
    c.pause("Break");
    assert!(c.current().unwrap().completed_at.is_none());
    assert!(c.current().unwrap().temporary);
    c.select(ids[2], at(5)).unwrap();
    assert!(c.active.is_none());
    assert_eq!(
        c.tasks
            .iter()
            .find(|t| t.id == temporary)
            .unwrap()
            .completed_at,
        Some(at(5).wall)
    );
    assert_eq!(c.pending[0].elapsed_ms, 3000);
    assert_eq!(c.selected, Some(ids[2]));
    assert_eq!(c.next_id(), Some(ids[3]));
}

#[test]
fn single_and_consecutive_temporary_tasks_complete_without_repeating() {
    let mut c = core();
    let first = c.add_temporary_task("Meeting", at(0)).unwrap();
    let second = c.add_temporary_task("Urgent call", at(3)).unwrap();
    assert_eq!(
        c.tasks.iter().find(|t| t.id == first).unwrap().completed_at,
        Some(at(3).wall)
    );
    assert_eq!(c.current().unwrap().id, second);
    c.advance(at(7)).unwrap();
    assert!(c.current().is_none());
    assert!(c.active.is_none());
    assert!(c.queue().is_empty());
    assert_eq!(
        c.pending.iter().map(|s| s.elapsed_ms).collect::<Vec<_>>(),
        [3000, 4000]
    );
    assert!(
        c.tasks
            .iter()
            .all(|t| t.temporary && t.completed_at.is_some())
    );
}

#[test]
fn invalid_temporary_tasks_do_not_interrupt_work() {
    let mut c = core();
    tasks(&mut c);
    c.start(at(0)).unwrap();
    let session = c.active.as_ref().unwrap().id;
    assert!(c.add_temporary_task("  ", at(1)).is_err());
    assert_eq!(c.tasks.len(), 4);
    assert_eq!(c.active.as_ref().unwrap().id, session);
    let local = at(2).wall.with_timezone(&Local);
    c.settings.exclusions.push(Exclusion {
        id: Uuid::new_v4(),
        days: vec![0, 1, 2, 3, 4, 5, 6],
        start: (local - Duration::minutes(1)).time(),
        end: (local + Duration::minutes(1)).time(),
    });
    assert!(c.add_temporary_task("Meeting", at(2)).is_err());
    assert_eq!(c.tasks.len(), 4);
    assert_eq!(c.active.as_ref().unwrap().id, session);
    assert!(c.pending.is_empty());
}

#[test]
fn temporary_tasks_keep_behavior_and_history_across_restart() {
    let mut store = Store::in_memory().unwrap();
    let mut c = core();
    let ids = tasks(&mut c);
    store.save(&mut c).unwrap();
    c.select(ids[1], at(0)).unwrap();
    let temporary = c.add_temporary_task("Meeting", at(0)).unwrap();
    c.tick(at(5));
    store.save(&mut c).unwrap();
    let mut recovered = store.load().unwrap();
    assert!(recovered.active.is_none());
    assert_eq!(recovered.current().unwrap().id, temporary);
    assert!(recovered.current().unwrap().temporary);
    assert_eq!(recovered.next_id(), Some(ids[1]));
    recovered.advance(at(6)).unwrap();
    assert_eq!(recovered.selected, Some(ids[1]));
    assert!(recovered.active.is_none());
    store.save(&mut recovered).unwrap();
    let loaded = store.load().unwrap();
    assert_eq!(loaded.queue().iter().map(|t| t.id).collect::<Vec<_>>(), ids);
    let task = loaded.tasks.iter().find(|t| t.id == temporary).unwrap();
    assert_eq!(task.completed_at, Some(at(6).wall));
    let report = report::load(&store, &loaded.tasks, at(0).wall, at(60).wall).unwrap();
    assert_eq!(report.total_ms, 5000);
    assert_eq!(report.totals[0].task_id, temporary);
    let mut csv = vec![];
    report::write_csv(&report, Export::Sessions, &mut csv).unwrap();
    assert!(String::from_utf8(csv).unwrap().contains("Meeting"));
}

#[test]
fn version_two_database_keeps_completed_tasks_when_adding_temporary_support() {
    let directory = std::env::temp_dir().join(format!("workcircuit-migration-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("workcircuit.db");
    let id = Uuid::new_v4();
    {
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch(include_str!("../migrations/001_initial.sql"))
            .unwrap();
        connection
            .execute_batch(include_str!("../migrations/002_task_completion.sql"))
            .unwrap();
        connection
            .execute(
                "INSERT INTO tasks VALUES(?1,'Completed task','Keep this',0,0,?2,?2)",
                rusqlite::params![id.to_string(), at(0).wall.to_rfc3339()],
            )
            .unwrap();
    }
    {
        let mut store = Store::open(&path).unwrap();
        let c = store.load().unwrap();
        assert_eq!(c.tasks[0].id, id);
        assert_eq!(c.tasks[0].completed_at, Some(at(0).wall));
        assert!(!c.tasks[0].temporary);
        assert!(c.queue().is_empty());
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn completion_continues_rotation_without_losing_time() {
    let mut c = core();
    let ids = tasks(&mut c);
    c.select(ids[2], at(0)).unwrap();
    c.start(at(0)).unwrap();
    c.complete(ids[2], at(5)).unwrap();
    assert_eq!(c.tasks[2].completed_at, Some(at(5).wall));
    assert!(!c.tasks[2].archived);
    assert_eq!(c.selected, Some(ids[3]));
    assert_eq!(c.pending[0].elapsed_ms, 5000);
    assert_eq!(c.pending[0].ended_at, c.active.as_ref().unwrap().started_at);
    assert!(!c.pending[0].open);
    assert_eq!(c.active.as_ref().unwrap().task_id, ids[3]);
    assert_eq!(c.visit_ms, 0);
    assert!(c.select(ids[2], at(6)).is_err());
    assert!(c.complete(ids[2], at(6)).is_err());
    c.advance(at(7)).unwrap();
    assert_eq!(c.selected, Some(ids[0]));
    c.advance(at(8)).unwrap();
    assert_eq!(c.selected, Some(ids[1]));
    c.advance(at(9)).unwrap();
    assert_eq!(c.selected, Some(ids[3]));
}

#[test]
fn completing_another_task_preserves_current_session_and_pause_state() {
    let mut c = core();
    let ids = tasks(&mut c);
    c.start(at(0)).unwrap();
    let session = c.active.as_ref().unwrap().id;
    c.complete(ids[2], at(3)).unwrap();
    assert_eq!(c.selected, Some(ids[0]));
    assert_eq!(c.active.as_ref().unwrap().id, session);
    assert_eq!(c.active.as_ref().unwrap().elapsed_ms, 3000);
    assert!(c.pending.is_empty());
    c.tick(at(4));
    c.pause("Break");
    c.complete(ids[0], at(5)).unwrap();
    assert_eq!(c.selected, Some(ids[1]));
    assert!(c.active.is_none());
    assert_eq!(c.pending[0].elapsed_ms, 4000);
    assert_eq!(c.queue().len(), 2);
}

#[test]
fn completing_last_task_stops_tracking_and_reopening_preserves_identity() {
    let mut c = core();
    let id = c.add_task("Only task", "Keep this", at(0).wall).unwrap();
    c.start(at(0)).unwrap();
    c.complete(id, at(5)).unwrap();
    assert!(c.active.is_none());
    assert!(c.selected.is_none());
    assert!(c.queue().is_empty());
    assert_eq!(c.pending[0].elapsed_ms, 5000);
    assert!(c.start(at(6)).is_err());
    assert!(c.advance(at(6)).is_err());
    c.restore(id, at(7).wall);
    assert_eq!(c.selected, Some(id));
    assert!(c.active.is_none());
    assert!(c.tasks[0].completed_at.is_none());
    assert_eq!(c.tasks[0].description, "Keep this");
    assert_eq!(c.pending[0].task_id, id);
    c.start(at(8)).unwrap();
}

#[test]
fn completion_during_scheduled_break_does_not_start_next_task() {
    let mut c = core();
    let ids = tasks(&mut c);
    let start = Local
        .with_ymd_and_hms(2026, 10, 13, 11, 59, 58)
        .unwrap()
        .with_timezone(&Utc);
    let sample = |sec: u64| Sample {
        wall: start + Duration::seconds(sec as i64),
        mono_ms: sec * 1000,
    };
    c.settings.exclusions.push(Exclusion {
        id: Uuid::new_v4(),
        days: vec![0, 1, 2, 3, 4],
        start: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        end: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
    });
    c.start(sample(0)).unwrap();
    c.complete(ids[0], sample(4)).unwrap();
    assert!(c.active.is_none());
    assert_eq!(c.selected, Some(ids[1]));
    assert_eq!(c.pending[0].elapsed_ms, 2000);
    assert_eq!(c.pending[0].ended_at, sample(2).wall);
}

#[test]
fn completed_tasks_persist_and_remain_in_reports_and_exports() {
    let mut store = Store::in_memory().unwrap();
    let mut c = core();
    let ids = tasks(&mut c);
    c.start(at(0)).unwrap();
    c.complete(ids[0], at(5)).unwrap();
    store.save(&mut c).unwrap();
    let mut loaded = store.load().unwrap();
    assert_eq!(loaded.tasks[0].completed_at, Some(at(5).wall));
    assert_eq!(loaded.selected, Some(ids[1]));
    assert_eq!(loaded.queue().len(), 3);
    let report = report::load(&store, &loaded.tasks, at(0).wall, at(60).wall).unwrap();
    assert_eq!(report.total_ms, 5000);
    assert_eq!(report.totals[0].task_id, ids[0]);
    let mut csv = vec![];
    report::write_csv(&report, Export::Totals, &mut csv).unwrap();
    let row = csv::Reader::from_reader(csv.as_slice())
        .records()
        .next()
        .unwrap()
        .unwrap();
    assert_eq!(&row[1], "A");
    loaded.restore(ids[0], at(6).wall);
    assert_eq!(loaded.queue().last().unwrap().id, ids[0]);
    store.save(&mut loaded).unwrap();
    assert!(
        store
            .load()
            .unwrap()
            .tasks
            .iter()
            .all(|t| t.completed_at.is_none())
    );
}

#[test]
fn version_one_database_migrates_without_losing_tasks_or_sessions() {
    let directory = std::env::temp_dir().join(format!("workcircuit-migration-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("workcircuit.db");
    let id = Uuid::new_v4();
    {
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch(include_str!("../migrations/001_initial.sql"))
            .unwrap();
        connection
            .execute(
                "INSERT INTO tasks VALUES(?1,'Existing task','Keep history',0,0,?2)",
                rusqlite::params![
                    id.to_string(),
                    at(0)
                        .wall
                        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
                ],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO sessions VALUES(?1,?2,?3,?4,5000,0)",
                rusqlite::params![
                    Uuid::new_v4().to_string(),
                    id.to_string(),
                    at(0).wall.to_rfc3339(),
                    at(5).wall.to_rfc3339()
                ],
            )
            .unwrap();
    }
    {
        let mut store = Store::open(&path).unwrap();
        let mut c = store.load().unwrap();
        assert_eq!(c.selected, Some(id));
        assert!(c.tasks[0].completed_at.is_none());
        assert_eq!(c.tasks[0].description, "Keep history");
        c.complete(id, at(0)).unwrap();
        store.save(&mut c).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.tasks[0].completed_at, Some(at(0).wall));
        assert!(loaded.selected.is_none());
        let report = report::load(&store, &loaded.tasks, at(0).wall, at(60).wall).unwrap();
        assert_eq!(report.total_ms, 5000);
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn reminder_survives_pause_and_resets_on_switch() {
    let mut c = core();
    let ids = tasks(&mut c);
    c.settings.reminder_minutes = 1;
    c.start(at(0)).unwrap();
    for second in 1..=30 {
        assert!(!c.tick(at(second)));
    }
    c.pause("Break");
    c.start(at(50)).unwrap();
    for second in 51..80 {
        assert!(!c.tick(at(second)));
    }
    assert!(c.tick(at(80)));
    assert!(!c.tick(at(81)));
    c.select(ids[1], at(82)).unwrap();
    assert_eq!(c.visit_ms, 0);
    assert!(!c.reminder_sent);
}

#[test]
fn overnight_weekday_rules_and_half_open_end() {
    let rule = Exclusion {
        id: Uuid::new_v4(),
        days: vec![0],
        start: NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
        end: NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
    };
    let tz = FixedOffset::west_opt(3 * 3600).unwrap();
    assert!(rule.contains(&tz.with_ymd_and_hms(2026, 10, 13, 0, 30, 0).unwrap()));
    assert!(!rule.contains(&tz.with_ymd_and_hms(2026, 10, 13, 1, 0, 0).unwrap()));
    assert!(!rule.contains(&tz.with_ymd_and_hms(2026, 10, 14, 0, 30, 0).unwrap()));
}

#[test]
fn exclusions_stop_at_boundary_and_require_manual_resume() {
    let mut c = core();
    tasks(&mut c);
    let start = Local
        .with_ymd_and_hms(2026, 10, 13, 11, 59, 58)
        .unwrap()
        .with_timezone(&Utc);
    let sample = |sec: u64| Sample {
        wall: start + Duration::seconds(sec as i64),
        mono_ms: sec * 1000,
    };
    let rule = Exclusion {
        id: Uuid::new_v4(),
        days: vec![0, 1, 2, 3, 4],
        start: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        end: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
    };
    c.settings.exclusions = vec![rule.clone(), rule];
    c.start(sample(0)).unwrap();
    c.tick(sample(4));
    assert!(c.active.is_none());
    assert_eq!(c.pending[0].elapsed_ms, 2000);
    assert!(c.start(sample(5)).is_err());
    c.tick(sample(4000));
    assert!(c.active.is_none());
    c.start(sample(4001)).unwrap();
    assert!(c.active.is_some());
}

#[test]
fn sleep_gap_pauses_at_last_checkpoint_and_backward_clock_is_nonnegative() {
    let mut c = core();
    tasks(&mut c);
    c.start(at(0)).unwrap();
    c.tick(at(1));
    c.tick(Sample {
        wall: at(3600).wall,
        mono_ms: 2000,
    });
    assert!(c.active.is_none());
    assert_eq!(c.pending[0].elapsed_ms, 1000);
    c.start(at(3601)).unwrap();
    c.tick(Sample {
        wall: at(0).wall,
        mono_ms: 3_602_000,
    });
    assert_eq!(c.active.unwrap().elapsed_ms, 1000);
}

#[test]
fn transactional_persistence_recovery_and_archived_history() {
    let mut store = Store::in_memory().unwrap();
    let mut c = store.load().unwrap();
    let ids = tasks(&mut c);
    c.start(at(0)).unwrap();
    c.tick(at(5));
    store.save(&mut c).unwrap();
    let mut recovered = store.load().unwrap();
    assert!(recovered.active.is_none());
    assert!(recovered.notice.contains("Recovered"));
    assert_eq!(
        store.sessions(at(0).wall, at(60).wall).unwrap()[0].elapsed_ms,
        5000
    );
    recovered.archive(ids[0], at(10));
    store.save(&mut recovered).unwrap();
    let loaded = store.load().unwrap();
    assert!(
        loaded
            .tasks
            .iter()
            .find(|t| t.id == ids[0])
            .unwrap()
            .archived
    );
    assert_eq!(loaded.settings.reminder_minutes, 0);
}

#[test]
fn sqlite_failure_does_not_clear_pending_or_partially_commit() {
    let mut store = Store::in_memory().unwrap();
    let mut c = core();
    tasks(&mut c);
    store.save(&mut c).unwrap();
    c.start(at(0)).unwrap();
    c.tick(at(5));
    c.pause("Pause");
    c.tasks[0].title = "".into();
    assert!(store.save(&mut c).is_err());
    assert_eq!(c.pending.len(), 1);
    assert!(store.sessions(at(0).wall, at(60).wall).unwrap().is_empty());
    assert_eq!(store.load().unwrap().tasks[0].title, "A");
}

#[test]
fn reports_clip_midnight_and_csv_roundtrips_special_cells() {
    let mut c = core();
    let id = c
        .add_task("=A,\"B\"\nC", "description,\nquoted", at(0).wall)
        .unwrap();
    let tz = FixedOffset::west_opt(3 * 3600).unwrap();
    let start = tz
        .with_ymd_and_hms(2026, 10, 12, 23, 30, 0)
        .unwrap()
        .with_timezone(&Utc);
    let session = Session {
        id: Uuid::new_v4(),
        task_id: id,
        started_at: start,
        ended_at: start + Duration::hours(1),
        elapsed_ms: 3_600_000,
        open: false,
    };
    let date = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
    let (from, to) = report::date_range(date, date, &tz).unwrap();
    let report = report::aggregate(vec![session], &c.tasks, from, to);
    assert_eq!(report.total_ms, 1_800_000);
    let mut csv = vec![];
    report::write_csv(&report, Export::Totals, &mut csv).unwrap();
    let mut reader = csv::Reader::from_reader(csv.as_slice());
    let row = reader.records().next().unwrap().unwrap();
    assert_eq!(&row[1], "'=A,\"B\"\nC");
    assert_eq!(&row[2], "description,\nquoted");
    assert!(report::date_range(date, date.pred_opt().unwrap(), &tz).is_err());
}

#[test]
fn disk_restart_restores_settings_order_and_selection() {
    let directory = std::env::temp_dir().join(format!("workcircuit-test-{}", Uuid::new_v4()));
    let path = directory.join("workcircuit.db");
    let selected;
    {
        let mut store = Store::open(&path).unwrap();
        let mut c = store.load().unwrap();
        let ids = tasks(&mut c);
        selected = ids[2];
        c.reorder(selected, -1, at(0).wall);
        c.select(selected, at(0)).unwrap();
        c.settings.reminder_minutes = 25;
        c.settings.language = workcircuit::i18n::Language::PortugueseBrazil;
        c.settings.exclusions.push(Exclusion {
            id: Uuid::new_v4(),
            days: vec![0, 1],
            start: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
        });
        store.save(&mut c).unwrap();
    }
    {
        let mut store = Store::open(&path).unwrap();
        let mut c = store.load().unwrap();
        assert_eq!(c.selected, Some(selected));
        assert_eq!(c.queue()[1].id, selected);
        assert_eq!(c.settings.reminder_minutes, 25);
        assert_eq!(
            c.settings.language,
            workcircuit::i18n::Language::PortugueseBrazil
        );
        assert_eq!(c.settings.exclusions.len(), 1);
        c.archive(selected, at(5));
        c.restore(selected, at(6).wall);
        assert_eq!(c.queue().last().unwrap().id, selected);
        store.save(&mut c).unwrap();
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn date_ranges_respect_offsets_and_nonpositive_wall_sessions_keep_measured_time() {
    let date = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
    let tz = FixedOffset::east_opt(5 * 3600 + 30 * 60).unwrap();
    let (start, end) = report::date_range(date, date, &tz).unwrap();
    assert_eq!(end - start, Duration::days(1));
    assert_eq!(
        start,
        tz.with_ymd_and_hms(2026, 10, 13, 0, 0, 0)
            .unwrap()
            .with_timezone(&Utc)
    );
    let s = Session {
        id: Uuid::new_v4(),
        task_id: Uuid::new_v4(),
        started_at: start + Duration::hours(1),
        ended_at: start,
        elapsed_ms: 1000,
        open: false,
    };
    assert_eq!(report::contribution(&s, start, end), 1000);
    assert_eq!(report::contribution(&s, end, end + Duration::days(1)), 0);
}

#[test]
fn language_defaults_to_english_and_legacy_preferences_still_load() {
    use workcircuit::i18n::Language;
    let legacy: Settings =
        serde_json::from_str(r#"{"exclusions":[],"reminder_minutes":25}"#).unwrap();
    assert_eq!(legacy.language, Language::English);
    assert_eq!(Settings::default().language, Language::English);
    assert_eq!(Language::English.text("Start"), "Start");
    assert_eq!(Language::PortugueseBrazil.text("Start"), "Iniciar");
    assert_eq!(
        Language::PortugueseBrazil.text("A user's task"),
        "A user's task"
    );
}
