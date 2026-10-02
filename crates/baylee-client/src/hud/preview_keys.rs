//! Quiet keyboard hints outside the card image; the card alone turns.

use super::{UiFonts, keycap, palette, tf};
use baylee_client_core::{Lang, i18n::Phrase};
use bevy::prelude::*;

pub(super) const HEIGHT: f32 = 34.0;

#[derive(Component)]
pub(super) struct PreviewKeys;

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

/// A footer with equal columns, unaffected by the preview's flip transform.
pub(super) fn spawn(commands: &mut Commands, fonts: &UiFonts, lang: Lang, width: f32) -> Entity {
    let root = commands
        .spawn((
            PreviewKeys,
            Node {
                width: px(width),
                height: px(HEIGHT),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(palette::PANEL.with_alpha(0.98)),
            Pickable::IGNORE,
        ))
        .id();
    let phrases = if width < 280.0 {
        [Phrase::PreviewTurnCompact, Phrase::PreviewAlternateCompact]
    } else {
        [Phrase::PreviewTurn, Phrase::PreviewAlternate]
    };
    for (option, phrase) in [false, true].into_iter().zip(phrases) {
        let item = commands
            .spawn((
                Node {
                    flex_basis: percent(50),
                    flex_grow: 1.0,
                    min_width: px(0),
                    height: percent(100),
                    column_gap: px(5),
                    padding: UiRect::horizontal(px(5)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let key = if apple_keyboard() {
            symbol_key(commands, option)
        } else {
            keycap(
                commands,
                fonts,
                if option { "Alt" } else { "Shift" },
                Color::NONE,
                palette::MUTED,
                palette::MUTED.with_alpha(0.35),
                10.0,
            )
        };
        let words = commands
            .spawn((
                Text::new(phrase.text(lang)),
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
        commands.entity(item).add_children(&[key, words]);
        commands.entity(root).add_child(item);
    }
    root
}

/// Standard Shift/Option marks drawn as lines: the text font lacks these glyphs.
fn symbol_key(commands: &mut Commands, option: bool) -> Entity {
    let key = commands
        .spawn((
            Node {
                width: px(23),
                height: px(20),
                flex_shrink: 0.0,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BorderColor::all(palette::MUTED.with_alpha(0.35)),
            Pickable::IGNORE,
        ))
        .id();
    let shift = [
        ([4.0, 9.0], [10.0, 3.0]),
        ([10.0, 3.0], [16.0, 9.0]),
        ([16.0, 9.0], [12.5, 9.0]),
        ([12.5, 9.0], [12.5, 14.5]),
        ([12.5, 14.5], [7.5, 14.5]),
        ([7.5, 14.5], [7.5, 9.0]),
        ([7.5, 9.0], [4.0, 9.0]),
    ];
    let alt = [
        ([3.5, 5.0], [7.0, 5.0]),
        ([7.0, 5.0], [12.0, 13.0]),
        ([12.0, 13.0], [17.0, 13.0]),
        ([12.0, 5.0], [17.0, 5.0]),
    ];
    for &(from, to) in if option {
        alt.as_slice()
    } else {
        shift.as_slice()
    } {
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
