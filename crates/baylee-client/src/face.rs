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
use baylee_client_core::cardplate::Plate;
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::textface;
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
/// draw *at this moment*.
///
/// That last one covers more than it looks like. An object with no card behind
/// it at all — every token — and a printing whose art failed, but also a
/// printing whose art has simply not arrived yet, and those are the same
/// situation for one frame at a time: a `Some(handle)` whose bytes are still
/// in flight cannot be bound, so the material never prepares and the card is
/// drawn as *nothing*. A readable face that turns into art a moment later is
/// the honest version of that wait, and it is free — the machinery is the one
/// a failed load already uses.
#[must_use]
pub fn wants_face(
    mode: &FaceMode,
    settings: &crate::settings::ClientSettings,
    textures: &crate::textures::CardTextures,
    art: Option<baylee_client_core::images::ImageKey>,
) -> bool {
    mode.held || settings.prefer_text_view || art.is_none_or(|key| !textures.has_arrived(key))
}

// ------------------------------------------------------------ building faces

/// The face for an object on the board, stack, graveyard, exile or command
/// zone.
///
/// `view` is what lets an **ability** be drawn in the player's language. An
/// ability on the stack has no card of its own, so `object.card` is `None`
/// and there is nothing to look text up by; the picture beside it is the
/// source permanent's, and so is the text. Finding that source needs the
/// view, which a caller that is drawing a permanent does not need and may
/// pass as `None` — a permanent always carries its own printing.
#[must_use]
pub fn of_object(
    object: &baylee_view::PublicObject,
    view: Option<&baylee_view::PlayerView>,
    texts: &crate::cardtext::CardTexts,
) -> CardFace {
    let printed = object
        .card
        .and_then(|c| baylee_cards::by_index(c.index).map(|def| (def, c)))
        .and_then(|(def, c)| def.faces.get(c.face as usize));
    let cost = printed.map(|f| &f.mana_cost);
    let text = card_of(object, view).and_then(|(card, face)| texts.face(card, face));
    CardFace::from_object(object, cost, printed.map(printed_types), text.as_ref())
}

/// The three type bitsets a registry face was printed with.
///
/// `FaceDef` keeps its subtypes as a slice of ids because that is what a card
/// file writes; the comparison wants the set, and building it here is the one
/// place the renderer has both `baylee-cards` and the client's model in view.
fn printed_types(face: &baylee_cards::dsl::FaceDef) -> baylee_client_core::card_face::PrintedTypes {
    use baylee_core::types::SubtypeSet;
    baylee_client_core::card_face::PrintedTypes {
        supertypes: face.supertypes,
        types: face.types,
        subtypes: SubtypeSet::from_slice(face.subtypes),
    }
}

/// The name to write for an object, in the player's own language.
///
/// The renderer's half of [`baylee_client_core::card_face::shown_name`]: that
/// function holds the clone guard and knows nothing about where text comes
/// from, and this one finds the printing whose text names the object.
///
/// For an **ability** that printing is the *source's*, because an ability has
/// no card of its own — and the face is the one the host says the ability's
/// sentence is printed on rather than the one the source is showing now, for
/// the reason [`baylee_view::StackText::face`] gives: a Sheoldred who has
/// turned back over while her chapter ability waits on the stack must not lend
/// that ability the other side's name. A source that has already left
/// (CR 113.7a) leaves the name English, because then there is nothing else to
/// call it by.
#[must_use]
pub fn name_of(
    object: &baylee_view::PublicObject,
    view: &baylee_view::PlayerView,
    texts: &crate::cardtext::CardTexts,
) -> String {
    let text = card_of(object, Some(view)).and_then(|(card, face)| texts.face(card, face));
    baylee_client_core::card_face::shown_name(&object.name, text.as_ref()).to_string()
}

/// What one *named* face of a card is called, in the player's own language.
///
/// [`name_of`] answers for the face an object is showing. This answers for a
/// face it is being *offered*: a pathway (CR 712.12) is one object showing one
/// of its two land faces, and the question is about both of them.
///
/// The hand is searched first and by hand, because
/// [`baylee_view::PlayerView::object`] does not look there — and a land being
/// played is in the hand, which is the whole case this exists for.
///
/// Three rungs, and each is the honest answer to its own failure: the
/// catalog's text where the gateway served it, the compiled registry's
/// English where it did not (still the card's own name), and `None` for a
/// card the view cannot place or the registry does not have, which leaves the
/// caller to say what kind of thing the option is instead.
#[must_use]
pub fn face_name(
    object: baylee_core::ids::ObjectId,
    face: usize,
    view: &baylee_view::PlayerView,
    texts: Option<&crate::cardtext::CardTexts>,
) -> Option<String> {
    let card = view
        .hand
        .iter()
        .find(|c| c.id == object)
        .map(|c| c.card.index)
        .or_else(|| {
            let object = view.object(object)?;
            object
                .rules
                .map(|r| r.card)
                .or_else(|| object.card.map(|c| c.index))
        })?;
    let served = u8::try_from(face)
        .ok()
        .zip(texts)
        .and_then(|(face, texts)| texts.get(card, face))
        .map(|text| text.name);
    served.or_else(|| {
        Some(
            baylee_cards::by_index(card)?
                .faces
                .get(face)?
                .name
                .to_string(),
        )
    })
}

/// Which card's text names this object, and which face of it.
///
/// The card its abilities are printed on ([`baylee_view::PublicObject::rules`])
/// before the card it is. The two differ only for a copy, and a copy is
/// drawn with the copied card's name, so the copied card's text is the one
/// that describes it — the clone guard in `CardFace::build` compares exactly
/// that. An ability on the stack has no card of its own: its text is the
/// card its sentence is printed on (its own `rules`), else its source's.
fn card_of(
    object: &baylee_view::PublicObject,
    view: Option<&baylee_view::PlayerView>,
) -> Option<(baylee_core::ids::CardIndex, u8)> {
    if let Some(rules) = object.rules {
        return Some((rules.card, rules.face));
    }
    if let Some(card) = object.card {
        return Some((card.index, card.face));
    }
    let Some(baylee_view::StackItem::Ability {
        source,
        text,
        rules,
        ..
    }) = object.stack_item
    else {
        return None;
    };
    let (card, face) = if let Some(rules) = rules {
        (rules.card, rules.face)
    } else {
        let card = view?.object(source)?.card?;
        (card.index, card.face)
    };
    Some((card, text.map_or(face, |t| t.face)))
}

/// The face for a card in hand.
///
/// A hand card arrives as a [`baylee_view::HandObject`], which carries only
/// what the hand zone needed — no subtypes, no power. The rest comes from the
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
    let text = texts.face(card.card.index, card.card.face);
    CardFace::build(
        &chars,
        face_def.map(|f| &f.mana_cost),
        face_def.map(printed_types),
        text.as_ref(),
    )
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
///
/// **Every node here carries [`Pickable::IGNORE`]**, and it is one rule rather
/// than eight decisions: a drawn face is never the pointer's target — the hand
/// card, the tray row or the stack entry around it is — and marking only the
/// root would achieve nothing. `bevy_ui`'s picking backend reports the
/// *deepest* node under the pointer and `bevy_picking::hover::build_hover_map`
/// stops at the first entity that carries no `Pickable` at all, so one
/// unmarked `Text` in the middle of the face is as opaque as the whole card:
/// it keeps `PickingInteraction` off the row and the row never lights.
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
            Pickable::IGNORE,
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
        .spawn((
            Pickable::IGNORE,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(pad * 0.4),
                ..default()
            },
        ))
        .id();
    let name = commands
        .spawn((
            Pickable::IGNORE,
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
            Pickable::IGNORE,
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
            Pickable::IGNORE,
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
            // Rich: this is the face a player reads when there is no printing
            // to show, so `{T}: Add {G}` has to be the symbols the card
            // prints rather than the letters they are written with.
            let node = crate::manaui::spawn_rich(commands, fonts, &text, body_size, color);
            commands.entity(paper).add_child(node);
        }
        if face.text_pending {
            // Not an error: the catalog answer may still be in flight, or the
            // gateway may not be configured. Either way the card is playable,
            // and saying so beats an empty box.
            let node = commands
                .spawn((
                    Pickable::IGNORE,
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
                Pickable::IGNORE,
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
                    TextColor(stats_color(stats, PANEL_INKS)),
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
        .spawn((
            Pickable::IGNORE,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(size * 0.12),
                flex_shrink: 0.0,
                ..default()
            },
        ))
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

/// The overlay's inks for the body's numbers on its dark panel: plain, and
/// lethal.
const PANEL_INKS: (Color, Color) = (INK, Color::srgb(0.91, 0.47, 0.42));

/// Red once damage has made the toughness matter, out of a face's pair of
/// inks: light on the overlay's dark panel, dark on the table's paper.
fn stats_color(stats: Stats, (ink, lethal): (Color, Color)) -> Color {
    match stats {
        Stats::PowerToughness {
            toughness, damage, ..
        } if toughness - damage as i16 <= 0 => lethal,
        _ => ink,
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

/// `Text2d` is laid out in pixels and scaled into the card's units: a
/// hundred to a card width, so an em in card widths is a hundredth of its
/// font size. A small font scaled up stays sharp, because the glyphs are
/// rasterised at the size the camera actually needs.
const PX_PER_UNIT: f32 = 100.0;

/// The body's numbers on the table's face, as an em in card widths, when the
/// plate does not already say them ([`world_stats`]).
const STATS_EM: f32 = 0.14;

/// The table face's ink, and its red for lethal damage: dark, because the
/// material draws its bars and its paper light ([`textface::INK`]).
const FACE_INKS: (Color, Color) = (
    Color::srgb_from_array(textface::INK),
    Color::srgb_from_array(textface::LETHAL_INK),
);

/// How wide a line of text is before anything has laid it out (#259).
///
/// The table's face fits a name and a type line to its bars ([`textface`]),
/// and a two-line name bar is a different face, so the width has to be known
/// when the face is chosen — not a frame later, when bevy has laid the text
/// out. So it is read off the font the
/// face is set in: the shipped Alegreya Sans Regular's own advances, summed.
/// Kerning is left out, which makes a width a hair long and never short: a
/// name the sum says fits, fits. A character the font does not have is taken
/// as a full em, which is what the fallback face bevy borrows for it will
/// roughly spend.
///
/// Until the font has arrived — on the web it is an HTTP fetch — there is
/// nothing to read, and the width is [`textface::average_width`]'s. Nothing
/// is drawn in the font until it arrives either, and a face fitted by the
/// average is fitted again then ([`Self::measured`]).
pub struct Widths<'a> {
    font: Option<swash::FontRef<'a>>,
}

impl<'a> Widths<'a> {
    /// Widths read off `font`, or the average without one.
    #[must_use]
    pub fn of(font: Option<&'a Font>) -> Self {
        Self {
            font: font.and_then(|font| swash::FontRef::from_index(font.data.data(), 0)),
        }
    }

    /// Whether these are the font's widths and not the average's.
    #[must_use]
    pub fn measured(&self) -> bool {
        self.font.is_some()
    }

    /// How wide `text` is at an em of one.
    #[must_use]
    pub fn width(&self, text: &str) -> f32 {
        let Some(font) = self.font else {
            return textface::average_width(text);
        };
        let charmap = font.charmap();
        let metrics = font.glyph_metrics(&[]).scale(1.0);
        text.chars()
            .map(|ch| match charmap.map(ch) {
                0 => 1.0,
                glyph => metrics.advance_width(glyph),
            })
            .sum()
    }
}

/// The table's face set to fit its bars: the name and the type line, each
/// at its size and on its lines.
///
/// Fitted apart from being spawned, because the name's lines are the name
/// bar's height, which the card's material draws: the two have to come from
/// one fitting.
#[derive(Clone, PartialEq, Debug)]
pub struct WorldFit {
    /// The name: one line or two.
    pub name: textface::Fitted,
    /// The type line: always one.
    pub kind: textface::Fitted,
    /// Whether the widths were the font's ([`Widths::measured`]).
    pub measured: bool,
}

impl WorldFit {
    /// `face`'s name and type line, fitted by `widths`.
    #[must_use]
    pub fn of(face: &CardFace, widths: &Widths<'_>) -> Self {
        use textface::{fit_name, fit_type};
        Self {
            name: fit_name(&face.name, |s| widths.width(s)),
            kind: fit_type(&face.type_line, |s| widths.width(s)),
            measured: widths.measured(),
        }
    }

    /// How many lines the name takes, which is what the name bar is.
    #[must_use]
    pub fn lines(&self) -> usize {
        self.name.lines.len()
    }
}

/// Where a point on the card, in card widths from its top-left corner with
/// `y` down the card, is in the card's own space: centred, `y` up.
fn on_the_card(x: f32, y: f32) -> Vec2 {
    use baylee_client_core::layout::{CARD_HEIGHT, CARD_WIDTH};
    Vec2::new(
        (x - 0.5) * CARD_WIDTH,
        CARD_HEIGHT * 0.5 - y * crate::table::DOWN_THE_CARD,
    )
}

/// Attaches the compact face to a card quad on the table.
///
/// Laid out by [`textface`], in the parts of a card the
/// shader draws in the print's window (#259): the name in the name bar, the
/// cost on the art box's first line, the type line in the type bar, each as
/// `fit` set it, in the dark ink the bars and the paper are drawn for. The
/// cost's ink is the one its art box lets it ([`textface::cost_ink`] of
/// `word`, the card's [`textface::face_word`]). Children inherit the
/// parent's rotation, so a tapped card's face turns with it and needs no
/// special case. `plate` is what the card's ledge already says, so the face
/// does not say it twice ([`world_stats`]).
pub fn spawn_world(
    commands: &mut Commands,
    card: Entity,
    face: &CardFace,
    fit: &WorldFit,
    word: u32,
    plate: Plate,
    fonts: &UiFonts,
) -> Vec<Entity> {
    use bevy::sprite::Anchor;
    use bevy::text::LineBreak;
    use textface::{BAR_PAD, LINE_BOX, Regions, TEXT_INSET};

    let WorldFit { name, kind, .. } = fit;
    let regions = Regions::table(fit.lines());

    let mut texts = Vec::with_capacity(4);
    {
        let mut line = |text: String, em: f32, color: Color, at: Vec2, anchor: Anchor| {
            let entity = commands
                .spawn((
                    WorldFace,
                    Text2d::new(text),
                    TextFont {
                        font: bevy::text::FontSource::Handle(fonts.text.clone()),
                        font_size: bevy::text::FontSize::Px(em * PX_PER_UNIT),
                        ..default()
                    },
                    TextColor(color),
                    // The lines are already broken: a fitted name is one line
                    // or two, and bevy breaking it again would undo the fit.
                    TextLayout::new(Justify::Left, LineBreak::NoWrap),
                    anchor,
                    // z lifts the text off the quad so it is never z-fought
                    // by the card.
                    Transform::from_translation(at.extend(0.002))
                        .with_scale(Vec3::splat(1.0 / PX_PER_UNIT)),
                    ChildOf(card),
                ))
                .id();
            texts.push(entity);
        };

        let [x0, y0, x1, _] = regions.name_bar;
        line(
            name.lines.join("\n"),
            name.em,
            FACE_INKS.0,
            on_the_card(x0 + TEXT_INSET, y0 + BAR_PAD),
            Anchor::TOP_LEFT,
        );
        if !face.cost.is_empty() {
            let cost: String = face
                .cost
                .iter()
                .map(|s| pip_label(*s))
                .collect::<Vec<_>>()
                .join(" ");
            let [_, top, _, _] = regions.art_box;
            line(
                cost,
                textface::SMALL_EM,
                Color::srgb_from_array(textface::cost_ink(word).srgb()),
                on_the_card(x1 - TEXT_INSET, top + BAR_PAD),
                Anchor::TOP_RIGHT,
            );
        }
        // Centred down the bar, which is sized for the type line's own size:
        // one that had to shrink stands in the middle of it.
        let [_, top, _, bottom] = regions.type_bar;
        line(
            kind.lines.concat(),
            kind.em,
            FACE_INKS.0,
            on_the_card(
                x0 + TEXT_INSET,
                top + (bottom - top - LINE_BOX * kind.em) * 0.5,
            ),
            Anchor::TOP_LEFT,
        );
        // Where a print has its power and toughness: the text box's foot,
        // right. Only for the one shape the plate cannot say.
        if let Some(stats) = world_stats(face.stats, plate) {
            let [_, _, right, bottom] = regions.text_box;
            line(
                stats_label(stats),
                STATS_EM,
                stats_color(stats, FACE_INKS),
                on_the_card(right - TEXT_INSET, bottom - BAR_PAD),
                Anchor::BOTTOM_RIGHT,
            );
        }
    }
    texts
}

/// The body line a table card's text face still has to write: none when the
/// plate on its ledge already says the same number (#274).
///
/// The plate is drawn by the card's material under every face, text or art,
/// so a creature drawn as text used to say `3/3` twice, a hand's width
/// apart. What it keeps is the one shape where the two differ — a
/// planeswalker that is also a creature plates its loyalty, and its power
/// and toughness are then said nowhere else on the table.
fn world_stats(stats: Option<Stats>, plate: Plate) -> Option<Stats> {
    match (stats?, plate) {
        (Stats::PowerToughness { .. }, Plate::Fight { .. })
        | (Stats::Loyalty(_), Plate::Loyalty(_)) => None,
        (stats, _) => Some(stats),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::mana::ManaCost;
    use baylee_core::types::{SubtypeSet, TypeSet};

    /// The shipped Regular cut, read the way the client reads it.
    fn regular() -> Font {
        let path = format!(
            "{}/assets/fonts/AlegreyaSans-Regular.ttf",
            env!("CARGO_MANIFEST_DIR")
        );
        Font::from_bytes(std::fs::read(path).expect("the bundled Regular"))
    }

    fn test_fonts() -> UiFonts {
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

    /// A creature face with this name and type line, costing {1}{G}.
    fn creature(name: &str, type_line: &str) -> CardFace {
        CardFace {
            name: name.to_owned(),
            cost: vec![ManaSymbol::Generic(1), ManaSymbol::Green],
            type_line: type_line.to_owned(),
            body: Vec::new(),
            stats: Some(Stats::PowerToughness {
                power: 2,
                toughness: 2,
                damage: 0,
            }),
            colors: ColorSet::EMPTY,
            text_pending: false,
        }
    }

    /// The widths are the shipped font's own advances, summed: read off the
    /// file's `hmtx` for these strings, "Llanowar Elves" is 5.739 em, the
    /// German "Llanowarelfen" 5.579, and the ellipsis a cut line ends on
    /// 0.581. A character the font lacks is a full em; before the font, the
    /// average answers.
    #[test]
    fn the_widths_are_the_shipped_font_s_own() {
        let font = regular();
        let widths = Widths::of(Some(&font));
        assert!(widths.measured());
        for (text, em) in [
            ("Llanowar Elves", 5.739),
            ("Llanowarelfen", 5.579),
            ("…", 0.581),
            ("\u{6f22}", 1.0),
        ] {
            let got = widths.width(text);
            assert!((got - em).abs() < 1e-3, "{text:?} measures {got}, not {em}");
        }
        let average = Widths::of(None);
        assert!(!average.measured());
        assert!(
            (average.width("Llanowar Elves") - textface::average_width("Llanowar Elves")).abs()
                < 1e-6
        );
    }

    /// The average is a stand-in, and on a real name it answers the
    /// one-line-or-two question differently from the font — which is why a
    /// face fitted by it is fitted again when the font arrives.
    #[test]
    fn the_average_and_the_font_can_disagree_about_a_name_s_lines() {
        use textface::fit_name;
        let font = regular();
        let widths = Widths::of(Some(&font));
        let name = "Abandoned Outpost";
        let guessed = fit_name(name, textface::average_width);
        let measured = fit_name(name, |s| widths.width(s));
        assert_eq!(guessed.lines.len(), 1);
        assert_eq!(measured.lines.len(), 2, "{measured:?}");
    }

    /// Each line of the table's face stands inside its part of the card:
    /// the name in the name bar, the cost on the art box's first line, the
    /// type line in the type bar. A long name takes two lines and the name
    /// bar its two-line height.
    #[test]
    fn the_table_face_stands_in_its_bars() {
        use bevy::ecs::world::CommandQueue;
        use bevy::sprite::Anchor;
        use textface::{LINE_BOX, Regions};

        let font = regular();
        let widths = Widths::of(Some(&font));
        // The word is the material's: a green card's art box is light enough
        // for the dark ink under its cost, a black one's only for the light.
        for (name, lines, color, cost_ink) in [
            ("Llanowar Elves", 1, MagicColor::Green, textface::INK),
            (
                "Okina, Temple to the Grandfathers",
                2,
                MagicColor::Black,
                textface::LIGHT_INK,
            ),
        ] {
            let mut world = World::new();
            let card = world.spawn_empty().id();
            let face = creature(name, "Legendary Creature — Elf Druid Warrior");
            let mut queue = CommandQueue::default();
            let mut commands = Commands::new(&mut queue, &world);
            let fit = WorldFit::of(&face, &widths);
            let word = textface::face_word(
                ColorSet::from_slice(&[color]),
                TypeSet::CREATURE,
                SubtypeSet::EMPTY,
                textface::Depths::table(fit.lines()),
            );
            let texts = spawn_world(
                &mut commands,
                card,
                &face,
                &fit,
                word,
                // What the ledge shows for this 2/2.
                Plate::Fight {
                    power: 2,
                    toughness: 2,
                    damage: 0,
                },
                &test_fonts(),
            );
            queue.apply(&mut world);
            assert_eq!(fit.lines(), lines, "{name}");
            let regions = Regions::table(lines);

            // Each text's box on the card, in card widths with y down.
            let boxes: Vec<(String, [f32; 4])> = texts
                .iter()
                .map(|&text| {
                    let entity = world.entity(text);
                    let words = entity.get::<Text2d>().expect("a Text2d").0.clone();
                    let em = match entity.get::<TextFont>().expect("a font").font_size {
                        bevy::text::FontSize::Px(px) => px / PX_PER_UNIT,
                        other => panic!("{other:?}"),
                    };
                    let at = entity.get::<Transform>().expect("a place").translation;
                    let anchor = entity.get::<Anchor>().expect("an anchor").as_vec();
                    let rows: Vec<&str> = words.split('\n').collect();
                    let w = rows.iter().map(|r| widths.width(r)).fold(0.0, f32::max) * em;
                    #[allow(clippy::cast_precision_loss)]
                    let h = rows.len() as f32 * LINE_BOX * em;
                    // Back from the card's space to card widths from its
                    // top-left, then from the anchor to the box's corner.
                    let x = at.x / baylee_client_core::layout::CARD_WIDTH + 0.5;
                    let y = (baylee_client_core::layout::CARD_HEIGHT * 0.5 - at.y)
                        / crate::table::DOWN_THE_CARD;
                    let x0 = x - (anchor.x + 0.5) * w;
                    let y0 = y - (0.5 - anchor.y) * h;
                    (words, [x0, y0, x0 + w, y0 + h])
                })
                .collect();

            let within = |label: &str, part: [f32; 4]| {
                let (words, b) = boxes
                    .iter()
                    .find(|(words, _)| words.contains(label))
                    .unwrap_or_else(|| panic!("{name}: no text holding {label:?}"));
                assert!(
                    b[0] >= part[0] - 1e-4
                        && b[1] >= part[1] - 1e-4
                        && b[2] <= part[2] + 1e-4
                        && b[3] <= part[3] + 1e-4,
                    "{name}: {words:?} at {b:?} leaves {part:?}"
                );
            };
            within(name.split(' ').next().expect("a word"), regions.name_bar);
            within("1 G", regions.art_box);
            // Whatever the type line was fitted to — here its subtypes alone.
            within(&fit.kind.lines[0], regions.type_bar);
            // The plate says the body, so the face does not.
            assert_eq!(boxes.len(), 3, "{name}: {boxes:?}");

            // Dark on the light bars; the cost in the ink its art box lets it.
            for &text in &texts {
                let entity = world.entity(text);
                let words = &entity.get::<Text2d>().expect("a Text2d").0;
                let ink = entity.get::<TextColor>().expect("an ink").0;
                let want = if words.contains("1 G") {
                    Color::srgb_from_array(cost_ink)
                } else {
                    FACE_INKS.0
                };
                assert_eq!(ink, want, "{name}: {words:?}");
            }
        }
    }

    /// A number the ledge already shows is not written on the face again,
    /// and a number it does not show is.
    #[test]
    fn the_face_leaves_the_body_to_the_plate() {
        let body = Stats::PowerToughness {
            power: 3,
            toughness: 3,
            damage: 0,
        };
        let fight = Plate::Fight {
            power: 3,
            toughness: 3,
            damage: 0,
        };
        assert_eq!(world_stats(Some(body), fight), None);
        assert_eq!(
            world_stats(Some(Stats::Loyalty(4)), Plate::Loyalty(4)),
            None
        );
        assert_eq!(
            world_stats(Some(body), Plate::Loyalty(4)),
            Some(body),
            "an animated planeswalker's body is said nowhere else"
        );
        assert_eq!(world_stats(Some(body), Plate::None), Some(body));
        assert_eq!(world_stats(None, fight), None);
    }

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
        for inks in [PANEL_INKS, FACE_INKS] {
            assert_eq!(stats_color(lethal, inks), inks.1);
            assert_ne!(inks.1, inks.0);
            assert_eq!(
                stats_color(
                    Stats::PowerToughness {
                        power: 1,
                        toughness: 2,
                        damage: 1
                    },
                    inks
                ),
                inks.0
            );
        }
    }

    /// The five independent reasons to draw the face. Each one alone is
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

        // Art still in flight. This one used to read the other way — the
        // image won the moment a handle existed — and that is precisely how a
        // card came to be drawn as nothing: an unloaded texture cannot be
        // bound, so the material never prepares.
        assert!(wants_face(&quiet, &plain, &textures, Some(art)));

        textures.mark_arrived(art);
        // Art that is on the GPU: the image wins.
        assert!(!wants_face(&quiet, &plain, &textures, Some(art)));
        // The modifier, the latch, and a token with no printing at all.
        assert!(wants_face(&held, &plain, &textures, Some(art)));
        assert!(wants_face(&quiet, &latched, &textures, Some(art)));
        assert!(wants_face(&quiet, &plain, &textures, None));
        // And art that will never arrive.
        let lost = ImageKey::new(PrintRef::new(1), 0, ArtSize::Small);
        textures.mark_failed(lost, crate::textures::Failure::Load(1));
        assert!(wants_face(&quiet, &plain, &textures, Some(lost)));
    }

    /// A face's helper for the tests below: one card, its name translated.
    fn german(english: &str, translated: &str) -> crate::cardtext::CardTexts {
        crate::cardtext::CardTexts::filed(crate::cardtext::fixture::german(
            english, translated, None,
        ))
    }

    /// An ability has no card of its own, so its name has to be looked up
    /// through the permanent it came from. Nothing else in the client reaches
    /// a printing that way, which is why this one is worth a test: the
    /// obvious implementation answers `None` for every ability on the stack
    /// and leaves the whole panel English.
    #[test]
    fn an_ability_is_named_through_the_permanent_it_came_from() {
        use baylee_client_core::test_support::{ViewBuilder, printed, token};

        let texts = german("Flooded Strand", "Gefluteter Strand");
        let card = crate::cardtext::fixture::card("Flooded Strand");
        let mut ability = token(30, 0, "Flooded Strand", 0, 0);
        ability.card = None;
        ability.stack_item = Some(baylee_view::StackItem::Ability {
            source: baylee_core::ids::ObjectId::new(7, 0),
            ability: None,
            rules: Some(baylee_view::RulesFace { card, face: 0 }),
            text: Some(baylee_view::StackText {
                face: 0,
                line: 0,
                of: 1,
            }),
        });
        let view = ViewBuilder::new(2)
            .with_battlefield(
                0,
                vec![crate::cardtext::fixture::showing(
                    printed(7, 0, "Flooded Strand", 7),
                    card,
                )],
            )
            .with_stack(vec![ability.clone()])
            .build();

        assert_eq!(
            name_of(&ability, &view, &texts),
            "Gefluteter Strand",
            "the ability borrows its source's printing"
        );
        // The source itself, which is the ordinary path.
        assert_eq!(
            name_of(
                view.object(baylee_core::ids::ObjectId::new(7, 0)).unwrap(),
                &view,
                &texts
            ),
            "Gefluteter Strand"
        );
    }

    /// The stack panel draws an ability as the *picture* of the permanent it
    /// came from, and the picture carries a name. Found live: a Marsh Flats
    /// ability read "Brackmarsch" in the row's title and "Marsh Flats" on the
    /// thumbnail two inches to its left, because the thumbnail is built from
    /// the stack object and a stack object for an ability has no `card` to
    /// look text up by. The view is what closes it.
    #[test]
    fn the_picture_beside_an_ability_is_named_like_the_ability() {
        use baylee_client_core::test_support::{ViewBuilder, printed, token};

        let texts = german("Marsh Flats", "Brackmarsch");
        let card = crate::cardtext::fixture::card("Marsh Flats");
        let mut ability = token(30, 0, "Marsh Flats", 0, 0);
        ability.card = None;
        ability.stack_item = Some(baylee_view::StackItem::Ability {
            source: baylee_core::ids::ObjectId::new(7, 0),
            ability: None,
            rules: Some(baylee_view::RulesFace { card, face: 0 }),
            text: Some(baylee_view::StackText {
                face: 0,
                line: 0,
                of: 1,
            }),
        });
        let view = ViewBuilder::new(2)
            .with_battlefield(
                0,
                vec![crate::cardtext::fixture::showing(
                    printed(7, 0, "Marsh Flats", 7),
                    card,
                )],
            )
            .with_stack(vec![ability.clone()])
            .build();

        assert_eq!(
            of_object(&ability, Some(&view), &texts).name,
            "Brackmarsch",
            "the thumbnail borrows the source's printing, like the title above it"
        );
        // The counter-test: a caller with no view is drawing a permanent, and
        // a permanent carries its own printing.
        assert_eq!(
            of_object(
                view.object(baylee_core::ids::ObjectId::new(7, 0)).unwrap(),
                None,
                &texts
            )
            .name,
            "Brackmarsch"
        );
    }

    /// An ability outlives its source (CR 113.7a). There is then no printing
    /// to ask, and the projected name is all there is — which is the honest
    /// answer, not a bug to paper over.
    #[test]
    fn an_ability_whose_source_has_left_keeps_the_name_it_has() {
        use baylee_client_core::test_support::{ViewBuilder, token};

        let texts = german("Flooded Strand", "Gefluteter Strand");
        let mut ability = token(30, 0, "Flooded Strand", 0, 0);
        ability.card = None;
        ability.stack_item = Some(baylee_view::StackItem::Ability {
            source: baylee_core::ids::ObjectId::new(7, 0),
            ability: None,
            rules: None,
            text: None,
        });
        let view = ViewBuilder::new(2)
            .with_stack(vec![ability.clone()])
            .build();

        assert_eq!(name_of(&ability, &view, &texts), "Flooded Strand");
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
