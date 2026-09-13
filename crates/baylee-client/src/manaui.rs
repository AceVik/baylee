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
        Pip::Number { value } => spawn_number(commands, fonts, disc, value, size),
        Pip::Loyalty(loy) => {
            spawn_loyalty(
                commands,
                fonts,
                disc,
                loy,
                size,
                palette::PARCHMENT_INK,
                palette::PARCHMENT,
            );
        }
    }
    disc
}

/// A generic cost the font has no glyph for, set as digits on a wider disc.
///
/// `{1000000}` has no symbol, and a disc wide enough to hold the number is
/// still a mana symbol — so the disc grows with the digits rather than the
/// digits shrinking into it.
fn spawn_number(commands: &mut Commands, fonts: &UiFonts, disc: Entity, value: u32, size: f32) {
    commands
        .entity(disc)
        .insert(BackgroundColor(disc_color(Disc::Generic)));
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

/// The side of the badge's turned square, as a fraction of the mark's size.
///
/// What shows of it is the half-diagonal, `CAP·√2/2`, and what stands proud of
/// its own un-rotated box is `CAP·(√2−1)/2`. Both are written out where they
/// are used rather than kept as a second constant: a bare 0.707 in a layout is
/// a number nobody can check.
const CAP: f32 = 0.86;

/// How far up into its own point the number is lifted, as a share of the rise.
///
/// Zero centres the number in the whole badge, one centres it in the body
/// alone — and both are wrong for the same reason from opposite sides: the
/// point is part of the shape a reader sees, so a number centred under it sits
/// low, and a number centred through it rides up into the taper where the
/// shoulders pinch. The card splits the difference, and so does this.
const NUMBER_LIFT: f32 = 0.55;

/// How wide a loyalty badge is at its narrowest, as a share of its size.
///
/// The badge is wider than it is a mark across, because the card's is: a
/// lozenge carrying two digits is a wide shape and one carrying a single
/// digit keeps that width rather than shrinking into a lozenge of its own.
/// Named because a caller that lays a badge beside prose has to take the
/// width out of the prose's budget, and a `1.55` written twice is a number
/// that drifts.
pub(crate) const BADGE_SPAN: f32 = 1.55;

/// A planeswalker's loyalty cost, built the way the card prints it.
///
/// Three children on one parent, and their order *is* the drawing: `bevy_ui`
/// paints later siblings in front, so the turned square goes down first and
/// the body covers the half of it that would stick out the other end — which
/// makes a pentagon out of a square and a diamond without a single clip. The
/// number goes on last.
///
/// Absolute children are placed against the parent's **padding** box, so the
/// padding moves neither the body nor the point. What it moves is the
/// *number*, which is laid out in the content box, and [`NUMBER_LIFT`] is the
/// share of the point that padding gives back to it.
///
/// A mana pip is a light disc carrying dark ink. The sheet's badge is
/// deliberately the other way round — dark body, parchment numeral — because
/// a loyalty cost drawn in the pip's own register would read as generic mana,
/// and the two stand in the same column of the same row.
///
/// Which is why the pair is an argument. The badge is a solid mark with a
/// number cut out of it, so it is legible only against the ground it is laid
/// on, and this client lays it on two at once: the sheet's parchment, which
/// takes a dark body, and the stack's dark panel, which takes a light one.
fn spawn_loyalty(
    commands: &mut Commands,
    fonts: &UiFonts,
    badge: Entity,
    loy: baylee_client_core::manapip::Loyalty,
    size: f32,
    body_ink: Color,
    numeral: Color,
) -> LoyaltyBadge {
    use baylee_client_core::manapip::Tick;

    let square = CAP * size;
    let down = loy.tick == Tick::Down;
    // The point that stands proud of the body: half the turned square's
    // diagonal, the other half being what the body covers.
    //
    // [`Tick::Flat`] points **up** with the rest, which the printed card does
    // not — a zero is a flat lozenge there. It is drawn as a plus because that
    // is what it costs the player: an ability you may use without spending
    // loyalty is one you may always use, which is the claim `+N` makes. The
    // model still keeps the three ticks apart (the parser reads them and the
    // prose prints them), so this is a drawing decision and nothing else in
    // the client reads it.
    let rise = square * std::f32::consts::SQRT_2 / 2.0;
    let body = size;
    // A pointed badge keeps its blunt end nearly square, the way the card does.
    let round = body * 0.18;
    let lift = rise * NUMBER_LIFT;
    commands.entity(badge).insert(Node {
        width: Val::Auto,
        min_width: px(size * BADGE_SPAN),
        height: px(body + rise),
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        padding: bevy::ui::UiRect {
            left: px(size * 0.26),
            right: px(size * 0.26),
            top: px(if down { 0.0 } else { lift }),
            bottom: px(if down { lift } else { 0.0 }),
        },
        ..default()
    });

    // A square turned about its own middle overhangs its box by the same
    // `(√2−1)/2` on every side; insetting by that puts the far vertex on
    // the badge's own edge.
    let inset = square * (std::f32::consts::SQRT_2 - 1.0) / 2.0;
    let lane = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                right: px(0.0),
                top: if down { Val::Auto } else { px(inset) },
                bottom: if down { px(inset) } else { Val::Auto },
                height: px(square),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let point = commands
        .spawn((
            Node {
                width: px(square),
                height: px(square),
                ..default()
            },
            UiTransform::from_rotation(Rot2::radians(std::f32::consts::FRAC_PI_4)),
            BackgroundColor(body_ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(lane).add_child(point);
    commands.entity(badge).add_child(lane);

    let slab = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                right: px(0.0),
                top: if down { px(0.0) } else { px(rise) },
                bottom: if down { px(rise) } else { px(0.0) },
                border_radius: BorderRadius::all(px(round)),
                ..default()
            },
            BackgroundColor(body_ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(badge).add_child(slab);

    let text = commands
        .spawn((
            Text::new(loy.caption()),
            crate::hud::tf_bold(fonts, size * 0.82),
            TextColor(numeral),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(badge).add_child(text);

    LoyaltyBadge {
        root: badge,
        body: [point, slab],
        numeral: text,
    }
}

/// The nodes a loyalty badge is made of, handed back to whoever asked for one.
///
/// There is no subtree opacity in `bevy_ui`, so a caller that fades its own
/// tree in — the stack panel, where every node carries an `Arriving` — has to
/// reach each piece of the badge itself. Two fills and one numeral, because
/// those are two different colours on two different components and a single
/// blanket bundle would animate one of them and silently miss the other.
pub struct LoyaltyBadge {
    /// The badge itself, to be put in a tree.
    pub root: Entity,
    /// The two nodes painted in the body colour: the point and the slab.
    pub body: [Entity; 2],
    /// The node carrying the number written on it.
    pub numeral: Entity,
}

/// A loyalty badge on its own, in the two colours the ground asks for.
///
/// [`spawn_pip`] is the door for a badge quoted inside a printed cost, where
/// it is one symbol among the mana. This is the door for a badge standing as
/// an **initial** before a sentence — the way a card prints a planeswalker's
/// line — which is a different job: the caller picks the colours, and gets
/// back the pieces so it can animate them.
pub fn spawn_loyalty_badge(
    commands: &mut Commands,
    fonts: &UiFonts,
    loy: baylee_client_core::manapip::Loyalty,
    size: f32,
    body_ink: Color,
    numeral: Color,
) -> LoyaltyBadge {
    // `spawn_loyalty` writes the whole `Node` itself, so the entity it is
    // handed needs nothing on it but `Pickable::IGNORE`.
    let badge = commands.spawn(Pickable::IGNORE).id();
    spawn_loyalty(commands, fonts, badge, loy, size, body_ink, numeral)
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
