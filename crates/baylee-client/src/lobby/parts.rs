//! Pieces the Play, Decks and room screens share (WP2, WP3), built on the
//! shell kit: an icon button, the deck tile with its art band and credit,
//! a badge, a section label, the footer's key hint, and the clock.

use crate::hud::{icon_tf, tf, tf_bold};
use crate::shellkit::controls::{self, Kit};
use crate::shellkit::{Frame as ShellFrame, Role, px_fixed, tokens};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::lobby::shelf::DeckArt;
use bevy::prelude::*;

/// Font Awesome's `⋯`.
pub(super) const ELLIPSIS: char = '\u{f141}';
/// Font Awesome's star: a favourite.
pub(super) const STAR: char = '\u{f005}';
/// Font Awesome's lock: a password on a table.
pub(super) const LOCK: char = '\u{f023}';
/// Font Awesome's warning triangle: the format warning (S-4).
pub(super) const WARNING: char = '\u{f071}';
/// Font Awesome's caret down: a menu button.
pub(super) const CARET: char = '\u{f0d7}';

/// A glyph button in a 44-px hit area (under a pointer, the control's
/// height).
pub(super) fn icon_button(
    commands: &mut Commands,
    kit: Kit,
    glyph: char,
    action: impl Bundle,
) -> Entity {
    let mark = commands
        .spawn((
            Text::new(glyph.to_string()),
            icon_tf(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let face = commands
        .spawn((
            Role::Button,
            Node {
                min_width: px_fixed(kit.m.control),
                min_height: px_fixed(kit.m.control),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::BORDER),
            crate::ambience::Feel::new(tokens::CONTROL),
        ))
        .id();
    commands.entity(face).add_child(mark);
    controls::hit(commands, kit, face, action)
}

/// A menu button: the kit's button with a caret after its words, in the
/// icon face (the interface's faces have no `▾`). The caret stands in the
/// button's face, not in its label, so the label keeps its budget.
pub(super) fn menu_button(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    weight: controls::Weight,
    action: impl Bundle,
) -> Entity {
    let wrapper = controls::button(
        commands,
        kit,
        text,
        weight,
        controls::Live::Yes,
        None,
        action,
    );
    let caret = commands
        .spawn((
            Text::new(CARET.to_string()),
            icon_tf(kit.fonts, kit.m.small * 0.85),
            TextColor(tokens::MUTED),
            Node {
                margin: UiRect::left(kit.m.px(6.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.queue(move |world: &mut World| {
        let face = world
            .get::<Children>(wrapper)
            .and_then(|children| children.first().copied());
        if let Some(face) = face
            && let Ok(mut face) = world.get_entity_mut(face)
        {
            face.add_child(caret);
        }
    });
    wrapper
}

/// A small pill of text: a badge on a tile or a row ("next game", "2/4
/// seats").
pub(super) fn badge(commands: &mut Commands, kit: Kit, text: &str, ink: Color) -> Entity {
    let pill = commands
        .spawn((
            Role::Tag,
            Node {
                padding: UiRect::axes(kit.m.px(8.0), kit.m.px(2.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(ink.with_alpha(0.55)),
            BackgroundColor(ink.with_alpha(0.10)),
            Pickable::IGNORE,
        ))
        .id();
    let words = controls::label(commands, kit, text, kit.m.small, ink);
    commands.entity(pill).add_child(words);
    pill
}

/// A badge led by a glyph in the icon face (a lock, a warning).
pub(super) fn badge_with(
    commands: &mut Commands,
    kit: Kit,
    glyph: char,
    text: &str,
    ink: Color,
) -> Entity {
    let pill = badge(commands, kit, text, ink);
    let mark = commands
        .spawn((
            Text::new(glyph.to_string()),
            icon_tf(kit.fonts, kit.m.small * 0.85),
            TextColor(ink),
            Node {
                margin: UiRect::right(kit.m.px(4.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(pill).insert_children(0, &[mark]);
    commands
        .entity(pill)
        .entry::<Node>()
        .and_modify(|mut n| n.align_items = AlignItems::Center);
    pill
}

/// A line led by a glyph in the icon face: the format warning before a
/// Join (S-4), in gold.
pub(super) fn glyph_line(
    commands: &mut Commands,
    kit: Kit,
    glyph: char,
    text: &str,
    ink: Color,
) -> Entity {
    let row = commands
        .spawn((
            Node {
                column_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mark = commands
        .spawn((
            Text::new(glyph.to_string()),
            icon_tf(kit.fonts, kit.m.small * 0.85),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    let words = line(commands, kit, text, kit.m.small, ink);
    commands.entity(row).add_children(&[mark, words]);
    row
}

/// A section's caption: small capitals in muted ink ("YOUR NEXT GAME").
pub(super) fn caption(commands: &mut Commands, kit: Kit, text: &str) -> Entity {
    commands
        .spawn((
            Text::new(text.to_uppercase()),
            tf_bold(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id()
}

/// A line of text that wraps.
pub(super) fn line(commands: &mut Commands, kit: Kit, text: &str, size: f32, ink: Color) -> Entity {
    commands
        .spawn((
            Text::new(text),
            tf(kit.fonts, size),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id()
}

/// A row of nodes with a gap, wrapping where it must.
pub(super) fn row(commands: &mut Commands, kit: Kit, wrap: bool) -> Entity {
    commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: px_fixed(kit.m.gap),
                row_gap: px_fixed(kit.m.gap * 0.5),
                flex_wrap: if wrap {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// A node that takes the room left in a row.
pub(super) fn grow(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// The ground of a colour's disc and the ink of its letter.
pub(super) fn identity_colour(letter: char) -> (Color, Color) {
    let dark = Color::srgb(0.06, 0.05, 0.04);
    match letter {
        'W' => (Color::srgb(0.95, 0.92, 0.80), dark),
        'U' => (Color::srgb(0.45, 0.66, 0.90), dark),
        'B' => (Color::srgb(0.42, 0.38, 0.40), tokens::INK),
        'R' => (Color::srgb(0.90, 0.48, 0.38), dark),
        'G' => (Color::srgb(0.45, 0.72, 0.48), dark),
        _ => (Color::srgb(0.66, 0.64, 0.60), dark),
    }
}

/// The colour identity as discs with the colour's letter in each (S4-14):
/// colour is never the only carrier. Discs stay 16 px or more.
pub(super) fn identity_discs(commands: &mut Commands, kit: Kit, identity: &str) -> Entity {
    let discs = commands
        .spawn((
            Node {
                column_gap: kit.m.px(3.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let side = 16.0_f32.max(kit.m.scaled(16.0));
    for letter in identity.chars() {
        let (ground, ink) = identity_colour(letter);
        let disc = commands
            .spawn((
                Node {
                    width: px_fixed(side),
                    height: px_fixed(side),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                    ..default()
                },
                BackgroundColor(ground),
                Pickable::IGNORE,
            ))
            .id();
        let mark = commands
            .spawn((
                Text::new(letter.to_string()),
                tf_bold(kit.fonts, 9.0_f32.max(kit.m.scaled(9.5))),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(disc).add_child(mark);
        commands.entity(discs).add_child(disc);
    }
    discs
}

/// The identity gradient behind a band: its first colour fading to its
/// last, faint enough that ink on it keeps its contrast.
pub(super) fn identity_gradient(identity: &str) -> BackgroundGradient {
    let first = identity.chars().next().unwrap_or('C');
    let last = identity.chars().last().unwrap_or('C');
    BackgroundGradient::from(LinearGradient::to_right(vec![
        identity_colour(first).0.with_alpha(0.20).into(),
        identity_colour(last).0.with_alpha(0.06).into(),
    ]))
}

/// The art band's picture and its credit under it: Scryfall's `art_crop`,
/// whole (`width` × `height` keep its 626 × 457 aspect), and "Art · <artist>"
/// in small muted ink. Nothing is drawn on the picture (#274).
pub(super) fn art_with_credit(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    art: &DeckArt,
    width: f32,
) -> Entity {
    let column = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(3.0),
                flex_shrink: 0.0,
                max_width: kit.m.px(width.max(120.0)),
                overflow: Overflow::clip_x(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    // 626 × 457: the crop's own aspect, so nothing is cut or stretched.
    let height = width * 457.0 / 626.0;
    let picture =
        super::thumbnails::art_band(commands, &art.url, kit.m.px(width), kit.m.px(height));
    let credit = commands
        .spawn((
            Role::Credit,
            Text::new(Phrase::ArtCredit.fill(lang, &[&art.artist])),
            crate::hud::tf_italic(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(column).add_children(&[picture, credit]);
    column
}

/// The key hint line at the foot of a screen, under a pointer on a wide
/// frame only (`KEYBOARD.md` §4.4): `(key, words)` pairs.
pub(super) fn key_hint(
    commands: &mut Commands,
    kit: Kit,
    pairs: &[(&str, &str)],
) -> Option<Entity> {
    if kit.m.touch() || !matches!(kit.m.frame, ShellFrame::Wide | ShellFrame::Vast) {
        return None;
    }
    let plate = commands
        .spawn((
            Role::Mist,
            Node {
                align_self: AlignSelf::FlexStart,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(6.0),
                padding: UiRect::axes(kit.m.px(10.0), kit.m.px(4.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tokens::MIST),
            Pickable::IGNORE,
        ))
        .id();
    for (i, (keys, words)) in pairs.iter().enumerate() {
        if i > 0 {
            let dot = controls::label(commands, kit, "·", kit.m.small, tokens::INK);
            commands.entity(plate).add_child(dot);
        }
        if let Some(cap) = controls::key_cap(commands, kit, keys) {
            commands.entity(cap).insert(Node {
                padding: UiRect::axes(kit.m.px(5.0), px_fixed(1.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(4.0)),
                ..default()
            });
            commands.entity(plate).add_child(cap);
        }
        let said = controls::label(commands, kit, words, kit.m.small, tokens::INK);
        commands.entity(plate).add_child(said);
    }
    Some(plate)
}

/// Now, in unix seconds (`web_time`: the browser's clock on wasm).
pub(super) fn now_secs() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// Now on the device's own clock, "14:05", the way the game log says a
/// line's time (#300); UTC where the platform will not say its offset.
pub(super) fn local_hh_mm() -> String {
    let unix = i64::try_from(now_secs()).unwrap_or(0);
    let Ok(utc) = time::OffsetDateTime::from_unix_timestamp(unix) else {
        return String::new();
    };
    let at = time::UtcOffset::local_offset_at(utc).map_or(utc, |offset| utc.to_offset(offset));
    format!("{:02}:{:02}", at.hour(), at.minute())
}
