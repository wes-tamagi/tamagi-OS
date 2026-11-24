//! Material 3 for the installer window, built on libcosmic.
//!
//! The colour roles of Material 3 (surface containers, primary and secondary
//! containers, error container) are derived from the COSMIC theme of the
//! session: the accent is the primary colour, so the window follows light and
//! dark mode and the accent the user picked. Shapes and sizes follow the M3
//! guidelines: 16 px cards, 56 px list items with a tonal selected state,
//! headline type for page titles, a 4 px rounded progress track.

use cosmic::iced::{Alignment, Background, Border, Color, Length};
use cosmic::{theme, widget, Element};

/// Corner radius of cards and list items (M3 "large").
pub const RADIUS: f32 = 16.0;
/// The tonal circle behind a page icon.
const HERO_SIZE: f32 = 56.0;
/// M3 headline-medium.
pub const HEADLINE: f32 = 28.0;
/// M3 body-large.
pub const BODY: f32 = 16.0;
/// M3 label-large.
pub const LABEL: f32 = 14.0;

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    Color { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: 1.0 }
}

pub fn primary(t: &cosmic::Theme) -> Color {
    t.cosmic().accent_color().into()
}

pub fn surface(t: &cosmic::Theme) -> Color {
    t.cosmic().bg_color().into()
}

pub fn on_surface(t: &cosmic::Theme) -> Color {
    t.cosmic().on_bg_color().into()
}

pub fn on_surface_variant(t: &cosmic::Theme) -> Color {
    Color { a: 0.72, ..on_surface(t) }
}

/// surfaceContainerLow: cards.
pub fn surface_container(t: &cosmic::Theme) -> Color {
    mix(surface(t), on_surface(t), 0.05)
}

/// secondaryContainer: the selected list item.
pub fn secondary_container(t: &cosmic::Theme) -> Color {
    mix(surface_container(t), primary(t), 0.22)
}

/// primaryContainer: the circle behind a page icon.
pub fn primary_container(t: &cosmic::Theme) -> Color {
    mix(surface(t), primary(t), 0.28)
}

/// errorContainer: what cannot be undone.
pub fn error_container(t: &cosmic::Theme) -> Color {
    mix(surface(t), t.cosmic().destructive_color().into(), 0.20)
}

fn variant_text(t: &cosmic::Theme) -> cosmic::iced::widget::text::Style {
    cosmic::iced::widget::text::Style { color: Some(on_surface_variant(t)), ..Default::default() }
}

fn primary_text(t: &cosmic::Theme) -> cosmic::iced::widget::text::Style {
    cosmic::iced::widget::text::Style { color: Some(primary(t)), ..Default::default() }
}

fn error_text(t: &cosmic::Theme) -> cosmic::iced::widget::text::Style {
    cosmic::iced::widget::text::Style { color: Some(t.cosmic().destructive_color().into()), ..Default::default() }
}

/// Body text in onSurfaceVariant: supporting text.
pub fn supporting<'a>(content: impl Into<String>) -> widget::Text<'a, cosmic::Theme> {
    widget::text::body(content.into()).size(LABEL).class(theme::Text::Custom(variant_text))
}

pub fn body<'a>(content: impl Into<String>) -> widget::Text<'a, cosmic::Theme> {
    widget::text::body(content.into()).size(BODY)
}

pub fn label<'a>(content: impl Into<String>) -> widget::Text<'a, cosmic::Theme> {
    widget::text::heading(content.into()).size(LABEL)
}

pub fn error<'a>(content: impl Into<String>) -> widget::Text<'a, cosmic::Theme> {
    widget::text::body(content.into()).size(LABEL).class(theme::Text::Custom(error_text))
}

/// A card: surfaceContainerLow, 16 px corners.
pub fn card<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> widget::Container<'a, M, cosmic::Theme> {
    widget::container(content).padding(16).width(Length::Fill).class(theme::Container::custom(|t| widget::container::Style {
        background: Some(Background::Color(surface_container(t))),
        text_color: Some(on_surface(t)),
        icon_color: Some(on_surface(t)),
        border: Border { radius: RADIUS.into(), ..Default::default() },
        ..Default::default()
    }))
}

/// A card in errorContainer, for what cannot be undone.
pub fn warning_card<'a, M: 'a>(icon: &'static str, content: impl Into<String>) -> Element<'a, M> {
    let row = widget::row::with_capacity(2).spacing(12).align_y(Alignment::Center).push(widget::icon::from_name(icon).size(24).icon()).push(body(content).width(Length::Fill));
    widget::container(row)
        .padding(16)
        .width(Length::Fill)
        .class(theme::Container::custom(|t| widget::container::Style {
            background: Some(Background::Color(error_container(t))),
            text_color: Some(on_surface(t)),
            icon_color: Some(t.cosmic().destructive_color().into()),
            border: Border { radius: RADIUS.into(), ..Default::default() },
            ..Default::default()
        }))
        .into()
}

/// The top of a page: the icon in a primaryContainer circle beside the title
/// in headline type, with the supporting text under the title.
pub fn hero<'a, M: 'a>(icon: &'static str, title: String, text: String) -> Element<'a, M> {
    let circle = widget::container(widget::icon::from_name(icon).size(26).icon())
        .width(Length::Fixed(HERO_SIZE))
        .height(Length::Fixed(HERO_SIZE))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .class(theme::Container::custom(|t| widget::container::Style {
            background: Some(Background::Color(primary_container(t))),
            icon_color: Some(primary(t)),
            border: Border { radius: (HERO_SIZE / 2.0).into(), ..Default::default() },
            ..Default::default()
        }));
    let mut words = widget::column::with_capacity(2).spacing(4).width(Length::Fill).push(widget::text::title1(title).size(HEADLINE));
    if !text.is_empty() {
        words = words.push(widget::text::body(text).size(BODY).class(theme::Text::Custom(variant_text)));
    }
    widget::row::with_capacity(2).spacing(20).align_y(Alignment::Center).push(circle).push(words).into()
}

/// The M3 radio button: a 2 px ring, with a dot of the primary colour when
/// selected. Only a picture: the list item around it is the control.
pub fn radio<'a, M: 'a>(selected: bool) -> Element<'a, M> {
    const SIZE: f32 = 20.0;
    const DOT: f32 = 10.0;
    let dot: Element<'a, M> = if selected {
        widget::container(widget::space::horizontal().width(Length::Fixed(DOT)))
            .width(Length::Fixed(DOT))
            .height(Length::Fixed(DOT))
            .class(theme::Container::custom(|t| widget::container::Style {
                background: Some(Background::Color(primary(t))),
                border: Border { radius: (DOT / 2.0).into(), ..Default::default() },
                ..Default::default()
            }))
            .into()
    } else {
        widget::space::horizontal().width(Length::Fixed(DOT)).into()
    };
    widget::container(dot)
        .width(Length::Fixed(SIZE))
        .height(Length::Fixed(SIZE))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .class(theme::Container::custom(move |t| widget::container::Style {
            border: Border { radius: (SIZE / 2.0).into(), width: 2.0, color: if selected { primary(t) } else { on_surface_variant(t) } },
            ..Default::default()
        }))
        .into()
}

fn item_style(selected: bool, state: f32) -> impl Fn(bool, &cosmic::Theme) -> widget::button::Style {
    move |_focused, t| {
        let base = if selected { secondary_container(t) } else { surface_container(t) };
        // M3 state layers: 8 % of onSurface when hovered, 12 % when pressed.
        let background = mix(base, on_surface(t), state);
        let mut style = widget::button::Style::new();
        style.background = Some(Background::Color(background));
        style.border_radius = RADIUS.into();
        style.text_color = Some(on_surface(t));
        style.icon_color = Some(on_surface(t));
        style
    }
}

/// An M3 list item: leading control, headline and supporting text, trailing
/// text; the whole item is the control, tonal when selected.
pub fn list_item<'a, M: Clone + 'a>(selected: bool, headline: String, supporting_text: Option<String>, trailing: Option<String>, on_press: M) -> Element<'a, M> {
    let mut words = widget::column::with_capacity(2).spacing(2).width(Length::Fill).push(body(headline));
    if let Some(text) = supporting_text.filter(|t| !t.is_empty()) {
        words = words.push(supporting(text));
    }
    let mut row = widget::row::with_capacity(3).spacing(16).align_y(Alignment::Center).push(radio(selected)).push(words);
    if let Some(text) = trailing {
        row = row.push(widget::text::body(text).size(LABEL).class(theme::Text::Custom(variant_text)));
    }
    widget::button::custom(widget::container(row).padding([10, 16]).width(Length::Fill).height(Length::Shrink))
        .width(Length::Fill)
        .class(theme::Button::Custom {
            active: Box::new(item_style(selected, 0.0)),
            disabled: Box::new(move |t| item_style(selected, 0.0)(false, t)),
            hovered: Box::new(item_style(selected, 0.08)),
            pressed: Box::new(item_style(selected, 0.12)),
        })
        .on_press(on_press)
        .into()
}

/// List items with the 2 px gap M3 uses between items of a segmented list.
pub fn list<'a, M: 'a>(items: Vec<Element<'a, M>>) -> Element<'a, M> {
    widget::column::with_children(items).spacing(2).width(Length::Fill).into()
}

/// A row of a card with a switch: headline, supporting text, the switch.
pub fn switch_row<'a, M: Clone + 'static>(headline: String, supporting_text: Option<String>, on: bool, message: impl Fn(bool) -> M + 'a) -> Element<'a, M> {
    let mut words = widget::column::with_capacity(2).spacing(2).width(Length::Fill).push(body(headline));
    if let Some(text) = supporting_text.filter(|t| !t.is_empty()) {
        words = words.push(supporting(text));
    }
    widget::row::with_capacity(2).spacing(16).align_y(Alignment::Center).push(words).push(widget::toggler(on).on_toggle(message)).into()
}

/// A line of the review: the label in onSurfaceVariant over the value.
pub fn info_row<'a, M: 'a>(label_text: String, value: String) -> Element<'a, M> {
    widget::column::with_capacity(2).spacing(2).push(supporting(label_text)).push(body(value)).into()
}

/// Where the installation is: "Step 3 of 10 · Keyboard" over a rounded
/// 4 px progress track.
pub fn step_header<'a, M: Clone + 'a>(step: usize, total: usize, step_word: String, page: String) -> Element<'a, M> {
    let title = widget::row::with_capacity(2)
        .spacing(8)
        .push(widget::text::heading(format!("{step_word} {step}/{total}")).size(LABEL).class(theme::Text::Custom(primary_text)))
        .push(widget::text::body(format!("· {page}")).size(LABEL).class(theme::Text::Custom(variant_text)));
    widget::column::with_capacity(2)
        .spacing(8)
        .push(title)
        .push(widget::progress_bar::linear::Linear::new().girth(4.0).progress(step as f32 / total as f32).width(Length::Fill))
        .into()
}
