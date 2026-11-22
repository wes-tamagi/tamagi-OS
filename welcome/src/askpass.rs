//! `tamagi-welcome --askpass "<prompt>"`: the password dialog `sudo` opens
//! (through SUDO_ASKPASS) when an installer needs the password and there is no
//! terminal. The password goes to stdout; cancelling exits with 1.

use crate::i18n::tr;
use crate::ui::LOCK_ICON;
use cosmic::app::{Core, Settings, Task};
use cosmic::iced::{Alignment, Length, Size};
use cosmic::{executor, widget, Application, ApplicationExt, Element};
use std::sync::Mutex;

/// What the dialog ended with: the password, or nothing when cancelled.
static ANSWER: Mutex<Option<String>> = Mutex::new(None);

/// Shows the dialog and returns the password typed, or None when cancelled
/// (or when the window could not be opened).
pub fn run(prompt: String) -> Option<String> {
    let settings = Settings::default().size(Size::new(480.0, 264.0)).resizable(None).exit_on_close(true);
    if let Err(e) = cosmic::app::run::<Dialog>(settings, prompt) {
        eprintln!("tamagi-welcome: {e}");
        return None;
    }
    ANSWER.lock().ok()?.take()
}

#[derive(Debug, Clone)]
pub enum Message {
    Input(String),
    ToggleVisible,
    Submit,
    Cancel,
    /// The window is up: put the cursor in the password field.
    Focus,
}

struct Dialog {
    core: Core,
    prompt: String,
    password: String,
    hidden: bool,
    input_id: widget::Id,
    /// How many times the focus was requested after the start.
    focus_requests: u8,
}

impl Application for Dialog {
    type Executor = executor::Default;
    type Flags = String;
    type Message = Message;

    const APP_ID: &'static str = "org.tamagi.Welcome";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, prompt: Self::Flags) -> (Self, Task<Self::Message>) {
        core.window.show_maximize = false;
        core.window.show_minimize = false;
        let mut dialog = Dialog { core, prompt, password: String::new(), hidden: true, input_id: widget::Id::unique(), focus_requests: 0 };
        let title = tr("Password needed");
        dialog.set_header_title(title.clone());
        let mut tasks = vec![widget::text_input::focus(dialog.input_id.clone())];
        if let Some(id) = dialog.core.main_window_id() {
            tasks.push(dialog.set_window_title(title, id));
        }
        (dialog, Task::batch(tasks))
    }

    /// The focus asked for at start can come before the window exists: ask
    /// again a couple of times, shortly after.
    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        if self.focus_requests < 3 {
            cosmic::iced::time::every(std::time::Duration::from_millis(150)).map(|_| Message::Focus)
        } else {
            cosmic::iced::Subscription::none()
        }
    }

    fn on_escape(&mut self) -> Task<Self::Message> {
        cosmic::task::message(Message::Cancel)
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::Input(value) => self.password = value,
            Message::ToggleVisible => self.hidden = !self.hidden,
            Message::Submit => {
                if let Ok(mut answer) = ANSWER.lock() {
                    *answer = Some(std::mem::take(&mut self.password));
                }
                return cosmic::iced::exit();
            }
            Message::Focus => {
                self.focus_requests += 1;
                // Not once something was typed: the cursor is already there.
                if self.password.is_empty() {
                    return widget::text_input::focus(self.input_id.clone());
                }
            }
            Message::Cancel => {
                self.password.clear();
                return cosmic::iced::exit();
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let spacing = cosmic::theme::spacing();
        // What sudo asks ("[sudo] password for ana: "), under our own sentence.
        let prompt = self.prompt.trim().trim_end_matches(':').trim();
        let mut text = widget::column::with_capacity(2).spacing(spacing.space_xxs).push(widget::text::body(tr("Installing native items changes the system: type your password to continue.")));
        if !prompt.is_empty() {
            text = text.push(widget::text::caption(prompt.to_string()));
        }
        let header = widget::row::with_capacity(2)
            .spacing(spacing.space_s)
            .align_y(Alignment::Center)
            .push(widget::icon::from_name(LOCK_ICON).size(32).icon())
            .push(text.width(Length::Fill));
        let input = widget::text_input::secure_input(tr("Password"), self.password.as_str(), Some(Message::ToggleVisible), self.hidden)
            .id(self.input_id.clone())
            .on_input(Message::Input)
            .on_submit(|_| Message::Submit)
            .width(Length::Fill);
        let buttons = widget::row::with_capacity(3)
            .spacing(spacing.space_xs)
            .push(widget::space::horizontal())
            .push(widget::button::standard(tr("Cancel")).on_press(Message::Cancel))
            .push(widget::button::suggested(tr("Continue")).on_press(Message::Submit));
        widget::column::with_capacity(3)
            .spacing(spacing.space_s)
            .padding([spacing.space_xs, spacing.space_m, spacing.space_m, spacing.space_m])
            .push(header)
            .push(input)
            .push(buttons)
            .into()
    }
}
