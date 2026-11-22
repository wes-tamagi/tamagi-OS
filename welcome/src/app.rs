//! The window: Welcome → Profile → Browser → Catalog → Review → Installing → Done.

use crate::catalog::{self, format_size, pretty_size, short_name, tagline, Item};
use crate::flow::{self, Page};
use crate::i18n::{capitalized, tr};
use crate::method::{classify, Method};
use crate::plan::{self, browser_action, Choices, Step, BROWSERS, BROWSER_METHODS};
use crate::profile::{self, Entry, PROFILES};
use crate::progress::{parse_line, Line, Tracker};
use crate::review::{self, Row};
use crate::system::{self, Event};
use crate::ui::{badge, card_class, chip, legend, panel, radio_mark, LOCK_ICON};
use cosmic::app::{Core, Settings, Task};
use cosmic::iced::futures::channel::mpsc;
use cosmic::iced::futures::StreamExt;
use cosmic::iced::{keyboard, Alignment, Color, Length, Size, Subscription};
use cosmic::{executor, theme, widget, Application, ApplicationExt, Element};
use std::collections::BTreeSet;

/// The cards of the profiles: "Title: what it brings" (keys shared with the
/// terminal assistant), in the order of `profile::PROFILES`.
const PROFILE_KEYS: [&str; 4] = [
    "AI and agents: Claude Code, Codex, opencode and the development box",
    "Development: Fedora box, Node and Python, Zed and opencode",
    "Games: native Steam, gamescope with HDR, MangoHud and Heroic",
    "Office: Firefox, OnlyOffice and Thunderbird",
];

/// Lines of output kept for the details panel.
const LOG_LINES: usize = 400;
/// Widest the content gets, centred in a larger window.
const CONTENT_WIDTH: f32 = 860.0;

pub fn run() -> cosmic::iced::Result {
    let catalog = catalog::load();
    let profiles = PROFILES.iter().map(|name| profile::load(name, &catalog)).collect();
    let flags = Flags { catalog, profiles, disk_question: system::tpm_question_applies() };
    let settings = Settings::default().size(Size::new(900.0, 680.0)).size_limits(cosmic::iced::Limits::NONE.min_width(640.0).min_height(480.0));
    cosmic::app::run::<Welcome>(settings, flags)
}

pub struct Flags {
    catalog: Vec<Item>,
    profiles: Vec<Vec<Entry>>,
    disk_question: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileChoice {
    /// Index in `profile::PROFILES`.
    Ready(usize),
    /// Choose item by item from the catalog.
    ItemByItem,
    NotNow,
}

#[derive(Debug, Clone)]
pub enum Message {
    Next,
    Back,
    Skip,
    Close,
    Profile(ProfileChoice),
    /// Index in `plan::BROWSERS`; `BROWSERS.len()` is "not now".
    Browser(usize),
    /// Index in `plan::BROWSER_METHODS`.
    BrowserMethod(usize),
    /// A catalog item (index) ticked or not.
    Toggle(usize, bool),
    Search(String),
    Tpm(bool),
    ToggleLog,
    Run(Event),
}

pub struct Welcome {
    core: Core,
    page: Page,
    catalog: Vec<Item>,
    /// What each profile installs, in the order of `profile::PROFILES`.
    profiles: Vec<Vec<Entry>>,
    profile: ProfileChoice,
    browser: usize,
    browser_method: usize,
    checked: BTreeSet<usize>,
    search: String,
    search_id: widget::Id,
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
            profiles: flags.profiles,
            // The suggestion: AI and agents.
            profile: ProfileChoice::Ready(0),
            browser: 0,
            browser_method: 0,
            checked: BTreeSet::new(),
            search: String::new(),
            search_id: widget::Id::unique(),
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
                    self.profile = ProfileChoice::NotNow;
                    self.browser = BROWSERS.len();
                    self.tpm = false;
                    return self.install();
                }
            }
            Message::Close => return cosmic::iced::exit(),
            Message::Profile(choice) => self.profile = choice,
            Message::Browser(index) => self.browser = index.min(BROWSERS.len()),
            Message::BrowserMethod(index) => self.browser_method = index.min(BROWSER_METHODS.len() - 1),
            Message::Toggle(index, on) => {
                if on {
                    self.checked.insert(index);
                } else {
                    self.checked.remove(&index);
                }
            }
            Message::Search(text) => self.search = text,
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
            Page::Profile => self.view_profile(),
            Page::Browser => self.view_browser(),
            Page::Catalog => self.view_catalog(),
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
        if matches!(self.page, Page::Profile | Page::Catalog) {
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
const BADGE_COLUMN: f32 = 132.0;

/// Text a little dimmer than the body, for what is secondary.
fn dim(theme: &cosmic::Theme) -> cosmic::iced::widget::text::Style {
    let on: Color = theme.cosmic().on_bg_color().into();
    cosmic::iced::widget::text::Style { color: Some(Color { a: 0.7, ..on }), ..Default::default() }
}

fn dim_class() -> theme::Text {
    theme::Text::Custom(dim)
}

/// "Title: the rest" -> ("Title", "the rest").
fn split_title(text: &str) -> (String, String) {
    match text.split_once(": ") {
        Some((title, rest)) => (title.to_string(), rest.to_string()),
        None => (text.to_string(), String::new()),
    }
}

/// The heading of a page.
fn heading<'a>(title: String, subtitle: String) -> Element<'a, Message> {
    let mut column = widget::column::with_capacity(2).spacing(theme::spacing().space_xxs).push(widget::text::title3(title));
    if !subtitle.is_empty() {
        column = column.push(widget::text::body(subtitle).class(dim_class()));
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
    let row = widget::row::with_capacity(2).spacing(theme::spacing().space_s).align_y(Alignment::Center).push(radio_mark(value == selected)).push(label);
    widget::list::button(row).on_press(message(value))
}

fn approx(mb: f64) -> String {
    format!("≈ {}", format_size(mb))
}

impl Welcome {
    fn pages(&self) -> Vec<Page> {
        flow::pages(self.profile == ProfileChoice::ItemByItem, self.disk_question)
    }

    /// Goes to a page. The catalog opens with the search field focused:
    /// typing filters right away.
    fn show(&mut self, page: Page) -> Task<Message> {
        self.page = page;
        if page == Page::Catalog {
            return widget::text_input::focus(self.search_id.clone());
        }
        // A new page starts with nothing focused, so Enter means "next" again
        // (and not the button that had the focus on the page before).
        cosmic::iced::runtime::task::widget(cosmic::iced::core::widget::operation::focusable::unfocus())
    }

    fn profile_title(index: usize) -> String {
        split_title(&tr(PROFILE_KEYS[index])).0
    }

    fn choices(&self) -> Choices {
        Choices {
            profile: match self.profile {
                ProfileChoice::Ready(index) => PROFILES.get(index).map(|p| p.to_string()),
                _ => None,
            },
            browser: BROWSERS.get(self.browser).map(|(id, _)| (id.to_string(), BROWSER_METHODS[self.browser_method].0.to_string())),
            picked: match self.profile {
                ProfileChoice::ItemByItem => self.checked.iter().filter_map(|i| self.catalog.get(*i)).map(|item| item.action.clone()).collect(),
                _ => Vec::new(),
            },
            tpm: self.disk_question && self.tpm,
        }
    }

    /// Everything that will be installed, for the review page.
    fn review_rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        if let Some((id, name)) = BROWSERS.get(self.browser) {
            let action = browser_action(id, BROWSER_METHODS[self.browser_method].0);
            let size_mb = self.catalog.iter().find(|item| item.id == *id).and_then(Item::size_mb);
            rows.push(Row { name: name.to_string(), method: classify(&action), size_mb });
        }
        match self.profile {
            ProfileChoice::Ready(index) => {
                for entry in self.profiles.get(index).into_iter().flatten() {
                    rows.push(Row { name: entry.name(tr), method: entry.method, size_mb: entry.size_mb });
                }
            }
            ProfileChoice::ItemByItem => {
                for item in self.checked.iter().filter_map(|i| self.catalog.get(*i)) {
                    rows.push(Row { name: short_name(&tr(&item.desc)), method: item.method(), size_mb: item.size_mb() });
                }
            }
            ProfileChoice::NotNow => {}
        }
        review::dedup(rows)
    }

    /// Starts the installation (or just finishes, when nothing was chosen).
    fn install(&mut self) -> Task<Message> {
        let choices = self.choices();
        let profile_items = match self.profile {
            ProfileChoice::Ready(index) => self.profiles.get(index).map_or(0, Vec::len),
            _ => 0,
        };
        let steps = plan::steps(&choices, profile_items);
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
                if !ok && self.steps.get(index).is_some_and(|s| s.weight > 0) {
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
                row = row.push(widget::text::caption("›").class(dim_class()));
            }
            let label = tr(page.label());
            row = row.push(if index == current {
                widget::text::caption_heading(label).class(theme::Text::Accent)
            } else if index < current {
                widget::text::caption(label)
            } else {
                widget::text::caption(label).class(dim_class())
            });
        }
        row = row.push(widget::space::horizontal());
        row = row.push(widget::text::caption(format!("{} {}/{}", tr("Step"), current + 1, pages.len())).class(dim_class()));
        widget::column::with_capacity(2)
            .spacing(spacing.space_xxs)
            .push(row)
            .push(widget::progress_bar::linear::Linear::new().girth(4.0).progress((current + 1) as f32 / pages.len() as f32).width(Length::Fill))
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
        let mut methods = widget::column::with_capacity(Method::LEGEND.len() + 1).spacing(spacing.space_xs).push(widget::text::heading(tr("Every item says how it is installed")));
        for method in Method::LEGEND {
            methods = methods.push(
                widget::row::with_capacity(2)
                    .spacing(spacing.space_s)
                    .align_y(Alignment::Center)
                    .push(widget::container(badge(method)).width(Length::Fixed(BADGE_COLUMN)))
                    .push(widget::text::body(tr(method.explanation()))),
            );
        }
        let column = widget::column::with_capacity(6)
            .spacing(spacing.space_s)
            .push(widget::space::vertical().height(Length::Fixed(f32::from(spacing.space_s))))
            .push(widget::text::title1(tr("Welcome to Tamagi")))
            .push(
                widget::column::with_capacity(2)
                    .spacing(spacing.space_xxs)
                    .push(widget::text::body(tr("Atomic, lean system: the host is read-only and updates itself.")))
                    .push(widget::text::body(tr("Almost nothing comes installed: you choose what you want, now or later."))),
            )
            .push(panel(methods))
            .push(widget::text::body(tr("Pick a profile and a browser, then review. Nothing is installed before you confirm.")).class(dim_class()));
        scrolling(column)
    }

    fn view_profile(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        // The explanation sits beside the title: the six cards need the height.
        let title = widget::row::with_capacity(3)
            .spacing(spacing.space_s)
            .align_y(Alignment::End)
            .push(widget::text::title3(tr("Which profile?")))
            .push(widget::space::horizontal())
            .push(widget::text::caption(tr("A profile installs a ready-made set. You can apply others later: tamagi profiles")).class(dim_class()));
        let mut column = widget::column::with_capacity(7).spacing(spacing.space_xxs).push(title);
        for index in 0..PROFILES.len() {
            column = column.push(self.profile_card(index));
        }
        let small = |title: String, note: String, choice: ProfileChoice| {
            let content = widget::column::with_capacity(2).spacing(spacing.space_xxxs).push(widget::text::heading(title)).push(widget::text::caption(note).class(dim_class()));
            widget::button::custom(content)
                .class(card_class(self.profile == choice))
                .selected(self.profile == choice)
                .padding([spacing.space_xs, spacing.space_s])
                .width(Length::Fill)
                .on_press(Message::Profile(choice))
        };
        column = column.push(
            widget::row::with_capacity(2)
                .spacing(spacing.space_xxs)
                .push(small(capitalized(&tr("choose item by item from the catalog")), tr("Tick what you want, with sizes and install methods."), ProfileChoice::ItemByItem))
                .push(small(capitalized(&tr("not now")), tr("Install nothing from a profile."), ProfileChoice::NotNow)),
        );
        scrolling(column)
    }

    /// A profile: its name on the left, what it installs on the right, one
    /// line per install method.
    fn profile_card(&self, index: usize) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let (title, subtitle) = split_title(&tr(PROFILE_KEYS[index]));
        let entries = self.profiles.get(index).map(Vec::as_slice).unwrap_or(&[]);
        let choice = ProfileChoice::Ready(index);
        let selected = self.profile == choice;

        let groups = profile::by_method(entries);
        let total: f64 = entries.iter().filter_map(|e| e.size_mb).sum();
        let mut extra = widget::row::with_capacity(2).spacing(spacing.space_xs).align_y(Alignment::Center);
        if total > 0.0 {
            extra = extra.push(widget::text::caption(approx(total)).class(dim_class()));
        }
        if index == 0 {
            extra = extra.push(chip(tr("Suggested")));
        }
        // Size (and "Suggested") under the name; beside it when the card has
        // a single line, so it does not grow for that.
        let left: Element<'_, Message> = if groups.len() > 1 {
            widget::column::with_capacity(2).spacing(spacing.space_xxs).push(widget::text::heading(title)).push(extra).into()
        } else {
            widget::row::with_capacity(2).spacing(spacing.space_xs).align_y(Alignment::Center).push(widget::text::heading(title)).push(extra).into()
        };
        let left = widget::container(left).width(Length::Fixed(180.0));

        let right: Element<'_, Message> = if entries.is_empty() {
            // The profile file could not be read: its one-line description.
            widget::text::body(subtitle).width(Length::Fill).into()
        } else {
            let mut lines = widget::column::with_capacity(4).spacing(spacing.space_xxs).width(Length::Fill);
            for (method, group) in groups {
                let names: Vec<String> = group.iter().map(|e| e.name(tr)).collect();
                lines = lines.push(
                    widget::row::with_capacity(2)
                        .spacing(spacing.space_xs)
                        .push(widget::container(badge(method)).width(Length::Fixed(BADGE_COLUMN)))
                        .push(widget::text::body(names.join(", ")).width(Length::Fill)),
                );
            }
            lines.into()
        };

        widget::button::custom(widget::row::with_capacity(2).spacing(spacing.space_s).push(left).push(right))
            .class(card_class(selected))
            .selected(selected)
            .padding([spacing.space_xs, spacing.space_s])
            .width(Length::Fill)
            .on_press(Message::Profile(choice))
            .into()
    }

    /// The browser on the left, how to install it on the right.
    fn view_browser(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let none = BROWSERS.len();
        let mut browsers = widget::settings::section().title(tr("Which browser?"));
        for (index, (id, name)) in BROWSERS.iter().enumerate() {
            let size = self.catalog.iter().find(|item| item.id == *id).map(|item| format!("≈ {}", pretty_size(&item.size))).unwrap_or_default();
            let label = widget::row::with_capacity(3)
                .align_y(Alignment::Center)
                .push(widget::text::body(*name).width(Length::Fill))
                .push(widget::text::caption(size).class(dim_class()));
            browsers = browsers.add(choice(index, self.browser, Message::Browser, label));
        }
        browsers = browsers.add(choice(none, self.browser, Message::Browser, widget::text::body(capitalized(&tr("not now"))).width(Length::Fill)));

        let right: Element<'_, Message> = if self.browser == none {
            panel(widget::text::body(tr("No browser now. Later: tamagi install")).class(dim_class())).into()
        } else {
            let mut methods = widget::settings::section().title(tr("Install how?"));
            for (index, (flag, key)) in BROWSER_METHODS.iter().enumerate() {
                let method = classify(&browser_action("x", flag));
                let text = tr(key);
                let label = widget::column::with_capacity(2)
                    .spacing(spacing.space_xxs)
                    .push(badge(method))
                    .push(widget::text::body(capitalized(&tagline(&text).unwrap_or(text))).width(Length::Fill));
                methods = methods.add(choice(index, self.browser_method, Message::BrowserMethod, label.width(Length::Fill)));
            }
            methods.into()
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

    fn view_catalog(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let picked: Vec<&Item> = self.checked.iter().filter_map(|i| self.catalog.get(*i)).collect();
        let total: f64 = picked.iter().filter_map(|item| item.size_mb()).sum();
        let summary = if picked.is_empty() { tr("Nothing selected") } else { format!("{} {} · {}", picked.len(), tr("selected"), approx(total)) };
        let search = widget::row::with_capacity(2)
            .spacing(spacing.space_s)
            .align_y(Alignment::Center)
            .push(widget::search_input(tr("Search the catalog"), self.search.as_str()).id(self.search_id.clone()).on_input(Message::Search).on_clear(Message::Search(String::new())).width(Length::Fill))
            .push(widget::text::caption_heading(summary));

        let mut sections = widget::column::with_capacity(10).spacing(spacing.space_m);
        let mut shown = 0;
        for category in catalog::categories(&self.catalog) {
            let category_name = tr(category);
            let mut section = widget::settings::section().title(category_name.clone());
            let mut rows = 0;
            for (index, item) in self.catalog.iter().enumerate().filter(|(_, item)| item.category == category) {
                let desc = tr(&item.desc);
                let method = item.method();
                let method_name = tr(method.label());
                if !catalog::matches(&self.search, &[&desc, &category_name, &method_name, &item.id]) {
                    continue;
                }
                // What comes after the name; the badge already says "asks for
                // the password", so that note is dropped from the text.
                let note = tagline(&desc).map(|t| capitalized(if method.asks_password() { catalog::without_trailing_note(&t) } else { &t })).unwrap_or_default();
                let checked = self.checked.contains(&index);
                let row = widget::row::with_capacity(5)
                    .spacing(spacing.space_s)
                    .align_y(Alignment::Center)
                    .push(widget::checkbox(checked).on_toggle(move |on| Message::Toggle(index, on)))
                    .push(widget::text::body(short_name(&desc)).width(Length::Fixed(270.0)))
                    .push(widget::text::caption(note).class(dim_class()).width(Length::Fill))
                    .push(widget::container(badge(method)).width(Length::Fixed(BADGE_COLUMN)).align_x(Alignment::End))
                    .push(widget::text::caption(pretty_size(&item.size)).width(Length::Fixed(60.0)).align_x(Alignment::End));
                // The whole row is the control: click or Tab + Enter.
                section = section.add(widget::list::button(row).on_press(Message::Toggle(index, !checked)));
                rows += 1;
            }
            if rows > 0 {
                sections = sections.push(section);
                shown += rows;
            }
        }
        if shown == 0 {
            sections = sections.push(widget::text::body(tr("Nothing in the catalog matches the search.")).class(dim_class()));
        }

        widget::column::with_capacity(3)
            .spacing(spacing.space_s)
            .push(fixed(
                widget::column::with_capacity(2)
                    .spacing(spacing.space_xs)
                    .push(heading(tr("What to install"), tr("Everything here is optional and stays available later in: tamagi install")))
                    .push(search),
            ))
            .push(scrolling(sections))
            .into()
    }

    fn view_disk(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let question = widget::settings::section().add(
            widget::settings::item::builder(tr("Unlock the disk with the TPM chip at boot, without typing the password?"))
                .icon(widget::icon::from_name(LOCK_ICON).size(24).icon())
                .toggler(self.tpm, Message::Tpm),
        );
        let note = if self.tpm { tr("The password is asked for when this is set up.") } else { tr("Later: tamagi setup tpm-unlock on") };
        let column = widget::column::with_capacity(3)
            .spacing(spacing.space_m)
            .push(heading(tr("Encrypted disk"), String::new()))
            .push(question)
            .push(widget::text::body(note).class(dim_class()));
        scrolling(column)
    }

    /// Everything chosen, grouped by install method; the total and what asks
    /// for the password stay in view under the list.
    fn view_review(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let rows = self.review_rows();
        let tpm = self.disk_question && self.tpm;
        let subtitle = match self.profile {
            ProfileChoice::Ready(index) => format!("{}: {}", tr("Profile"), Self::profile_title(index)),
            _ => String::new(),
        };
        let title = fixed(heading(tr("Review"), subtitle));

        if rows.is_empty() && !tpm {
            let nothing = panel(
                widget::column::with_capacity(2)
                    .spacing(spacing.space_xxs)
                    .push(widget::text::heading(tr("Nothing selected: nothing will be installed.")))
                    .push(widget::text::body(tr("Everything here is optional and stays available later in: tamagi install")).class(dim_class())),
            );
            return widget::column::with_capacity(2).spacing(spacing.space_s).push(title).push(fixed(nothing)).into();
        }

        let mut groups = widget::column::with_capacity(6).spacing(spacing.space_s);
        for (method, group) in review::grouped(&rows) {
            let header = widget::row::with_capacity(2)
                .spacing(spacing.space_xs)
                .align_y(Alignment::Center)
                .push(badge(method))
                .push(widget::text::body(tr(method.explanation())).class(dim_class()));
            let mut section = widget::settings::section().header(header);
            for row in group {
                let size = row.size_mb.map(format_size).unwrap_or_default();
                section = section.add(
                    widget::row::with_capacity(2)
                        .align_y(Alignment::Center)
                        .push(widget::text::body(row.name.clone()).width(Length::Fill))
                        .push(widget::text::caption(size).class(dim_class())),
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
                        .push(widget::text::body(tr("Unlock the disk with the TPM chip at boot")).width(Length::Fill))
                        .push(widget::icon::from_name(LOCK_ICON).size(16).icon()),
                ),
            );
        }

        let mut names = review::password_names(&rows);
        if tpm {
            names.push(tr("Encrypted disk"));
        }
        let password: Element<'_, Message> = if names.is_empty() {
            widget::text::body(tr("Nothing here asks for the password.")).class(dim_class()).into()
        } else {
            widget::row::with_capacity(2)
                .spacing(spacing.space_xs)
                .push(widget::icon::from_name(LOCK_ICON).size(16).icon())
                .push(widget::text::body(format!("{}: {}", tr("Asks for the password"), names.join(", "))).width(Length::Fill))
                .into()
        };
        let summary = widget::column::with_capacity(2)
            .spacing(spacing.space_xxs)
            .push(
                widget::row::with_capacity(3)
                    .align_y(Alignment::Center)
                    .push(widget::text::heading(tr("Approximate download")))
                    .push(widget::space::horizontal())
                    .push(widget::text::heading(approx(review::total_mb(&rows)))),
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
            .push(widget::text::body(format!("{:.0}%", fraction * 100.0)).width(Length::Fixed(44.0)).align_x(Alignment::End));

        // The commands of the plan: done, running (with its current stage), to come.
        let mut list = widget::settings::section();
        for (index, step) in self.steps.iter().enumerate().filter(|(_, step)| step.weight > 0) {
            let state: Element<'_, Message> = match self.running {
                Some(current) if current == index => widget::progress_bar::circular::Circular::new().size(16.0).into(),
                Some(current) if current > index => widget::icon::from_name("object-select-symbolic").size(16).icon().into(),
                _ => widget::space::horizontal().width(Length::Fixed(16.0)).into(),
            };
            // The profile step says which profile.
            let title = match (step.stages, self.profile) {
                (plan::Stages::Counted, ProfileChoice::Ready(profile)) => format!("{}: {}", tr(step.title), Self::profile_title(profile)),
                _ => tr(step.title),
            };
            let mut text = widget::column::with_capacity(2).spacing(spacing.space_xxxs).width(Length::Fill).push(widget::text::body(title));
            if self.running == Some(index) && !self.stage.is_empty() {
                text = text.push(widget::text::caption(self.stage.clone()).class(dim_class()));
            }
            list = list.add(widget::row::with_capacity(2).spacing(spacing.space_s).align_y(Alignment::Center).push(widget::container(state).width(Length::Fixed(16.0))).push(text));
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
        let text = widget::text::monotext(self.log.join("\n")).size(12).width(Length::Fill);
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
                lines = lines.push(widget::text::body(failure.clone()));
            }
            lines = lines.push(widget::text::body(format!("{}: tamagi install", tr("Try again with"))).class(dim_class()));
            column = column.push(panel(lines));
        }
        let mut tips = widget::settings::section().title(tr("Good to know"));
        for tip in [
            "Super+Space opens the menu · Super+A the command palette · Super+K the shortcuts.",
            "Themes and wallpapers: COSMIC Settings (Appearance and Wallpaper).",
            "AI agents jailed in the project: tamagi agents run claude (inside the project folder).",
            "Every command of the system: tamagi",
        ] {
            tips = tips.add(widget::text::body(tr(tip)).width(Length::Fill));
        }
        column = column.push(tips);
        if self.installed && !self.log.is_empty() {
            let details = if self.show_log { tr("Hide details") } else { tr("Show details") };
            column = column.push(widget::row::with_capacity(2).push(widget::space::horizontal()).push(widget::button::text(details).on_press(Message::ToggleLog)));
            if self.show_log {
                column = column.push(panel(widget::text::monotext(self.log.join("\n")).size(12).width(Length::Fill)));
            }
        }
        scrolling(column)
    }
}
