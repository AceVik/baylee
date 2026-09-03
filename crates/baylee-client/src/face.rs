//! Drawing the constructed card face.
//!
//! The model lives in [`baylee_client_core::card_face`] and knows nothing about
//! a renderer; this module is the part that turns it into pixels, twice:
//!
//! - as **UI nodes** for everything in the overlay — hand, preview, the
//!   own-board panel, the command zone;
//! - as **`Text2d` children of the card quad** for permanents on the table,
//!   where they inherit tap rotation, hover lift and stacking from the parent
//!   transform for free, and stay crisp at any zoom because they are laid out
//!   by the text pipeline rather than baked into a texture.
//!
//! # Two detail levels, on purpose
//!
//! A permanent on the table is roughly a centimetre tall on screen at a normal
//! camera distance. Rules text there is not small, it is invisible — and three
//! hundred permanents' worth of paragraphs is a lot of glyphs to lay out for
//! something nobody can read. So the table draws the identifying half of a card
//! (name, cost, type line, the numbers) and the overlay, where a player is
//! actually reading, draws all of it.
//!
//! # Mana symbols
//!
//! Drawn with the open-licensed `mana` font on discs this module paints —
//! see [`crate::manaui`]. `docs/legal.md` §2 allows that font by name and
//! rules out `WotC`'s own symbol artwork. The table path still sets the cost
//! as letters, because there it is one line of ordinary text rather than a
//! row of nodes.

use baylee_client_core::card_face::{CardFace, Stats, TextBlock};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_core::color::{Color as MagicColor, ColorSet};
use baylee_core::mana::{ManaSymbol, Variable};
use bevy::prelude::*;

use crate::hud::UiFonts;

/// How much of a card to draw.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Detail {
    /// Name, cost, type line, stats — what identifies the card at board size.
    Compact,
    /// Everything, including rules text.
    Full,
}

// --------------------------------------------------------------- when to use

/// Whether the player is holding the "show me the text" modifier.
#[derive(Resource, Default)]
pub struct FaceMode {
    /// True while Cmd or Alt is down.
    pub held: bool,
}

/// Tracks the modifier key.
///
/// Cmd *and* Alt, because the two platforms disagree about which one is the
/// harmless one to hold: Cmd is natural on macOS and Alt everywhere else, and
/// a browser may swallow either depending on the page.
pub fn track_modifier(keys: Res<ButtonInput<KeyCode>>, mut mode: ResMut<FaceMode>) {
    let held = keys.any_pressed([
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
    ]);
    if mode.held != held {
        mode.held = held;
    }
}

/// Whether the constructed face should be drawn instead of the image.
///
/// Three independent reasons, and any one of them is enough: the player is
/// holding the modifier, the player always wants text, or there is no image to
/// draw — a printing whose art failed, or an object with no card behind it at
/// all, which is the case for every token.
#[must_use]
pub fn wants_face(
    mode: &FaceMode,
    settings: &crate::settings::ClientSettings,
    textures: &crate::textures::CardTextures,
    art: Option<baylee_client_core::images::ImageKey>,
) -> bool {
    mode.held || settings.prefer_text_view || art.is_none_or(|key| textures.has_failed(key))
}

// ------------------------------------------------------------ building faces

/// The face for an object on the board, stack, graveyard, exile or command
/// zone.
#[must_use]
pub fn of_object(
    object: &baylee_view::PublicObject,
    texts: &crate::cardtext::CardTexts,
) -> CardFace {
    let printed = object
        .card
        .and_then(|c| baylee_cards::by_index(c.index).map(|def| (def, c)));
    let cost = printed.and_then(|(def, c)| def.faces.get(c.face as usize).map(|f| &f.mana_cost));
    let text = object.card.and_then(|c| texts.get(c.print, c.face));
    CardFace::from_object(object, cost, text.as_ref())
}

/// The face for a card in hand.
///
/// A hand card arrives as a [`baylee_view::HandObject`], which carries only
/// what the hand bar needed — no subtypes, no power. The rest comes from the
/// compiled registry, which is the right source anyway: a card in hand is the
/// printed card until something says otherwise.
#[must_use]
pub fn of_hand(card: &baylee_view::HandObject, texts: &crate::cardtext::CardTexts) -> CardFace {
    use baylee_client_core::card_face::Characteristics;
    use baylee_core::types::{SubtypeSet, SupertypeSet};

    let face_def = baylee_cards::by_index(card.card.index)
        .and_then(|def| def.faces.get(card.card.face as usize));
    let chars = Characteristics {
        name: card.name.clone(),
        types: card.types,
        supertypes: face_def.map_or(SupertypeSet::EMPTY, |f| f.supertypes),
        subtypes: face_def.map_or(SubtypeSet::EMPTY, |f| SubtypeSet::from_slice(f.subtypes)),
        colors: card.colors,
        power: face_def.and_then(|f| f.power),
        toughness: face_def.and_then(|f| f.toughness),
        loyalty: face_def.and_then(|f| f.loyalty),
        damage: 0,
    };
    let text = texts.get(card.card.print, card.card.face);
    CardFace::build(&chars, face_def.map(|f| &f.mana_cost), text.as_ref())
}

// ------------------------------------------------------------------ palette

/// The card's frame colour, from its projected colours.
///
/// Follows the printed frames closely enough to be read at a glance: one
/// colour gets that colour, two or more get gold, none gets the artifact grey.
fn frame_color(colors: ColorSet) -> Color {
    let mut found: Option<MagicColor> = None;
    let mut count = 0;
    for color in [
        MagicColor::White,
        MagicColor::Blue,
        MagicColor::Black,
        MagicColor::Red,
        MagicColor::Green,
    ] {
        if colors.contains(color) {
            count += 1;
            found = Some(color);
        }
    }
    match (count, found) {
        (0, _) => Color::srgb(0.29, 0.30, 0.33),
        (1, Some(MagicColor::White)) => Color::srgb(0.51, 0.47, 0.38),
        (1, Some(MagicColor::Blue)) => Color::srgb(0.16, 0.35, 0.52),
        (1, Some(MagicColor::Black)) => Color::srgb(0.20, 0.19, 0.23),
        (1, Some(MagicColor::Red)) => Color::srgb(0.55, 0.24, 0.19),
        (1, Some(MagicColor::Green)) => Color::srgb(0.20, 0.40, 0.26),
        _ => Color::srgb(0.52, 0.44, 0.22),
    }
}

/// The face's inner panel: a dark surface the text sits on.
const PAPER: Color = Color::srgb(0.09, 0.10, 0.12);
/// Primary text on the face.
const INK: Color = Color::srgb(0.91, 0.93, 0.95);
/// Type line and reminder text.
const MUTED: Color = Color::srgb(0.62, 0.67, 0.71);

/// The colour of a card quad that is drawing its face instead of its art.
///
/// Paper, washed with the card's own colour identity. At board zoom that wash
/// is often all a player reads, and it is the same thing the printed frame
/// would have told them.
#[must_use]
pub fn table_color(colors: ColorSet) -> Color {
    use bevy::color::Mix;
    PAPER.mix(&frame_color(colors), 0.30)
}

/// The label inside one mana pip.
fn pip_label(symbol: ManaSymbol) -> String {
    match symbol {
        ManaSymbol::Generic(n) => n.to_string(),
        ManaSymbol::Colorless => "C".to_string(),
        ManaSymbol::White => "W".to_string(),
        ManaSymbol::Blue => "U".to_string(),
        ManaSymbol::Black => "B".to_string(),
        ManaSymbol::Red => "R".to_string(),
        ManaSymbol::Green => "G".to_string(),
        ManaSymbol::Snow => "S".to_string(),
        ManaSymbol::Hybrid(pair) | ManaSymbol::HybridPhyrexian(pair) => {
            format!(
                "{}{}",
                color_letter(pair.first()),
                color_letter(pair.second())
            )
        }
        ManaSymbol::TwoOrColor(c) => format!("2{}", color_letter(c)),
        ManaSymbol::Phyrexian(c) => format!("{}φ", color_letter(c)),
        ManaSymbol::Variable(Variable::X) => "X".to_string(),
        ManaSymbol::Variable(Variable::Y) => "Y".to_string(),
        ManaSymbol::Variable(Variable::Z) => "Z".to_string(),
        ManaSymbol::HalfGeneric => "½".to_string(),
        ManaSymbol::Infinite => "∞".to_string(),
    }
}

/// The one-letter code for a colour.
const fn color_letter(color: MagicColor) -> &'static str {
    match color {
        MagicColor::White => "W",
        MagicColor::Blue => "U",
        MagicColor::Black => "B",
        MagicColor::Red => "R",
        MagicColor::Green => "G",
    }
}

// ----------------------------------------------------------------- UI nodes

/// Builds the face as a UI subtree and returns its root entity.
///
/// `width` is the rendered card width in pixels; every size on the face is a
/// fraction of it, so one function serves a 110-pixel hand card and a
/// 500-pixel preview without a second set of constants.
#[allow(clippy::too_many_lines)] // one card, top to bottom
pub fn spawn_ui(
    commands: &mut Commands,
    lang: Lang,
    face: &CardFace,
    width: f32,
    detail: Detail,
    fonts: &UiFonts,
) -> Entity {
    let pad = width * 0.05;
    let title_size = (width * 0.082).clamp(7.0, 22.0);
    let type_size = (width * 0.062).clamp(6.0, 16.0);
    let body_size = (width * 0.058).clamp(6.0, 15.0);

    let root = commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(pad)),
                row_gap: Val::Px(pad * 0.5),
                ..default()
            },
            BackgroundColor(frame_color(face.colors)),
        ))
        .id();

    // ---- title row: name on the left, cost pips on the right ------------
    let title = commands
        .spawn(Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(pad * 0.4),
            ..default()
        })
        .id();
    let name = commands
        .spawn((
            Text::new(face.name.clone()),
            text_font(fonts, title_size),
            TextColor(INK),
            Node {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                width: Val::Px(0.0),
                ..default()
            },
        ))
        .id();
    commands.entity(title).add_child(name);
    let pips = spawn_pips(commands, face, width, fonts);
    commands.entity(title).add_child(pips);
    commands.entity(root).add_child(title);

    // ---- the paper: type line, then rules text --------------------------
    let paper = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(pad * 0.7)),
                row_gap: Val::Px(pad * 0.5),
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(Val::Px(width * 0.03)),
                ..default()
            },
            BackgroundColor(PAPER),
        ))
        .id();

    let type_line = commands
        .spawn((
            Text::new(face.type_line.clone()),
            text_font(fonts, type_size),
            TextColor(MUTED),
        ))
        .id();
    commands.entity(paper).add_child(type_line);

    if detail == Detail::Full {
        for block in &face.body {
            let (text, color) = match block {
                TextBlock::Rules(t) => (t.clone(), INK),
                TextBlock::Reminder(t) => (format!("({t})"), MUTED),
            };
            let node = commands
                .spawn((
                    Text::new(text),
                    text_font(fonts, body_size),
                    TextColor(color),
                ))
                .id();
            commands.entity(paper).add_child(node);
        }
        if face.text_pending {
            // Not an error: the catalog answer may still be in flight, or the
            // gateway may not be configured. Either way the card is playable,
            // and saying so beats an empty box.
            let node = commands
                .spawn((
                    Text::new(Phrase::NoRulesTextHere.text(lang)),
                    text_font(fonts, body_size),
                    TextColor(MUTED),
                ))
                .id();
            commands.entity(paper).add_child(node);
        }
    }
    commands.entity(root).add_child(paper);

    // ---- the numbers, bottom right --------------------------------------
    if let Some(stats) = face.stats {
        let badge = commands
            .spawn((
                Node {
                    align_self: AlignSelf::FlexEnd,
                    padding: UiRect::axes(Val::Px(pad * 0.6), Val::Px(pad * 0.2)),
                    border_radius: BorderRadius::all(Val::Px(width * 0.03)),
                    ..default()
                },
                BackgroundColor(PAPER),
                children![(
                    Text::new(stats_label(stats)),
                    text_font(fonts, title_size),
                    TextColor(stats_color(stats)),
                )],
            ))
            .id();
        commands.entity(root).add_child(badge);
    }
    root
}

/// The row of mana pips.
fn spawn_pips(commands: &mut Commands, face: &CardFace, width: f32, fonts: &UiFonts) -> Entity {
    let size = (width * 0.11).clamp(8.0, 26.0);
    let row = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(size * 0.12),
            flex_shrink: 0.0,
            ..default()
        })
        .id();
    for symbol in &face.cost {
        // The `mana` font draws the printed mark; the letter this used to set
        // was standing in for it, and a hybrid could not be spelled at all.
        let pip = crate::manaui::spawn_pip(
            commands,
            fonts,
            baylee_client_core::manapip::pip(*symbol),
            size,
        );
        commands.entity(row).add_child(pip);
    }
    row
}

/// `4/4`, or a loyalty number.
fn stats_label(stats: Stats) -> String {
    match stats {
        Stats::PowerToughness {
            power,
            toughness,
            damage,
        } => {
            let remaining = toughness - damage as i16;
            if damage > 0 {
                // Damage is what decides whether a block is lethal, so the
                // face shows the number that matters and the printed one
                // behind it rather than making the player subtract.
                format!("{power}/{remaining} ({toughness})")
            } else {
                format!("{power}/{toughness}")
            }
        }
        Stats::Loyalty(l) => l.to_string(),
    }
}

/// Red once damage has made the toughness matter.
fn stats_color(stats: Stats) -> Color {
    match stats {
        Stats::PowerToughness {
            toughness, damage, ..
        } if toughness - damage as i16 <= 0 => Color::srgb(0.91, 0.47, 0.42),
        _ => INK,
    }
}

/// A text font handle at a size.
fn text_font(fonts: &UiFonts, size: f32) -> TextFont {
    TextFont {
        font: bevy::text::FontSource::Handle(fonts.text.clone()),
        font_size: bevy::text::FontSize::Px(size),
        ..default()
    }
}

// ------------------------------------------------------------- world (2.5D)

/// Marks the text entities belonging to one card's face on the table, so a
/// redraw can remove them without touching the card itself.
#[derive(Component)]
pub struct WorldFace;

/// Attaches the compact face to a card quad on the table.
///
/// Positions are in the quad's local space, where the card is
/// [`baylee_client_core::layout::CARD_WIDTH`] by `CARD_HEIGHT` units centred on
/// the origin. Children inherit the parent's rotation, so a tapped card's face
/// turns with it and needs no special case.
pub fn spawn_world(
    commands: &mut Commands,
    card: Entity,
    face: &CardFace,
    fonts: &UiFonts,
) -> Vec<Entity> {
    use baylee_client_core::layout::{CARD_HEIGHT, CARD_WIDTH};

    // Text2d is laid out in pixels and then scaled into world units; a small
    // font scaled up stays sharp because the glyphs are rasterised at the
    // size the camera actually needs.
    const PX_PER_UNIT: f32 = 100.0;
    let scale = 1.0 / PX_PER_UNIT;
    let half_h = CARD_HEIGHT / 2.0;
    let width_px = CARD_WIDTH * PX_PER_UNIT * 0.88;

    let mut spawned = Vec::with_capacity(4);
    {
        let mut line = |text: String, size: f32, color: Color, y: f32, z: f32| {
            let entity = commands
                .spawn((
                    WorldFace,
                    Text2d::new(text),
                    TextFont {
                        font: bevy::text::FontSource::Handle(fonts.text.clone()),
                        font_size: bevy::text::FontSize::Px(size),
                        ..default()
                    },
                    TextColor(color),
                    TextLayout::default().with_justify(Justify::Center),
                    TextBounds::new(width_px, f32::INFINITY),
                    Transform::from_xyz(0.0, y, z).with_scale(Vec3::splat(scale)),
                    ChildOf(card),
                ))
                .id();
            spawned.push(entity);
        };

        // z lifts the text off the quad so it is never z-fought by the card.
        let z = 0.002;
        line(face.name.clone(), 13.0, INK, half_h * 0.72, z);
        if !face.cost.is_empty() {
            let cost: String = face
                .cost
                .iter()
                .map(|s| pip_label(*s))
                .collect::<Vec<_>>()
                .join(" ");
            line(cost, 11.0, Color::srgb(0.85, 0.82, 0.72), half_h * 0.44, z);
        }
        line(face.type_line.clone(), 10.0, MUTED, half_h * 0.06, z);
        if let Some(stats) = face.stats {
            line(
                stats_label(stats),
                14.0,
                stats_color(stats),
                -half_h * 0.66,
                z,
            );
        }
    }
    spawned
}

use bevy::text::TextBounds;

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::mana::ManaCost;

    #[test]
    fn pips_label_every_symbol_a_cost_can_contain() {
        let cost = ManaCost::parse("{2}{W}{U/B}{2/R}{G/P}{X}{S}{C}");
        for symbol in cost.symbols() {
            let label = pip_label(symbol);
            assert!(!label.is_empty(), "{symbol:?} has no label");
        }
    }

    /// A player reads the damaged toughness to decide a block, so it has to be
    /// the number in front, with the printed one kept for context.
    #[test]
    fn a_damaged_creature_shows_what_is_left() {
        let stats = Stats::PowerToughness {
            power: 3,
            toughness: 4,
            damage: 3,
        };
        assert_eq!(stats_label(stats), "3/1 (4)");
        assert_eq!(
            stats_label(Stats::PowerToughness {
                power: 2,
                toughness: 2,
                damage: 0
            }),
            "2/2"
        );
        assert_eq!(stats_label(Stats::Loyalty(4)), "4");
    }

    /// Lethal damage is the one state that must be visible without reading
    /// the numbers.
    #[test]
    fn lethal_damage_turns_the_numbers_red() {
        let lethal = Stats::PowerToughness {
            power: 1,
            toughness: 2,
            damage: 2,
        };
        assert_ne!(stats_color(lethal), INK);
        assert_eq!(
            stats_color(Stats::PowerToughness {
                power: 1,
                toughness: 2,
                damage: 1
            }),
            INK
        );
    }

    /// The four independent reasons to draw the face. Each one alone is
    /// enough, and none of them may need the others.
    #[test]
    fn every_reason_to_draw_the_face_stands_on_its_own() {
        use baylee_client_core::images::{ArtSize, ImageKey};
        use baylee_core::ids::PrintRef;

        let mut images = Assets::<Image>::default();
        let mut textures = crate::textures::CardTextures::new(&mut images, 1 << 20);
        let art = ImageKey::new(PrintRef::new(0), 0, ArtSize::Small);
        let quiet = FaceMode { held: false };
        let held = FaceMode { held: true };
        let plain = crate::settings::ClientSettings::default();
        let latched = crate::settings::ClientSettings {
            prefer_text_view: true,
            ..crate::settings::ClientSettings::default()
        };

        // Art that is loading or loaded: the image wins.
        assert!(!wants_face(&quiet, &plain, &textures, Some(art)));
        // The modifier, the latch, and a token with no printing at all.
        assert!(wants_face(&held, &plain, &textures, Some(art)));
        assert!(wants_face(&quiet, &latched, &textures, Some(art)));
        assert!(wants_face(&quiet, &plain, &textures, None));
        // And art that will never arrive.
        textures.mark_failed(art);
        assert!(wants_face(&quiet, &plain, &textures, Some(art)));
    }

    /// The table quad is tinted by colour identity, so two different decks
    /// never read as the same wall of grey rectangles.
    #[test]
    fn the_table_face_is_tinted_by_colour_identity() {
        let red = table_color(ColorSet::from_slice(&[MagicColor::Red]));
        let blue = table_color(ColorSet::from_slice(&[MagicColor::Blue]));
        assert_ne!(red, blue);
        assert_ne!(red, PAPER);
    }

    /// Frames follow the printed convention: mono gets its colour, multicolour
    /// gets gold, colourless gets grey.
    #[test]
    fn frames_follow_the_printed_convention() {
        let mono = frame_color(ColorSet::from_slice(&[MagicColor::Blue]));
        let gold = frame_color(ColorSet::from_slice(&[MagicColor::Blue, MagicColor::Red]));
        let colorless = frame_color(ColorSet::default());
        assert_ne!(mono, gold);
        assert_ne!(gold, colorless);
        assert_ne!(mono, colorless);
    }
}
