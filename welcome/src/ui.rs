//! Small pieces shared by the pages: the type sizes, the install-method
//! badge, the legend. Colours come from the COSMIC theme only; the shapes
//! are the Material 3 ones of `m3` (the same as the installer).

use crate::i18n::tr;
use crate::method::Method;
use cosmic::iced::{Alignment, Background, Border, Color};
use cosmic::{theme, widget, Element};

/// Type sizes. COSMIC's "caption" is 10 px and its "body" 14 px: too small
/// to read and to hit on a laptop panel, so the choices use these instead.
pub const TEXT: f32 = 16.0;
/// Explanations under or beside a choice.
pub const NOTE: f32 = 14.0;
/// Badges, sizes, the step indicator.
pub const SMALL: f32 = 13.0;

/// The name of a choice.
pub fn text<'a>(content: impl Into<String>) -> widget::Text<'a, cosmic::Theme> {
    widget::text::body(content.into()).size(TEXT)
}

/// An explanation.
pub fn note<'a>(content: impl Into<String>) -> widget::Text<'a, cosmic::Theme> {
    widget::text::body(content.into()).size(NOTE)
}

/// The smallest text used.
pub fn small<'a>(content: impl Into<String>) -> widget::Text<'a, cosmic::Theme> {
    widget::text::body(content.into()).size(SMALL)
}

/// The icon that marks what asks for the password.
pub const LOCK_ICON: &str = "system-lock-screen-symbolic";

fn with_alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

/// The theme colour a badge is tinted with.
fn tint(theme: &cosmic::Theme, method: Method) -> Color {
    let c = theme.cosmic();
    match method {
        Method::Flatpak => c.accent_color().into(),
        Method::Native => c.warning_color().into(),
        Method::User => c.success_color().into(),
        Method::Box | Method::Container | Method::Command => c.on_bg_color().into(),
    }
}

/// The badge of an install method: "Flatpak", "Native" (with a lock: it asks
/// for the password), "Box", "Container", "In your user".
pub fn badge<'a, M: 'a>(method: Method) -> Element<'a, M> {
    let mut row = widget::row::with_capacity(2).spacing(5).align_y(Alignment::Center);
    if method.asks_password() {
        row = row.push(widget::icon::from_name(LOCK_ICON).size(14).icon());
    }
    row = row.push(small(tr(method.label())));
    widget::container(row)
        .padding([3, 10])
        .class(theme::Container::custom(move |theme| {
            let tint = tint(theme, method);
            let on: Color = theme.cosmic().on_bg_color().into();
            widget::container::Style {
                icon_color: Some(on),
                text_color: Some(on),
                background: Some(Background::Color(with_alpha(tint, 0.16))),
                border: Border { radius: theme.cosmic().corner_radii.radius_xl.into(), width: 1.0, color: with_alpha(tint, 0.6) },
                ..Default::default()
            }
        }))
        .into()
}

/// One line explaining every badge.
pub fn legend<'a, M: Clone + 'static>() -> Element<'a, M> {
    let spacing = theme::spacing();
    let items = Method::LEGEND
        .iter()
        .map(|&method| {
            widget::row::with_capacity(2)
                .spacing(spacing.space_xxs)
                .align_y(Alignment::Center)
                .push(badge(method))
                .push(small(tr(method.short())))
                .into()
        })
        .collect();
    widget::flex_row(items).row_spacing(spacing.space_xxs).column_spacing(spacing.space_s).into()
}

/// The mark of a single-choice row: the Material 3 radio button of
/// `m3::radio`. Only a picture: the row around it is the control, so a click
/// anywhere on the row and Tab + Enter behave the same.
pub fn radio_mark<'a, M: 'a>(selected: bool) -> Element<'a, M> {
    crate::m3::radio(selected)
}

/// A panel for secondary content (the log, a note): the Material 3 card of
/// `m3::card` (surfaceContainerLow, 16 px corners).
pub fn panel<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> widget::Container<'a, M, cosmic::Theme> {
    crate::m3::card(content).padding(theme::spacing().space_s)
}
