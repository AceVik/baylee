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
use baylee_client_core::cardplate::{self, Plate};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::textface::{self, BAR_PAD, Fitted, LINE_BOX, Regions, Sizes, TEXT_INSET};
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

/// The face for a card in the deckbuilder's pool (#259): its printed front
/// face out of the compiled registry, and the words the pool row carries —
/// the name, type line and rules text the gateway served in the player's
/// language.
#[must_use]
pub fn of_pool(card: &baylee_client_core::deckbuilder::PoolCard) -> CardFace {
    use baylee_client_core::card_face::{CardText, Characteristics};
    use baylee_core::types::{SubtypeSet, SupertypeSet, TypeSet};

    let index = baylee_core::ids::CardIndex::new(card.index);
    let printed = baylee_cards::by_index(index).and_then(|def| def.faces.first());
    // Named in English, as the registry names it, so the row's text is
    // taken as describing it and its own name is the one written.
    let chars = Characteristics {
        name: card.english_name.clone(),
        types: printed.map_or(TypeSet::EMPTY, |f| f.types),
        supertypes: printed.map_or(SupertypeSet::EMPTY, |f| f.supertypes),
        subtypes: printed.map_or(SubtypeSet::EMPTY, |f| SubtypeSet::from_slice(f.subtypes)),
        colors: card
            .colors
            .chars()
            .filter_map(MagicColor::from_symbol)
            .fold(ColorSet::EMPTY, |set, color| set.union(ColorSet::of(color))),
        power: printed.and_then(|f| f.power),
        toughness: printed.and_then(|f| f.toughness),
        loyalty: printed.and_then(|f| f.loyalty),
        damage: 0,
    };
    // A gateway with no catalog serves no rules text, and a fallback is the
    // English Oracle's sentence, never a blank box.
    let oracle = if card.oracle_text.is_empty() {
        baylee_cards::generated_oracle::ORACLE
            .get(card.index as usize)
            .and_then(|faces| faces.first())
            .map_or_else(String::new, |text| (*text).to_owned())
    } else {
        card.oracle_text.clone()
    };
    let text = CardText {
        lang: String::new(),
        name: card.name.clone(),
        type_line: card.type_line.clone(),
        oracle_text: oracle,
        mana_cost: card.mana_cost.clone(),
        english_name: card.english_name.clone(),
    };
    CardFace::build(
        &chars,
        printed.map(|f| &f.mana_cost),
        printed.map(printed_types),
        Some(&text),
    )
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

/// What a card with no art is washed from before the shader stands its face
/// in the window: the flat colour under the face, and the whole of a card
/// that draws none.
const PAPER: Color = Color::srgb(0.09, 0.10, 0.12);

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

/// Marks the rules text box of a text face in the overlay: the node a wheel
/// over the hovered table card scrolls ([`crate::hud::scroll`]).
///
/// That system scrolls every one there is, and `keep_the_preview_scrolled`
/// stands every new one at the preview's offset, which is right while the
/// preview is the only face in the duel's overlay drawn with its rules text
/// (`Detail::Full`): the hand, the stack and the tray draw the compact face.
/// A second full face there would need the two scoped to the preview's.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct FaceTextBox;

/// A text box's scrollbar track, naming its box. Its one child is the thumb
/// ([`FaceScrollThumb`]), and [`show_scrollbars`] shows the two only while
/// the text runs over.
#[derive(Component, Clone, Copy, Debug)]
pub struct FaceScrollbar {
    /// The box whose offset it shows.
    pub text_box: Entity,
}

/// A scrollbar's thumb.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct FaceScrollThumb;

/// The scrollbar, in card widths (Fable's §4): the track's width, its inset
/// from the text box's right edge, and from its top and foot.
const TRACK_WIDTH: f32 = 0.012;
const TRACK_INSET: f32 = 0.008;
const TRACK_END: f32 = 0.010;
/// How round the track's and the thumb's ends are, in card widths.
const TRACK_ROUND: f32 = 0.004;
/// The shortest thumb, as a share of the track.
const THUMB_MIN: f32 = 0.06;
/// The track is the paper this much darker; the thumb is the bars' colour at
/// this opacity.
const TRACK_SHADE: f32 = 0.92;
const THUMB_ALPHA: f32 = 0.85;

/// The gap between two pips of a cost, as a share of a pip.
const PIP_GAP: f32 = 0.12;

/// A text face laid out for the overlay, before anything is spawned.
///
/// Laid out first because the card's material is keyed by the face's word,
/// and the word carries the bars' depths, which are the fit's: a long name
/// on two lines is a deeper name bar, drawn by the shader under the text
/// this places (#259).
pub struct UiFace {
    /// The card's width in pixels.
    width: f32,
    sizes: Sizes,
    name: Fitted,
    kind: Fitted,
    regions: Regions,
    /// The rules text's em, for a face that writes it; `None` for a
    /// compact one.
    body: Option<f32>,
    /// The numbers the card's plate does not already say.
    stats: Option<Stats>,
    /// A cost pip's diameter, in pixels.
    pip: f32,
    /// The material's face word ([`textface::face_word`]).
    pub word: u32,
}

impl UiFace {
    /// `face` laid out on a card `width` pixels wide, its lines measured by
    /// `widths`. `plate` is what the card's corner says, packed
    /// (`CardLook::plate`): numbers it already shows are not written twice.
    #[must_use]
    pub fn lay(
        face: &CardFace,
        lang: Lang,
        width: f32,
        detail: Detail,
        widths: &Widths<'_>,
        plate: u32,
    ) -> Self {
        let pip = textface::ui_em(textface::UI_NAME, width) * width;
        #[allow(clippy::cast_precision_loss)] // a cost has a handful of pips
        let pips = face.cost.len() as f32;
        let cost = if pips > 0.0 {
            (pips * pip + (pips - 1.0) * pip * PIP_GAP) / width
        } else {
            0.0
        };
        let sizes = Sizes::overlay(width, cost);
        let name = textface::fit_name_in(&sizes, &face.name, |s| widths.width(s));
        let kind = textface::fit_type_in(&sizes, &face.type_line, |s| widths.width(s));
        let depths = sizes.depths(name.lines.len());
        let regions = Regions::new(depths);
        let stats = world_stats(face.stats, plate >> cardplate::KIND_SHIFT);
        let body = (detail == Detail::Full).then(|| {
            let room = body_room(&regions, stats, &sizes);
            let blocks = body_blocks(face, lang);
            textface::fit_body(width, room, |em| body_depth(&blocks, em, width, widths))
        });
        Self {
            width,
            sizes,
            name,
            kind,
            regions,
            body,
            stats,
            pip,
            word: textface::face_word(face.colors, face.types, face.subtypes, depths),
        }
    }
}

/// How deep the rules text may stand, in card widths: the text box inside
/// its padding, less the line the numbers keep.
fn body_room(regions: &Regions, stats: Option<Stats>, sizes: &Sizes) -> f32 {
    let [_, top, _, foot] = regions.text_box;
    foot - top - 2.0 * BAR_PAD - stats_line(stats, sizes)
}

/// How much of the text box's foot the numbers keep, in card widths: one
/// line at the name's size, or none.
fn stats_line(stats: Option<Stats>, sizes: &Sizes) -> f32 {
    stats.map_or(0.0, |_| LINE_BOX * sizes.name)
}

/// The rules text's blocks as they are written, each with its ink: the
/// rules, reminders in parentheses, and a quiet line where the text has not
/// arrived.
fn body_blocks(face: &CardFace, lang: Lang) -> Vec<(String, Color)> {
    let muted = Color::srgb_from_array(textface::MUTED_INK);
    let mut blocks: Vec<(String, Color)> = face
        .body
        .iter()
        .map(|block| match block {
            TextBlock::Rules(t) => (t.clone(), FACE_INKS.0),
            TextBlock::Reminder(t) => (format!("({t})"), muted),
        })
        .collect();
    if face.text_pending {
        // Not an error: the catalog answer may still be in flight, or the
        // gateway may not be configured. Either way the card is playable,
        // and saying so beats an empty box.
        blocks.push((Phrase::NoRulesTextHere.text(lang).to_owned(), muted));
    }
    blocks
}

/// How deep the rules text stands at `em` on a card `width` pixels wide, in
/// card widths: each block as [`crate::manaui::spawn_rich_in`] sets it, in
/// the face's font measured by `widths`, and the text box's gap between two
/// blocks.
///
/// Nothing for bevy rounding each node to whole pixels: over the whole pool
/// at three widths, no face this says fits showed its bar without it
/// (`how_much_of_the_pool_runs_over_at_the_floor`), and a pixel kept for it
/// cost twenty cards a size.
fn body_depth(blocks: &[(String, Color)], em: f32, width: f32, widths: &Widths<'_>) -> f32 {
    let px = em * width;
    let column = textface::column() * width;
    let text: f32 = blocks
        .iter()
        .map(|(block, _)| crate::manaui::rich_depth(block, px, column, |s| widths.width(s) * px))
        .sum();
    #[allow(clippy::cast_precision_loss)] // a card has a handful of blocks
    let gaps = blocks.len().saturating_sub(1) as f32 * textface::BLOCK_GAP * LINE_BOX * px;
    (text + gaps) / width
}

/// A node standing at `x`, `y` card widths from the card's top-left, in a
/// card `width` pixels wide.
fn placed(width: f32, x: f32, y: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(x * width),
        top: Val::Px(y * width),
        ..default()
    }
}

/// Puts `laid`'s face on `card`, a node the size of the card whose material
/// draws the bars under it (or, with no material store, whose colour stands
/// in for them).
///
/// Every node sits where [`textface`] placed it, in card widths scaled by
/// the card's width in pixels, so the text stands on the bars the shader
/// draws from the same word: the name in the name bar with the cost at its
/// right end, as on a print, the type line in the type bar, and the rules
/// text in the text box, set a pixel smaller at a time down to ten pixels
/// to fit and scrolling past that.
///
/// **Every node here carries [`Pickable::IGNORE`]**, and it is one rule rather
/// than eight decisions: a drawn face is never the pointer's target — the hand
/// card, the tray row or the stack entry around it is — and marking only the
/// root would achieve nothing. `bevy_ui`'s picking backend reports the
/// *deepest* node under the pointer and `bevy_picking::hover::build_hover_map`
/// stops at the first entity that carries no `Pickable` at all, so one
/// unmarked `Text` in the middle of the face is as opaque as the whole card:
/// it keeps `PickingInteraction` off the row and the row never lights. The
/// text box is no exception: the wheel reaches it through the hovered card
/// rather than by being under the pointer.
pub fn spawn_ui(
    commands: &mut Commands,
    card: Entity,
    lang: Lang,
    face: &CardFace,
    laid: &UiFace,
    fonts: &UiFonts,
) {
    use bevy::text::LineBreak;

    let w = laid.width;
    let [x0, name_top, x1, _] = laid.regions.name_bar;
    let line = |commands: &mut Commands, text: String, em: f32, color: Color, node: Node| {
        let entity = commands
            .spawn((
                Pickable::IGNORE,
                Text::new(text),
                text_font(fonts, em * w),
                TextColor(color),
                // The lines are already broken, by the fit that sized the
                // bars under them.
                TextLayout::new(Justify::Left, LineBreak::NoWrap),
                node,
            ))
            .id();
        commands.entity(card).add_child(entity);
    };

    line(
        commands,
        laid.name.lines.join("\n"),
        laid.name.em,
        FACE_INKS.0,
        placed(w, x0 + TEXT_INSET, name_top + BAR_PAD),
    );
    if !face.cost.is_empty() {
        // At the name bar's right end, centred on the name's first line.
        let first = LINE_BOX * laid.name.em * w;
        let pips = spawn_pips(commands, face, laid.pip, fonts);
        commands.entity(pips).insert(Node {
            position_type: PositionType::Absolute,
            right: Val::Px((1.0 - x1 + TEXT_INSET) * w),
            top: Val::Px((name_top + BAR_PAD) * w + (first - laid.pip) * 0.5),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(laid.pip * PIP_GAP),
            ..default()
        });
        commands.entity(card).add_child(pips);
    }
    let [_, type_top, _, type_foot] = laid.regions.type_bar;
    line(
        commands,
        laid.kind.lines.concat(),
        laid.kind.em,
        FACE_INKS.0,
        placed(
            w,
            x0 + TEXT_INSET,
            type_top + (type_foot - type_top - LINE_BOX * laid.kind.em) * 0.5,
        ),
    );

    if let Some(em) = laid.body {
        spawn_text_box(commands, card, lang, face, laid, fonts, em);
    }
    let box_foot = laid.regions.text_box[3];
    if let Some(stats) = laid.stats {
        let right = x1 - TEXT_INSET;
        let em = laid.sizes.name;
        let entity = commands
            .spawn((
                Pickable::IGNORE,
                Text::new(stats_label(stats)),
                text_font(fonts, em * w),
                TextColor(stats_color(stats)),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px((1.0 - right) * w),
                    top: Val::Px((box_foot - BAR_PAD - LINE_BOX * em) * w),
                    ..default()
                },
            ))
            .id();
        commands.entity(card).add_child(entity);
    }
}

/// The rules text, set at `em`, in a box that scrolls past the floor, and
/// the box's scrollbar.
fn spawn_text_box(
    commands: &mut Commands,
    card: Entity,
    lang: Lang,
    face: &CardFace,
    laid: &UiFace,
    fonts: &UiFonts,
    em: f32,
) {
    let w = laid.width;
    let [x0, box_top, x1, box_foot] = laid.regions.text_box;
    let kept = stats_line(laid.stats, &laid.sizes);
    let px = em * w;
    let text_box = commands
        .spawn((
            Pickable::IGNORE,
            FaceTextBox,
            ScrollPosition::default(),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x0 * w),
                top: Val::Px(box_top * w),
                width: Val::Px((x1 - x0) * w),
                height: Val::Px((box_foot - box_top - kept) * w),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(textface::BLOCK_GAP * LINE_BOX * px),
                padding: UiRect {
                    left: Val::Px(TEXT_INSET * w),
                    right: Val::Px(textface::SCROLL_MARGIN * w),
                    top: Val::Px(BAR_PAD * w),
                    bottom: Val::Px(BAR_PAD * w),
                },
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    for (text, color) in body_blocks(face, lang) {
        // Rich: this is the face a player reads when there is no printing
        // to show, so `{T}: Add {G}` has to be the symbols the card
        // prints rather than the letters they are written with.
        // In the face's own font at the size it was fitted at, which is not
        // `hud::tf`'s: that one is `UI_SCALE` larger and a weight up when
        // small.
        let block = crate::manaui::spawn_rich_in(commands, fonts, &text, px, color, text_font);
        commands.entity(text_box).add_child(block);
    }
    commands.entity(card).add_child(text_box);
    spawn_scrollbar(commands, card, text_box, laid);
}

/// The text box's scrollbar, hidden until [`show_scrollbars`] finds the text
/// running over.
fn spawn_scrollbar(commands: &mut Commands, card: Entity, text_box: Entity, laid: &UiFace) {
    let w = laid.width;
    let [_, top, x1, foot] = laid.regions.text_box;
    let foot = foot - stats_line(laid.stats, &laid.sizes);
    let hue = textface::Hue::from_code(laid.word >> textface::FACE_BARS_SHIFT);
    let [r, g, b] = textface::paper(hue).map(|c| c * TRACK_SHADE);
    let [tr, tg, tb] = hue.tone();
    let round = BorderRadius::all(Val::Px(TRACK_ROUND * w));
    let thumb = commands
        .spawn((
            Pickable::IGNORE,
            FaceScrollThumb,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                border_radius: round,
                ..default()
            },
            BackgroundColor(Color::linear_rgba(tr, tg, tb, THUMB_ALPHA)),
        ))
        .id();
    let track = commands
        .spawn((
            Pickable::IGNORE,
            FaceScrollbar { text_box },
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                left: Val::Px((x1 - TRACK_INSET - TRACK_WIDTH) * w),
                top: Val::Px((top + TRACK_END) * w),
                width: Val::Px(TRACK_WIDTH * w),
                height: Val::Px((foot - top - 2.0 * TRACK_END) * w),
                border_radius: round,
                ..default()
            },
            BackgroundColor(Color::linear_rgb(r, g, b)),
        ))
        .add_child(thumb)
        .id();
    commands.entity(card).add_child(track);
}

/// Where a scrollbar's thumb stands on its track, as shares of the track:
/// its top and its length, or `None` when the text fits and there is no bar.
///
/// `view` and `content` are the box's and its contents' heights, in the
/// same pixels; `offset` is how far it has scrolled, in those pixels too.
#[must_use]
pub fn thumb(view: f32, content: f32, offset: f32) -> Option<(f32, f32)> {
    // Half a pixel, so a box its text fills exactly shows no bar.
    if content <= view + 0.5 {
        return None;
    }
    let length = (view / content).max(THUMB_MIN);
    let room = content - view;
    Some(((offset / room).clamp(0.0, 1.0) * (1.0 - length), length))
}

/// Shows each face's scrollbar while its text runs over, and stands the
/// thumb where the box has scrolled to.
///
/// Written only on a change, because a `Node` written every frame is a
/// layout every frame.
pub fn show_scrollbars(
    boxes: Query<(&ScrollPosition, &ComputedNode), With<FaceTextBox>>,
    mut tracks: Query<(&FaceScrollbar, &mut Node, &Children)>,
    mut thumbs: Query<&mut Node, (With<FaceScrollThumb>, Without<FaceScrollbar>)>,
) {
    for (bar, mut node, children) in &mut tracks {
        let Ok((position, computed)) = boxes.get(bar.text_box) else {
            continue;
        };
        let scale = computed.inverse_scale_factor();
        let shown = thumb(
            computed.size().y * scale,
            computed.content_size().y * scale,
            position.y,
        );
        let display = if shown.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        let Some((top, length)) = shown else {
            continue;
        };
        for &child in children {
            if let Ok(mut thumb) = thumbs.get_mut(child) {
                let (top, height) = (Val::Percent(top * 100.0), Val::Percent(length * 100.0));
                if thumb.top != top || thumb.height != height {
                    thumb.top = top;
                    thumb.height = height;
                }
            }
        }
    }
}

/// The row of mana pips, `size` pixels across each.
fn spawn_pips(commands: &mut Commands, face: &CardFace, size: f32, fonts: &UiFonts) -> Entity {
    let row = commands.spawn((Pickable::IGNORE, Node::default())).id();
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

/// The face's ink, or its red once damage has made the toughness matter.
fn stats_color(stats: Stats) -> Color {
    match stats {
        Stats::PowerToughness {
            toughness, damage, ..
        } if toughness - damage as i16 <= 0 => FACE_INKS.1,
        _ => FACE_INKS.0,
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
        if let Some(stats) = world_stats(face.stats, plate.kind()) {
            let [_, _, right, bottom] = regions.text_box;
            line(
                stats_label(stats),
                STATS_EM,
                stats_color(stats),
                on_the_card(right - TEXT_INSET, bottom - BAR_PAD),
                Anchor::BOTTOM_RIGHT,
            );
        }
    }
    texts
}

/// The body line a text face still has to write: none when the plate on its
/// ledge already says the same number (#274). `plate` is the plate's kind
/// ([`Plate::kind`]).
///
/// The plate is drawn by the card's material under every face, text or art,
/// so a creature drawn as text used to say `3/3` twice, a hand's width
/// apart. What it keeps is the one shape where the two differ — a
/// planeswalker that is also a creature plates its loyalty, and its power
/// and toughness are then said nowhere else on the table — and every card
/// whose look carries no plate, as a card in hand does.
fn world_stats(stats: Option<Stats>, plate: u32) -> Option<Stats> {
    match (stats?, plate) {
        (Stats::PowerToughness { .. }, cardplate::KIND_FIGHT)
        | (Stats::Loyalty(_), cardplate::KIND_LOYALTY) => None,
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

    /// A handle for each face, all different, so a test can tell which one a
    /// line is set in.
    fn test_fonts() -> UiFonts {
        use bevy::asset::uuid_handle;
        UiFonts {
            text: uuid_handle!("b259f0c1-0000-4000-8000-000000000001"),
            medium: uuid_handle!("b259f0c1-0000-4000-8000-000000000002"),
            bold: uuid_handle!("b259f0c1-0000-4000-8000-000000000003"),
            italic: uuid_handle!("b259f0c1-0000-4000-8000-000000000004"),
            medium_italic: uuid_handle!("b259f0c1-0000-4000-8000-000000000005"),
            serif: uuid_handle!("b259f0c1-0000-4000-8000-000000000006"),
            serif_italic: uuid_handle!("b259f0c1-0000-4000-8000-000000000007"),
            icons: uuid_handle!("b259f0c1-0000-4000-8000-000000000008"),
            mana: uuid_handle!("b259f0c1-0000-4000-8000-000000000009"),
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
            types: TypeSet::CREATURE,
            subtypes: SubtypeSet::EMPTY,
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

    /// A face laid out for the overlay and spawned into a world, with its
    /// nodes by what they hold.
    fn overlay_face(face: &CardFace, width: f32, detail: Detail) -> (World, UiFace, Entity) {
        use bevy::ecs::world::CommandQueue;
        let font = regular();
        let widths = Widths::of(Some(&font));
        let laid = UiFace::lay(face, Lang::En, width, detail, &widths, 0);
        let mut world = World::new();
        let card = world.spawn(Node::default()).id();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        spawn_ui(&mut commands, card, Lang::En, face, &laid, &test_fonts());
        queue.apply(&mut world);
        (world, laid, card)
    }

    /// Every node under `root`, depth first.
    fn nodes_under(world: &World, root: Entity) -> Vec<Entity> {
        let mut all = Vec::new();
        let mut open = vec![root];
        while let Some(entity) = open.pop() {
            if let Some(children) = world.entity(entity).get::<Children>() {
                for &child in children {
                    all.push(child);
                    open.push(child);
                }
            }
        }
        all
    }

    /// Where a node's top-left stands, in card widths.
    fn corner_of(node: &Node, width: f32) -> Vec2 {
        let px = |v: Val| match v {
            Val::Px(px) => px / width,
            other => panic!("{other:?}"),
        };
        Vec2::new(px(node.left), px(node.top))
    }

    /// The overlay's face stands where `textface` put it (#259): the name in
    /// the name bar with the cost at its right end, the type line in the type
    /// bar, the rules text in the text box with its scrollbar hidden, and
    /// no node the pointer could take from the card around it.
    #[test]
    fn the_overlay_face_stands_in_its_bars() {
        let mut face = creature("Llanowar Elves", "Creature — Elf Druid");
        face.body = vec![TextBlock::Rules("{T}: Add {G}.".to_owned())];
        let width = 308.0;
        let (world, laid, card) = overlay_face(&face, width, Detail::Full);
        let inside = |at: Vec2, [x0, y0, x1, y1]: [f32; 4]| {
            at.x >= x0 && at.x <= x1 && at.y >= y0 && at.y <= y1
        };
        let nodes = nodes_under(&world, card);
        let text_of = |words: &str| {
            nodes
                .iter()
                .copied()
                .find(|&e| world.entity(e).get::<Text>().is_some_and(|t| t.0 == words))
                .unwrap_or_else(|| panic!("no {words:?}"))
        };
        let node = |e: Entity| world.entity(e).get::<Node>().expect("a node");

        let name = corner_of(node(text_of("Llanowar Elves")), width);
        assert!(inside(name, laid.regions.name_bar), "{name}");
        let kind = corner_of(node(text_of(&laid.kind.lines[0])), width);
        assert!(inside(kind, laid.regions.type_bar), "{kind}");

        let text_box = nodes
            .iter()
            .copied()
            .find(|&e| world.entity(e).contains::<FaceTextBox>())
            .expect("a full face has a text box");
        let at = corner_of(node(text_box), width);
        assert!((at.y - laid.regions.text_box[1]).abs() < 1e-5, "{at}");
        assert!(matches!(node(text_box).overflow, o if o == Overflow::scroll_y()));

        let track = nodes
            .iter()
            .copied()
            .find(|&e| world.entity(e).contains::<FaceScrollbar>())
            .expect("a text box has its scrollbar");
        assert_eq!(
            node(track).display,
            Display::None,
            "until the text runs over"
        );

        // The cost at the name bar's right end, measured from the right.
        let pips = nodes
            .iter()
            .copied()
            .find(|&e| {
                matches!(node(e).right, Val::Px(_)) && node(e).flex_direction == FlexDirection::Row
            })
            .expect("the cost's row");
        let Val::Px(right) = node(pips).right else {
            unreachable!()
        };
        let edge = 1.0 - right / width;
        assert!(
            (edge - (laid.regions.name_bar[2] - TEXT_INSET)).abs() < 1e-5,
            "{edge}"
        );

        for e in nodes {
            assert!(
                world.entity(e).contains::<Pickable>(),
                "{:?} can take the pointer from the card",
                world.entity(e).get::<Name>()
            );
        }
    }

    /// A compact face writes no rules text and has no box to scroll, and a
    /// card whose corner shows no plate says its numbers itself.
    #[test]
    fn a_compact_overlay_face_has_no_text_box() {
        let face = creature("Llanowar Elves", "Creature — Elf Druid");
        let (world, laid, card) = overlay_face(&face, 92.0, Detail::Compact);
        let nodes = nodes_under(&world, card);
        assert!(laid.body.is_none());
        assert!(
            !nodes
                .iter()
                .any(|&e| world.entity(e).contains::<FaceTextBox>())
        );
        assert!(
            nodes
                .iter()
                .any(|&e| world.entity(e).get::<Text>().is_some_and(|t| t.0 == "2/2")),
            "no plate in hand, so the face says 2/2"
        );

        // A card whose corner plates the same body is not told it twice.
        let font = regular();
        let plate = Plate::Fight {
            power: 2,
            toughness: 2,
            damage: 0,
        };
        let laid = UiFace::lay(
            &face,
            Lang::En,
            92.0,
            Detail::Compact,
            &Widths::of(Some(&font)),
            plate.packed(),
        );
        assert_eq!(laid.stats, None);
    }

    /// The word the overlay's material is keyed by carries the depths the
    /// fit chose: a name that takes two lines is a deeper name bar, drawn
    /// under the lines that need it. And the rules text steps down to fit.
    #[test]
    fn the_overlay_s_word_is_its_own_fit() {
        use textface::{Depths, FACE_NAME_SHIFT};
        let font = regular();
        let widths = Widths::of(Some(&font));
        let lay = |face: &CardFace, width: f32| {
            UiFace::lay(face, Lang::En, width, Detail::Full, &widths, 0)
        };
        let short = lay(&creature("Elves", "Creature — Elf"), 92.0);
        let long = lay(
            &creature("Okina, Temple to the Grandfathers", "Legendary Land"),
            92.0,
        );
        assert_eq!(short.name.lines.len(), 1);
        assert_eq!(long.name.lines.len(), 2);
        let depth = |laid: &UiFace| laid.word >> FACE_NAME_SHIFT & 0xff;
        assert!(depth(&long) > depth(&short));
        assert_eq!(
            depth(&long),
            u32::from(Depths::of(long.sizes.name_bar(2), 0.0).name)
        );

        let mut wordy = creature("Elves", "Creature — Elf");
        wordy.body = vec![TextBlock::Rules("Flying. ".repeat(30))];
        let fitted = lay(&wordy, 308.0).body.expect("a full face") * 308.0;
        assert!(
            (textface::BODY_FLOOR_PX..15.0).contains(&fitted),
            "{fitted}"
        );
        let own = lay(&creature("Elves", "Creature — Elf"), 308.0)
            .body
            .expect("a full face")
            * 308.0;
        assert!((own - 15.0).abs() < 1e-3, "{own}");
    }

    /// The rules text is drawn in the font the fit measured it in, at the
    /// size the fit chose. It was drawn through `hud::tf`, which sets a line
    /// `UI_SCALE` larger than it is asked for and a weight up when small, so
    /// a text fitted to its box ran a fifth over it (#259).
    #[test]
    fn the_rules_text_is_drawn_as_it_was_fitted() {
        use bevy::text::{FontSize, FontSource};
        let mut face = creature("Llanowar Elves", "Creature — Elf Druid");
        face.body = vec![
            TextBlock::Rules("{T}: Add {G}.".to_owned()),
            TextBlock::Reminder("It taps for mana.".to_owned()),
        ];
        let width = 308.0;
        let (world, laid, card) = overlay_face(&face, width, Detail::Full);
        let fonts = test_fonts();
        let px = laid.body.expect("a full face") * width;
        let text_box = nodes_under(&world, card)
            .into_iter()
            .find(|&e| world.entity(e).contains::<FaceTextBox>())
            .expect("a full face has a text box");
        let mut words = 0;
        for e in nodes_under(&world, text_box) {
            let Some(font) = world.entity(e).get::<TextFont>() else {
                continue;
            };
            if font.font == FontSource::Handle(fonts.mana.clone()) {
                // A mark's glyph, on its disc.
                continue;
            }
            let said = world.entity(e).get::<Text>();
            assert_eq!(
                font.font,
                FontSource::Handle(fonts.text.clone()),
                "{said:?}"
            );
            assert_eq!(font.font_size, FontSize::Px(px), "{said:?}");
            words += 1;
        }
        assert!(words >= 3, "{words} runs of words");
    }

    /// An app that lays the interface out as the client does, with the
    /// shipped fonts in it: bevy's own layout, headless, and the scrollbars
    /// shown or hidden by what it measured.
    fn layout_app() -> (App, UiFonts) {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            bevy::text::TextPlugin,
            bevy::ui::UiPlugin,
            bevy::window::WindowPlugin {
                primary_window: None,
                ..default()
            },
            // Asked for by `UiPlugin`'s focus and picking systems, which a
            // measurement never uses.
            bevy::input::InputPlugin,
            bevy::picking::DefaultPickingPlugins,
        ));
        app.init_asset::<Image>()
            .init_asset::<bevy::image::TextureAtlasLayout>()
            .add_systems(
                PostUpdate,
                show_scrollbars.after(bevy::ui::UiSystems::Layout),
            );
        let mut store = app.world_mut().resource_mut::<Assets<Font>>();
        let mut load = |file: &str| {
            let path = format!("{}/assets/fonts/{file}", env!("CARGO_MANIFEST_DIR"));
            store.add(Font::from_bytes(std::fs::read(path).expect(file)))
        };
        let fonts = UiFonts {
            text: load("AlegreyaSans-Regular.ttf"),
            medium: load("AlegreyaSans-Medium.ttf"),
            bold: load("AlegreyaSans-Bold.ttf"),
            italic: load("AlegreyaSans-Italic.ttf"),
            medium_italic: load("AlegreyaSans-MediumItalic.ttf"),
            serif: load("Faustina.ttf"),
            serif_italic: load("Faustina-Italic.ttf"),
            icons: load("fa-solid-900.ttf"),
            mana: load("mana.ttf"),
        };
        (app, fonts)
    }

    /// What became of one face's rules text in [`layout_app`].
    struct Measured {
        /// The size the fit set it at, and the size it would have had.
        fitted: f32,
        own: f32,
        /// Whether the fit's model said it fits its box.
        fits: bool,
        /// How deep the model and bevy stand it, in pixels, against the room.
        model: f32,
        real: f32,
        room: f32,
        /// Whether bevy's layout showed the scrollbar.
        bar: bool,
    }

    fn measure(
        app: &mut App,
        fonts: &UiFonts,
        face: &CardFace,
        width: f32,
        plate: u32,
    ) -> Measured {
        let (laid, model) = {
            let assets = app.world().resource::<Assets<Font>>();
            let widths = Widths::of(assets.get(&fonts.text));
            let laid = UiFace::lay(face, Lang::En, width, Detail::Full, &widths, plate);
            let em = laid.body.expect("a full face");
            let model = body_depth(&body_blocks(face, Lang::En), em, width, &widths);
            (laid, model)
        };
        let room = body_room(&laid.regions, laid.stats, &laid.sizes);
        let card = app
            .world_mut()
            .spawn(Node {
                width: Val::Px(width),
                height: Val::Px(width * 88.0 / 63.0),
                ..default()
            })
            .id();
        let mut commands = app.world_mut().commands();
        spawn_ui(&mut commands, card, Lang::En, face, &laid, fonts);
        app.world_mut().flush();
        app.update();
        app.update();
        let world = app.world();
        let under = nodes_under(world, card);
        let text_box = under
            .iter()
            .copied()
            .find(|&e| world.entity(e).contains::<FaceTextBox>())
            .expect("a full face has a text box");
        let track = under
            .iter()
            .copied()
            .find(|&e| world.entity(e).contains::<FaceScrollbar>())
            .expect("and a scrollbar");
        let computed = world.get::<ComputedNode>(text_box).expect("laid out");
        let pad = 2.0 * BAR_PAD * width;
        let measured = Measured {
            fitted: laid.body.expect("a full face") * width,
            own: textface::ui_em(textface::UI_BODY, width) * width,
            fits: model <= room,
            model: model * width,
            real: computed.content_size().y * computed.inverse_scale_factor() - pad,
            room: room * width,
            bar: world.get::<Node>(track).expect("a node").display == Display::Flex,
        };
        app.world_mut().entity_mut(card).despawn();
        measured
    }

    /// The first face of pool card `index`, with its English Oracle text,
    /// and the plate its corner would show.
    fn pool_face(index: usize) -> Option<(CardFace, u32)> {
        use baylee_client_core::card_face::{CardText, Characteristics};
        let def =
            baylee_cards::by_index(baylee_core::ids::CardIndex::new(u32::try_from(index).ok()?))?;
        let printed = def.faces.first()?;
        let oracle = baylee_cards::generated_oracle::ORACLE.get(index)?.first()?;
        let chars = Characteristics {
            name: printed.name.to_owned(),
            types: printed.types,
            supertypes: printed.supertypes,
            subtypes: SubtypeSet::from_slice(printed.subtypes),
            colors: ColorSet::EMPTY,
            power: printed.power,
            toughness: printed.toughness,
            loyalty: printed.loyalty,
            damage: 0,
        };
        let text = CardText {
            lang: "en".to_owned(),
            name: printed.name.to_owned(),
            type_line: String::new(),
            oracle_text: (*oracle).to_owned(),
            mana_cost: String::new(),
            english_name: printed.name.to_owned(),
        };
        let face = CardFace::build(
            &chars,
            Some(&printed.mana_cost),
            Some(printed_types(printed)),
            Some(&text),
        );
        let kind = if printed.loyalty.is_some() {
            cardplate::KIND_LOYALTY
        } else if printed.power.is_some() {
            cardplate::KIND_FIGHT
        } else {
            cardplate::KIND_NONE
        };
        Some((face, kind << cardplate::KIND_SHIFT))
    }

    /// The pool's cards with rules text, hardest to model first: the most
    /// marks, then the longest.
    fn hardest_first() -> Vec<usize> {
        use std::cmp::Reverse;
        let oracle = baylee_cards::generated_oracle::ORACLE;
        let mut cards: Vec<usize> = (0..oracle.len())
            .filter(|&i| oracle[i].first().is_some_and(|t| !t.is_empty()))
            .collect();
        cards.sort_by_key(|&i| {
            let text = oracle[i][0];
            (Reverse(text.matches('{').count()), Reverse(text.len()))
        });
        cards
    }

    /// A face whose rules text the fit says fits shows no scrollbar when
    /// bevy lays it out: the fit's model errs deep and never shallow. Over
    /// the forty pool cards whose text is hardest to model, at three preview
    /// widths (#259).
    #[test]
    fn a_face_fitted_to_its_box_fits_it_in_bevy_s_layout() {
        let (mut app, fonts) = layout_app();
        let (mut fits, mut stepped) = (0, 0);
        for index in hardest_first().into_iter().take(40) {
            let (face, plate) = pool_face(index).expect("a pool card");
            for width in [231.0, 308.0, 372.0] {
                let m = measure(&mut app, &fonts, &face, width, plate);
                if m.fits {
                    fits += 1;
                    stepped += usize::from(m.fitted < m.own);
                    assert!(
                        !m.bar,
                        "{} at {width}: fitted at {} px to {:.1} px of room, and bevy \
                         stands it {:.1} deep (the model said {:.1})",
                        face.name, m.fitted, m.room, m.real, m.model
                    );
                }
            }
        }
        assert!(
            fits >= 30 && stepped >= 10,
            "{fits} fitted, {stepped} of them stepped down"
        );
    }

    /// How much of the pool's rules text still runs over at the floor, by
    /// bevy's layout, in English. A measurement and not a check: run it by
    /// name with `--ignored --nocapture`.
    #[test]
    #[ignore = "a measurement over the whole pool; prints, asserts nothing"]
    fn how_much_of_the_pool_runs_over_at_the_floor() {
        let (mut app, fonts) = layout_app();
        let cards: Vec<(CardFace, u32)> =
            hardest_first().into_iter().filter_map(pool_face).collect();
        for width in [231.0, 308.0, 372.0] {
            let (mut over, mut own, mut worst) = (0, 0, 0.0_f32);
            let mut wrong = Vec::new();
            for (face, plate) in &cards {
                let m = measure(&mut app, &fonts, face, width, *plate);
                over += usize::from(m.bar);
                if m.fits && m.bar {
                    wrong.push(format!(
                        "{} ({} px, room {:.1}, model {:.1}, bevy {:.1})",
                        face.name, m.fitted, m.room, m.model, m.real
                    ));
                }
                own += usize::from((m.fitted - m.own).abs() < 1e-3 && !m.bar);
                if m.real > 0.0 {
                    worst = worst.max(m.real / m.model);
                }
            }
            #[allow(clippy::cast_precision_loss)]
            let share = |n: usize| 100.0 * n as f32 / cards.len() as f32;
            println!(
                "{width} px: {} cards; {over} run over at the floor ({:.1}%); {own} fit at \
                 their own size ({:.1}%); bevy's depth against the model's at most {worst:.3}",
                cards.len(),
                share(over),
                share(own)
            );
            println!("  fitted and still running over: {} {wrong:?}", wrong.len());
        }
    }

    /// The thumb stands in proportion to what is shown, never shorter than
    /// its minimum, and there is none while the text fits.
    #[test]
    fn a_thumb_shows_the_share_and_the_place() {
        assert_eq!(thumb(100.0, 100.0, 0.0), None);
        assert_eq!(thumb(100.0, 100.4, 0.0), None, "half a pixel is a fit");
        assert_eq!(thumb(100.0, 200.0, 0.0), Some((0.0, 0.5)));
        assert_eq!(thumb(100.0, 200.0, 100.0), Some((0.5, 0.5)));
        let (top, length) = thumb(100.0, 100_000.0, 99_900.0).expect("a bar");
        assert!((length - THUMB_MIN).abs() < f32::EPSILON);
        assert!((top + length - 1.0).abs() < 1e-5, "at the end");
    }

    /// The system shows a scrollbar only while its box runs over, and stands
    /// its thumb where the box has scrolled to.
    #[test]
    fn a_scrollbar_is_shown_while_its_text_runs_over() {
        let mut app = App::new();
        app.add_systems(Update, show_scrollbars);
        let text_box = app
            .world_mut()
            .spawn((
                FaceTextBox,
                ScrollPosition(Vec2::new(0.0, 100.0)),
                ComputedNode {
                    size: Vec2::new(200.0, 100.0),
                    content_size: Vec2::new(200.0, 200.0),
                    ..default()
                },
            ))
            .id();
        let thumb = app
            .world_mut()
            .spawn((FaceScrollThumb, Node::default()))
            .id();
        let track = app
            .world_mut()
            .spawn((
                FaceScrollbar { text_box },
                Node {
                    display: Display::None,
                    ..default()
                },
            ))
            .add_child(thumb)
            .id();
        app.update();
        let node = |e: Entity, app: &App| {
            app.world()
                .entity(e)
                .get::<Node>()
                .cloned()
                .expect("a node")
        };
        assert_eq!(node(track, &app).display, Display::Flex);
        assert_eq!(node(thumb, &app).top, Val::Percent(50.0));
        assert_eq!(node(thumb, &app).height, Val::Percent(50.0));

        app.world_mut().entity_mut(text_box).insert(ComputedNode {
            size: Vec2::new(200.0, 100.0),
            content_size: Vec2::new(200.0, 100.0),
            ..default()
        });
        app.update();
        assert_eq!(node(track, &app).display, Display::None);
    }

    /// A pool card's face is the row's words on the printed card (#259): the
    /// name and type line the gateway served in the player's language, the
    /// colours the row names, the numbers the registry prints — and without
    /// a catalog, the English Oracle rather than an empty box.
    #[test]
    fn a_pool_card_s_face_is_the_row_s_words_on_the_printed_card() {
        use baylee_client_core::deckbuilder::PoolCard;
        let row = PoolCard {
            index: baylee_cards::decks::by_name("Birds of Paradise")
                .expect("in the pool")
                .get(),
            name: "Paradiesvögel".to_owned(),
            english_name: "Birds of Paradise".to_owned(),
            mana_cost: "{G}".to_owned(),
            colors: "G".to_owned(),
            type_line: "Kreatur — Vogel".to_owned(),
            oracle_text: "Fliegend".to_owned(),
            ..PoolCard::default()
        };
        let face = of_pool(&row);
        assert_eq!(face.name, "Paradiesvögel");
        assert_eq!(face.type_line, "Kreatur — Vogel");
        assert_eq!(face.cost, vec![ManaSymbol::Green]);
        assert_eq!(face.colors, ColorSet::of(MagicColor::Green));
        assert_eq!(
            face.stats,
            Some(Stats::PowerToughness {
                power: 0,
                toughness: 1,
                damage: 0
            })
        );
        assert_eq!(face.body, vec![TextBlock::Rules("Fliegend".to_owned())]);

        let bare = of_pool(&PoolCard {
            oracle_text: String::new(),
            ..row
        });
        let text: Vec<_> = bare.body.iter().map(TextBlock::text).collect();
        assert_eq!(text, ["Flying", "{T}: Add one mana of any color."]);
        assert!(!bare.text_pending);
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
        assert_eq!(world_stats(Some(body), fight.kind()), None);
        assert_eq!(
            world_stats(Some(Stats::Loyalty(4)), Plate::Loyalty(4).kind()),
            None
        );
        assert_eq!(
            world_stats(Some(body), Plate::Loyalty(4).kind()),
            Some(body),
            "an animated planeswalker's body is said nowhere else"
        );
        assert_eq!(world_stats(Some(body), Plate::None.kind()), Some(body));
        assert_eq!(world_stats(None, fight.kind()), None);
        // The overlay asks with the packed word its look carries.
        assert_eq!(fight.packed() >> cardplate::KIND_SHIFT, fight.kind());
        assert_eq!(
            Plate::None.packed() >> cardplate::KIND_SHIFT,
            Plate::None.kind()
        );
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
        assert_eq!(stats_color(lethal), FACE_INKS.1);
        assert_ne!(FACE_INKS.1, FACE_INKS.0);
        assert_eq!(
            stats_color(Stats::PowerToughness {
                power: 1,
                toughness: 2,
                damage: 1
            }),
            FACE_INKS.0
        );
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
