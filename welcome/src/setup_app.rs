//! The installer window of the live media (`tamagi-welcome --install`), in
//! eight steps: Welcome (with the language) → Region (keyboard and time zone)
//! → Account → Disk → Encryption → Review → Installing → Done. The disk comes
//! before the encryption: an encrypted home folder kept on it already has its
//! password, and then only how it opens at startup is asked.
//!
//! Like the installer of openSUSE Aeon: a few questions, then the disk, and
//! nothing is touched before the review is confirmed. The work is done by
//! `/usr/libexec/tamagi-install` (see `setup` for the answers it gets).

use crate::i18n::{self, tr};
use crate::setup::{self, check_username, default_keyboard, default_timezone, filter_zones, format_bytes, suggest_username, Answers, Disk, Found, TpmMode, KEYBOARDS, LANGS, MIN_DISK_PASSWORD, MIN_TPM_PIN};
use crate::setup_system::{self as sys, Event};
use crate::m3;
use crate::ui::SMALL;
use cosmic::app::{Core, Settings, Task};
use cosmic::iced::futures::channel::{mpsc, oneshot};
use cosmic::iced::futures::StreamExt;
use cosmic::iced::{keyboard, Alignment, Length, Size, Subscription};
use cosmic::{executor, theme, widget, Application, ApplicationExt, Element};

const CONTENT_WIDTH: f32 = 860.0;
const LOG_LINES: usize = 400;
/// Zones listed at once on the time zone page (the search narrows them).
const ZONES_SHOWN: usize = 120;

pub fn run() -> cosmic::iced::Result {
    let settings = Settings::default().size(Size::new(920.0, 700.0)).size_limits(cosmic::iced::Limits::NONE.min_width(640.0).min_height(480.0));
    cosmic::app::run::<Setup>(settings, ())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Welcome,
    Region,
    Account,
    Encryption,
    Disk,
    Review,
    Installing,
    Done,
}

const PAGES: [Page; 8] = [Page::Welcome, Page::Region, Page::Account, Page::Disk, Page::Encryption, Page::Review, Page::Installing, Page::Done];

impl Page {
    fn label(self) -> &'static str {
        match self {
            Page::Welcome => "Welcome",
            Page::Region => "Keyboard and time zone",
            Page::Account => "Your account",
            Page::Encryption => "Encryption",
            Page::Disk => "Disk",
            Page::Review => "Review",
            Page::Installing => "Installing",
            Page::Done => "Done",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Next,
    Back,
    Close,
    Lang(usize),
    Keyboard(usize),
    ZoneQuery(String),
    Zone(usize),
    FullName(String),
    User(String),
    Pass1(String),
    Pass2(String),
    ShowPass,
    Encrypt(bool),
    Luks1(String),
    Luks2(String),
    Tpm(TpmMode),
    Pin1(String),
    Pin2(String),
    Disk(usize),
    Detected(String, Found, i32),
    OldLuks(String),
    Unlock,
    Keep(bool),
    Run(Event),
    ToggleLog,
    Reboot,
}

pub struct Setup {
    core: Core,
    page: Page,
    lang: usize,
    keyboard: usize,
    keyboard_touched: bool,
    zones: Vec<String>,
    zone: usize,
    zone_touched: bool,
    zone_query: String,
    fullname: String,
    user: String,
    user_touched: bool,
    pass1: String,
    pass2: String,
    show_pass: bool,
    encrypt: bool,
    luks1: String,
    luks2: String,
    tpm_present: bool,
    tpm: TpmMode,
    pin1: String,
    pin2: String,
    disks: Vec<Disk>,
    disk: Option<usize>,
    detecting: bool,
    found: Option<Found>,
    old_luks: String,
    unlock_error: bool,
    keep: bool,
    error: Option<String>,
    // Installing
    progress: u8,
    stage: String,
    log: Vec<String>,
    show_log: bool,
    result: Option<i32>,
}

fn scrolling<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    widget::scrollable(widget::container(content).width(Length::Fill).padding([0, theme::spacing().space_xs, 0, 0])).width(Length::Fill).height(Length::Fill).into()
}

impl Application for Setup {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "org.tamagi.Installer";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: ()) -> (Self, Task<Message>) {
        let default = std::fs::read_to_string("/usr/share/tamagi/default-lang").ok().map(|s| s.trim().to_string()).unwrap_or_else(|| i18n::lang().to_string());
        let lang = LANGS.iter().position(|(code, _, _)| *code == default).unwrap_or(0);
        i18n::set_lang(LANGS[lang].0);
        let zones = sys::timezones();
        let zone = zones.iter().position(|z| z == default_timezone(LANGS[lang].0)).unwrap_or(0);
        sys::start_media_check(&sys::work_dir().join("verify"));
        // Secrets left by a window that was killed: never reused.
        for name in ["luks.key", "existing-luks.key", "tpm.pin", "answers"] {
            let _ = std::fs::remove_file(sys::work_dir().join(name));
        }
        let mut app = Setup {
            core,
            page: Page::Welcome,
            lang,
            keyboard: default_keyboard(LANGS[lang].0),
            keyboard_touched: false,
            zones,
            zone,
            zone_touched: false,
            zone_query: String::new(),
            fullname: String::new(),
            user: String::new(),
            user_touched: false,
            pass1: String::new(),
            pass2: String::new(),
            show_pass: false,
            encrypt: false,
            luks1: String::new(),
            luks2: String::new(),
            tpm_present: sys::has_tpm2(),
            tpm: TpmMode::Pin,
            pin1: String::new(),
            pin2: String::new(),
            disks: sys::disks(),
            disk: None,
            detecting: false,
            found: None,
            old_luks: String::new(),
            unlock_error: false,
            keep: false,
            error: None,
            progress: 0,
            stage: String::new(),
            log: Vec::new(),
            show_log: false,
            result: None,
        };
        // One disk that fits: chosen already.
        if app.disks.iter().filter(|d| d.big_enough()).count() == 1 {
            let index = app.disks.iter().position(|d| d.big_enough());
            let task = index.map(|i| app.select_disk(i)).unwrap_or_else(Task::none);
            let title = app.retitle();
            return (app, Task::batch([task, title]));
        }
        let title = app.retitle();
        (app, title)
    }

    fn on_escape(&mut self) -> Task<Message> {
        cosmic::task::message(Message::Back)
    }

    fn subscription(&self) -> Subscription<Message> {
        use keyboard::key::Named;
        cosmic::iced::event::listen_with(|event, status, _window| {
            let cosmic::iced::Event::Keyboard(keyboard::Event::KeyPressed { key: keyboard::Key::Named(key), modifiers, .. }) = event else {
                return None;
            };
            let unused = status == cosmic::iced::event::Status::Ignored;
            match key {
                Named::Enter if modifiers.control() || unused => Some(Message::Next),
                Named::ArrowRight if modifiers.alt() => Some(Message::Next),
                Named::ArrowLeft if modifiers.alt() => Some(Message::Back),
                _ => None,
            }
        })
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Next => return self.next(),
            Message::Back => {
                if !matches!(self.page, Page::Installing | Page::Done) {
                    if let Some(pos) = PAGES.iter().position(|p| *p == self.page).and_then(|p| p.checked_sub(1)) {
                        self.error = None;
                        return self.show(PAGES[pos]);
                    }
                }
            }
            Message::Close => {
                if self.page != Page::Installing {
                    self.forget_secrets();
                    return cosmic::iced::exit();
                }
            }
            Message::Lang(index) => {
                self.lang = index.min(LANGS.len() - 1);
                let code = LANGS[self.lang].0;
                i18n::set_lang(code);
                if !self.keyboard_touched {
                    self.keyboard = default_keyboard(code);
                }
                if !self.zone_touched {
                    self.zone = self.zones.iter().position(|z| z == default_timezone(code)).unwrap_or(self.zone);
                }
                return self.retitle();
            }
            Message::Keyboard(index) => {
                self.keyboard = index.min(KEYBOARDS.len() - 1);
                self.keyboard_touched = true;
            }
            Message::ZoneQuery(query) => self.zone_query = query,
            Message::Zone(index) => {
                self.zone = index;
                self.zone_touched = true;
            }
            Message::FullName(name) => {
                if !self.user_touched {
                    self.user = suggest_username(&name);
                }
                self.fullname = name;
            }
            Message::User(user) => {
                self.user = user.trim().to_string();
                self.user_touched = true;
            }
            Message::Pass1(p) => self.pass1 = p,
            Message::Pass2(p) => self.pass2 = p,
            Message::ShowPass => self.show_pass = !self.show_pass,
            Message::Encrypt(on) => self.encrypt = on,
            Message::Luks1(p) => self.luks1 = p,
            Message::Luks2(p) => self.luks2 = p,
            Message::Tpm(mode) => self.tpm = mode,
            Message::Pin1(p) => self.pin1 = p,
            Message::Pin2(p) => self.pin2 = p,
            Message::Disk(index) => return self.select_disk(index),
            Message::Detected(path, found, code) => {
                if self.disk.and_then(|i| self.disks.get(i)).is_some_and(|d| d.path == path) {
                    self.detecting = false;
                    self.unlock_error = code == 2;
                    if code == 2 {
                        // The old found (locked) stays: the password can be typed again.
                        return Task::none();
                    }
                    self.keep = found.preservable();
                    self.found = Some(found);
                }
            }
            Message::OldLuks(p) => self.old_luks = p,
            Message::Unlock => {
                if let Some(disk) = self.disk.and_then(|i| self.disks.get(i)).cloned() {
                    let file = sys::work_dir().join("existing-luks.key");
                    if sys::write_secret(&file, &self.old_luks) {
                        self.detecting = true;
                        self.unlock_error = false;
                        return Self::detect(disk.path, Some(file.to_string_lossy().into_owned()));
                    }
                }
            }
            Message::Keep(on) => self.keep = on,
            Message::Run(event) => match event {
                Event::Line(line) => {
                    if let Some((value, message)) = setup::parse_progress(&line) {
                        self.progress = value;
                        if !message.is_empty() {
                            self.stage = message;
                        }
                    } else {
                        self.push_log(line);
                    }
                }
                Event::Ended(code) => {
                    self.result = Some(code);
                    self.forget_secrets();
                    self.page = Page::Done;
                }
            },
            Message::ToggleLog => self.show_log = !self.show_log,
            Message::Reboot => sys::reboot(),
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let page = match self.page {
            Page::Welcome => self.view_welcome(),
            Page::Region => self.view_region(),
            Page::Account => self.view_account(),
            Page::Encryption => self.view_encryption(),
            Page::Disk => self.view_disk(),
            Page::Review => self.view_review(),
            Page::Installing => self.view_installing(),
            Page::Done => self.view_done(),
        };
        let mut content = widget::column::with_capacity(4).spacing(spacing.space_m).max_width(CONTENT_WIDTH).push(self.view_steps()).push(widget::container(page).width(Length::Fill).height(Length::Fill));
        if let Some(error) = &self.error {
            content = content.push(m3::error(error.clone()));
        }
        content = content.push(self.view_footer());
        widget::container(content).width(Length::Fill).height(Length::Fill).align_x(Alignment::Center).padding([spacing.space_s, spacing.space_l, spacing.space_m, spacing.space_l]).into()
    }
}

impl Setup {
    fn lang_code(&self) -> &'static str {
        LANGS[self.lang].0
    }

    fn retitle(&mut self) -> Task<Message> {
        let title = tr("Install Tamagi");
        self.set_header_title(title.clone());
        match self.core.main_window_id() {
            Some(id) => self.set_window_title(title, id),
            None => Task::none(),
        }
    }

    fn show(&mut self, page: Page) -> Task<Message> {
        self.page = page;
        cosmic::iced::runtime::task::widget(cosmic::iced::core::widget::operation::focusable::unfocus())
    }

    fn push_log(&mut self, line: String) {
        if self.log.len() >= LOG_LINES {
            self.log.remove(0);
        }
        self.log.push(line);
    }

    fn forget_secrets(&mut self) {
        self.pass1.clear();
        self.pass2.clear();
        self.luks1.clear();
        self.luks2.clear();
        self.old_luks.clear();
        self.pin1.clear();
        self.pin2.clear();
        let dir = sys::work_dir();
        for name in ["luks.key", "existing-luks.key", "tpm.pin", "answers"] {
            let _ = std::fs::remove_file(dir.join(name));
        }
    }

    fn detect(path: String, keyfile: Option<String>) -> Task<Message> {
        let (sender, receiver) = oneshot::channel();
        std::thread::spawn(move || {
            let (found, code) = sys::detect_home(&path, keyfile.as_deref());
            let _ = sender.send(Message::Detected(path, found, code));
        });
        Task::perform(receiver, |m| cosmic::Action::App(m.unwrap_or(Message::Back)))
    }

    fn select_disk(&mut self, index: usize) -> Task<Message> {
        let Some(disk) = self.disks.get(index).cloned() else { return Task::none() };
        if self.disk == Some(index) {
            return Task::none();
        }
        self.disk = Some(index);
        self.found = None;
        self.keep = false;
        self.old_luks.clear();
        // The password of another disk's encrypted partition goes too.
        let _ = std::fs::remove_file(sys::work_dir().join("existing-luks.key"));
        self.unlock_error = false;
        self.error = None;
        if !disk.big_enough() {
            return Task::none();
        }
        self.detecting = true;
        Self::detect(disk.path, None)
    }

    /// Checks the page and goes on, or says what is missing.
    fn next(&mut self) -> Task<Message> {
        self.error = None;
        let problem: Option<String> = match self.page {
            Page::Welcome if !sys::uefi() => Some(tr("Tamagi needs a computer started in UEFI mode. Check the firmware (BIOS) options.")),
            Page::Region => {
                sys::apply_keyboard(&KEYBOARDS[self.keyboard]);
                None
            }
            Page::Account => {
                let fullname_user = if self.user.is_empty() { suggest_username(&self.fullname) } else { self.user.clone() };
                self.user = fullname_user;
                if let Err(key) = check_username(&self.user, sys::user_taken) {
                    Some(tr(key))
                } else if self.pass1.is_empty() {
                    Some(tr("The password cannot be empty."))
                } else if self.pass1 != self.pass2 {
                    Some(tr("The two passwords are not the same."))
                } else {
                    None
                }
            }
            Page::Encryption => {
                if self.new_encryption() && self.luks1.chars().count() < MIN_DISK_PASSWORD {
                    Some(tr("The disk password needs at least 8 characters."))
                } else if self.new_encryption() && self.luks1 != self.luks2 {
                    Some(tr("The two passwords are not the same."))
                } else if self.tpm_mode() == TpmMode::Pin && self.pin1.chars().count() < MIN_TPM_PIN {
                    Some(tr("The PIN needs at least 6 characters."))
                } else if self.tpm_mode() == TpmMode::Pin && self.pin1 != self.pin2 {
                    Some(tr("The two PINs are not the same."))
                } else {
                    None
                }
            }
            Page::Disk => match self.disk.and_then(|i| self.disks.get(i)) {
                None => Some(tr("Choose a disk.")),
                Some(d) if !d.big_enough() => Some(tr("This disk is too small: Tamagi needs at least 24 GB.")),
                Some(_) if self.detecting => Some(tr("Looking at the disk… one moment.")),
                Some(_) => None,
            },
            Page::Review => return self.start(),
            Page::Installing => return Task::none(),
            Page::Done => {
                if self.result == Some(0) {
                    sys::reboot();
                    return Task::none();
                }
                return cosmic::iced::exit();
            }
            _ => None,
        };
        if let Some(problem) = problem {
            self.error = Some(problem);
            return Task::none();
        }
        match PAGES.iter().position(|p| *p == self.page) {
            Some(pos) if pos + 1 < PAGES.len() => self.show(PAGES[pos + 1]),
            _ => Task::none(),
        }
    }

    fn keeping(&self) -> Option<&Found> {
        self.found.as_ref().filter(|f| self.keep && f.preservable())
    }

    /// An encrypted home folder is kept: it keeps its own password.
    fn keeping_luks(&self) -> bool {
        self.keeping().is_some_and(|f| f.get("PRESERVE_LUKS") == "1")
    }

    /// A home folder without encryption is kept: encrypting would erase it.
    fn keeping_plain(&self) -> bool {
        self.keeping().is_some() && !self.keeping_luks()
    }

    /// A new disk password is asked (the disk is erased and encrypted).
    fn new_encryption(&self) -> bool {
        self.encrypt && self.keeping().is_none()
    }

    /// The disk ends up encrypted: a new password, or an encrypted home kept.
    fn encrypted(&self) -> bool {
        self.keeping_luks() || self.new_encryption()
    }

    /// How the disk will open: no TPM chip or no encryption, the password.
    fn tpm_mode(&self) -> TpmMode {
        if self.tpm_present && self.encrypted() { self.tpm } else { TpmMode::Off }
    }

    fn start(&mut self) -> Task<Message> {
        let Some(disk) = self.disk.and_then(|i| self.disks.get(i)).cloned() else { return Task::none() };
        let dir = sys::work_dir();
        let Some(hash) = sys::hash_password(&self.pass1) else {
            self.error = Some(tr("Could not prepare the password (openssl)."));
            return Task::none();
        };
        let mut answers = Answers {
            lang: self.lang_code().to_string(),
            keyboard: self.keyboard,
            timezone: self.zones.get(self.zone).cloned().unwrap_or_else(|| "UTC".into()),
            user: self.user.clone(),
            fullname: self.fullname.clone(),
            password_hash: hash,
            tpm: self.tpm_mode(),
            keep: self.keeping().cloned(),
            verify_state: dir.join("verify").to_string_lossy().into_owned(),
            ..Answers::default()
        };
        if self.new_encryption() {
            let file = dir.join("luks.key");
            if !sys::write_secret(&file, &self.luks1) {
                self.error = Some(tr("Could not prepare the disk password."));
                return Task::none();
            }
            answers.luks_keyfile = Some(file.to_string_lossy().into_owned());
        }
        // The password of the encrypted partition kept, only when it is kept.
        let existing = dir.join("existing-luks.key");
        if self.keeping_luks() && existing.is_file() {
            answers.existing_keyfile = Some(existing.to_string_lossy().into_owned());
        } else {
            let _ = std::fs::remove_file(&existing);
        }
        if self.tpm_mode() == TpmMode::Pin {
            let file = dir.join("tpm.pin");
            if !sys::write_secret(&file, &self.pin1) {
                self.error = Some(tr("Could not prepare the disk password."));
                return Task::none();
            }
            answers.tpm_pinfile = Some(file.to_string_lossy().into_owned());
        }
        let answers_path = dir.join("answers");
        if !sys::write_secret(&answers_path, &answers.to_text()) {
            self.error = Some(tr("Could not write the answers of the installation."));
            return Task::none();
        }
        self.page = Page::Installing;
        self.progress = 0;
        self.stage = tr("Preparing…");
        self.log.clear();
        let log = dir.join("install.log");
        let (sender, receiver) = mpsc::unbounded();
        std::thread::spawn(move || {
            sys::install(&disk.path, &answers_path, &log, move |event| {
                let _ = sender.unbounded_send(event);
            });
        });
        Task::stream(receiver.map(|event| cosmic::Action::App(Message::Run(event))))
    }

    // ── Around the page ────────────────────────────────────────────────────

    fn view_steps(&self) -> Element<'_, Message> {
        let current = PAGES.iter().position(|p| *p == self.page).unwrap_or(0);
        m3::step_header(current + 1, PAGES.len(), tr("Step"), tr(self.page.label()))
    }

    fn view_footer(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let mut row = widget::row::with_capacity(4).spacing(spacing.space_xs).align_y(Alignment::Center);
        match self.page {
            Page::Welcome => {
                row = row
                    .push(widget::space::horizontal())
                    .push(widget::button::standard(tr("Just try it")).on_press(Message::Close))
                    .push(widget::button::suggested(tr("Install now")).on_press(Message::Next));
            }
            Page::Review => {
                let label = if self.keeping().is_some() { tr("Reinstall, keeping my files") } else { tr("Erase the disk and install") };
                // Red only for what erases everything; keeping the home folder is the safe path.
                let go = if self.keeping().is_some() { widget::button::suggested(label) } else { widget::button::destructive(label) };
                row = row.push(widget::button::text(tr("Back")).on_press(Message::Back)).push(widget::space::horizontal()).push(go.on_press(Message::Next));
            }
            Page::Installing => {}
            Page::Done => {
                if self.result == Some(0) {
                    row = row
                        .push(widget::space::horizontal())
                        .push(widget::button::standard(tr("Stay in live mode")).on_press(Message::Close))
                        .push(widget::button::suggested(tr("Restart now")).on_press(Message::Reboot));
                } else {
                    row = row.push(widget::space::horizontal()).push(widget::button::standard(tr("Close")).on_press(Message::Close));
                }
            }
            _ => {
                row = row.push(widget::button::text(tr("Back")).on_press(Message::Back)).push(widget::space::horizontal()).push(widget::button::suggested(tr("Next")).on_press(Message::Next));
            }
        }
        row.into()
    }

    // ── Pages ──────────────────────────────────────────────────────────────

    fn page<'a>(hero: Element<'a, Message>, body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
        scrolling(widget::column::with_capacity(2).spacing(theme::spacing().space_l).push(hero).push(body))
    }

    /// The first page: the language (asked in every language) and what the
    /// installer does.
    fn view_welcome(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let items = LANGS.iter().enumerate().map(|(index, (_, name, _))| m3::list_item(index == self.lang, name.to_string(), None, None, Message::Lang(index))).collect();
        let body = widget::column::with_capacity(4)
            .spacing(spacing.space_s)
            .push(m3::label("Language · Idioma · Langue"))
            .push(m3::list(items))
            .push(widget::space::vertical().height(Length::Fixed(spacing.space_xxs.into())))
            .push(m3::supporting(format!(
                "{} {}",
                tr("The disk only starts Tamagi at the very last step. If anything stops halfway, it is not left with a half-installed system."),
                tr("To only try the system, choose Just try it. The installer stays in the launcher: Install Tamagi.")
            )));
        Self::page(
            m3::hero("tamagi-menu", tr("Welcome to Tamagi"), tr("This assistant installs Tamagi on this computer. Just a few questions: keyboard, time zone, your account and the disk.")),
            body,
        )
    }

    /// Keyboard (a drop-down: it already follows the language) and time zone.
    fn view_region(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let chosen = self.zones.get(self.zone).cloned().unwrap_or_default().replace('_', " ");
        let matches = filter_zones(&self.zones, &self.zone_query);
        let items: Vec<m3::Item<Message>> = matches
            .iter()
            .take(ZONES_SHOWN)
            .map(|zone| {
                let index = self.zones.iter().position(|z| z == *zone).unwrap_or(0);
                let (region, city) = zone.rsplit_once('/').unwrap_or(("", zone.as_str()));
                m3::list_item(index == self.zone, city.replace('_', " "), Some(region.replace('_', " ").replace('/', " · ")), None, Message::Zone(index))
            })
            .collect();
        let search = widget::search_input(tr("Search a city or zone"), &self.zone_query).on_input(Message::ZoneQuery).on_clear(Message::ZoneQuery(String::new()));
        let names: Vec<&'static str> = KEYBOARDS.iter().map(|kb| kb.name).collect();
        let keyboard = m3::card(
            widget::row::with_capacity(2)
                .spacing(spacing.space_s)
                .align_y(Alignment::Center)
                .push(widget::column::with_capacity(2).spacing(2).width(Length::Fill).push(m3::body(tr("Keyboard"))).push(m3::supporting(tr("Which keyboard does this computer have?"))))
                .push(widget::dropdown(names, Some(self.keyboard), Message::Keyboard)),
        );
        let mut column = widget::column::with_capacity(6)
            .spacing(spacing.space_m)
            .push(m3::hero("input-keyboard-symbolic", tr("Keyboard and time zone"), String::new()))
            .push(keyboard)
            .push(widget::row::with_capacity(2).spacing(spacing.space_s).push(m3::body(tr("Time zone"))).push(m3::supporting(format!("{}: {}", tr("Chosen"), chosen))))
            .push(search);
        if matches.is_empty() {
            column = column.push(m3::supporting(tr("Nothing found.")));
        }
        column.push(scrolling(m3::list(items))).into()
    }

    fn view_account(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let fields = widget::column::with_capacity(4)
            .spacing(spacing.space_s)
            .push(widget::text_input(tr("Full name"), &self.fullname).label(tr("Full name")).on_input(Message::FullName))
            .push(widget::text_input("maria", &self.user).label(tr("Username")).on_input(Message::User))
            .push(widget::secure_input(tr("Password"), &self.pass1, Some(Message::ShowPass), !self.show_pass).label(tr("Password")).on_input(Message::Pass1))
            .push(widget::secure_input(tr("Repeat the password"), &self.pass2, Some(Message::ShowPass), !self.show_pass).label(tr("Repeat the password")).on_input(Message::Pass2));
        Self::page(m3::hero("system-users-symbolic", tr("Your account"), tr("This will be the administrator account of the computer.")), m3::card(fields))
    }

    fn view_encryption(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let mut body = widget::column::with_capacity(4).spacing(spacing.space_m);
        if self.keeping_luks() {
            // Said in the title line, to leave room for the PIN.
        } else if self.keeping_plain() {
            body = body.push(m3::notice_card("dialog-information-symbolic", tr("The files kept on this disk are not encrypted. Encrypting needs erasing the disk.")));
        } else {
            let mut card = widget::column::with_capacity(4).spacing(spacing.space_s).push(m3::switch_row(tr("Encrypt the disk (LUKS2)"), None, self.encrypt, Message::Encrypt));
            if self.encrypt {
                let warning = tr("Without the disk password there is no way to recover the files.");
                card = card
                    .push(widget::secure_input(tr("Disk password"), &self.luks1, Some(Message::ShowPass), !self.show_pass).label(tr("Disk password (at least 8 characters)")).on_input(Message::Luks1))
                    .push(widget::secure_input(tr("Repeat the password"), &self.luks2, Some(Message::ShowPass), !self.show_pass).label(tr("Repeat the password")).on_input(Message::Luks2))
                    .push(m3::supporting(warning));
            }
            body = body.push(m3::card(card));
        }
        if self.encrypted() && self.tpm_present {
            let modes = [
                (TpmMode::Pin, tr("TPM chip + PIN (recommended)"), tr("A short PIN at startup. Safe if the computer is stolen, and updates never lock the disk.")),
                (TpmMode::Auto, tr("TPM chip, automatic"), tr("Opens by itself. Protects a disk taken out of the computer, not a stolen computer.")),
                (TpmMode::Off, tr("Disk password only"), tr("The long password at every startup.")),
            ];
            let items = modes.into_iter().map(|(mode, name, text)| m3::list_item(self.tpm == mode, name, Some(text), None, Message::Tpm(mode))).collect();
            body = body.push(widget::column::with_capacity(2).spacing(spacing.space_s).push(m3::label(tr("Unlock at startup"))).push(m3::list(items)));
            if self.tpm == TpmMode::Pin {
                let fields = widget::row::with_capacity(2)
                    .spacing(spacing.space_s)
                    .push(widget::secure_input(tr("PIN (at least 6 characters)"), &self.pin1, Some(Message::ShowPass), !self.show_pass).on_input(Message::Pin1).width(Length::Fill))
                    .push(widget::secure_input(tr("Repeat the PIN"), &self.pin2, Some(Message::ShowPass), !self.show_pass).on_input(Message::Pin2).width(Length::Fill));
                body = body.push(m3::card(widget::column::with_capacity(2).spacing(spacing.space_xs).push(fields).push(m3::supporting(tr("The disk password stays as the recovery key.")))));
            }
        }
        let subtitle = if self.keeping_luks() { tr("This disk is already encrypted and keeps its current password.") } else { tr("With encryption, nobody can read your files if the computer is lost or stolen.") };
        Self::page(m3::hero("system-lock-screen-symbolic", tr("Encryption"), subtitle), body)
    }

    fn view_disk(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let items: Vec<m3::Item<Message>> = self
            .disks
            .iter()
            .enumerate()
            .map(|(index, disk)| {
                let mut detail = vec![disk.path.clone()];
                if !disk.transport.is_empty() {
                    detail.push(disk.transport.to_uppercase());
                }
                if disk.removable {
                    detail.push(tr("removable"));
                }
                if !disk.big_enough() {
                    detail.push(tr("too small"));
                }
                let name = if disk.model.is_empty() { disk.path.clone() } else { disk.model.clone() };
                m3::list_item(self.disk == Some(index), name, Some(detail.join(" · ")), Some(format_bytes(disk.bytes)), Message::Disk(index)).disabled(!disk.big_enough())
            })
            .collect();
        // The cards below are about the selected disk: they say its name.
        let selected_name = self.disk.and_then(|i| self.disks.get(i)).map(|d| if d.model.is_empty() { d.path.clone() } else { format!("{} · {}", d.model, d.path) }).unwrap_or_default();
        let mut body = widget::column::with_capacity(3).spacing(spacing.space_m);
        body = if self.disks.is_empty() { body.push(m3::card(m3::body(tr("No disk found for the installation.")))) } else { body.push(m3::list(items)) };
        if self.detecting {
            body = body.push(widget::row::with_capacity(2).spacing(spacing.space_s).align_y(Alignment::Center).push(widget::progress_bar::circular::Circular::new().size(18.0)).push(m3::supporting(tr("Looking at the disk…"))));
        } else if let Some(found) = &self.found {
            if found.locked() {
                let mut unlock = widget::column::with_capacity(5)
                    .spacing(spacing.space_s)
                    .push(m3::supporting(selected_name.clone()))
                    .push(m3::label(tr("This disk has an encrypted partition")))
                    .push(m3::supporting(tr("Type its current password to look for your files and keep them. Or go on: the disk is erased.")))
                    .push(
                        widget::row::with_capacity(2)
                            .spacing(spacing.space_xs)
                            .align_y(Alignment::Center)
                            .push(widget::secure_input(tr("Current disk password"), &self.old_luks, Some(Message::ShowPass), !self.show_pass).on_input(Message::OldLuks).on_submit(|_| Message::Unlock))
                            .push(widget::button::standard(tr("Unlock")).on_press(Message::Unlock)),
                    );
                if self.unlock_error {
                    unlock = unlock.push(m3::error(tr("Incorrect password for the encrypted partition.")));
                }
                body = body.push(m3::card(unlock));
            } else if found.preservable() {
                body = body.push(m3::card(m3::switch_row(
                    tr("Keep my files (/var/home) and reinstall only the system"),
                    Some(format!("{} · {}: {}", selected_name, tr("Users found"), found.users())),
                    self.keep,
                    Message::Keep,
                )));
            }
        }
        Self::page(m3::hero("drive-harddisk-symbolic", tr("Disk"), tr("Choose the disk where Tamagi will be installed. Everything on it will be erased, unless you keep your files below.")), body)
    }

    fn view_review(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let disk = self.disk.and_then(|i| self.disks.get(i));
        let disk_name = disk.map(|d| format!("{} · {} · {}", if d.model.is_empty() { &d.path } else { &d.model }, d.path, format_bytes(d.bytes))).unwrap_or_default();
        let yes_no = |on: bool| if on { tr("yes") } else { tr("no") };
        let mut rows: Vec<(String, String)> = vec![
            (tr("Disk"), disk_name),
            (tr("Language"), LANGS[self.lang].1.to_string()),
            (tr("Keyboard"), KEYBOARDS[self.keyboard].name.to_string()),
            (tr("Time zone"), self.zones.get(self.zone).cloned().unwrap_or_default().replace('_', " ")),
            (tr("Your account"), if self.fullname.is_empty() { self.user.clone() } else { format!("{} ({})", setup::clean_fullname(&self.fullname), self.user) }),
            (tr("Encryption"), yes_no(self.encrypted())),
        ];
        if self.encrypted() {
            let unlock = match self.tpm_mode() {
                TpmMode::Pin => tr("TPM chip + PIN"),
                TpmMode::Auto => tr("TPM chip, automatic"),
                TpmMode::Off => tr("Disk password only"),
            };
            rows.push((tr("Unlock at startup"), unlock));
        }
        let mut grid = widget::column::with_capacity(rows.len() * 2).spacing(spacing.space_xs);
        for (index, (label, value)) in rows.into_iter().enumerate() {
            if index > 0 {
                grid = grid.push(widget::divider::horizontal::light());
            }
            grid = grid.push(m3::info_row(label, value));
        }
        // Erasing is an error-coloured warning; keeping the home folder is a notice.
        let warning = match self.keeping() {
            Some(found) => m3::notice_card("dialog-information-symbolic", format!("{} ({}: {})", tr("The system on this disk is replaced and your home folder is kept"), tr("Users found"), found.users())),
            None => m3::warning_card("dialog-warning-symbolic", tr("Everything on this disk will be erased. Once it starts there is no way to undo it.")),
        };
        // The warning first: what cannot be undone is read before the summary.
        let body = widget::column::with_capacity(2).spacing(spacing.space_m).push(warning).push(m3::card(grid));
        Self::page(m3::hero("emblem-documents-symbolic", tr("Review"), String::new()), body)
    }

    fn view_installing(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        let fraction = f32::from(self.progress) / 100.0;
        let progress = m3::card(
            widget::column::with_capacity(2)
                .spacing(spacing.space_s)
                .push(widget::row::with_capacity(2).align_y(Alignment::Center).push(m3::body(self.stage.clone()).width(Length::Fill)).push(m3::label(format!("{}%", self.progress))))
                .push(m3::progress(fraction)),
        );
        let details = if self.show_log { tr("Hide details") } else { tr("Show details") };
        let mut column = widget::column::with_capacity(4)
            .spacing(spacing.space_l)
            .push(m3::hero("folder-download-symbolic", tr("Installing Tamagi"), tr("This takes a few minutes. Do not turn the computer off.")))
            .push(progress)
            .push(widget::row::with_capacity(2).push(widget::space::horizontal()).push(widget::button::text(details).on_press(Message::ToggleLog)));
        if self.show_log {
            column = column.push(m3::card(widget::scrollable(widget::text::monotext(self.log.join("\n")).size(SMALL).width(Length::Fill)).anchor_bottom().width(Length::Fill).height(Length::Fill)).height(Length::Fill));
        }
        widget::container(column).width(Length::Fill).into()
    }

    fn view_done(&self) -> Element<'_, Message> {
        let spacing = theme::spacing();
        if self.result == Some(0) {
            return Self::page(m3::hero("emblem-ok-symbolic", tr("Tamagi has been installed"), tr("Remove the installation media when the computer restarts.")), widget::space::vertical().height(Length::Fixed(1.0)));
        }
        let log = sys::work_dir().join("install.log");
        let tail = self.log.iter().rev().take(40).rev().cloned().collect::<Vec<_>>().join("\n");
        widget::column::with_capacity(3)
            .spacing(spacing.space_l)
            .push(m3::hero("dialog-error-symbolic", tr("The installation failed"), tr("The installation did not finish. You can try again from the launcher: Install Tamagi.")))
            .push(m3::supporting(format!("{}: {}", tr("Log"), log.display())))
            .push(m3::card(widget::scrollable(widget::text::monotext(tail).size(SMALL).width(Length::Fill)).anchor_bottom().height(Length::Fill)).height(Length::Fill))
            .into()
    }
}
