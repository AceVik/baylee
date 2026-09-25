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

/// How big the number on a loyalty badge is, as a share of the mark's size.
///
/// Larger than a mana pip's numeral, because a badge is [`BADGE_SPAN`] wider
/// than a disc and the number is the whole of what it says: the shape carries
/// the sign, so the digits are not competing with a `+` for the room.
const NUMERAL: f32 = 0.82;

/// The same, for a cost of two digits or more.
///
/// A badge is a glyph now and a glyph is the width the font drew it at, so a
/// `−12` cannot widen the shape the way a printed card does — the digits give
/// way instead. The measure is characters and not digits, so the `X` a few
/// walkers print is one character and keeps the full size.
const WIDE_NUMERAL: f32 = 0.60;

/// Where a body colour stops being dark, in **linear** luminance.
///
/// `L* = 50`, the middle of perceptual lightness, which is `Y ≈ 0.1833` and
/// not the 0.5 an sRGB byte suggests — an sRGB 50% grey is already 0.214 of
/// the light. The pivot is the standard one for choosing black or white over
/// a fill, and it is written here rather than inlined because a bare 0.18 in
/// a colour comparison is a number nobody can check.
const HALF_LIGHT: f32 = 0.1833;

/// The colour a number is knocked out of a solid mark in.
///
/// A loyalty badge is the one mark in this client with **no colour of its
/// own**. A `{W}` pip is white wherever it is drawn, because white is what
/// the symbol is; a badge is ink on a ground, so its body has to be whatever
/// reads against the surface it lands on — and its number then has to read
/// against *that*. One follows from the other, so only one is a decision.
///
/// It is a flip and not a tint deliberately. A body lightened or darkened by
/// a fixed step keeps its contrast only where the ground happened to be the
/// one it was tuned on, which is the whole fault this exists to end: the
/// badge carried `PARCHMENT_INK` on `PARCHMENT` from a time when the sheet
/// was parchment, and went on carrying it after the sheet turned dark —
/// measured at **1.03:1** against an unselected row, which is a shape a
/// player cannot see at all.
fn knockout(body: Color) -> Color {
    let lit = body.to_linear();
    let luminance = 0.2126 * lit.red + 0.7152 * lit.green + 0.0722 * lit.blue;
    if luminance > HALF_LIGHT {
        palette::PARCHMENT_INK
    } else {
        palette::PARCHMENT
    }
}

/// How wide a loyalty badge is at its narrowest, as a share of its size.
///
/// The badge is wider than it is a mark across, because the card's is: a
/// lozenge carrying two digits is a wide shape and one carrying a single
/// digit keeps that width rather than shrinking into a lozenge of its own.
/// Named because a caller that lays a badge beside prose has to take the
/// width out of the prose's budget, and a `1.55` written twice is a number
/// that drifts.
pub(crate) const BADGE_SPAN: f32 = 1.55;

/// How big a mark set *in* a sentence is, as a share of the letters round it.
///
/// Roughly the cap height, so `{T}: Add {G}` reads as one sentence rather
/// than as prose with badges dropped into it. It is a default and not a rule:
/// [`spawn_rich_marks`] is the door for a line whose marks are what a player
/// reads and whose words are the aside.
const MARK_SHARE: f32 = 0.88;

/// A planeswalker's loyalty cost, drawn with the badge the card prints.
///
/// The shape is the Mana font's own — `manapip::loyalty_glyph`, the fourth
/// glyph door and the only place the three codepoints are written. The client
/// used to build the badge instead, out of a rounded slab with a square turned
/// 45° behind it, and the trick was sound: `bevy_ui` paints later siblings in
/// front, so the body covered the half of the diamond that would have stuck
/// out the far end, and a pentagon came out of two rectangles with no clip at
/// all. What it could not make was a **zero** — a flat lozenge is not a slab
/// with a point on it — so the flat tick was given the upward badge, with a
/// comment conceding that no printed card does that. The font has all three.
///
/// Two children on one parent. The glyph goes down first, absolutely
/// positioned so it fills the badge; the number goes on last, laid out in the
/// content box. Absolute children are placed against the parent's **padding**
/// box, which is what makes the padding here a lever on the *number* alone:
/// a badge's ink centre is not its box centre — a point at one end carries no
/// digits — so the padding is the difference between the two, and
/// `manapip::loyalty_numeral_centre` is where that difference is measured.
///
/// A mana pip is a light disc carrying dark ink. The badge is deliberately the
/// other way round — dark body, parchment numeral — because a loyalty cost
/// drawn in the pip's own register would read as generic mana, and the two
/// stand in the same column of the same row.
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
    use baylee_client_core::manapip;

    let span = size * BADGE_SPAN;
    let box_height = span * manapip::LOYALTY_BOX;
    // Where the digits want to sit, against where the flex box would put
    // them. A centred content box has its middle at half the height; the
    // padding on one side is twice the distance between the two, because
    // padding on one side moves the middle by half of itself.
    let drift = (manapip::loyalty_numeral_centre(loy.tick) - 0.5) * box_height * 2.0;
    commands.entity(badge).insert(Node {
        width: px(span),
        height: px(box_height),
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        padding: bevy::ui::UiRect {
            top: px(drift.max(0.0)),
            bottom: px((-drift).max(0.0)),
            ..default()
        },
        ..default()
    });

    let shape = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                right: px(0.0),
                top: px(0.0),
                bottom: px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(manapip::loyalty_glyph(loy.tick).to_string()),
            mana_tf(fonts, span),
            TextColor(body_ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(shape).add_child(glyph);
    commands.entity(badge).add_child(shape);

    // A two-digit cost is real — `−12` is printed — and the badge cannot grow
    // to take it, because a glyph has the width the font drew it at. So the
    // digits give way instead, which is the one thing about this that a card
    // does differently: there the badge widens.
    let caption = loy.caption();
    let share = if caption.chars().count() > 1 {
        WIDE_NUMERAL
    } else {
        NUMERAL
    };
    let text = commands
        .spawn((
            Text::new(caption),
            crate::hud::tf_bold(fonts, size * share),
            TextColor(numeral),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(badge).add_child(text);

    LoyaltyBadge {
        root: badge,
        body: glyph,
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
    /// The node painted in the body colour: the badge's own glyph.
    pub body: Entity,
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
/// [`MARK_SHARE`] of it, so the line reads as a sentence rather than as prose
/// with badges dropped into it.
pub fn spawn_rich(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
    color: Color,
) -> Entity {
    rich(
        commands,
        fonts,
        text,
        size,
        size * MARK_SHARE,
        color,
        crate::hud::tf,
    )
}

/// The same line with the **marks set apart from the words**.
///
/// [`spawn_rich`] ties the two together at [`MARK_SHARE`], which is right for
/// a sentence and wrong for a *cost*: a cost is read for its marks, and the
/// words in it (`, `, `Sacrifice this`) are the aside. Tying them would mean
/// choosing between an unreadable mark and a cost set larger than the ability
/// it belongs to — a loyalty badge's numeral is [`NUMERAL`] of its size, so
/// the digit on a badge set at a sentence's own size comes out under ten
/// pixels.
///
/// `marks` is the disc's diameter and not a share, because the caller sizing
/// it is sizing a mark and not scaling a sentence.
pub fn spawn_rich_marks(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
    marks: f32,
    color: Color,
) -> Entity {
    rich(commands, fonts, text, size, marks, color, crate::hud::tf)
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
    rich(
        commands,
        fonts,
        text,
        size,
        size * MARK_SHARE,
        color,
        crate::hud::tf_bold,
    )
}

/// The same line, set in `face` at exactly `size`.
///
/// For a caller that fitted the line itself: a card's text face measures its
/// rules text in the face's own font at the size it chose
/// ([`rich_depth`]), and has to be drawn in that font at that size — not at
/// [`crate::hud::tf`]'s nominal size, which is [`crate::hud::UI_SCALE`]
/// larger and a weight up when small (#259).
pub fn spawn_rich_in(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
    color: Color,
    face: fn(&UiFonts, f32) -> TextFont,
) -> Entity {
    rich(commands, fonts, text, size, size * MARK_SHARE, color, face)
}

/// How deep [`spawn_rich_in`] stands `text` at `size` in a column `room`
/// pixels wide, in pixels. `width` measures a run of words at `size`, in
/// pixels.
///
/// A model of [`rich`]'s layout, not the layout: its row is a flex row that
/// wraps, and its items are the runs of words and the marks. So a run that
/// does not fit beside the items before it starts a line of its own, whole,
/// and wraps there at the column's width, and the item after a wrapped run
/// starts the next line. A mark with the punctuation it holds on to is one
/// item. Lines of text are bevy's 1.2 em ([`textface::LINE_BOX`]), and bevy
/// measures a run of text up to whole pixels, so a line of 13.2 stands 14
/// deep and three stand 40. Flex lines are [`air`] apart.
///
/// Where it cannot know, it errs deep: a run is measured with its spaces and
/// without kerning, and a word wider than the column is taken to break. A
/// face that this says fits must fit when laid out, which
/// `face::tests::a_face_fitted_to_its_box_fits_it_in_bevy_s_layout` holds it
/// to over cards whose text is hardest to model.
///
/// [`textface::LINE_BOX`]: baylee_client_core::textface::LINE_BOX
pub(crate) fn rich_depth(text: &str, size: f32, room: f32, width: impl Fn(&str) -> f32) -> f32 {
    use baylee_client_core::manapip::{self, Segment};
    use baylee_client_core::textface::LINE_BOX;

    // A run of `n` lines, as bevy measures it: up to the whole pixel.
    let lines = |n: f32| (n * LINE_BOX * size).ceil();
    let marks = size * MARK_SHARE;
    let gap = air(size);
    let segments = manapip::segments(text);
    // The flex line being filled: how far it runs and how deep it is.
    let mut open: Option<(f32, f32)> = None;
    let mut depth = 0.0;
    let mut place = |w: f32, h: f32| {
        open = Some(match open {
            Some((run, deep)) if run + gap + w <= room => (run + gap + w, deep.max(h)),
            Some((_, deep)) => {
                depth += deep + gap;
                (w, h)
            }
            None => (w, h),
        });
    };
    let mut taken = 0usize;
    for (at, segment) in segments.iter().enumerate() {
        match segment {
            Segment::Text(words) => {
                let words = &words[taken.min(words.len())..];
                taken = 0;
                if words.is_empty() {
                    continue;
                }
                let run = width(words).ceil();
                if run <= room {
                    place(run, lines(1.0));
                } else {
                    place(room, lines(wrapped(words, room, &width)));
                }
            }
            Segment::Symbol(pip) => {
                let (mark, deep) = match pip {
                    manapip::Pip::Loyalty(_) => {
                        let span = marks * BADGE_SPAN;
                        (span, span * manapip::LOYALTY_BOX)
                    }
                    // `spawn_number`'s disc grows with its digits; half the
                    // disc a digit, which is more than one takes.
                    manapip::Pip::Number { value } => {
                        #[allow(clippy::cast_precision_loss)] // a handful of digits
                        let digits = value.to_string().len() as f32;
                        (marks.max(marks * (0.4 + 0.5 * digits)), marks)
                    }
                    manapip::Pip::Solid { .. } | manapip::Pip::Split { .. } => (marks, marks),
                };
                let glued = match segments.get(at + 1) {
                    Some(Segment::Text(next)) => manapip::clings(next),
                    _ => 0,
                };
                taken = glued;
                if glued == 0 {
                    place(mark, deep);
                } else {
                    let Some(Segment::Text(next)) = segments.get(at + 1) else {
                        unreachable!("`glued` is non-zero only for a text segment")
                    };
                    place(mark + width(&next[..glued]).ceil(), deep.max(lines(1.0)));
                }
            }
        }
    }
    open.map_or(0.0, |(_, deep)| depth + deep)
}

/// How many lines `words` wrap to in a column `room` wide, broken after the
/// last word that fits; a word wider than the column breaks inside itself.
fn wrapped(words: &str, room: f32, width: impl Fn(&str) -> f32) -> f32 {
    let space = width(" ");
    let mut lines = 1.0_f32;
    // How far the current line runs; `None` before its first word.
    let mut run: Option<f32> = None;
    for word in words.split(' ') {
        let w = width(word);
        let mut now = match run {
            Some(r) if r + space + w <= room => r + space + w,
            Some(_) => {
                lines += 1.0;
                w
            }
            None => w,
        };
        if now > room {
            let over = (now / room).ceil();
            lines += over - 1.0;
            now -= (over - 1.0) * room;
        }
        run = Some(now);
    }
    lines
}

/// The air between two marks in a line, measured against the **words**.
///
/// This gap is the space in a line of writing, and a line does not open up
/// because one character in it is drawn larger — so it comes off the
/// sentence's size and not the mark's, even on a line where the two have been
/// set apart.
///
/// `const` because [`crate::hud::sheet`] builds a column's width out of it,
/// and a second copy of `0.12` over there is a number that drifts from this
/// one.
pub(crate) const fn air(size: f32) -> f32 {
    if size * 0.12 > 1.0 { size * 0.12 } else { 1.0 }
}

/// How wide a line of **marks** is, laid out the way [`rich`] lays one out.
///
/// `None` where the line is not marks alone. A [`Pip::Number`] grows with its
/// digits and a text run is prose, and neither has a width this side of the
/// text engine — a caller that needs a number back is told so rather than
/// handed a guess.
///
/// It exists because a *column* of costs is a shared width, and that width has
/// to be known before anything is laid out. Predicting it here is the point:
/// [`BADGE_SPAN`] is `pub(crate)` precisely so a caller mirroring a width
/// reads it instead of retyping `1.55`, and the gap comes off [`air`] for the
/// same reason.
pub(crate) fn marks_span(text: &str, size: f32, marks: f32) -> Option<f32> {
    use baylee_client_core::manapip::Segment;
    let mut span = 0.0;
    let mut count: u32 = 0;
    for segment in baylee_client_core::manapip::segments(text) {
        match segment {
            // A blank run between two marks is a child with no width of its
            // own; it still sits in the flex row, so it still opens a gap.
            Segment::Text(words) if words.trim().is_empty() => count += 1,
            Segment::Symbol(Pip::Loyalty(_)) => {
                span += marks * BADGE_SPAN;
                count += 1;
            }
            Segment::Symbol(Pip::Solid { .. } | Pip::Split { .. }) => {
                span += marks;
                count += 1;
            }
            Segment::Text(_) | Segment::Symbol(Pip::Number { .. }) => return None,
        }
    }
    // Rounded **up**: a column a pixel wider than its content shows nothing,
    // and one a pixel narrower wraps its last mark onto a line of its own —
    // which is the ragged cost this width was computed to prevent.
    (count > 0).then(|| (span + air(size) * (count - 1) as f32).ceil())
}

/// All three of the above, with what they differ in passed in.
#[allow(clippy::too_many_arguments)] // two sizes, a colour and a face
fn rich(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
    marks: f32,
    color: Color,
    face: fn(&UiFonts, f32) -> TextFont,
) -> Entity {
    let row = commands
        .spawn((
            Node {
                // Both gaps are [`air`] — see there for why it is measured
                // against the words. The row one is only ever seen when a
                // line wraps, and then it is leading: five marks folded into
                // a narrow column touch without it.
                column_gap: px(air(size)),
                row_gap: px(air(size)),
                align_items: AlignItems::Center,
                flex_wrap: bevy::ui::FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let segments = baylee_client_core::manapip::segments(text);
    // How much of the *next* segment the mark just drawn took with it. A row
    // may wrap between any two of its items, so a segment beginning `". This
    // artifact deals…"` put the full stop at the head of the second line —
    // see `manapip::clings`, which is where the rule is and why.
    let mut taken = 0usize;
    for (at, segment) in segments.iter().enumerate() {
        let child = match segment {
            baylee_client_core::manapip::Segment::Text(words) => {
                let words = &words[taken.min(words.len())..];
                taken = 0;
                if words.is_empty() {
                    continue;
                }
                commands
                    .spawn((
                        Text::new(words.to_string()),
                        face(fonts, size),
                        TextColor(color),
                        Pickable::IGNORE,
                    ))
                    .id()
            }
            baylee_client_core::manapip::Segment::Symbol(pip) => {
                // A loyalty badge is the one mark with no colour of its own.
                // A `{W}` disc is white on a parchment row and white on a
                // dark one, because white is what the symbol *is*; a badge is
                // ink on a ground, and which ink reads is the caller's
                // question. This is the only place a badge is drawn inside a
                // line, and the only place the line's own ink is in hand, so
                // it is drawn here rather than through [`spawn_pip`] — which
                // took the parchment pair on trust and drew the badge at
                // 1.03:1 against the sheet's own rows for as long as the
                // sheet has been dark.
                let pip = match pip {
                    baylee_client_core::manapip::Pip::Loyalty(loy) => {
                        spawn_loyalty_badge(commands, fonts, *loy, marks, color, knockout(color))
                            .root
                    }
                    other => spawn_pip(commands, fonts, *other, marks),
                };
                // The punctuation after a mark rides with it, in a row of
                // their own that cannot wrap and has no gap in it — which is
                // where a printed card puts a full stop too.
                let glued = match segments.get(at + 1) {
                    Some(baylee_client_core::manapip::Segment::Text(next)) => {
                        baylee_client_core::manapip::clings(next)
                    }
                    _ => 0,
                };
                taken = glued;
                if glued == 0 {
                    pip
                } else {
                    let Some(baylee_client_core::manapip::Segment::Text(next)) =
                        segments.get(at + 1)
                    else {
                        unreachable!("`glued` is non-zero only for a text segment")
                    };
                    let mark = commands
                        .spawn((
                            Text::new(next[..glued].to_string()),
                            face(fonts, size),
                            TextColor(color),
                            Pickable::IGNORE,
                        ))
                        .id();
                    let pair = commands
                        .spawn((
                            Node {
                                align_items: AlignItems::Center,
                                flex_wrap: bevy::ui::FlexWrap::NoWrap,
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .id();
                    commands.entity(pair).add_children(&[pip, mark]);
                    pair
                }
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

    /// WCAG's contrast ratio between two opaque colours.
    fn contrast(a: Color, b: Color) -> f32 {
        let y = |c: Color| {
            let lit = c.to_linear();
            0.2126 * lit.red + 0.7152 * lit.green + 0.0722 * lit.blue
        };
        let (hi, lo) = if y(a) > y(b) {
            (y(a), y(b))
        } else {
            (y(b), y(a))
        };
        (hi + 0.05) / (lo + 0.05)
    }

    /// The number on a badge is legible whatever ink the line is set in.
    ///
    /// The bound is 4.5:1, the floor for body text, and it is asked of every
    /// ink this client actually writes a line in — because the fault this
    /// guards against was not a wrong colour but a colour that stopped being
    /// right when the surface under it changed. `PARCHMENT_INK` on
    /// `PARCHMENT` was a correct pair, and it measured 1.03:1 on the sheet
    /// the day the sheet went dark.
    #[test]
    fn a_badges_number_reads_against_the_badge_whatever_the_line_is_set_in() {
        for (name, ink) in [
            ("INK", palette::INK),
            ("MUTED", palette::MUTED),
            ("PARCHMENT_INK", palette::PARCHMENT_INK),
            ("PARCHMENT", palette::PARCHMENT),
            ("DIALOG_LIT", palette::DIALOG_LIT),
        ] {
            let ratio = contrast(ink, knockout(ink));
            assert!(ratio >= 4.5, "{name}: the numeral reads at {ratio:.2}:1");
        }
        // And it is a flip, not a tint: a light body takes a dark number and
        // a dark body a light one, which is what makes the bound hold at both
        // ends rather than on the ground it was tuned against.
        assert_eq!(knockout(palette::INK), palette::PARCHMENT_INK);
        assert_eq!(knockout(palette::PARCHMENT_INK), palette::PARCHMENT);
    }

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
    /// What it asserts is the split — prose stays prose, a symbol becomes a
    /// disc, and **the punctuation after a mark cannot be parted from it** —
    /// and that nothing anywhere inside the row is pickable. A `Text` is a
    /// `Node`, so one pickable label sits in front of the button it labels and
    /// the middle of that button goes dead.
    ///
    /// The cling is the half with a picture behind it. The row wraps, so it
    /// may break between any two of its items, and `{T}: Add {U} or {B}. This
    /// artifact deals 1 damage to you.` put the full stop at the head of its
    /// second line — in German, where the sentence is long enough to wrap.
    /// `manapip::clings` is the rule and this is where it is drawn.
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
        // Three, because each mark took its own punctuation with it: `{T}:`,
        // then ` Add `, then `{G}.` — and those are the only three places this
        // line may break.
        assert_eq!(children.len(), 3, "two glued marks and the words between");

        let texts: Vec<String> = children
            .iter()
            .filter_map(|e| app.world().entity(*e).get::<Text>().map(|t| t.0.clone()))
            .collect();
        assert_eq!(
            texts,
            vec![" Add ".to_string()],
            "the braces are gone from the prose because they became discs, \
             and the colon and the full stop went with their marks",
        );

        // Each glued pair is a disc and its punctuation, in that order.
        let glued: Vec<Vec<String>> = children
            .iter()
            .filter(|e| app.world().entity(**e).get::<Text>().is_none())
            .map(|e| {
                app.world()
                    .entity(*e)
                    .get::<Children>()
                    .expect("a glued pair has children")
                    .iter()
                    .filter_map(|c| app.world().entity(c).get::<Text>().map(|t| t.0.clone()))
                    .collect()
            })
            .collect();
        assert_eq!(glued.len(), 2, "{{T}} and {{G}} are drawn, not spelled");
        assert!(glued[0].contains(&":".to_string()), "{glued:?}");
        assert!(glued[1].contains(&".".to_string()), "{glued:?}");

        // Everywhere, and not only at the top: the glued pair put a `Text` one
        // level further down than this assertion used to reach.
        let mut stack = children.clone();
        while let Some(entity) = stack.pop() {
            assert!(
                app.world().entity(entity).contains::<Pickable>(),
                "every node inside a label carries Pickable::IGNORE",
            );
            if let Some(kids) = app.world().entity(entity).get::<Children>() {
                stack.extend(kids.iter());
            }
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

    /// A mark's width is what [`marks_span`] says it is, or the columns built
    /// on it are built on a guess.
    ///
    /// Both halves matter. The **numbers** are what a cost column is sized
    /// from, and a badge is [`BADGE_SPAN`] wider than a disc — the one case
    /// where a cost of one mark is not one mark across. The **refusals** are
    /// what keeps the guess out: a payment with words in it has no width this
    /// side of the text engine, and answering with the marks alone would size
    /// a column to `{1}, {T},` and let `Sacrifice this artifact` hang off the
    /// paper, which is exactly what it used to do.
    #[test]
    fn a_line_of_marks_is_as_wide_as_the_marks_in_it() {
        let air = air(13.0);
        assert!(
            (air - 1.56).abs() < 0.01,
            "the gap comes off the words: {air}"
        );
        assert_eq!(marks_span("{T}", 13.0, 16.0), Some(16.0));
        assert_eq!(
            marks_span("{2}{U}{U}", 13.0, 16.0),
            Some((3.0 * 16.0 + 2.0 * air).ceil())
        );
        assert_eq!(
            marks_span("{W}{U}{B}{R}{G}", 13.0, 16.0),
            Some((5.0 * 16.0 + 4.0 * air).ceil())
        );
        assert_eq!(
            marks_span("{L+2}", 13.0, 16.0),
            Some((16.0 * BADGE_SPAN).ceil()),
            "a loyalty badge is wider than the mark it is drawn at"
        );
        assert_eq!(marks_span("Sacrifice this artifact", 13.0, 16.0), None);
        assert_eq!(marks_span("{1}, {T}, Pay 1 life", 13.0, 16.0), None);
        assert_eq!(marks_span("", 13.0, 16.0), None);
        // A badge lives behind its token, the way every other mark does.
        // `+2` with no braces is prose and is measured as prose: refused.
        assert_eq!(marks_span("+2", 13.0, 16.0), None);
    }

    /// The model of a rich line counts it as [`rich`] lays it out: words
    /// broken after the last that fits, a word wider than the column broken
    /// inside itself, a run that will not fit beside a mark moved to a line
    /// of its own, whole, and every line measured up to the whole pixel.
    #[test]
    fn a_rich_line_is_counted_as_it_is_laid_out() {
        // Ten pixels a line of 1.2 em; five pixels a character, so twenty to
        // a line of a hundred.
        let (size, room) = (10.0, 100.0);
        #[allow(clippy::cast_precision_loss)]
        let depth = |text: &str| rich_depth(text, size, room, |s| s.chars().count() as f32 * 5.0);
        let near = |got: f32, want: f32| (got - want).abs() < 1e-4;

        assert!(near(depth("aaaa bbbb"), 12.0), "one line");
        assert!(
            near(depth("aaaa bbbb cccc dddd eeee"), 24.0),
            "two, broken after dddd"
        );
        assert!(near(depth(&"a".repeat(45)), 36.0), "a word of 45 is three");
        // `{T}:` is one item and the run after it another; twenty characters
        // fill a line, so they cannot stand beside the mark.
        let run = format!("{{T}}: {}", "b".repeat(19));
        assert!(
            near(depth(&run), 12.0 + air(size) + 12.0),
            "{}",
            depth(&run)
        );
        // Nineteen can: one line.
        let run = format!("{{T}}: {}", "b".repeat(15));
        assert!(near(depth(&run), 12.0), "{}", depth(&run));
        // Marks alone stand a mark deep.
        assert!(near(depth("{W}{U}"), size * MARK_SHARE));
        // 13.2 is measured as 14.
        assert!(near(rich_depth("a", 11.0, room, |_| 5.0), 14.0));
        // And a run's width is measured up too: 90.495 is 91, which will
        // not stand beside a mark in 90.5 of the line.
        #[allow(clippy::cast_precision_loss)]
        let run = rich_depth("{W} bbbbbbbbbbbbbbbbb", size, 100.5, |s| {
            s.chars().count() as f32 * 5.0275
        });
        assert!(near(run, size * MARK_SHARE + air(size) + 12.0), "{run}");
    }
}
