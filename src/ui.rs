use adw::prelude::*;
use anyhow::Result;
use chrono::{Local, NaiveDate, NaiveTime, Utc};
use gtk::{gio, glib};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc,
    time::{Duration, Instant},
};
use uuid::Uuid;
use workcircuit::{
    domain::{Core, Exclusion, Sample, Settings, Task, format_duration},
    i18n::Language,
    report::{self, Export, Report},
    storage::{Store, data_path},
};

struct View {
    window: gtk::ApplicationWindow,
    title: gtk::Label,
    timer: gtk::Label,
    status: gtk::Label,
    next_label: gtk::Label,
    toggle: gtk::Button,
    next: gtk::Button,
    tasks: gtk::ListBox,
    compact: bool,
}

struct Desktop {
    app: adw::Application,
    core: RefCell<Core>,
    store: RefCell<Store>,
    origin: Instant,
    dashboard: RefCell<Option<View>>,
    overlay: RefCell<Option<View>>,
    checkpoint: Cell<u64>,
    write_failed: Cell<bool>,
    overlay_display: RefCell<Option<gtk::gdk::Display>>,
}

pub fn activate(app: &adw::Application) {
    gtk::Window::set_default_icon_name("io.github.workcircuit.WorkCircuit-chevron-symbolic");
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("../data/style.css"));
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    if let Some(window) = app.active_window() {
        window.present();
        return;
    }
    let loaded = (|| -> Result<_> {
        let mut store = Store::open(&data_path()?)?;
        let core = store.load()?;
        Ok((store, core))
    })();
    match loaded {
        Ok((store, core)) => {
            let desktop = Desktop::new(app, store, core);
            let quit = gio::SimpleAction::new("quit", None);
            let application = app.clone();
            let d = desktop.clone();
            quit.connect_activate(move |_, _| match d.stop_on_exit() {
                Ok(()) => application.quit(),
                Err(error) => d.error(error),
            });
            app.add_action(&quit);
            app.set_accels_for_action("app.quit", &["<Primary>q"]);
            desktop.open_dashboard();
            desktop.install_timer();
            let d = desktop.clone();
            app.connect_shutdown(move |_| {
                if let Err(error) = d.stop_on_exit() {
                    eprintln!("Could not save final checkpoint: {error}");
                }
            });
        }
        Err(error) => {
            let window = adw::ApplicationWindow::builder()
                .application(app)
                .title("WorkCircuit")
                .default_width(480)
                .default_height(240)
                .build();
            let page = adw::StatusPage::builder()
                .title("Cannot open local data")
                .description(error.to_string())
                .icon_name("dialog-error-symbolic")
                .build();
            window.set_content(Some(&page));
            window.present();
        }
    }
}

impl Desktop {
    fn new(app: &adw::Application, store: Store, core: Core) -> Rc<Self> {
        Rc::new(Self {
            app: app.clone(),
            core: RefCell::new(core),
            store: RefCell::new(store),
            origin: Instant::now(),
            dashboard: RefCell::new(None),
            overlay: RefCell::new(None),
            checkpoint: Cell::new(0),
            write_failed: Cell::new(false),
            overlay_display: RefCell::new(None),
        })
    }

    fn sample(&self) -> Sample {
        Sample {
            wall: Utc::now(),
            mono_ms: self.origin.elapsed().as_millis() as u64,
        }
    }

    fn parent(&self) -> Option<gtk::ApplicationWindow> {
        self.dashboard
            .borrow()
            .as_ref()
            .map(|v| v.window.clone())
            .or_else(|| self.overlay.borrow().as_ref().map(|v| v.window.clone()))
    }

    fn error(&self, error: impl std::fmt::Display) {
        let lang = self.core.borrow().settings.language;
        let dialog = adw::MessageDialog::builder()
            .heading(lang.text("Could not complete action"))
            .body(lang.text(&error.to_string()))
            .build();
        if let Some(parent) = self.parent() {
            dialog.set_transient_for(Some(&parent));
        }
        dialog.add_response("close", lang.text("Close"));
        dialog.set_close_response("close");
        dialog.present();
    }

    fn apply(self: &Rc<Self>, action: impl FnOnce(&mut Core, Sample) -> Result<()>) -> bool {
        let mut candidate = self.core.borrow().clone();
        let now = self.sample();
        let result =
            action(&mut candidate, now).and_then(|_| self.store.borrow_mut().save(&mut candidate));
        match result {
            Ok(()) => {
                let notify = !self.core.borrow().reminder_sent && candidate.reminder_sent;
                let language_changed =
                    self.core.borrow().settings.language != candidate.settings.language;
                *self.core.borrow_mut() = candidate;
                if notify {
                    self.notify();
                }
                self.write_failed.set(false);
                self.checkpoint.set(now.mono_ms);
                if language_changed {
                    self.rebuild_views();
                } else {
                    self.refresh(true);
                }
                true
            }
            Err(error) => {
                self.error(error);
                false
            }
        }
    }

    fn notify(&self) {
        let lang = self.core.borrow().settings.language;
        let notification = gio::Notification::new(lang.text("Check your next task"));
        notification.set_body(Some(
            lang.text("You've reached your reminder time. Switch when you're ready."),
        ));
        self.app
            .send_notification(Some("task-reminder"), &notification);
    }

    fn toggle(self: &Rc<Self>) {
        self.apply(|core, now| {
            core.tick(now);
            if core.active.is_some() {
                core.pause("Paused — resume when ready");
                Ok(())
            } else {
                core.start(now)
            }
        });
    }

    fn stop_on_exit(&self) -> Result<()> {
        let mut core = self.core.borrow().clone();
        core.tick(self.sample());
        core.pause("Session ended");
        self.store.borrow_mut().save(&mut core)?;
        *self.core.borrow_mut() = core;
        Ok(())
    }

    fn install_timer(self: &Rc<Self>) {
        let (sender, receiver) = mpsc::channel();
        let origin = self.origin;
        std::thread::spawn(move || {
            let listen = || -> Result<()> {
                let connection = zbus::blocking::Connection::system()?;
                let proxy = zbus::blocking::Proxy::new(
                    &connection,
                    "org.freedesktop.login1",
                    "/org/freedesktop/login1",
                    "org.freedesktop.login1.Manager",
                )?;
                for message in proxy.receive_signal("PrepareForSleep")? {
                    if let Ok((true,)) = message.body().deserialize::<(bool,)>() {
                        let sample = Sample {
                            wall: Utc::now(),
                            mono_ms: origin.elapsed().as_millis() as u64,
                        };
                        if sender.send(sample).is_err() {
                            break;
                        }
                    }
                }
                Ok(())
            };
            if let Err(error) = listen() {
                eprintln!("Suspend signal unavailable; clock-gap fallback active: {error}");
            }
        });
        let weak = Rc::downgrade(self);
        glib::timeout_add_local(Duration::from_secs(1), move || {
            let Some(d) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let now = d.sample();
            let mut core = d.core.borrow_mut();
            let was_running = core.active.is_some();
            for sample in receiver.try_iter() {
                core.tick(sample);
                core.pause("Paused for system sleep");
            }
            let notify = core.tick(now);
            let transitioned = was_running != core.active.is_some();
            if transitioned
                || !core.pending.is_empty()
                || (core.active.is_some() && now.mono_ms.saturating_sub(d.checkpoint.get()) >= 5000)
            {
                match d.store.borrow_mut().save(&mut core) {
                    Ok(()) => {
                        d.checkpoint.set(now.mono_ms);
                        d.write_failed.set(false);
                    }
                    Err(error) => {
                        core.pause("Storage error — tracking paused");
                        core.notice = format!("Storage error — tracking paused: {error}");
                        d.write_failed.set(true);
                    }
                }
            }
            drop(core);
            if notify {
                d.notify();
            }
            d.refresh(transitioned);
            glib::ControlFlow::Continue
        });
    }

    fn open_dashboard(self: &Rc<Self>) {
        if let Some(view) = self.dashboard.borrow().as_ref() {
            view.window.present();
            return;
        }
        let view = self.build_dashboard();
        view.window.present();
        *self.dashboard.borrow_mut() = Some(view);
        self.refresh(true);
    }

    fn open_overlay(self: &Rc<Self>) {
        if let Some(view) = self.overlay.borrow().as_ref() {
            view.window.present();
            return;
        }
        if self.overlay_display.borrow().is_none() {
            match crate::overlay::display() {
                Ok(display) => {
                    let provider = gtk::CssProvider::new();
                    provider.load_from_string(include_str!("../data/style.css"));
                    gtk::style_context_add_provider_for_display(
                        &display,
                        &provider,
                        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                    );
                    *self.overlay_display.borrow_mut() = Some(display);
                }
                Err(error) => {
                    self.error(error);
                    return;
                }
            }
        }
        let view = self.build_overlay();
        view.window
            .set_display(self.overlay_display.borrow().as_ref().unwrap());
        let d = self.clone();
        view.window.connect_realize(move |window| {
            if let Err(error) = crate::overlay::request_above(window, true) {
                d.error(error);
            }
        });
        let d = self.clone();
        view.window.connect_map(move |window| {
            if let Err(error) = crate::overlay::request_above(window, false) {
                d.error(error);
            }
        });
        view.window.present();
        *self.overlay.borrow_mut() = Some(view);
        self.refresh(true);
    }

    fn build_dashboard(self: &Rc<Self>) -> View {
        let lang = self.core.borrow().settings.language;
        let window = adw::ApplicationWindow::builder()
            .application(&self.app)
            .title("WorkCircuit")
            .default_width(820)
            .default_height(680)
            .build();
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let header = adw::HeaderBar::new();
        let name = adw::WindowTitle::new("WorkCircuit", lang.text("Your work, in rotation"));
        header.set_title_widget(Some(&name));
        let settings = icon_button("emblem-system-symbolic", lang.text("Settings"));
        let d = self.clone();
        settings.connect_clicked(move |_| d.settings_dialog());
        header.pack_start(&settings);
        let overlay = gtk::Button::with_label(lang.text("Overlay"));
        let d = self.clone();
        overlay.connect_clicked(move |_| d.open_overlay());
        header.pack_end(&overlay);
        let reports = gtk::Button::with_label(lang.text("Reports"));
        let d = self.clone();
        reports.connect_clicked(move |_| d.reports_dialog());
        header.pack_end(&reports);
        let menu = gio::Menu::new();
        menu.append(Some(lang.text("Quit")), Some("app.quit"));
        let button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&menu)
            .build();
        header.pack_end(&button);
        root.append(&header);
        let body = gtk::Box::new(gtk::Orientation::Vertical, 20);
        margins(&body, 28);
        let kicker = gtk::Label::new(Some(lang.text("CURRENT TASK")));
        kicker.add_css_class("caption");
        kicker.add_css_class("dim-label");
        body.append(&kicker);
        let title = gtk::Label::new(None);
        title.add_css_class("title-1");
        title.set_wrap(true);
        title.set_selectable(true);
        body.append(&title);
        let timer = gtk::Label::new(Some("00:00:00"));
        timer.add_css_class("workcircuit-timer");
        body.append(&timer);
        let status = gtk::Label::new(None);
        status.set_wrap(true);
        status.add_css_class("dim-label");
        body.append(&status);
        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        controls.set_halign(gtk::Align::Center);
        let toggle = gtk::Button::with_label(lang.text("Start"));
        toggle.add_css_class("suggested-action");
        toggle.set_width_request(110);
        let next = gtk::Button::with_label(lang.text("Next →"));
        next.set_width_request(110);
        let d = self.clone();
        toggle.connect_clicked(move |_| d.toggle());
        let d = self.clone();
        next.connect_clicked(move |_| {
            d.apply(|core, now| core.advance(now));
        });
        controls.append(&toggle);
        controls.append(&next);
        body.append(&controls);
        let next_label = gtk::Label::new(None);
        next_label.add_css_class("dim-label");
        next_label.set_wrap(true);
        body.append(&next_label);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let heading = gtk::Label::new(Some(lang.text("Task rotation")));
        heading.add_css_class("title-3");
        heading.set_hexpand(true);
        heading.set_xalign(0.0);
        row.append(&heading);
        let add = gtk::Button::with_label(lang.text("Add Task"));
        let d = self.clone();
        add.connect_clicked(move |_| d.task_dialog(None));
        row.append(&add);
        body.append(&row);
        let tasks = gtk::ListBox::new();
        tasks.set_selection_mode(gtk::SelectionMode::None);
        tasks.add_css_class("boxed-list");
        let scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .min_content_height(160)
            .child(&tasks)
            .build();
        body.append(&scroll);
        root.append(&body);
        body.set_vexpand(true);
        window.set_content(Some(&root));
        self.connect_close(window.upcast_ref(), false);
        View {
            window: window.upcast(),
            title,
            timer,
            status,
            next_label,
            toggle,
            next,
            tasks,
            compact: false,
        }
    }

    fn build_overlay(self: &Rc<Self>) -> View {
        let lang = self.core.borrow().settings.language;
        let window = gtk::ApplicationWindow::builder()
            .application(&self.app)
            .title("WorkCircuit · Overlay")
            .decorated(false)
            .default_width(300)
            .default_height(140)
            .resizable(false)
            .build();
        window.add_css_class("workcircuit-overlay");
        let body = gtk::Box::new(gtk::Orientation::Vertical, 4);
        margins(&body, 10);
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        let title = gtk::Label::new(None);
        title.set_xalign(0.0);
        title.set_hexpand(true);
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title.set_width_chars(8);
        title.set_max_width_chars(22);
        title.add_css_class("heading");
        header.append(&title);
        let tasks = gtk::ListBox::new();
        tasks.set_selection_mode(gtk::SelectionMode::None);
        tasks.add_css_class("boxed-list");
        let scroll = gtk::ScrolledWindow::builder()
            .child(&tasks)
            .min_content_height(120)
            .max_content_height(300)
            .propagate_natural_height(true)
            .min_content_width(260)
            .width_request(260)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let popover = gtk::Popover::new();
        popover.set_size_request(280, -1);
        popover.set_child(Some(&scroll));
        let picker = gtk::MenuButton::builder()
            .icon_name("view-list-symbolic")
            .popover(&popover)
            .tooltip_text(lang.text("Choose task"))
            .build();
        picker.add_css_class("flat");
        header.append(&picker);
        let dashboard = icon_button("view-restore-symbolic", lang.text("Open dashboard"));
        let d = self.clone();
        dashboard.connect_clicked(move |_| d.open_dashboard());
        header.append(&dashboard);
        let close = icon_button("window-close-symbolic", lang.text("Close overlay"));
        let weak = window.downgrade();
        close.connect_clicked(move |_| {
            if let Some(window) = weak.upgrade() {
                window.close();
            }
        });
        header.append(&close);
        let handle = gtk::WindowHandle::new();
        handle.set_child(Some(&header));
        body.append(&handle);
        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let timer = gtk::Label::new(Some("00:00:00"));
        timer.add_css_class("workcircuit-overlay-timer");
        timer.set_hexpand(true);
        timer.set_xalign(0.0);
        controls.append(&timer);
        let toggle = gtk::Button::with_label(lang.text("Start"));
        toggle.add_css_class("suggested-action");
        let d = self.clone();
        toggle.connect_clicked(move |_| d.toggle());
        controls.append(&toggle);
        let next = gtk::Button::with_label(lang.text("Next →"));
        let d = self.clone();
        next.connect_clicked(move |_| {
            d.apply(|core, now| core.advance(now));
        });
        controls.append(&next);
        body.append(&controls);
        let status = gtk::Label::new(None);
        status.set_xalign(0.0);
        status.set_ellipsize(gtk::pango::EllipsizeMode::End);
        status.add_css_class("caption");
        status.add_css_class("dim-label");
        body.append(&status);
        let next_label = gtk::Label::new(None);
        next_label.set_xalign(0.0);
        next_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        next_label.add_css_class("caption");
        next_label.add_css_class("dim-label");
        body.append(&next_label);
        window.set_child(Some(&body));
        self.connect_close(&window, true);
        View {
            window,
            title,
            timer,
            status,
            next_label,
            toggle,
            next,
            tasks,
            compact: true,
        }
    }

    fn connect_close(self: &Rc<Self>, window: &gtk::ApplicationWindow, compact: bool) {
        let d = self.clone();
        window.connect_close_request(move |window| {
            let current = if compact {
                d.overlay.borrow().as_ref().map(|v| v.window.clone())
            } else {
                d.dashboard.borrow().as_ref().map(|v| v.window.clone())
            };
            // Replaced windows must not clear their replacements or stop tracking.
            if current.as_ref() != Some(window) {
                return glib::Propagation::Proceed;
            }
            let last_companion = if compact {
                d.dashboard.borrow().is_none()
            } else {
                d.overlay.borrow().is_none()
            };
            if last_companion && let Err(error) = d.stop_on_exit() {
                d.error(error);
                return glib::Propagation::Stop;
            }
            if compact {
                d.overlay.borrow_mut().take();
            } else {
                d.dashboard.borrow_mut().take();
            }
            glib::Propagation::Proceed
        });
    }

    fn rebuild_views(self: &Rc<Self>) {
        let dashboard = self.dashboard.borrow_mut().take();
        let overlay = self.overlay.borrow_mut().take();
        if dashboard.is_some() {
            self.open_dashboard();
        }
        if overlay.is_some() {
            self.open_overlay();
        }
        if let Some(view) = dashboard {
            view.window.close();
        }
        if let Some(view) = overlay {
            view.window.close();
        }
    }

    fn refresh(self: &Rc<Self>, rebuild: bool) {
        let lang = self.core.borrow().settings.language;
        let core = self.core.borrow();
        let current = core
            .current()
            .map_or(lang.text("Add a task to get started"), |t| t.title.as_str());
        let blocked = core.blocked(Utc::now());
        let notice = if let Some(detail) = core
            .notice
            .strip_prefix("Storage error — tracking paused: ")
        {
            format!("{}: {detail}", lang.text("Storage error — tracking paused"))
        } else {
            lang.text(&core.notice).to_owned()
        };
        let next_title = core
            .next_id()
            .and_then(|id| core.tasks.iter().find(|t| t.id == id))
            .map_or("—", |t| t.title.as_str());
        for view in [self.dashboard.borrow(), self.overlay.borrow()]
            .iter()
            .filter_map(|v| v.as_ref())
        {
            view.title.set_label(current);
            view.title.set_tooltip_text(Some(current));
            view.timer.set_label(&format_duration(core.visit_ms));
            view.status.set_label(if blocked {
                lang.text("Scheduled break · resume afterwards")
            } else {
                &notice
            });
            view.next_label
                .set_label(&format!("{} {next_title}", lang.text("Up next:")));
            view.toggle.set_label(if core.active.is_some() {
                lang.text("Pause")
            } else if core.visit_ms > 0 {
                lang.text("Resume")
            } else {
                lang.text("Start")
            });
            view.toggle
                .set_sensitive(core.current().is_some() && !blocked && !self.write_failed.get());
            view.next
                .set_sensitive(core.current().is_some() && !self.write_failed.get());
            if rebuild {
                while let Some(child) = view.tasks.first_child() {
                    view.tasks.remove(&child);
                }
                let compact = view.compact;
                if core.queue().is_empty() {
                    let empty = gtk::Label::new(Some(
                        lang.text("No active tasks yet. Add your first task in the dashboard."),
                    ));
                    empty.set_wrap(true);
                    margins(&empty, 20);
                    view.tasks.append(&empty);
                }
                for task in core.queue() {
                    let selected = Some(task.id) == core.selected;
                    let row = adw::ActionRow::new();
                    // ActionRow titles are markup by default; task text must remain literal.
                    row.set_use_markup(false);
                    row.set_title(&task.title);
                    if !compact && !task.description.is_empty() {
                        row.set_subtitle(&task.description);
                        row.set_subtitle_lines(2);
                    }
                    if selected {
                        row.add_css_class("workcircuit-selected");
                    }
                    let choose = icon_button(
                        if selected {
                            "media-record-symbolic"
                        } else {
                            "media-playback-start-symbolic"
                        },
                        lang.text("Select task"),
                    );
                    let d = self.clone();
                    let id = task.id;
                    choose.connect_clicked(move |button| {
                        let popover = button
                            .ancestor(gtk::Popover::static_type())
                            .and_then(|p| p.downcast::<gtk::Popover>().ok());
                        d.apply(|core, now| core.select(id, now));
                        if let Some(popover) = popover {
                            popover.popdown();
                        }
                    });
                    row.add_prefix(&choose);
                    if compact {
                        row.set_title_lines(1);
                        row.set_tooltip_text(Some(&task.title));
                        row.set_activatable_widget(Some(&choose));
                    }
                    if !compact {
                        for (icon, tip, dir) in [
                            ("go-up-symbolic", lang.text("Move up"), -1),
                            ("go-down-symbolic", lang.text("Move down"), 1),
                        ] {
                            let button = icon_button(icon, tip);
                            let d = self.clone();
                            button.connect_clicked(move |_| {
                                d.apply(|core, now| {
                                    core.reorder(id, dir, now.wall);
                                    Ok(())
                                });
                            });
                            row.add_suffix(&button);
                        }
                        let edit = icon_button("document-edit-symbolic", lang.text("Edit task"));
                        let d = self.clone();
                        let task = task.clone();
                        edit.connect_clicked(move |_| d.task_dialog(Some(task.clone())));
                        row.add_suffix(&edit);
                        let archive = icon_button(
                            "user-trash-symbolic",
                            lang.text("Archive task (preserves history)"),
                        );
                        let d = self.clone();
                        archive.connect_clicked(move |_| {
                            d.apply(|core, now| {
                                core.archive(id, now);
                                Ok(())
                            });
                        });
                        row.add_suffix(&archive);
                    }
                    view.tasks.append(&row);
                }
                if !compact && core.tasks.iter().any(|t| t.archived) {
                    let archived = adw::ExpanderRow::builder()
                        .title(lang.text("Archived tasks"))
                        .build();
                    for task in core.tasks.iter().filter(|t| t.archived) {
                        let row = adw::ActionRow::new();
                        row.set_use_markup(false);
                        row.set_title(&task.title);
                        let restore = gtk::Button::with_label(lang.text("Restore"));
                        restore.set_valign(gtk::Align::Center);
                        let d = self.clone();
                        let id = task.id;
                        restore.connect_clicked(move |_| {
                            d.apply(|core, now| {
                                core.restore(id, now.wall);
                                Ok(())
                            });
                        });
                        row.add_suffix(&restore);
                        archived.add_row(&row);
                    }
                    view.tasks.append(&archived);
                }
            }
        }
    }

    fn task_dialog(self: &Rc<Self>, task: Option<Task>) {
        let lang = self.core.borrow().settings.language;
        let dialog = self.editor_window(
            if task.is_some() {
                lang.text("Edit Task")
            } else {
                lang.text("Add Task")
            },
            480,
            320,
        );
        let body = gtk::Box::new(gtk::Orientation::Vertical, 16);
        margins(&body, 24);
        let title = gtk::Entry::builder()
            .placeholder_text(lang.text("Task title (required)"))
            .text(task.as_ref().map_or("", |t| t.title.as_str()))
            .build();
        title.set_activates_default(true);
        let description = gtk::TextView::new();
        description.set_wrap_mode(gtk::WrapMode::WordChar);
        description
            .buffer()
            .set_text(task.as_ref().map_or("", |t| t.description.as_str()));
        let label = gtk::Label::new(Some(lang.text("Description (optional)")));
        label.set_xalign(0.0);
        let scroll = gtk::ScrolledWindow::builder()
            .min_content_height(100)
            .child(&description)
            .build();
        scroll.add_css_class("card");
        let save = gtk::Button::with_label(lang.text("Save Task"));
        save.add_css_class("suggested-action");
        save.set_sensitive(!title.text().trim().is_empty());
        let save_clone = save.clone();
        title.connect_changed(move |entry| {
            save_clone.set_sensitive(!entry.text().trim().is_empty())
        });
        body.append(&title);
        body.append(&label);
        body.append(&scroll);
        body.append(&save);
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.append(&adw::HeaderBar::new());
        root.append(&body);
        dialog.set_content(Some(&root));
        dialog.set_default_widget(Some(&save));
        let d = self.clone();
        let w = dialog.clone();
        save.connect_clicked(move |_| {
            let buffer = description.buffer();
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
            let ok = d.apply(|core, now| match &task {
                Some(task) => core.edit_task(task.id, &title.text(), &text, now.wall),
                None => core.add_task(&title.text(), &text, now.wall).map(|_| ()),
            });
            if ok {
                w.close();
            }
        });
        dialog.present();
    }

    fn editor_window(&self, title: &str, width: i32, height: i32) -> adw::Window {
        let window = adw::Window::builder()
            .title(title)
            .modal(true)
            .default_width(width)
            .default_height(height)
            .build();
        if let Some(parent) = self.parent() {
            window.set_transient_for(Some(&parent));
            window.set_destroy_with_parent(true);
        }
        window
    }

    fn settings_dialog(self: &Rc<Self>) {
        let lang = self.core.borrow().settings.language;
        let dialog = self.editor_window(lang.text("Settings"), 560, 560);
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.append(&adw::HeaderBar::new());
        let body = gtk::Box::new(gtk::Orientation::Vertical, 16);
        margins(&body, 24);
        let group = adw::PreferencesGroup::builder()
            .title(lang.text("Language"))
            .description(lang.text("Choose your interface language. English is the default."))
            .build();
        let language = gtk::DropDown::from_strings(&["English", "Português (Brasil)"]);
        language.set_selected(if lang == Language::PortugueseBrazil {
            1
        } else {
            0
        });
        language.set_valign(gtk::Align::Center);
        let row = adw::ActionRow::builder()
            .title(lang.text("Language"))
            .build();
        row.add_suffix(&language);
        group.add(&row);
        body.append(&group);
        let group = adw::PreferencesGroup::builder()
            .title(lang.text("Reminder"))
            .description(lang.text(
                "Minutes per task visit. 0 disables reminders. Breaks preserve elapsed work.",
            ))
            .build();
        let reminder = gtk::SpinButton::with_range(0.0, 1440.0, 1.0);
        reminder.set_value(f64::from(self.core.borrow().settings.reminder_minutes));
        let row = adw::ActionRow::builder()
            .title(lang.text("Remind after (minutes)"))
            .build();
        row.add_suffix(&reminder);
        group.add(&row);
        body.append(&group);
        let group=adw::PreferencesGroup::builder().title(lang.text("Scheduled breaks")).description(lang.text("Local computer time. Overnight ranges belong to their starting weekday. Resume manually afterwards.")).build();
        let list = gtk::Box::new(gtk::Orientation::Vertical, 8);
        group.add(&list);
        body.append(&group);
        let rules = Rc::new(RefCell::new(self.core.borrow().settings.exclusions.clone()));
        self.render_rules(&list, &rules);
        let add = gtk::Button::with_label(lang.text("Add Scheduled Break"));
        let d = self.clone();
        let l = list.clone();
        let r = rules.clone();
        let parent = dialog.clone();
        add.connect_clicked(move |_| d.break_dialog(&parent, &l, &r));
        body.append(&add);
        let save = gtk::Button::with_label(lang.text("Save Settings"));
        save.add_css_class("suggested-action");
        body.append(&save);
        let d = self.clone();
        let w = dialog.clone();
        save.connect_clicked(move |_| {
            let settings = Settings {
                exclusions: rules.borrow().clone(),
                reminder_minutes: reminder.value_as_int() as u32,
                language: if language.selected() == 1 {
                    Language::PortugueseBrazil
                } else {
                    Language::English
                },
            };
            if d.apply(|core, now| core.set_settings(settings, now)) {
                w.close();
            }
        });
        let scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&body)
            .build();
        root.append(&scroll);
        dialog.set_content(Some(&root));
        dialog.present();
    }

    fn render_rules(self: &Rc<Self>, list: &gtk::Box, rules: &Rc<RefCell<Vec<Exclusion>>>) {
        let lang = self.core.borrow().settings.language;
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }
        if rules.borrow().is_empty() {
            list.append(&gtk::Label::new(Some(lang.text("No scheduled breaks"))));
        }
        for rule in rules.borrow().iter() {
            let days = [
                lang.text("Mon"),
                lang.text("Tue"),
                lang.text("Wed"),
                lang.text("Thu"),
                lang.text("Fri"),
                lang.text("Sat"),
                lang.text("Sun"),
            ];
            let text = format!(
                "{}  {}–{}",
                rule.days
                    .iter()
                    .map(|d| days[*d as usize])
                    .collect::<Vec<_>>()
                    .join(", "),
                rule.start.format("%H:%M"),
                rule.end.format("%H:%M")
            );
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let label = gtk::Label::new(Some(&text));
            label.set_wrap(true);
            label.set_hexpand(true);
            label.set_xalign(0.0);
            row.append(&label);
            let remove = icon_button("list-remove-symbolic", lang.text("Remove scheduled break"));
            let d = self.clone();
            let r = rules.clone();
            let l = list.clone();
            let id = rule.id;
            remove.connect_clicked(move |_| {
                r.borrow_mut().retain(|r| r.id != id);
                d.render_rules(&l, &r);
            });
            row.append(&remove);
            list.append(&row);
        }
    }

    fn break_dialog(
        self: &Rc<Self>,
        parent: &adw::Window,
        list: &gtk::Box,
        rules: &Rc<RefCell<Vec<Exclusion>>>,
    ) {
        let lang = self.core.borrow().settings.language;
        let dialog = adw::Window::builder()
            .title(lang.text("Scheduled Break"))
            .transient_for(parent)
            .modal(true)
            .default_width(460)
            .build();
        let body = gtk::Box::new(gtk::Orientation::Vertical, 16);
        margins(&body, 24);
        let days_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let mut days = vec![];
        for (i, name) in [
            lang.text("Mon"),
            lang.text("Tue"),
            lang.text("Wed"),
            lang.text("Thu"),
            lang.text("Fri"),
            lang.text("Sat"),
            lang.text("Sun"),
        ]
        .iter()
        .enumerate()
        {
            let button = gtk::ToggleButton::with_label(name);
            button.set_active(i < 5);
            days_box.append(&button);
            days.push(button);
        }
        body.append(&days_box);
        let start = gtk::Entry::builder()
            .text("12:00")
            .placeholder_text(lang.text("Start HH:MM"))
            .build();
        let end = gtk::Entry::builder()
            .text("13:00")
            .placeholder_text(lang.text("End HH:MM"))
            .build();
        body.append(&gtk::Label::new(Some(
            lang.text("Start / end in 24-hour time"),
        )));
        body.append(&start);
        body.append(&end);
        let error = gtk::Label::new(None);
        error.add_css_class("error");
        error.set_wrap(true);
        body.append(&error);
        let save = gtk::Button::with_label(lang.text("Add Break"));
        save.add_css_class("suggested-action");
        body.append(&save);
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.append(&adw::HeaderBar::new());
        root.append(&body);
        dialog.set_content(Some(&root));
        let d = self.clone();
        let l = list.clone();
        let r = rules.clone();
        let w = dialog.clone();
        save.connect_clicked(move |_| {
            let parse = || -> Result<Exclusion> {
                let rule = Exclusion {
                    id: Uuid::new_v4(),
                    days: days
                        .iter()
                        .enumerate()
                        .filter(|(_, b)| b.is_active())
                        .map(|(i, _)| i as u32)
                        .collect(),
                    start: NaiveTime::parse_from_str(&start.text(), "%H:%M")?,
                    end: NaiveTime::parse_from_str(&end.text(), "%H:%M")?,
                };
                rule.validate()?;
                Ok(rule)
            };
            match parse() {
                Ok(rule) => {
                    r.borrow_mut().push(rule);
                    d.render_rules(&l, &r);
                    w.close();
                }
                Err(e) => error.set_label(lang.text(&e.to_string())),
            }
        });
        dialog.present();
    }

    fn reports_dialog(self: &Rc<Self>) {
        let lang = self.core.borrow().settings.language;
        let dialog = self.editor_window(lang.text("Reports"), 760, 620);
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.append(&adw::HeaderBar::new());
        let body = gtk::Box::new(gtk::Orientation::Vertical, 16);
        margins(&body, 24);
        let filters = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let today = Local::now().date_naive().to_string();
        let from = gtk::Entry::builder()
            .text(&today)
            .placeholder_text(lang.text("From YYYY-MM-DD"))
            .hexpand(true)
            .build();
        let through = gtk::Entry::builder()
            .text(&today)
            .placeholder_text(lang.text("Through YYYY-MM-DD"))
            .hexpand(true)
            .build();
        let show = gtk::Button::with_label(lang.text("Show"));
        filters.append(&from);
        filters.append(&through);
        filters.append(&show);
        body.append(&gtk::Label::new(Some(
            lang.text("From / through (inclusive local dates, YYYY-MM-DD)"),
        )));
        body.append(&filters);
        let total = gtk::Label::new(Some(lang.text("Total · 00:00:00")));
        total.add_css_class("title-2");
        body.append(&total);
        let exports = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let totals = gtk::Button::with_label(lang.text("Export Totals CSV"));
        let sessions = gtk::Button::with_label(lang.text("Export Sessions CSV"));
        totals.set_sensitive(false);
        sessions.set_sensitive(false);
        exports.append(&totals);
        exports.append(&sessions);
        body.append(&exports);
        let result = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&result)
            .build();
        body.append(&scroll);
        root.append(&body);
        dialog.set_content(Some(&root));
        let report = Rc::new(RefCell::new(None::<Report>));
        let d = self.clone();
        let current = report.clone();
        let totals_button = totals.clone();
        let sessions_button = sessions.clone();
        show.connect_clicked(move |_| {
            let loaded = (|| -> Result<Report> {
                let (start, end) = report::date_range(
                    NaiveDate::parse_from_str(&from.text(), "%Y-%m-%d")?,
                    NaiveDate::parse_from_str(&through.text(), "%Y-%m-%d")?,
                    &Local,
                )?;
                if !d.apply(|core, now| {
                    core.tick(now);
                    Ok(())
                }) {
                    anyhow::bail!("Could not checkpoint current session");
                }
                report::load(&d.store.borrow(), &d.core.borrow().tasks, start, end)
            })();
            match loaded {
                Ok(value) => {
                    total.set_label(&format!("Total · {}", format_duration(value.total_ms)));
                    while let Some(child) = result.first_child() {
                        result.remove(&child);
                    }
                    if value.sessions.is_empty() {
                        result.append(&gtk::Label::new(Some(
                            lang.text("No tracked time in this period"),
                        )));
                    }
                    for task in &value.totals {
                        let row = adw::ActionRow::new();
                        row.set_use_markup(false);
                        row.set_title(&task.title);
                        row.set_subtitle(&format_duration(task.elapsed_ms));
                        result.append(&row);
                    }
                    let heading = gtk::Label::new(Some(lang.text("Session history")));
                    heading.add_css_class("title-3");
                    heading.set_xalign(0.0);
                    result.append(&heading);
                    for row in &value.sessions {
                        let start = row
                            .session
                            .started_at
                            .with_timezone(&Local)
                            .format("%Y-%m-%d %H:%M:%S");
                        let end = row
                            .session
                            .ended_at
                            .with_timezone(&Local)
                            .format("%Y-%m-%d %H:%M:%S");
                        let subtitle = format!(
                            "{start} → {end} · {} {}",
                            format_duration(row.contribution_ms),
                            lang.text("in period")
                        );
                        let item = adw::ActionRow::new();
                        item.set_use_markup(false);
                        item.set_title(&row.title);
                        item.set_subtitle(&subtitle);
                        result.append(&item);
                    }
                    *current.borrow_mut() = Some(value);
                    totals_button.set_sensitive(true);
                    sessions_button.set_sensitive(true);
                }
                Err(error) => d.error(error),
            }
        });
        for (button, kind) in [(totals, Export::Totals), (sessions, Export::Sessions)] {
            let d = self.clone();
            let r = report.clone();
            let parent = dialog.clone();
            button.connect_clicked(move |_| {
                if let Some(report) = r.borrow().as_ref() {
                    d.export_dialog(&parent, report.clone(), kind);
                }
            });
        }
        dialog.present();
        show.emit_clicked();
    }

    fn export_dialog(self: &Rc<Self>, parent: &adw::Window, report: Report, kind: Export) {
        let lang = self.core.borrow().settings.language;
        let dialog = gtk::FileDialog::builder()
            .title(lang.text("Export CSV"))
            .initial_name(match kind {
                Export::Totals => "workcircuit-totals.csv",
                Export::Sessions => "workcircuit-sessions.csv",
            })
            .build();
        let d = self.clone();
        dialog.save(
            Some(parent),
            None::<&gio::Cancellable>,
            move |file| match file {
                Ok(file) => {
                    let write = (|| -> Result<()> {
                        let mut bytes = vec![];
                        report::write_csv(&report, kind, &mut bytes)?;
                        file.replace_contents(
                            &bytes,
                            None,
                            false,
                            gio::FileCreateFlags::REPLACE_DESTINATION,
                            None::<&gio::Cancellable>,
                        )?;
                        Ok(())
                    })();
                    if let Err(error) = write {
                        d.error(error);
                    }
                }
                Err(error) => {
                    if !error.matches(gtk::DialogError::Dismissed) {
                        d.error(error);
                    }
                }
            },
        );
    }
}

fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    button.set_tooltip_text(Some(tooltip));
    button.add_css_class("flat");
    button.set_valign(gtk::Align::Center);
    button
}
fn margins(widget: &impl IsA<gtk::Widget>, size: i32) {
    widget.set_margin_top(size);
    widget.set_margin_bottom(size);
    widget.set_margin_start(size);
    widget.set_margin_end(size);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pump() {
        let context = glib::MainContext::default();
        let until = Instant::now() + Duration::from_millis(150);
        while Instant::now() < until {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn find<T: IsA<gtk::Widget> + Clone + glib::object::ObjectType>(
        widget: &gtk::Widget,
    ) -> Vec<T> {
        let mut found = vec![];
        if let Ok(value) = widget.clone().downcast::<T>() {
            found.push(value);
        }
        let mut child = widget.first_child();
        while let Some(value) = child {
            found.extend(find::<T>(&value));
            child = value.next_sibling();
        }
        found
    }

    fn button(window: &impl IsA<gtk::Widget>, label: &str) -> gtk::Button {
        find::<gtk::Button>(window.as_ref())
            .into_iter()
            .find(|b| {
                b.label().as_deref() == Some(label) || b.tooltip_text().as_deref() == Some(label)
            })
            .unwrap_or_else(|| panic!("Missing button: {label}"))
    }

    fn dialog(title: &str) -> gtk::Window {
        gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Window>().ok())
            .find(|w| w.title().as_deref() == Some(title))
            .unwrap_or_else(|| panic!("Missing dialog: {title}"))
    }

    fn capture(window: &(impl IsA<gtk::Widget> + IsA<gtk::Native>), path: &str) {
        pump();
        let paintable = gtk::WidgetPaintable::new(Some(window));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(
            &snapshot,
            f64::from(window.width()),
            f64::from(window.height()),
        );
        let node = snapshot.to_node().expect("Rendered window");
        window
            .renderer()
            .unwrap()
            .render_texture(&node, None)
            .save_to_png(path)
            .unwrap();
    }

    #[test]
    #[ignore = "requires a graphical display and a session bus"]
    fn desktop_controls_share_state_and_dialogs_work() {
        adw::init().unwrap();
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("../data/style.css"));
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let app = adw::Application::builder()
            .application_id("io.github.workcircuit.SmokeTest")
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gio::Cancellable>).unwrap();
        let mut store = Store::in_memory().unwrap();
        let core = store.load().unwrap();
        let d = Desktop::new(&app, store, core);
        d.open_dashboard();
        d.install_timer();
        pump();
        let main = d.dashboard.borrow().as_ref().unwrap().window.clone();
        if std::env::var_os("WORKCIRCUIT_WM_ONLY").is_some() {
            d.apply(|core, now| {
                core.add_task("Overlay integration check", "", now.wall)?;
                Ok(())
            });
            d.open_overlay();
            for _ in 0..4 {
                pump();
            }
            let overlay = d.overlay.borrow().as_ref().unwrap().window.clone();
            assert!(
                overlay.child().unwrap().width() <= 340 && overlay.child().unwrap().height() <= 170,
                "Content {}×{}, window {}×{}, scale {}",
                overlay.child().unwrap().width(),
                overlay.child().unwrap().height(),
                overlay.width(),
                overlay.height(),
                overlay.scale_factor()
            );
            assert!(
                crate::overlay::assert_above_if_managed(&overlay),
                "This check requires a window manager supporting EWMH above"
            );
            main.present();
            pump();
            assert!(crate::overlay::assert_above_if_managed(&overlay));
            let picker = find::<gtk::MenuButton>(overlay.upcast_ref()).remove(0);
            picker.popup();
            pump();
            let popover = picker.popover().unwrap();
            assert!(
                popover.width() >= 260,
                "Native picker width is {}",
                popover.width()
            );
            let title = find::<gtk::Label>(popover.upcast_ref())
                .into_iter()
                .find(|label| label.text() == "Overlay integration check")
                .unwrap();
            assert!(
                title.width() >= 150 && title.height() <= 40,
                "Native task title is {}×{}",
                title.width(),
                title.height()
            );
            popover.popdown();
            overlay.close();
            main.close();
            return;
        }
        button(&main, "Add Task").emit_clicked();
        pump();
        let add = dialog("Add Task");
        let entry = find::<gtk::Entry>(add.upcast_ref()).remove(0);
        assert!(!button(&add, "Save Task").is_sensitive());
        entry.set_text("Build the desktop app");
        button(&add, "Save Task").emit_clicked();
        pump();
        assert_eq!(d.core.borrow().tasks.len(), 1);
        d.apply(|core, now| {
            core.add_task(
                "Review project <B> & notes",
                "Check pending changes",
                now.wall,
            )?;
            core.add_task(
                "Research the next feature",
                "Keep the rotation moving",
                now.wall,
            )?;
            Ok(())
        });
        button(&main, "Overlay").emit_clicked();
        pump();
        let overlay = d.overlay.borrow().as_ref().unwrap().window.clone();
        assert!(
            overlay.child().unwrap().width() <= 340 && overlay.child().unwrap().height() <= 170,
            "Overlay is {}×{}",
            overlay.width(),
            overlay.height()
        );
        crate::overlay::assert_above_if_managed(&overlay);
        main.present();
        pump();
        crate::overlay::assert_above_if_managed(&overlay);
        let picker = find::<gtk::MenuButton>(overlay.upcast_ref())
            .into_iter()
            .find(|m| m.tooltip_text().as_deref() == Some("Choose task"))
            .unwrap();
        picker.popup();
        pump();
        let popover = picker.popover().unwrap();
        assert!(popover.is_visible());
        assert!(
            popover.width() >= 260,
            "Task picker must have a readable width, got {}",
            popover.width()
        );
        let task_title = find::<gtk::Label>(popover.upcast_ref())
            .into_iter()
            .find(|label| label.text() == "Build the desktop app")
            .unwrap();
        assert!(
            task_title.width() >= 150,
            "Task title has only {} pixels",
            task_title.width()
        );
        assert!(task_title.height() <= 40, "Task title must fit on one line");
        if let Ok(directory) = std::env::var("WORKCIRCUIT_CAPTURE_DIR") {
            capture(
                &popover,
                std::path::Path::new(&directory)
                    .join("workcircuit-task-picker.png")
                    .to_str()
                    .unwrap(),
            );
        }
        let first_row = find::<adw::ActionRow>(popover.upcast_ref()).remove(0);
        adw::prelude::ActionRowExt::activate(&first_row);
        for _ in 0..2 {
            pump();
        }
        assert!(!popover.is_visible());
        assert!(
            d.core.borrow().active.is_none(),
            "Selecting while paused must stay paused"
        );
        button(&overlay, "Start").emit_clicked();
        assert!(d.core.borrow().active.is_some());
        button(&overlay, "Next →").emit_clicked();
        for _ in 0..10 {
            pump();
        }
        assert!(
            d.core.borrow().visit_ms >= 500,
            "The GTK timer must update elapsed work"
        );
        assert_eq!(
            d.core.borrow().current().unwrap().title,
            "Review project <B> & notes"
        );
        assert_eq!(
            d.dashboard.borrow().as_ref().unwrap().title.text(),
            d.overlay.borrow().as_ref().unwrap().title.text()
        );
        button(&main, "Pause").emit_clicked();
        assert!(d.core.borrow().active.is_none());
        // Native Wayland can suppress rendering of occluded windows. Capture only
        // when explicitly requested in a controlled display such as Xvfb.
        if let Ok(directory) = std::env::var("WORKCIRCUIT_CAPTURE_DIR") {
            let directory = std::path::Path::new(&directory);
            capture(
                &main,
                directory
                    .join("workcircuit-dashboard.png")
                    .to_str()
                    .unwrap(),
            );
            capture(
                &overlay,
                directory.join("workcircuit-overlay.png").to_str().unwrap(),
            );
        }
        button(&main, "Settings").emit_clicked();
        pump();
        let settings = dialog("Settings");
        button(&settings, "Add Scheduled Break").emit_clicked();
        pump();
        let break_editor = dialog("Scheduled Break");
        button(&break_editor, "Add Break").emit_clicked();
        pump();
        button(&settings, "Save Settings").emit_clicked();
        assert_eq!(d.core.borrow().settings.exclusions.len(), 1);
        // Remove the break so the lifecycle check is independent of the local time.
        d.apply(|core, now| core.set_settings(Settings::default(), now));
        button(&main, "Reports").emit_clicked();
        pump();
        let reports = dialog("Reports");
        assert!(button(&reports, "Export Totals CSV").is_sensitive());
        reports.close();
        let id = d.core.borrow().selected.unwrap();
        d.apply(|core, now| {
            core.archive(id, now);
            Ok(())
        });
        assert!(d.core.borrow().active.is_none());
        button(&main, "Restore").emit_clicked();
        assert!(
            !d.core
                .borrow()
                .tasks
                .iter()
                .find(|t| t.id == id)
                .unwrap()
                .archived
        );
        let toggle = d.overlay.borrow().as_ref().unwrap().toggle.clone();
        toggle.emit_clicked();
        assert!(d.core.borrow().active.is_some());
        let session_id = d.core.borrow().active.as_ref().unwrap().id;
        button(&main, "Settings").emit_clicked();
        pump();
        let settings = dialog("Settings");
        find::<gtk::DropDown>(settings.upcast_ref())[0].set_selected(1);
        button(&settings, "Save Settings").emit_clicked();
        pump();
        let main = d.dashboard.borrow().as_ref().unwrap().window.clone();
        let overlay = d.overlay.borrow().as_ref().unwrap().window.clone();
        assert_eq!(
            d.core.borrow().settings.language,
            Language::PortugueseBrazil
        );
        assert_eq!(
            d.core.borrow().active.as_ref().unwrap().id,
            session_id,
            "Changing language must preserve the running session"
        );
        button(&main, "Minijanela");
        button(&main, "Configurações");
        button(&overlay, "Pausar");
        assert!(
            overlay.child().unwrap().width() <= 340 && overlay.child().unwrap().height() <= 170,
            "Portuguese overlay is {}×{}",
            overlay.width(),
            overlay.height()
        );
        crate::overlay::assert_above_if_managed(&overlay);
        main.close();
        pump();
        assert!(d.dashboard.borrow().is_none());
        assert!(d.core.borrow().active.is_some());
        overlay.close();
        pump();
        assert!(d.core.borrow().active.is_none());
        assert!(
            !d.store
                .borrow()
                .sessions(
                    Utc::now() - chrono::Duration::days(1),
                    Utc::now() + chrono::Duration::days(1)
                )
                .unwrap()
                .is_empty()
        );
    }
}
