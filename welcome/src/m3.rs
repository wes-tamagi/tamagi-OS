//! Material 3 for the installer window, built on libcosmic.
//!
//! The colour roles of Material 3 (surface containers, primary and secondary
//! containers, error container) are derived from the COSMIC theme of the
//! session: the accent is the primary colour, so the window follows light and
//! dark mode and the accent the user picked. Shapes and sizes follow the M3
//! guidelines: 16 px cards, segmented lists (large outer corners, 4 px inner
//! corners) with a tonal selected state, headline type for page titles, an
//! 8 px rounded progress track while installing.

use cosmic::iced::{Alignment, Background, Border, Color, Length};
use cosmic::{theme, widget, Element};

/// Corner radius of cards and of the outer corners of a list (M3 "large").
pub const RADIUS: f32 = 16.0;
/// Inner corners between the items of a segmented list (M3 "extra small").
const INNER: f32 = 4.0;
/// Opacity of a disabled item (M3: 38 % content).
const DISABLED: f32 = 0.38;
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

/// A card in secondaryContainer, for what is important but safe.
pub fn notice_card<'a, M: 'a>(icon: &'static str, content: impl Into<String>) -> Element<'a, M> {
    let row = widget::row::with_capacity(2).spacing(12).align_y(Alignment::Center).push(widget::icon::from_name(icon).size(24).icon()).push(body(content).width(Length::Fill));
    widget::container(row)
        .padding(16)
        .width(Length::Fill)
        .class(theme::Container::custom(|t| widget::container::Style {
            background: Some(Background::Color(secondary_container(t))),
            text_color: Some(on_surface(t)),
            icon_color: Some(primary(t)),
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

/// Where an item sits in its list: the outer corners of the list are large,
/// the corners between two items are small.
fn corners(index: usize, count: usize) -> [f32; 4] {
    let top = if index == 0 { RADIUS } else { INNER };
    let bottom = if index + 1 == count { RADIUS } else { INNER };
    [top, top, bottom, bottom]
}

fn item_style(selected: bool, enabled: bool, state: f32, radius: [f32; 4]) -> impl Fn(bool, &cosmic::Theme) -> widget::button::Style {
    move |_focused, t| {
        let base = if selected { secondary_container(t) } else { surface_container(t) };
        // M3 state layers: 8 % of onSurface when hovered, 12 % when pressed.
        let background = mix(base, on_surface(t), state);
        let content = if enabled { on_surface(t) } else { Color { a: DISABLED, ..on_surface(t) } };
        let mut style = widget::button::Style::new();
        style.background = Some(Background::Color(background));
        style.border_radius = radius.into();
        style.text_color = Some(content);
        style.icon_color = Some(content);
        style
    }
}

/// An M3 list item: leading control, headline and supporting text, trailing
/// text; the whole item is the control, tonal when selected. It is drawn by
/// [`list`], which knows where it sits.
pub struct Item<M> {
    selected: bool,
    enabled: bool,
    headline: String,
    supporting: Option<String>,
    trailing: Option<String>,
    on_press: M,
}

impl<M> Item<M> {
    /// Shown dimmed and not selectable (e.g. a disk too small).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.enabled = !disabled;
        self
    }
}

pub fn list_item<M>(selected: bool, headline: String, supporting_text: Option<String>, trailing: Option<String>, on_press: M) -> Item<M> {
    Item { selected, enabled: true, headline, supporting: supporting_text.filter(|t| !t.is_empty()), trailing, on_press }
}

fn draw_item<'a, M: Clone + 'a>(item: Item<M>, radius: [f32; 4]) -> Element<'a, M> {
    let Item { selected, enabled, headline, supporting: supporting_text, trailing, on_press } = item;
    let mut words = widget::column::with_capacity(2).spacing(2).width(Length::Fill).push(body(headline));
    if let Some(text) = supporting_text {
        words = words.push(supporting(text));
    }
    let mut row = widget::row::with_capacity(3).spacing(16).align_y(Alignment::Center).push(radio(selected && enabled)).push(words);
    if let Some(text) = trailing {
        row = row.push(widget::text::body(text).size(LABEL).class(theme::Text::Custom(variant_text)));
    }
    let button = widget::button::custom(widget::container(row).padding([10, 16]).width(Length::Fill).height(Length::Shrink)).width(Length::Fill).class(theme::Button::Custom {
        active: Box::new(item_style(selected, enabled, 0.0, radius)),
        disabled: Box::new(move |t| item_style(selected, false, 0.0, radius)(false, t)),
        hovered: Box::new(item_style(selected, enabled, 0.08, radius)),
        pressed: Box::new(item_style(selected, enabled, 0.12, radius)),
    });
    if enabled { button.on_press(on_press).into() } else { button.into() }
}

/// A segmented list: the items 2 px apart, the group with large outer corners
/// and small corners between items (M3 Expressive).
pub fn list<'a, M: Clone + 'a>(items: Vec<Item<M>>) -> Element<'a, M> {
    let count = items.len();
    let items: Vec<Element<'a, M>> = items.into_iter().enumerate().map(|(index, item)| draw_item(item, corners(index, count))).collect();
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

/// A line of the review: the label in onSurfaceVariant on the left, the
/// value beside it (one line per answer, so the whole review fits).
pub fn info_row<'a, M: 'a>(label_text: String, value: String) -> Element<'a, M> {
    widget::row::with_capacity(2)
        .spacing(16)
        .align_y(Alignment::Center)
        .push(supporting(label_text).width(Length::FillPortion(2)))
        .push(body(value).width(Length::FillPortion(5)))
        .into()
}

/// The progress of the installation: an 8 px rounded track.
pub fn progress<'a, M: Clone + 'a>(fraction: f32) -> Element<'a, M> {
    widget::progress_bar::linear::Linear::new().girth(8.0).progress(fraction).width(Length::Fill).into()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segmented_list_corners() {
        assert_eq!(corners(0, 1), [RADIUS; 4]);
        assert_eq!(corners(0, 3), [RADIUS, RADIUS, INNER, INNER]);
        assert_eq!(corners(1, 3), [INNER; 4]);
        assert_eq!(corners(2, 3), [INNER, INNER, RADIUS, RADIUS]);
    }
}
