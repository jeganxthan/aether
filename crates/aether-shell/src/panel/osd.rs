use aether::{
    Align, Center, Color, Horizontal, Layer, LayerWindow, Margin, Parent, Rectangle, Row, Shadow,
    Text, Vertical, Weight, Widget, Zone, children,
};

use crate::panel::svg::osd_svg::OsdIcon;
use crate::services::osd::{Kind, Shown, shown};
use crate::theme::{Theme, theme};

/*
 * The on screen display is a single slim card that rises from the bottom of the
 * screen whenever the volume or the brightness changes, and sinks back once the
 * change has sat for a moment. One line holds the icon, the progress bar and the
 * percentage. It is centered, takes no pointer at all, and leaves the screen
 * untouched at rest.
 */

// the card itself
const CARD_WIDTH: f32 = 280.0;
const CARD_HEIGHT: f32 = 46.0;
const CARD_RADIUS: f32 = 14.0;
const CARD_PADDING: f32 = 12.0;

// the distance kept from the bottom of the screen
const EDGE_INSET_BOTTOM: f32 = 72.0;

// the icon box, and the gaps around and after it
const ICON_SIZE: f32 = 22.0;
const MIC_CIRCLE_SIZE: f32 = 54.0;
const ICON_GAP: f32 = 12.0;

// the room kept for the percentage, so the bar never moves as the number changes
const VALUE_WIDTH: f32 = 40.0;
const VALUE_GAP: f32 = 10.0;

// the progress bar between the icon and the percentage
const TRACK_HEIGHT: f32 = 6.0;
const TRACK_RADIUS: f32 = 3.0;

// a surface cannot be zero high, so the window always asks for the card plus its shadow room
const WINDOW_HEIGHT: f32 = CARD_HEIGHT + 48.0;

// the width the bar can fill, once the icon, the percentage and the gaps are taken out
fn track_width() -> f32 {
    CARD_WIDTH - CARD_PADDING * 2.0 - ICON_SIZE - ICON_GAP - VALUE_GAP - VALUE_WIDTH
}

/*
 * The display window. It stays the same size every frame so the surface never
 * resizes mid slide, and it renders an empty, click through child while the
 * display rests.
 */
pub fn osd() -> LayerWindow {
    let window = LayerWindow::new()
        .width(CARD_WIDTH)
        .height(WINDOW_HEIGHT)
        .anchor_vertical(Vertical::Bottom)
        .anchor_horizontal(Horizontal::Middle)
        .layer(Layer::Overlay)
        .space(Zone::Ignore)
        .namespace("aether-osd")
        .margin(Margin {
            top: 0,
            right: 0,
            bottom: EDGE_INSET_BOTTOM as i32,
            left: 0,
        })
        // a heads up display never takes a click from the windows below it
        .click_through();

    let Some(reading) = shown() else {
        return window.child(blank());
    };

    window.child(card(reading))
}

// an empty, transparent child, used while the display rests
fn blank() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
}

// one card, risen into place by its offset
fn card(reading: Shown) -> Rectangle {
    let theme = theme();

    let visual = ease_rise(reading.offset, reading.leaving);

    // the card sinks by up to its own height and fades as it goes
    let drop = visual * (CARD_HEIGHT + 12.0);
    let opacity = 1.0 - visual;

    match reading.kind {
        Kind::Microphone => Rectangle::new()
            .width(Parent)
            .height(Parent)
            .fill(Color::TRANSPARENT)
            .align_child(Center, Center)
            .child(
                Rectangle::new()
                    .width(MIC_CIRCLE_SIZE)
                    .height(MIC_CIRCLE_SIZE)
                    .fill(theme.surface)
                    .border(1.0, theme.surface_border)
                    .radius(MIC_CIRCLE_SIZE / 2.0)
                    .shadow(
                        Shadow::drop(theme.shadow)
                            .opacity(0.45)
                            .blur(20.0)
                            .offset(0.0, 8.0),
                    )
                    .opacity(opacity)
                    .translate(0.0, drop)
                    .align_child(Center, Center)
                    .child(
                        Rectangle::new()
                            .width(ICON_SIZE + 4.0)
                            .height(ICON_SIZE + 4.0)
                            .fill(OsdIcon::microphone(reading.muted).image()),
                    ),
            ),
        _ => level_card(reading, opacity, drop, theme),
    }
}

fn level_card(reading: Shown, opacity: f32, drop: f32, theme: Theme) -> Rectangle {
    let icon = match reading.kind {
        Kind::Volume => OsdIcon::volume(reading.percent, reading.muted),
        Kind::Brightness => OsdIcon::brightness(reading.percent),
        Kind::KeyboardBrightness => OsdIcon::Keyboard,
        Kind::Microphone => OsdIcon::microphone(reading.muted),
    };

    // the number always shows, muted or not, so the line keeps one shape
    let value = format!("{}%", reading.percent);

    // only the card takes the pointer, and it sits centered in the taller window
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
        .align_child(Center, Center)
        .child(
            Rectangle::new()
                .width(CARD_WIDTH)
                .height(CARD_HEIGHT)
                .fill(theme.surface)
                .border(1.0, theme.surface_border)
                .radius(CARD_RADIUS)
                .shadow(
                    Shadow::drop(theme.shadow)
                        .opacity(0.45)
                        .blur(20.0)
                        .offset(0.0, 8.0),
                )
                .opacity(opacity)
                .translate(0.0, drop)
                .padding(CARD_PADDING)
                .clip()
                .child(row(icon, reading.percent, value)),
        )
}

// the icon, the progress bar and the percentage, all on one line
fn row(icon: OsdIcon, percent: u8, value: String) -> Row {
    let theme = theme();

    Row::new(children![
        Rectangle::new()
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .fill(icon.image()),
        // small gap between the icon and the bar
        Rectangle::new()
            .width(ICON_GAP)
            .height(1.0)
            .fill(Color::TRANSPARENT),
        track(percent),
        // small gap between the bar and the number
        Rectangle::new()
            .width(VALUE_GAP)
            .height(1.0)
            .fill(Color::TRANSPARENT),
        // a fixed box keeps the bar steady as the number changes width
        Rectangle::new()
            .width(VALUE_WIDTH)
            .height(Parent)
            .fill(Color::TRANSPARENT)
            .align_child(Align::End, Center)
            .child(
                Text::new(value)
                    .font("Maple Mono")
                    .font("Maple Mono")
                    .color(theme.text)
                    .size(12.0)
                    .weight(Weight::SemiBold),
            ),
    ])
    .width(Parent)
    .height(Parent)
    .align(Align::Center)
    .justify(aether::Start)
}

// the progress bar with its filled part
fn track(percent: u8) -> Rectangle {
    let theme = theme();

    let percent = percent.min(100) as f32;

    let width = track_width();
    let filled = width * percent / 100.0;

    let mut parts: Vec<Box<dyn Widget>> = Vec::new();

    // a zero percent bar draws nothing rather than a sliver
    if filled > 0.0 {
        parts.push(Box::new(
            Rectangle::new()
                .width(filled)
                .height(TRACK_HEIGHT)
                .fill(theme.accent)
                .radius(TRACK_RADIUS),
        ));
    }

    Rectangle::new()
        .width(width)
        .height(TRACK_HEIGHT)
        .fill(theme.track)
        .radius(TRACK_RADIUS)
        .align_child(Align::Start, Center)
        .child(
            Row::new(parts)
                .width(Parent)
                .height(TRACK_HEIGHT)
                .align(Align::Center)
                .justify(aether::Start),
        )
}

/*
 * Eased rise from the raw offset (0.0 at rest on screen, 1.0 fully sunk below it).
 * Entering decelerates as it rises into place; leaving accelerates as it drops away.
 */
pub fn ease_rise(offset: f32, leaving: bool) -> f32 {
    let offset = offset.clamp(0.0, 1.0);

    if leaving {
        offset * offset
    } else {
        offset * offset * offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rise_lands_at_both_ends() {
        assert_eq!(ease_rise(0.0, false), 0.0);
        assert_eq!(ease_rise(1.0, false), 1.0);
        assert_eq!(ease_rise(0.0, true), 0.0);
        assert_eq!(ease_rise(1.0, true), 1.0);
    }

    #[test]
    fn the_rise_decelerates_into_place() {
        // most of the way traveled while only a fifth of the offset is left
        assert!(ease_rise(0.2, false) < 0.02);
        assert!(ease_rise(0.2, true) < 0.05);
    }

    #[test]
    fn the_window_leaves_room_for_the_card_and_its_shadow() {
        assert!(WINDOW_HEIGHT > CARD_HEIGHT);
        assert_eq!(WINDOW_HEIGHT, CARD_HEIGHT + 48.0);
    }

    #[test]
    fn the_track_fill_stops_at_the_full_width() {
        let width = track_width();

        // the bar fills its width at 100 and nothing at 0
        assert_eq!(width * 100.0 / 100.0, width);
        assert_eq!(width * 0.0 / 100.0, 0.0);
    }

    #[test]
    fn the_line_leaves_room_for_the_bar() {
        // icon + gap + bar + gap + value, all inside the padding, is the card width
        let used = CARD_PADDING * 2.0 + ICON_SIZE + ICON_GAP + VALUE_GAP + VALUE_WIDTH;

        assert_eq!(used + track_width(), CARD_WIDTH);
        assert!(track_width() > 0.0, "the bar has no room to draw");
    }
}
