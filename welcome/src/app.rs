//! The window: Welcome → Apps → Browser → Review → Installing → Done.
//!
//! Like the first boot of openSUSE Aeon: a short list of basics to tick and a
//! browser to choose. Everything else in the catalog is installed later with
//! `tamagi install`.

use crate::catalog::{self, format_size, pretty_size, short_name, tagline, Item};
use crate::flow::{self, Page};
use crate::i18n::{capitalized, tr};
use crate::method::{classify, Method};
use crate::plan::{self, browser_action, methods_for, Choices, Step, BROWSERS};
use crate::progress::{parse_line, Line, Tracker};
use crate::review::{self, Row};
use crate::system::{self, Event};
use crate::ui::{badge, legend, note, panel, radio_mark, small, text, LOCK_ICON, NOTE, SMALL};
use cosmic::app::{Core, Settings, Task};
use cosmic::iced::futures::channel::mpsc;
use cosmic::iced::futures::StreamExt;
use cosmic::iced::{keyboard, Alignment, Color, Length, Size, Subscription};
use cosmic::{executor, theme, widget, Application, ApplicationExt, Element};
use std::collections::BTreeSet;

/// Lines of output kept for the details panel.
const LOG_LINES: usize = 400;
/// Widest the content gets, centred in a larger window.
const CONTENT_WIDTH: f32 = 900.0;

pub fn run() -> cosmic::iced::Result {
    let catalog = catalog::load();
    let basics = catalog::load_basics(&catalog);
    let flags = Flags { catalog, basics, disk_question: system::tpm_question_applies() };
    let settings = Settings::default().size(Size::new(960.0, 740.0)).size_limits(cosmic::iced::Limits::NONE.min_width(640.0).min_height(480.0));
    cosmic::app::run::<Welcome>(settings, flags)
}

pub struct Flags {
    catalog: Vec<Item>,
    /// The basics: (index in `catalog`, ticked by default).
    basics: Vec<(usize, bool)>,
    disk_question: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Next,
    Back,
    Skip,
    Close,
    /// Index in `plan::BROWSERS`; `BROWSERS.len()` is "not now".
    Browser(usize),
    /// Index in `plan::methods_for` of the browser chosen.
    BrowserMethod(usize),
    /// A catalog item (index) ticked or not.
    Toggle(usize, bool),
    Tpm(bool),
    ToggleLog,
    Run(Event),
}

pub struct Welcome {
    core: Core,
    page: Page,
    catalog: Vec<Item>,
    /// The basics offered, as indices in `catalog`, in the order shown.
    basics: Vec<usize>,
    browser: usize,
    browser_method: usize,
    checked: BTreeSet<usize>,
    disk_question: bool,
    tpm: bool,
    // Installing
    steps: Vec<Step>,
    tracker: Tracker,
    /// The command of the plan running now; `steps.len()` once all ended.
    running: Option<usize>,
    stage: String,
    log: Vec<String>,
    show_log: bool,
    /// "Could not install: …" lines printed by the installers.
    failures: Vec<String>,
    /// A command of the plan ended with an error.
    failed: bool,
    installed: bool,
}

impl Application for Welcome {
    type Executor = executor::Default;
    type Flags = Flags;
    type Message = Message;

    const APP_ID: &'static str = "org.tamagi.Welcome";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let mut app = Welcome {
            core,
            page: Page::Welcome,
            catalog: flags.catalog,
            basics: flags.basics.iter().map(|(index, _)| *index).collect(),
            browser: 0,
            browser_method: 0,
            checked: flags.basics.iter().filter(|(_, on)| *on).map(|(index, _)| *index).collect(),
            disk_question: flags.disk_question,
            tpm: false,
            steps: Vec::new(),
            tracker: Tracker::new(&[]),
            running: None,
            stage: String::new(),
            log: Vec::new(),
            show_log: false,
            failures: Vec::new(),
            failed: false,
            installed: false,
        };
        let title = tr("Welcome to Tamagi");
        app.set_header_title(title.clone());
        let task = match app.core.main_window_id() {
            Some(id) => app.set_window_title(title, id),
            None => Task::none(),
        };
        (app, task)
    }

    /// Esc goes back.
    fn on_escape(&mut self) -> Task<Self::Message> {
        cosmic::task::message(Message::Back)
    }

    /// Enter goes on, when no widget used the key (a focused button or the
    /// search field keep it).
    fn subscription(&self) -> Subscription<Self::Message> {
        use keyboard::key::Named;
        cosmic::iced::event::listen_with(|event, status, _window| {
            let cosmic::iced::Event::Keyboard(keyboard::Event::KeyPressed { key: keyboard::Key::Named(key), modifiers, .. }) = event else {
                return None;
            };
            let unused = status == cosmic::iced::event::Status::Ignored;
            match key {
                // Ctrl+Enter and Alt+arrows work wherever the focus is.
                Named::Enter if modifiers.control() || unused => Some(Message::Next),
                Named::ArrowRight if modifiers.alt() => Some(Message::Next),
                Named::ArrowLeft if modifiers.alt() => Some(Message::Back),
                _ => None,
            }
        })
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::Next => match self.page {
                Page::Review => return self.install(),
                Page::Installing => {}
                Page::Done => return cosmic::iced::exit(),
                page => {
                    if let Some(next) = flow::next(&self.pages(), page) {
                        return self.show(next);
                    }
                }
            },
            Message::Back => {
                if let Some(page) = flow::back(&self.pages(), self.page) {
                    return self.show(page);
                }
            }
            Message::Skip => {
                if self.page == Page::Welcome {
                    self.checked.clear();
                    self.browser = BROWSERS.len();
                    self.tpm = false;
                    return self.install();
                }
            }
            Message::Close => return cosmic::iced::exit(),
            Message::Browser(index) => {
                self.browser = index.min(BROWSERS.len());
                // A browser with fewer ways to install keeps a valid one.
                self.browser_method = self.browser_method.min(self.methods().len().saturating_sub(1));
            }
            Message::BrowserMethod(index) => self.browser_method = index.min(self.methods().len().saturating_sub(1)),
            Message::Toggle(index, on) => {
                if on {
                    self.checked.insert(index);
                } else {
                    self.checked.remove(&index);
                }
            }
            Message::Tpm(on) => self.tpm = on,
            Message::ToggleLog => self.show_log = !self.show_log,
            Message::Run(event) => self.installer_event(event),
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let spacing = theme::spacing();
        let page = match self.page {
            Page::Welcome => self.view_welcome(),
            Page::Apps => self.view_apps(),
            Page::Browser => self.view_browser(),
            Page::Disk => self.view_disk(),
            Page::Review => self.view_review(),
            Page::Installing => self.view_installing(),
            Page::Done => self.view_done(),
        };
        // The page keeps a gutter on the right for its scroll bar, so what it
        // shows lines up with the step indicator above and the buttons below.
        let gutter = [0, spacing.space_xs, 0, 0];
        let mut content = widget::column::with_capacity(4)
            .spacing(spacing.space_s)
            .max_width(CONTENT_WIDTH)
            .push(widget::container(self.view_steps()).padding(gutter))
            .push(widget::container(page).width(Length::Fill).height(Length::Fill));
        if self.page == Page::Apps {
            content = content.push(widget::container(legend()).padding(gutter));
        }
        content = content.push(widget::container(self.view_footer()).padding(gutter));
        widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .padding([0, spacing.space_l - spacing.space_xs, spacing.space_m, spacing.space_l])
            .into()
    }
}

/// Width of the column the badges sit in, so the names after them line up.
const BADGE_COLUMN: f32 = 150.0;

/// Headings inside a page.
const TEXT_HEADING: f32 = 17.0;

/// Text a little dimmer than the body, for what is secondary.
fn dim(theme: &cosmic::Theme) -> cosmic::iced::widget::text::Style {
    let on: Color = theme.cosmic().on_bg_color().into();
    cosmic::iced::widget::text::Style { color: Some(Color { a: 0.7, ..on }), ..Default::default() }
}

fn dim_class() -> theme::Text {
    theme::Text::Custom(dim)
}

/// The heading of a page.
fn heading<'a>(title: String, subtitle: String) -> Element<'a, Message> {
    let mut column = widget::column::with_capacity(2).spacing(theme::spacing().space_xxs).push(widget::text::title3(title));
    if !subtitle.is_empty() {
        column = column.push(note(subtitle).class(dim_class()));
    }
    column.into()
}

/// A page (or part of one) whose content scrolls when the window is small.
fn scrolling<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    widget::scrollable(fixed(content)).width(Length::Fill).height(Length::Fill).into()
}

/// Content that does not scroll, with the same gutter as what does.
fn fixed<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    widget::container(content).width(Length::Fill).padding([0, theme::spacing().space_xs, 0, 0]).into()
}

/// A row of a single-choice list: the whole row is the control (click, or
/// Tab and Enter), with the radio mark showing which one is chosen.
fn choice<'a>(value: usize, selected: usize, message: fn(usize) -> Message, label: impl Into<Element<'a, Message>>) -> widget::list::ListButton<'a, Message> {
    let spacing = theme::spacing();
    let row = widget::row::with_capacity(2).spacing(spacing.space_s).align_y(Alignment::Center).push(radio_mark(value == selected)).push(label);
    widget::list::button(widget::container(row).padding([spacing.space_xxs, 0])).on_press(message(value))
}

fn approx(mb: f64) -> String {
    format!("≈ {}", format_size(mb))
}

impl Welcome {
    fn pages(&self) -> Vec<Page> {
        flow::pages(self.disk_question)
    }

    /// Goes to a page. It starts with nothing focused, so Enter means "next"
    /// again (and not the button that had the focus on the page before).
    fn show(&mut self, page: Page) -> Task<Message> {
        self.page = page;
        cosmic::iced::runtime::task::widget(cosmic::iced::core::widget::operation::focusable::unfocus())
    }

    /// The ways the browser chosen can be installed.
    fn methods(&self) -> &'static [(&'static str, &'static str)] {
        BROWSERS.get(self.browser).map(|(id, _)| methods_for(id)).unwrap_or(&[])
    }

    /// The flag of the install method chosen for the browser.
    fn method_flag(&self) -> &'static str {
        self.methods().get(self.browser_method).map(|(flag, _)| *flag).unwrap_or("flatpak")
    }

    fn picked(&self) -> impl Iterator<Item = &Item> {
        self.basics.iter().filter(|index| self.checked.contains(index)).filter_map(|index| self.catalog.get(*index))
    }

    fn choices(&self) -> Choices {
        Choices {
            browser: BROWSERS.get(self.browser).map(|(id, _)| (id.to_string(), self.method_flag().to_string())),
            picked: self.picked().map(|item| item.action.clone()).collect(),
            tpm: self.disk_question && self.tpm,
        }
    }

    /// Everything that will be installed, for the review page.
    fn review_rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        if let Some((id, name)) = BROWSERS.get(self.browser) {
            let action = browser_action(id, self.method_flag());
            let size_mb = self.catalog.iter().find(|item| item.id == *id).and_then(Item::size_mb);
            rows.push(Row { name: name.to_string(), method: classify(&action), size_mb });
        }
        for item in self.picked() {
            rows.push(Row { name: short_name(&tr(&item.desc)), method: item.method(), size_mb: item.size_mb() });
        }
        review::dedup(rows)
    }

    /// Starts the installation (or just finishes, when nothing was chosen).
    fn install(&mut self) -> Task<Message> {
        let steps = plan::steps(&self.choices());
        self.tracker = Tracker::new(&steps);
        self.running = None;
        self.steps = steps.clone();
        self.installed = !steps.is_empty();
        self.stage.clear();
        self.log.clear();
        self.failures.clear();
        self.failed = false;
        // Nothing to install: straight to the end (the stamp is still written).
        self.page = if steps.is_empty() { Page::Done } else { Page::Installing };
        let notification = (tr("Your apps are ready"), tr("What you chose at first login has been installed."));
        let (sender, receiver) = mpsc::unbounded();
        std::thread::spawn(move || {
            system::run_plan(&steps, Some(notification), |event| {
                let _ = sender.unbounded_send(event);
            });
        });
        Task::stream(receiver.map(|event| cosmic::Action::App(Message::Run(event))))
    }

    fn installer_event(&mut self, event: Event) {
        match event {
            Event::Started(index) => {
                self.tracker.start(index);
                self.running = Some(index);
                self.stage.clear();
                if let Some(step) = self.steps.get(index) {
                    self.push_log(format!("$ {}", step.argv.join(" ")));
                }
            }
            Event::Output(raw) => {
                let line = parse_line(&raw);
                if let Some(stage) = self.tracker.line(&line) {
                    self.stage = stage;
                }
                let text = match line {
                    Line::Stage { n, total, title } => format!("── [{n}/{total}] {title}"),
                    Line::Heading(title) => format!("── {title}"),
                    Line::Text(text) => text,
                    Line::Blank => return,
                };
                let shown = text.trim_start();
                if [tr("Could not install"), tr("Finished with failures in")].iter().any(|prefix| shown.starts_with(prefix.as_str())) {
                    self.failures.push(shown.to_string());
                }
                self.push_log(text);
            }
            Event::Ended(index, ok) => {
                let _ = index;
                if !ok {
                    self.failed = true;
                }
            }
            Event::Finished => {
                self.tracker.finish();
                self.running = Some(self.steps.len());
                self.page = Page::Done;
            }
        }
    }

    fn push_log(&mut self, line: String) {
        if self.log.len() >= LOG_LINES {
            let extra = self.log.len() + 1 - LOG_LINES;
            self.log.drain(..extra);
        }
        self.log.push(line);
    }

    // ── Pieces around the page ─────────────────────────────────────────────

    /// Where we are: "Welcome › Profile › Browser › Review › Installing › Done".
    fn view_steps(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let pages = self.pages();
        let current = pages.iter().position(|p| *p == self.page).unwrap_or(0);
        let mut row = widget::row::with_capacity(pages.len() * 2 + 2).spacing(spacing.space_xxs).align_y(Alignment::Center);
        for (index, page) in pages.iter().enumerate() {
            if index > 0 {
                row = row.push(small("›").class(dim_class()));
            }
            let label = tr(page.label());
            row = row.push(if index == current {
                widget::text::heading(label).size(SMALL).class(theme::Text::Accent)
            } else if index < current {
                small(label)
            } else {
                small(label).class(dim_class())
            });
        }
        row = row.push(widget::space::horizontal());
        row = row.push(small(format!("{} {}/{}", tr("Step"), current + 1, pages.len())).class(dim_class()));
        widget::column::with_capacity(2)
            .spacing(spacing.space_xxs)
            .push(row)
            .push(widget::progress_bar::linear::Linear::new().girth(5.0).progress((current + 1) as f32 / pages.len() as f32).width(Length::Fill))
            .into()
    }

    /// Back on the left, the way forward on the right.
    fn view_footer(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let mut row = widget::row::with_capacity(4).spacing(spacing.space_xs).align_y(Alignment::Center);
        match self.page {
            Page::Welcome => {
                row = row
                    .push(widget::button::text(tr("Skip for now")).on_press(Message::Skip))
                    .push(widget::space::horizontal())
                    .push(widget::button::suggested(tr("Start")).on_press(Message::Next));
            }
            Page::Review => {
                let label = if self.choices().is_empty() { tr("Finish") } else { tr("Install") };
                row = row
                    .push(widget::button::standard(tr("Back")).on_press(Message::Back))
                    .push(widget::space::horizontal())
                    .push(widget::button::suggested(label).on_press(Message::Next));
            }
            // Nothing to press while it installs: the page moves on by itself.
            Page::Installing => {}
            Page::Done => {
                row = row.push(widget::space::horizontal()).push(widget::button::suggested(tr("Close")).on_press(Message::Close));
            }
            _ => {
                row = row
                    .push(widget::button::standard(tr("Back")).on_press(Message::Back))
                    .push(widget::space::horizontal())
                    .push(widget::button::suggested(tr("Next")).on_press(Message::Next));
            }
        }
        row.into()
    }

    // ── Pages ──────────────────────────────────────────────────────────────

    fn view_welcome(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let mut methods = widget::column::with_capacity(Method::LEGEND.len() + 1).spacing(spacing.space_s).push(widget::text::heading(tr("Every item says how it is installed")).size(TEXT_HEADING));
        for method in Method::LEGEND {
            methods = methods.push(
                widget::row::with_capacity(2)
                    .spacing(spacing.space_s)
                    .align_y(Alignment::Center)
                    .push(widget::container(badge(method)).width(Length::Fixed(BADGE_COLUMN)))
                    .push(text(tr(method.explanation())).width(Length::Fill)),
            );
        }
        let column = widget::column::with_capacity(6)
            .spacing(spacing.space_m)
            .push(widget::space::vertical().height(Length::Fixed(f32::from(spacing.space_xxs))))
            .push(widget::text::title1(tr("Welcome to Tamagi")))
            .push(
                widget::column::with_capacity(2)
                    .spacing(spacing.space_xxs)
                    .push(text(tr("Atomic, lean system: the host is read-only and updates itself.")))
                    .push(text(tr("Almost nothing comes installed: you choose what you want, now or later."))),
            )
            .push(panel(methods).padding(spacing.space_m))
            .push(note(tr("Tick the basics and pick a browser, then review. Nothing is installed before you confirm.")).class(dim_class()));
        scrolling(column)
    }

    /// The basics: one large row per app, grouped by category. The whole row
    /// is the control (click, or Tab and Enter).
    fn view_apps(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let total: f64 = self.picked().filter_map(Item::size_mb).sum();
        let count = self.picked().count();
        let summary = if count == 0 { tr("Nothing selected") } else { format!("{count} {} · {}", tr("selected"), approx(total)) };
        let title = widget::row::with_capacity(2)
            .spacing(spacing.space_s)
            .align_y(Alignment::End)
            .push(widget::container(heading(tr("Apps to start with"), tr("Tick what you want now. Everything else is one command away: tamagi install"))).width(Length::Fill))
            .push(widget::text::heading(summary).size(NOTE));

        let mut sections = widget::column::with_capacity(6).spacing(spacing.space_m);
        for category in catalog::categories(&self.catalog, &self.basics) {
            let mut section = widget::settings::section().title(tr(category));
            for &index in &self.basics {
                let Some(item) = self.catalog.get(index).filter(|item| item.category == category) else {
                    continue;
                };
                let desc = tr(&item.desc);
                let checked = self.checked.contains(&index);
                let mut name = widget::column::with_capacity(2).spacing(spacing.space_xxxs).width(Length::Fill).push(text(short_name(&desc)));
                if let Some(tagline) = tagline(&desc) {
                    name = name.push(note(capitalized(&tagline)).class(dim_class()));
                }
                let row = widget::row::with_capacity(4)
                    .spacing(spacing.space_m)
                    .align_y(Alignment::Center)
                    .push(widget::checkbox(checked).size(22).on_toggle(move |on| Message::Toggle(index, on)))
                    .push(name)
                    .push(badge(item.method()))
                    .push(small(pretty_size(&item.size)).class(dim_class()).width(Length::Fixed(72.0)).align_x(Alignment::End));
                section = section.add(widget::list::button(widget::container(row).padding([spacing.space_xs, 0])).on_press(Message::Toggle(index, !checked)));
            }
            sections = sections.push(section);
        }
        if self.basics.is_empty() {
            sections = sections.push(panel(text(tr("Nothing to offer here. Later: tamagi install")).class(dim_class())));
        }

        widget::column::with_capacity(2).spacing(spacing.space_s).push(fixed(title)).push(scrolling(sections)).into()
    }

    /// The browser on the left, how to install it on the right.
    fn view_browser(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let none = BROWSERS.len();
        let mut browsers = widget::settings::section().title(tr("Which browser?"));
        for (index, (id, name)) in BROWSERS.iter().enumerate() {
            let size = self.catalog.iter().find(|item| item.id == *id).map(|item| format!("≈ {}", pretty_size(&item.size))).unwrap_or_default();
            let label = widget::row::with_capacity(2).align_y(Alignment::Center).push(text(*name).width(Length::Fill)).push(small(size).class(dim_class()));
            browsers = browsers.add(choice(index, self.browser, Message::Browser, label));
        }
        browsers = browsers.add(choice(none, self.browser, Message::Browser, text(capitalized(&tr("not now"))).width(Length::Fill)));

        let right: Element<'_, Message> = if self.browser == none {
            panel(text(tr("No browser now. Later: tamagi install")).class(dim_class())).into()
        } else {
            let methods = self.methods();
            let mut section = widget::settings::section().title(tr("Install how?"));
            for (index, (flag, key)) in methods.iter().enumerate() {
                let method = classify(&browser_action("x", flag));
                let explained = tr(key);
                let label = widget::column::with_capacity(2)
                    .spacing(spacing.space_xxs)
                    .push(badge(method))
                    .push(note(capitalized(&tagline(&explained).unwrap_or(explained))).width(Length::Fill));
                section = section.add(choice(index, self.browser_method, Message::BrowserMethod, label.width(Length::Fill)));
            }
            let mut column = widget::column::with_capacity(2).spacing(spacing.space_xs).push(section);
            if methods.len() == 1 {
                column = column.push(note(tr("This browser exists for Linux only as a Flatpak.")).class(dim_class()));
            }
            column.into()
        };

        let columns = widget::row::with_capacity(2)
            .spacing(spacing.space_m)
            .push(widget::container(browsers).width(Length::FillPortion(1)))
            .push(widget::container(right).width(Length::FillPortion(1)));
        scrolling(
            widget::column::with_capacity(2)
                .spacing(spacing.space_s)
                .push(heading(tr("Browser"), tr("No browser ships in the image. Choose one (you can install others later).")))
                .push(columns),
        )
    }

    fn view_disk(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let question = widget::settings::section().add(
            widget::settings::item::builder(tr("Unlock the disk with the TPM chip at boot, without typing the password?"))
                .icon(widget::icon::from_name(LOCK_ICON).size(24).icon())
                .toggler(self.tpm, Message::Tpm),
        );
        let hint = if self.tpm { tr("The password is asked for when this is set up.") } else { tr("Later: tamagi setup tpm-unlock on") };
        let column = widget::column::with_capacity(3)
            .spacing(spacing.space_m)
            .push(heading(tr("Encrypted disk"), String::new()))
            .push(question)
            .push(text(hint).class(dim_class()));
        scrolling(column)
    }

    /// Everything chosen, grouped by install method; the total and what asks
    /// for the password stay in view under the list.
    fn view_review(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let rows = self.review_rows();
        let tpm = self.disk_question && self.tpm;
        let title = fixed(heading(tr("Review"), String::new()));

        if rows.is_empty() && !tpm {
            let nothing = panel(
                widget::column::with_capacity(2)
                    .spacing(spacing.space_xxs)
                    .push(widget::text::heading(tr("Nothing selected: nothing will be installed.")).size(TEXT_HEADING))
                    .push(note(tr("Everything here is optional and stays available later in: tamagi install")).class(dim_class())),
            );
            return widget::column::with_capacity(2).spacing(spacing.space_s).push(title).push(fixed(nothing)).into();
        }

        let mut groups = widget::column::with_capacity(6).spacing(spacing.space_s);
        for (method, group) in review::grouped(&rows) {
            let header = widget::row::with_capacity(2)
                .spacing(spacing.space_xs)
                .align_y(Alignment::Center)
                .push(badge(method))
                .push(note(tr(method.explanation())).class(dim_class()));
            let mut section = widget::settings::section().header(header);
            for row in group {
                let size = row.size_mb.map(format_size).unwrap_or_default();
                section = section.add(
                    widget::row::with_capacity(2)
                        .align_y(Alignment::Center)
                        .push(text(row.name.clone()).width(Length::Fill))
                        .push(small(size).class(dim_class())),
                );
            }
            groups = groups.push(section);
        }
        if tpm {
            groups = groups.push(
                widget::settings::section().title(tr("Encrypted disk")).add(
                    widget::row::with_capacity(2)
                        .spacing(spacing.space_xs)
                        .align_y(Alignment::Center)
                        .push(text(tr("Unlock the disk with the TPM chip at boot")).width(Length::Fill))
                        .push(widget::icon::from_name(LOCK_ICON).size(16).icon()),
                ),
            );
        }

        let mut names = review::password_names(&rows);
        if tpm {
            names.push(tr("Encrypted disk"));
        }
        let password: Element<'_, Message> = if names.is_empty() {
            note(tr("Nothing here asks for the password.")).class(dim_class()).into()
        } else {
            widget::row::with_capacity(2)
                .spacing(spacing.space_xs)
                .push(widget::icon::from_name(LOCK_ICON).size(16).icon())
                .push(note(format!("{}: {}", tr("Asks for the password"), names.join(", "))).width(Length::Fill))
                .into()
        };
        let summary = widget::column::with_capacity(2)
            .spacing(spacing.space_xxs)
            .push(
                widget::row::with_capacity(3)
                    .align_y(Alignment::Center)
                    .push(widget::text::heading(tr("Approximate download")).size(TEXT_HEADING))
                    .push(widget::space::horizontal())
                    .push(widget::text::heading(approx(review::total_mb(&rows))).size(TEXT_HEADING)),
            )
            .push(password);

        widget::column::with_capacity(3).spacing(spacing.space_s).push(title).push(scrolling(groups)).push(fixed(panel(summary).padding([spacing.space_xs, spacing.space_s]))).into()
    }

    fn view_installing(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let fraction = self.tracker.fraction();
        let bar = widget::row::with_capacity(2)
            .spacing(spacing.space_s)
            .align_y(Alignment::Center)
            .push(widget::progress_bar::linear::Linear::new().girth(8.0).progress(fraction).width(Length::Fill))
            .push(text(format!("{:.0}%", fraction * 100.0)).width(Length::Fixed(52.0)).align_x(Alignment::End));

        // The commands of the plan: done, running (with its current stage), to come.
        let mut list = widget::settings::section();
        for (index, step) in self.steps.iter().enumerate() {
            let state: Element<'_, Message> = match self.running {
                Some(current) if current == index => widget::progress_bar::circular::Circular::new().size(16.0).into(),
                Some(current) if current > index => widget::icon::from_name("object-select-symbolic").size(16).icon().into(),
                _ => widget::space::horizontal().width(Length::Fixed(16.0)).into(),
            };
            let mut label = widget::column::with_capacity(2).spacing(spacing.space_xxxs).width(Length::Fill).push(text(tr(step.title)));
            if self.running == Some(index) && !self.stage.is_empty() {
                label = label.push(note(self.stage.clone()).class(dim_class()));
            }
            list = list.add(widget::row::with_capacity(2).spacing(spacing.space_s).align_y(Alignment::Center).push(widget::container(state).width(Length::Fixed(16.0))).push(label));
        }

        let details = if self.show_log { tr("Hide details") } else { tr("Show details") };
        let mut column = widget::column::with_capacity(5)
            .spacing(spacing.space_s)
            .push(heading(tr("Installing what you chose"), tr("You can keep using the computer: this window tells you when it is done.")))
            .push(bar)
            .push(list)
            .push(widget::row::with_capacity(2).push(widget::space::horizontal()).push(widget::button::text(details).on_press(Message::ToggleLog)));
        if self.show_log {
            column = column.push(self.view_log());
        }
        fixed(column)
    }

    /// The output of the installers, newest line at the bottom.
    fn view_log(&self) -> Element<'_, Message> {
        let text = widget::text::monotext(self.log.join("\n")).size(SMALL).width(Length::Fill);
        panel(widget::scrollable(text).anchor_bottom().width(Length::Fill).height(Length::Fill)).height(Length::Fill).into()
    }

    fn view_done(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let problems = self.failed || !self.failures.is_empty();
        let (title, subtitle) = if problems {
            (tr("Finished, but not everything was installed"), String::new())
        } else if self.installed {
            (tr("Your apps are ready"), tr("What you chose at first login has been installed."))
        } else {
            (tr("Done"), tr("Everything here is optional and stays available later in: tamagi install"))
        };
        let mut column = widget::column::with_capacity(6).spacing(spacing.space_s).push(heading(title, subtitle));
        if problems {
            let mut lines = widget::column::with_capacity(self.failures.len() + 1).spacing(spacing.space_xxs);
            for failure in &self.failures {
                lines = lines.push(text(failure.clone()));
            }
            lines = lines.push(note(format!("{}: tamagi install", tr("Try again with"))).class(dim_class()));
            column = column.push(panel(lines));
        }
        let mut tips = widget::settings::section().title(tr("Good to know"));
        for tip in [
            "Super+Space opens the menu · Super+A the command palette · Super+K the shortcuts.",
            "Themes and wallpapers: COSMIC Settings (Appearance and Wallpaper).",
            "More apps, by name: tamagi install (what the catalog does not list comes from Flathub).",
            "Ready-made sets (AI, development, games, office): tamagi profiles",
            "Every command of the system: tamagi",
        ] {
            tips = tips.add(text(tr(tip)).width(Length::Fill));
        }
        column = column.push(tips);
        if self.installed && !self.log.is_empty() {
            let details = if self.show_log { tr("Hide details") } else { tr("Show details") };
            column = column.push(widget::row::with_capacity(2).push(widget::space::horizontal()).push(widget::button::text(details).on_press(Message::ToggleLog)));
            if self.show_log {
                column = column.push(panel(widget::text::monotext(self.log.join("\n")).size(SMALL).width(Length::Fill)));
            }
        }
        scrolling(column)
    }
}
