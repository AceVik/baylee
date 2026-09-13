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
pub(crate) fn mana_tf(fonts: &UiFonts, size: f32) -> TextFont {
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

/// Spawns a line that mixes prose with the symbols printed in it.
///
/// `{T}: Add {G}` is one sentence and has always been drawn as letters in it.
/// This is the one door that stops that: `manapip::segments` splits the line
/// in a crate with no GPU and carries the tests, and every child here is
/// `Pickable::IGNORE`, because a label inside a button is a node in front of
/// it and a hover that lands on the label is a hover the button never sees.
///
/// `size` is the text's font size; the discs are set a little under it, at
/// roughly the cap height, so the line reads as a sentence rather than as
/// prose with badges dropped into it.
pub fn spawn_rich(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
    color: Color,
) -> Entity {
    rich(commands, fonts, text, size, color, crate::hud::tf)
}

/// The same line, set as a **control's own label**.
///
/// A separate door rather than a `bool` at the end of [`spawn_rich`], because
/// the two callers mean different things and a flag in that position is a
/// thing to get backwards: a menu button says *"Play {0}"* in its own voice
/// and the ability sheet *quotes* `{T}: Add {G}` off a card. Only the first
/// is a word a player can press, and [`crate::hud::tf_bold`] is where that
/// line is drawn.
///
/// The discs do not change with it. A mana symbol is a printed mark and has
/// one weight; bolding the letters around it is what a bold label is.
pub fn spawn_rich_label(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
    color: Color,
) -> Entity {
    rich(commands, fonts, text, size, color, crate::hud::tf_bold)
}

/// Both of the above, with the face they differ in passed in.
fn rich(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
    color: Color,
    face: fn(&UiFonts, f32) -> TextFont,
) -> Entity {
    let row = commands
        .spawn((
            Node {
                column_gap: px((size * 0.12).max(1.0)),
                align_items: AlignItems::Center,
                flex_wrap: bevy::ui::FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for segment in baylee_client_core::manapip::segments(text) {
        let child = match segment {
            baylee_client_core::manapip::Segment::Text(words) => commands
                .spawn((
                    Text::new(words),
                    face(fonts, size),
                    TextColor(color),
                    Pickable::IGNORE,
                ))
                .id(),
            baylee_client_core::manapip::Segment::Symbol(pip) => {
                spawn_pip(commands, fonts, pip, size * 0.88)
            }
        };
        commands.entity(row).add_child(child);
    }
    row
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

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::manapip::{Disc, Pip};

    fn fonts() -> UiFonts {
        UiFonts {
            text: Handle::default(),
            medium: Handle::default(),
            bold: Handle::default(),
            italic: Handle::default(),
            medium_italic: Handle::default(),
            serif: Handle::default(),
            serif_italic: Handle::default(),
            icons: Handle::default(),
            mana: Handle::default(),
        }
    }

    /// Written and never called is a shape this client has shipped before, so
    /// the spawner is *run*: one line in, a row of real entities out.
    ///
    /// What it asserts is the split — prose stays prose and a symbol becomes a
    /// disc — and that nothing inside the row is pickable. A `Text` is a
    /// `Node`, so one pickable label sits in front of the button it labels and
    /// the middle of that button goes dead.
    #[test]
    fn a_rich_line_becomes_prose_and_discs_and_nothing_pickable() {
        let mut app = App::new();
        let fonts = fonts();
        let row = {
            let mut commands = app.world_mut().commands();
            spawn_rich(&mut commands, &fonts, "{T}: Add {G}.", 13.0, Color::WHITE)
        };
        app.world_mut().flush();

        let children: Vec<Entity> = app
            .world()
            .entity(row)
            .get::<Children>()
            .expect("the row has children")
            .iter()
            .collect();
        assert_eq!(children.len(), 4, "two symbols and the words between them");

        let texts: Vec<String> = children
            .iter()
            .filter_map(|e| app.world().entity(*e).get::<Text>().map(|t| t.0.clone()))
            .collect();
        assert_eq!(
            texts,
            vec![": Add ".to_string(), ".".to_string()],
            "the braces are gone from the prose because they became discs",
        );

        // A disc is a node with a glyph child, which is what the two
        // non-text children are.
        let discs = children
            .iter()
            .filter(|e| app.world().entity(**e).get::<Text>().is_none())
            .count();
        assert_eq!(discs, 2, "{{T}} and {{G}} are drawn, not spelled");

        for child in &children {
            assert!(
                app.world().entity(*child).contains::<Pickable>(),
                "every child of a label carries Pickable::IGNORE",
            );
        }
    }

    /// A line with nothing to draw is still a row, not a panic.
    #[test]
    fn a_line_with_no_symbols_is_one_run_of_words() {
        let mut app = App::new();
        let fonts = fonts();
        let row = {
            let mut commands = app.world_mut().commands();
            spawn_rich(&mut commands, &fonts, "Granted ability", 13.0, Color::WHITE)
        };
        app.world_mut().flush();
        let children: Vec<Entity> = app
            .world()
            .entity(row)
            .get::<Children>()
            .expect("the row has children")
            .iter()
            .collect();
        assert_eq!(children.len(), 1);
        assert_eq!(
            app.world()
                .entity(children[0])
                .get::<Text>()
                .expect("prose")
                .0,
            "Granted ability",
        );
    }

    /// The disc a pip asks for is the one the palette paints, for every
    /// variant — a new `Disc` with no colour would be an invisible symbol.
    #[test]
    fn every_disc_has_a_paint() {
        for disc in [
            Disc::White,
            Disc::Blue,
            Disc::Black,
            Disc::Red,
            Disc::Green,
            Disc::Generic,
            Disc::Snow,
        ] {
            let c = disc_color(disc).to_srgba();
            assert!(
                c.red + c.green + c.blue > 0.3,
                "{disc:?} is painted dark enough to hide its own ink",
            );
        }
        assert!(matches!(
            baylee_client_core::manapip::symbol("T"),
            Some(Pip::Solid { .. })
        ));
    }
}
