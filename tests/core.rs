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
