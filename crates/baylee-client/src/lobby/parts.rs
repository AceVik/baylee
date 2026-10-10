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

use crate::shellkit::surfaces::identity_colours as identity_colour;

/// The colour identity as its Mana-font symbols (`manaui::spawn_identity`).
pub(super) fn identity_discs(commands: &mut Commands, kit: Kit, identity: &str) -> Entity {
    let side = 16.0_f32.max(kit.m.scaled(16.0));
    crate::manaui::spawn_identity(commands, kit.fonts, identity, side, kit.m.px(3.0))
}

/// The identity gradient behind a band: its first colour fading to its
/// last, faint enough that ink on it keeps its contrast.
pub(super) fn identity_gradient(identity: &str) -> BackgroundGradient {
    let first = identity.chars().next().unwrap_or('C');
    let last = identity.chars().last().unwrap_or('C');
    BackgroundGradient::from(LinearGradient::to_right(vec![
        tinted(identity_colour(first).0, 0.18).into(),
        tinted(identity_colour(last).0, 0.08).into(),
    ]))
}

/// The opaque ground a band is drawn in: `share` of the colour over the
/// tile's own dark, mixed here rather than blended, so the name and the
/// credit on it keep their contrast (ink 7 : 1, muted 4.5 : 1) under a white
/// identity too.
fn tinted(colour: Color, share: f32) -> Color {
    let ground = tokens::OPAQUE.to_srgba();
    let colour = colour.to_srgba();
    let mix = |a: f32, b: f32| a + (b - a) * share;
    Color::srgb(
        mix(ground.red, colour.red),
        mix(ground.green, colour.green),
        mix(ground.blue, colour.blue),
    )
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
    // The credit stands on its own dark ground: a docked panel's leather and
    // a light identity band are both too bright for muted ink (4.5 : 1,
    // measured by `scripts/shell/screens.py`).
    let ground = commands
        .spawn((
            Node {
                align_self: AlignSelf::FlexStart,
                padding: UiRect::axes(px_fixed(5.0), px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(4.0)),
                max_width: percent(100),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(ground).add_child(credit);
    commands.entity(column).add_children(&[picture, ground]);
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

/// The device's offset east of UTC now, in seconds; 0 where the platform
/// will not say it.
pub(super) fn local_offset() -> i32 {
    let unix = i64::try_from(now_secs()).unwrap_or(0);
    time::OffsetDateTime::from_unix_timestamp(unix)
        .ok()
        .and_then(|utc| time::UtcOffset::local_offset_at(utc).ok())
        .map_or(0, time::UtcOffset::whole_seconds)
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
