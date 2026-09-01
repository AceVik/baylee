//! Drawing mana symbols with the `mana` font.
//!
//! The font supplies only the mark — the sun, the drop, the skull. The disc
//! under it is the page's job on the web and ours here, which is why every
//! symbol is a rounded node with a glyph centred on it rather than a single
//! piece of text.
//!
//! A hybrid has no glyph of its own: the font draws one by clipping two
//! colours' glyphs to opposite halves of one disc. [`spawn_pip`] does the
//! same with two clipped children, so `{W/U}` reads as the printed symbol and
//! not as two symbols side by side.
//!
//! What goes where is decided in [`baylee_client_core::manapip`], which needs
//! no GPU and carries the tests.

use crate::hud::{UiFonts, palette};
use baylee_client_core::manapip::{Disc, Pip};
use bevy::prelude::*;
use bevy::ui::widget::Text;
use bevy::ui::{
    AlignItems, BorderRadius, JustifyContent, Node, Overflow, PositionType,
    Val::Percent as percent, Val::Px as px,
};

/// The paint of one disc.
///
/// Card-face colours, muted a little: these sit next to body text in lists,
/// and a full-strength red shouts over it.
#[must_use]
fn disc_color(disc: Disc) -> Color {
    match disc {
        Disc::White => Color::srgb(0.96, 0.94, 0.84),
        Disc::Blue => Color::srgb(0.44, 0.68, 0.89),
        Disc::Black => Color::srgb(0.40, 0.37, 0.42),
        Disc::Red => Color::srgb(0.90, 0.50, 0.42),
        Disc::Green => Color::srgb(0.47, 0.74, 0.53),
        Disc::Generic => Color::srgb(0.76, 0.73, 0.70),
        Disc::Snow => Color::srgb(0.82, 0.88, 0.93),
    }
}

/// The ink a glyph is set in on a given disc.
///
/// Every disc in this palette is light enough to take dark ink, black
/// included — a black mana symbol is a grey disc with a dark skull, not a
/// black disc with a light one.
#[must_use]
fn ink_color() -> Color {
    Color::srgb(0.12, 0.11, 0.13)
}

/// Spawns one mana symbol, sized to `size` pixels across.
pub fn spawn_pip(commands: &mut Commands, fonts: &UiFonts, pip: Pip, size: f32) -> Entity {
    let disc = commands
        .spawn((
            Node {
                width: px(size),
                height: px(size),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(px(size / 2.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    match pip {
        Pip::Solid { glyph, disc: d } => {
            commands.entity(disc).insert(BackgroundColor(disc_color(d)));
            let mark = commands
                .spawn((
                    Text::new(glyph.to_string()),
                    mana_tf(fonts, size * 0.72),
                    TextColor(ink_color()),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(disc).add_child(mark);
        }
        Pip::Split { left, right } => {
            // The disc itself stays transparent; each half paints its own,
            // and the parent's clip rounds the pair back into one circle.
            for (at, (glyph, d)) in [(0.0, left), (50.0, right)] {
                let half = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: percent(at),
                            top: percent(0.0),
                            width: percent(50.0),
                            height: percent(100.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(disc_color(d)),
                        Pickable::IGNORE,
                    ))
                    .id();
                // The glyph is laid out against the whole disc and then
                // clipped, so the two halves meet on one continuous mark
                // instead of showing two shrunken ones.
                let mark = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(if at == 0.0 { 0.0 } else { -size / 2.0 }),
                            top: px(0.0),
                            width: px(size),
                            height: px(size),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
                let text = commands
                    .spawn((
                        Text::new(glyph.to_string()),
                        mana_tf(fonts, size * 0.72),
                        TextColor(ink_color()),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(mark).add_child(text);
                commands.entity(half).add_child(mark);
                commands.entity(disc).add_child(half);
            }
        }
        Pip::Number { value } => {
            commands
                .entity(disc)
                .insert(BackgroundColor(disc_color(Disc::Generic)));
            // Digits, not a glyph: `{1000000}` has no symbol in the font, and
            // a disc wide enough to hold the number is still a mana symbol.
            commands.entity(disc).insert(Node {
                width: Val::Auto,
                min_width: px(size),
                height: px(size),
                flex_shrink: 0.0,
                padding: bevy::ui::UiRect::horizontal(px(size * 0.2)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(size / 2.0)),
                ..default()
            });
            let text = commands
                .spawn((
                    Text::new(value.to_string()),
                    crate::hud::tf(fonts, size * 0.62),
                    TextColor(ink_color()),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(disc).add_child(text);
        }
    }
    disc
}

/// A mana-font handle at a size.
fn mana_tf(fonts: &UiFonts, size: f32) -> TextFont {
    TextFont {
        font: bevy::text::FontSource::Handle(fonts.mana.clone()),
        font_size: bevy::text::FontSize::Px(size),
        ..default()
    }
}

/// Spawns a whole printed cost as a row of symbols.
///
/// Returns `None` for a card with no cost at all — a land — so a caller can
/// leave the space empty rather than draw an empty row.
pub fn spawn_cost(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
) -> Option<Entity> {
    let pips = baylee_client_core::manapip::parse(text)?;
    if pips.is_empty() {
        return None;
    }
    let row = commands
        .spawn((
            Node {
                column_gap: px((size * 0.16).max(1.0)),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for pip in pips {
        let child = spawn_pip(commands, fonts, pip, size);
        commands.entity(row).add_child(child);
    }
    Some(row)
}

/// Spawns a cost, falling back to the raw string when it will not parse.
///
/// A cost the parser rejects is still information — showing `{Q}` beats
/// showing nothing — and the catalog carries costs this engine has no rules
/// for.
pub fn spawn_cost_or_text(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
) -> Option<Entity> {
    if text.trim().is_empty() {
        return None;
    }
    spawn_cost(commands, fonts, text, size).or_else(|| {
        Some(
            commands
                .spawn((
                    Text::new(text.to_string()),
                    crate::hud::tf(fonts, size),
                    TextColor(palette::MUTED),
                    Pickable::IGNORE,
                ))
                .id(),
        )
    })
}
