//! The preview's one keyboard hint: a pill under a card that has another side.
//!
//! It used to be a footer under every preview, two columns naming held Shift
//! and held Alt (owner, 12.11.2026: "die Anzeige für die Tastenkürzel,
//! vielleicht ganz weg"). A preview is a card held up, and a strip of keys
//! under each one was the loudest thing on it. What is left says something
//! about *this card*: that it has a second face, or a half that reads the
//! other way up, and which key shows it. A card with neither wears nothing;
//! both keys stand in `docs/keyboard-map.md`.

use super::{UiFonts, keycap, palette, tf};
use baylee_client_core::{Lang, i18n::Phrase};
use bevy::prelude::*;

/// The pill's own height, in logical pixels.
const PILL: f32 = 22.0;

/// The room the pill takes under the card: itself and the gap above it.
pub(super) const ROOM: f32 = PILL + 6.0;

#[derive(Component)]
pub(super) struct PreviewKeys;

/// What held Shift shows of a card read as `reading`, where that is worth a
/// word: the other face of a card that has one (`far`), or the other half of
/// a flip or an Aftermath card. `None` for a card whose far side is only the
/// printed back, and for a split card, whose halves are read at once.
pub(super) fn turn(reading: crate::flip::Reading, far: bool) -> Option<Phrase> {
    match (reading.over, reading.rest == reading.turned) {
        (true, _) => far.then_some(Phrase::PreviewTurn),
        (false, false) => Some(Phrase::PreviewRotate),
        (false, true) => None,
    }
}

/// The room a preview keeps under its card for the hint.
pub(super) fn room(turn: Option<Phrase>) -> f32 {
    if turn.is_some() { ROOM } else { 0.0 }
}

fn apple_keyboard() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|window| window.navigator().platform().ok())
            .is_some_and(|platform| {
                platform.contains("Mac") || platform.contains("iPad") || platform.contains("iPhone")
            })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        cfg!(any(target_os = "macos", target_os = "ios"))
    }
}

/// The pill, centred in a row `width` wide under the card and unaffected by
/// the card's turn: the key and what it shows.
pub(super) fn spawn(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    width: f32,
    turn: Phrase,
) -> Entity {
    let row = commands
        .spawn((
            PreviewKeys,
            Node {
                width: px(width),
                height: px(ROOM),
                flex_shrink: 0.0,
                align_items: AlignItems::FlexEnd,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let pill = commands
        .spawn((
            Node {
                height: px(PILL),
                column_gap: px(6),
                padding: UiRect::horizontal(px(9)),
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(PILL / 2.0)),
                ..default()
            },
            BackgroundColor(palette::PANEL.with_alpha(0.94)),
            BorderColor::all(palette::MUTED.with_alpha(0.22)),
            BoxShadow::new(
                palette::SHADOW.with_alpha(0.45),
                Val::Px(0.0),
                Val::Px(2.0),
                Val::Px(0.0),
                Val::Px(8.0),
            ),
            Pickable::IGNORE,
        ))
        .id();
    let key = if apple_keyboard() {
        shift_key(commands)
    } else {
        keycap(
            commands,
            fonts,
            "Shift",
            Color::NONE,
            palette::MUTED,
            palette::MUTED.with_alpha(0.35),
            10.0,
        )
    };
    let words = commands
        .spawn((
            Text::new(turn.text(lang)),
            TextLayout::no_wrap(),
            tf(fonts, 11.0),
            TextColor(palette::MUTED),
            Node {
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(pill).add_children(&[key, words]);
    commands.entity(row).add_child(pill);
    row
}

/// The standard Shift mark drawn as lines: the text font lacks the glyph.
fn shift_key(commands: &mut Commands) -> Entity {
    let key = commands
        .spawn((
            Node {
                width: px(20),
                height: px(16),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let shift = [
        ([4.0, 8.0], [10.0, 2.0]),
        ([10.0, 2.0], [16.0, 8.0]),
        ([16.0, 8.0], [12.5, 8.0]),
        ([12.5, 8.0], [12.5, 13.5]),
        ([12.5, 13.5], [7.5, 13.5]),
        ([7.5, 13.5], [7.5, 8.0]),
        ([7.5, 8.0], [4.0, 8.0]),
    ];
    for (from, to) in shift {
        let from = Vec2::from_array(from);
        let to = Vec2::from_array(to);
        let delta = to - from;
        let mid = (from + to) / 2.0;
        let line = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(mid.x - delta.length() / 2.0),
                    top: px(mid.y - 0.6),
                    width: px(delta.length()),
                    height: px(1.2),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::radians(delta.y.atan2(delta.x))),
                BackgroundColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(key).add_child(line);
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flip::Reading;

    /// The pill names a key only where the key shows more of the card: a
    /// second face, or a half that reads the other way up. The printed back
    /// and a split card, read at once, get no word and no room.
    #[test]
    fn only_a_card_with_another_side_is_given_the_hint() {
        let two_faced = Reading::of(&[0, 0], 0, true);
        assert_eq!(turn(two_faced, true), Some(Phrase::PreviewTurn));
        assert!((room(turn(two_faced, true)) - ROOM).abs() < f32::EPSILON);
        let plain = Reading::UPRIGHT;
        assert_eq!(turn(plain, false), None, "the printed back is no news");
        assert!(room(turn(plain, false)).abs() < f32::EPSILON);
        let split = Reading::of(&[1, 1], 0, false);
        assert_eq!(turn(split, false), None, "both halves are read at once");
        let flip = Reading::of(&[0, 2], 0, false);
        assert_eq!(turn(flip, false), Some(Phrase::PreviewRotate));
        let aftermath = Reading::of(&[0, 3], 0, false);
        assert_eq!(turn(aftermath, false), Some(Phrase::PreviewRotate));
    }
}
