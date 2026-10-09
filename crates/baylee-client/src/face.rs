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
use baylee_client_core::textface::{
    self, BAR_PAD, Fitted, LINE_BOX, Layout, Regions, Sizes, TEXT_INSET,
};
use baylee_core::color::{Color as MagicColor, ColorSet};
use baylee_core::mana::ManaSymbol;
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

/// Whether the player is holding the "show me the text" modifier, and the
/// text step the interface's faces are set at.
#[derive(Resource, Default)]
pub struct FaceMode {
    /// True while Cmd or Alt is down.
    pub held: bool,
    /// The interface's text step (WP6): the overlay's faces multiply their
    /// clamps by it, the table's ignore it. The setting that chooses it is
    /// the shell's (`ClientSettings::text_size`), which
    /// `shellkit::face_follows_the_text_size` writes here; a dev-control
    /// build launched with `BAYLEE_TEXT_STEP` (one to five) keeps that step
    /// instead, so the steps can be photographed.
    pub step: textface::Step,
}

impl FaceMode {
    /// The mode a client starts in: nothing held, and the default step — or
    /// in a dev-control build the step `BAYLEE_TEXT_STEP` names.
    #[must_use]
    pub fn initial() -> Self {
        #[cfg(feature = "dev-control")]
        let step = std::env::var("BAYLEE_TEXT_STEP")
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok())
            .map_or(textface::Step::DEFAULT, textface::Step::new);
        #[cfg(not(feature = "dev-control"))]
        let step = textface::Step::DEFAULT;
        Self { held: false, step }
    }
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
    let mut face = CardFace::from_object(object, cost, printed.map(printed_types), text.as_ref());
    let lang = texts.display_language();
    for change in &object.word_changes {
        let kind = if change.basic_land_type {
            baylee_cards_dsl::TextWordKind::BasicLandType
        } else {
            baylee_cards_dsl::TextWordKind::Color
        };
        let words = baylee_client_core::text_choice::words(kind, lang);
        if let (Some(from), Some(to)) = (
            words.get(usize::from(change.from)),
            words.get(usize::from(change.to)),
        ) {
            face.body.push(TextBlock::Rules(
                Phrase::TextReplacementSummary.fill(lang, &[from, to]),
            ));
        }
    }
    face
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
    let card = baylee_client_core::decision::known_cards(view)
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

/// A text face's cost as spans: each symbol's glyph in the Mana font (the
/// glyph beside it, for the door to fall back from), a hybrid as its two
/// halves' glyphs, and a generic past the font's range as digits.
fn cost_spans(cost: &[ManaSymbol]) -> Vec<(String, Option<char>)> {
    use baylee_client_core::manapip::{Pip, pip};
    cost.iter()
        .flat_map(|symbol| match pip(*symbol) {
            Pip::Solid { glyph, .. } => vec![(glyph.to_string(), Some(glyph))],
            Pip::Split {
                left: (left, _),
                right: (right, _),
            } => vec![
                (left.to_string(), Some(left)),
                (right.to_string(), Some(right)),
            ],
            Pip::Number { value } => vec![(value.to_string(), None)],
            Pip::Loyalty(loyalty) => vec![(loyalty.caption(), None)],
        })
        .collect()
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

/// A chip's padding round its word, across and down, as a share of its em;
/// the gap between two chips and between the band's two rows, likewise; and
/// how round a chip's ends are, in card widths.
const CHIP_PAD: (f32, f32) = (0.5, 0.18);
const CHIP_GAP: f32 = 0.4;
const CHIP_ROUND: f32 = 0.012;

/// A text face laid out for the overlay, before anything is spawned.
///
/// Laid out first because the card's material is keyed by the face's word,
/// and the word carries the bars' depths, which are the fit's, and the
/// layout the rules chose: a long name on two lines is a deeper name bar,
/// and long rules give up the band's room, both drawn by the shader under
/// the text this places (#259, WP6).
pub struct UiFace {
    /// The card's width in pixels.
    width: f32,
    sizes: Sizes,
    name: Fitted,
    kind: Fitted,
    regions: Regions,
    /// How the face is laid out down the card.
    pub layout: Layout,
    /// The rules text's em, for a face that writes all of it; `None` for a
    /// compact one.
    body: Option<f32>,
    /// How deep the rules are against their room, in pixels, where the
    /// fit's model says they run over at the floor: the scrollbar and the
    /// `▾` show from the first frame, before bevy has laid anything out.
    overflow: Option<(f32, f32)>,
    /// A compact face's one line of rules, cut to fit.
    line: Option<String>,
    /// The numbers the card's plate does not already say.
    stats: Option<Stats>,
    /// A cost pip's diameter, in pixels.
    pip: f32,
    /// The set and rarity at the type bar's right end, where the face
    /// credits a printing.
    mark: Option<String>,
    /// The foot's credit line.
    foot: Option<String>,
    /// The band's words: the subtypes.
    subtypes: Vec<String>,
    /// The band's keyword chips.
    chips: Vec<String>,
    /// The text step the face is set at.
    step: textface::Step,
    /// The material's face word ([`textface::face_word`], with its layout).
    pub word: u32,
}

impl UiFace {
    /// `face` laid out on a card `width` pixels wide, its lines measured by
    /// `widths` at the step `widths` carries. `plate` is what the strip over
    /// the card says, packed (`cardrail::Strip::plate`): numbers it already
    /// shows are not written twice.
    ///
    /// A full face is a preview: its rules at their own size if they fit
    /// under the full band, else a pixel smaller at a time; under
    /// [`textface::LONG_PX`] (times the step) or over the floor it takes the
    /// long layout, whose band is narrower, and fits again there. A compact
    /// face is a small card: the band is its keyword strip and the first
    /// sentence of its rules its one line.
    #[must_use]
    pub fn lay(
        face: &CardFace,
        lang: Lang,
        width: f32,
        detail: Detail,
        widths: &Widths<'_>,
        plate: u32,
    ) -> Self {
        let step = widths.step();
        let pip = textface::ui_em_at(textface::UI_NAME, width, step) * width;
        #[allow(clippy::cast_precision_loss)] // a cost has a handful of pips
        let pips = face.cost.len() as f32;
        let cost = if pips > 0.0 {
            (pips * pip + (pips - 1.0) * pip * PIP_GAP) / width
        } else {
            0.0
        };
        let full = detail == Detail::Full;
        let mark = face
            .credit
            .as_ref()
            .filter(|_| full)
            .and_then(baylee_client_core::card_face::Credit::mark);
        let base = Sizes::overlay_at(width, cost, step);
        let sizes = match &mark {
            Some(mark) => base.beside_type(widths.width(mark) * base.small),
            None => base,
        };
        let name = textface::fit_name_in(&sizes, &face.name, |s| widths.width(s));
        let kind = if full {
            textface::fit_type_front_in(&sizes, &face.type_line, |s| widths.width(s))
        } else {
            textface::fit_type_in(&sizes, &face.type_line, |s| widths.width(s))
        };
        let depths = sizes.depths(name.lines.len());
        let stats = world_stats(face.stats, plate >> cardplate::KIND_SHIFT);
        let chips = textface::keyword_chips(face.rules());
        let (layout, regions, body, overflow, line) = if full {
            let blocks = body_blocks(face, lang);
            let fit = |layout: Layout| {
                let regions = Regions::laid(layout, depths);
                let room = body_room(&regions, stats, &sizes);
                let depth = |em: f32| body_depth(&blocks, em, width, widths);
                let em = textface::fit_body_at(width, room, step, depth);
                (layout, regions, em, room, depth(em))
            };
            let mut chosen = fit(Layout::Preview);
            // Under 16 px (times the step), or the card's own size where
            // that is smaller already: a small preview whose rules fit at
            // their own size keeps its band.
            let own = textface::ui_em_at(textface::UI_BODY, width, step) * width;
            let long = (textface::LONG_PX * step.factor())
                .max(step.body_floor())
                .min(own);
            if chosen.2 * width < long - 1e-3 || chosen.4 > chosen.3 {
                chosen = fit(Layout::Long);
            }
            let (layout, regions, em, room, depth) = chosen;
            let overflow = (depth > room).then_some((room * width, depth * width));
            (layout, regions, Some(em), overflow, None)
        } else {
            let regions = Regions::laid(Layout::Small, depths);
            let line = small_line(face, stats, &regions, &sizes, width, widths);
            (Layout::Small, regions, None, None, line)
        };
        let subtypes = if full {
            textface::subtype_words(&face.type_line, face.subtypes.len() as usize)
        } else {
            Vec::new()
        };
        let foot = face
            .credit
            .as_ref()
            .filter(|_| full)
            .and_then(baylee_client_core::card_face::Credit::foot);
        let word = textface::face_word(face.colors, face.types, face.subtypes, depths);
        Self {
            width,
            sizes,
            name,
            kind,
            regions,
            layout,
            body,
            overflow,
            line,
            stats,
            pip,
            mark,
            foot,
            subtypes,
            chips,
            step,
            word: textface::laid(word, layout),
        }
    }

    /// Whether the fit's model says the rules run over their box at the
    /// floor, so the box scrolls.
    #[must_use]
    pub fn overflows(&self) -> bool {
        self.overflow.is_some()
    }
}

/// A compact face's one line of rules: the first sentence of its first
/// rules block, cut at a word with an ellipsis where it runs past the line,
/// which leaves room for the numbers at its right end. Nothing for a face
/// with no rules, or whose text has not arrived.
fn small_line(
    face: &CardFace,
    stats: Option<Stats>,
    regions: &Regions,
    sizes: &Sizes,
    width: f32,
    widths: &Widths<'_>,
) -> Option<String> {
    let first = textface::first_sentence(face.rules().next()?);
    let px = sizes.small * width;
    let [x0, _, x1, _] = regions.text_box;
    let numbers = stats.map_or(0.0, |s| {
        widths.width(&stats_label(s)) * px + TEXT_INSET * width
    });
    let room = (x1 - x0 - 2.0 * TEXT_INSET) * width - numbers;
    let one = (LINE_BOX * px).ceil() + 0.5;
    let fits = |s: &str| crate::manaui::rich_depth(s, px, room, |w| widths.width(w) * px) <= one;
    if fits(first) {
        return Some(first.to_owned());
    }
    let words: Vec<&str> = first.split(' ').collect();
    (1..words.len())
        .rev()
        .map(|keep| format!("{}{}", words[..keep].join(" "), textface::ELLIPSIS))
        .find(|line| fits(line))
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

/// A pool card drawn whole as its text face, filling `frame` as the front
/// side of a card that can turn (`crate::flip`): the card's ground in its
/// colours and the printing's finish, its face over that, `width` logical
/// pixels wide. No plate: a card in the pool is on no battlefield, so its
/// face writes its own numbers.
///
/// The deck builder's two windows draw a card this way where they have no
/// picture to show: the hover preview (#259), and the printing picker,
/// whose art does not load with no way out to Scryfall (the beta.6 review).
#[allow(clippy::too_many_arguments)] // one card: where, what, in which words and fonts
pub fn spawn_ui_card(
    commands: &mut Commands,
    cards: &mut crate::cardmat::UiCards,
    frame: Entity,
    lang: Lang,
    face: &CardFace,
    fonts: &UiFonts,
    widths: &Widths<'_>,
    finish: baylee_client_core::images::FinishTreatment,
    width: f32,
) -> Entity {
    use bevy::ui::{percent, px};
    let laid = UiFace::lay(face, lang, width, Detail::Full, widths, 0);
    let look = crate::cardmat::CardLook::back(finish).faced(table_color(face.colors), laid.word);
    let node = commands
        .spawn((
            MaterialNode(cards.get(look, None)),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            crate::flip::Side::Front,
            Visibility::Inherited,
            Pickable::IGNORE,
        ))
        .id();
    spawn_ui(commands, node, lang, face, &laid, fonts);
    commands.entity(frame).add_child(node);
    node
}

/// Puts `laid`'s face on `card`, a node the size of the card whose material
/// draws the bars under it (or, with no material store, whose colour stands
/// in for them).
///
/// Every node sits where [`textface`] placed it, in card widths scaled by
/// the card's width in pixels, so the text stands on the bars the shader
/// draws from the same word: the name in the name bar with the cost at its
/// right end, as on a print; the type line in the type bar, with the set and
/// rarity at its right end where the face credits a printing; the band's
/// subtypes and keyword chips; the rules text in the text box, set a pixel
/// smaller at a time down to the step's floor to fit and scrolling past
/// that, or a small card's one line; and the foot's credit.
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
///
/// Counted ([`FaceBuilds`]), so a test and the dev harness can hold the face
/// to being built on a change and never per frame.
///
/// A whole card in a window is [`spawn_ui_card`]: this face over a ground of
/// its own.
pub fn spawn_ui(
    commands: &mut Commands,
    card: Entity,
    lang: Lang,
    face: &CardFace,
    laid: &UiFace,
    fonts: &UiFonts,
) {
    use bevy::text::LineBreak;

    commands.queue(|world: &mut World| {
        if let Some(mut builds) = world.get_resource_mut::<FaceBuilds>() {
            builds.0 += 1;
        }
    });
    let w = laid.width;
    let [x0, name_top, x1, _] = laid.regions.name_bar;
    let line = |commands: &mut Commands, text: String, font: TextFont, color: Color, node: Node| {
        let entity = commands
            .spawn((
                Pickable::IGNORE,
                Text::new(text),
                font,
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
        text_font(fonts, laid.name.em * w),
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
    let centred = |em: f32| type_top + (type_foot - type_top - LINE_BOX * em) * 0.5;
    line(
        commands,
        laid.kind.lines.concat(),
        text_font(fonts, laid.kind.em * w),
        FACE_INKS.0,
        placed(w, x0 + TEXT_INSET, centred(laid.kind.em)),
    );
    if let Some(mark) = &laid.mark {
        let em = laid.sizes.small;
        line(
            commands,
            mark.clone(),
            text_font(fonts, em * w),
            FACE_INKS.0,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px((1.0 - x1 + TEXT_INSET) * w),
                top: Val::Px(centred(em) * w),
                ..default()
            },
        );
    }
    spawn_band(commands, card, laid, fonts);

    if let Some(em) = laid.body {
        spawn_text_box(commands, card, lang, face, laid, fonts, em);
    }
    if let Some(words) = &laid.line {
        spawn_small_line(commands, card, laid, fonts, words);
    }
    if let Some(stats) = laid.stats {
        spawn_stats(commands, card, laid, fonts, stats);
    }
    if let Some(foot) = &laid.foot {
        let [fx0, top, _, bottom] = laid.regions.foot;
        let em = textface::ui_em_at(textface::UI_FOOT, w, laid.step);
        line(
            commands,
            foot.clone(),
            TextFont {
                font: bevy::text::FontSource::Handle(fonts.italic.clone()),
                font_size: bevy::text::FontSize::Px(em * w),
                ..default()
            },
            Color::srgb_from_array(textface::FOOT_INK),
            placed(
                w,
                fx0 + TEXT_INSET,
                top + ((bottom - top) - LINE_BOX * em).max(0.0) * 0.5,
            ),
        );
    }
}

/// A small card's one line of rules, centred down its one-line text box and
/// clipped to it.
fn spawn_small_line(
    commands: &mut Commands,
    card: Entity,
    laid: &UiFace,
    fonts: &UiFonts,
    words: &str,
) {
    let w = laid.width;
    let [x0, top, x1, foot] = laid.regions.text_box;
    let px = laid.sizes.small * w;
    let block = crate::manaui::spawn_rich_in(commands, fonts, words, px, FACE_INKS.0, text_font);
    commands.entity(block).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px((x0 + TEXT_INSET) * w),
            top: Val::Px((top + (foot - top - LINE_BOX * laid.sizes.small) * 0.5) * w),
            max_width: Val::Px((x1 - x0 - 2.0 * TEXT_INSET) * w),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            overflow: Overflow::clip(),
            max_height: Val::Px((foot - top) * w),
            ..default()
        },
        Pickable::IGNORE,
    ));
    commands.entity(card).add_child(block);
}

/// The numbers the plate does not say: on a small card beside its one line,
/// on a preview at the text box's foot, right.
fn spawn_stats(
    commands: &mut Commands,
    card: Entity,
    laid: &UiFace,
    fonts: &UiFonts,
    stats: Stats,
) {
    let w = laid.width;
    let [_, top, x1, foot] = laid.regions.text_box;
    let (em, top) = if laid.layout == Layout::Small {
        let em = laid.sizes.small;
        (em, top + (foot - top - LINE_BOX * em) * 0.5)
    } else {
        let em = laid.sizes.name;
        (em, foot - BAR_PAD - LINE_BOX * em)
    };
    let entity = commands
        .spawn((
            Pickable::IGNORE,
            Text::new(stats_label(stats)),
            text_font(fonts, em * w),
            TextColor(stats_color(stats)),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px((1.0 - x1 + TEXT_INSET) * w),
                top: Val::Px(top * w),
                ..default()
            },
        ))
        .id();
    commands.entity(card).add_child(entity);
}

/// The band: the subtype words over the keyword chips on a preview, the
/// chips alone where the band is narrow (a long preview), and the chips at
/// its foot on a small card, where the band is its keyword strip. Clipped
/// to the band, so a card with more keywords than room shows the first.
fn spawn_band(commands: &mut Commands, card: Entity, laid: &UiFace, fonts: &UiFonts) {
    let w = laid.width;
    let [x0, top, x1, foot] = laid.regions.band;
    let em = textface::ui_em_at(textface::UI_CHIP, w, laid.step) * w;
    let words = match laid.layout {
        Layout::Preview => laid.subtypes.as_slice(),
        Layout::Long if laid.chips.is_empty() => laid.subtypes.as_slice(),
        _ => &[],
    };
    if words.is_empty() && laid.chips.is_empty() {
        return;
    }
    let pad = BAR_PAD * w;
    let band = commands
        .spawn((
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px((x0 + TEXT_INSET) * w),
                top: Val::Px(top * w),
                width: Val::Px((x1 - x0 - 2.0 * TEXT_INSET) * w),
                height: Val::Px((foot - top) * w),
                flex_direction: FlexDirection::Column,
                justify_content: if laid.layout == Layout::Small {
                    JustifyContent::FlexEnd
                } else {
                    JustifyContent::FlexStart
                },
                row_gap: Val::Px(CHIP_GAP * em),
                padding: UiRect::vertical(Val::Px(pad)),
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();
    if !words.is_empty() {
        let upper = words
            .iter()
            .map(|word| word.to_uppercase())
            .collect::<Vec<_>>()
            .join(" · ");
        let text = commands
            .spawn((
                Pickable::IGNORE,
                Text::new(upper),
                TextFont {
                    font: bevy::text::FontSource::Handle(fonts.medium.clone()),
                    font_size: bevy::text::FontSize::Px(em),
                    ..default()
                },
                TextColor(FACE_INKS.0),
                TextLayout::new(Justify::Left, bevy::text::LineBreak::NoWrap),
            ))
            .id();
        commands.entity(band).add_child(text);
    }
    if !laid.chips.is_empty() {
        let row = commands
            .spawn((
                Pickable::IGNORE,
                Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: if laid.layout == Layout::Small {
                        FlexWrap::WrapReverse
                    } else {
                        FlexWrap::Wrap
                    },
                    column_gap: Val::Px(CHIP_GAP * em),
                    row_gap: Val::Px(CHIP_GAP * em),
                    ..default()
                },
            ))
            .id();
        for chip in &laid.chips {
            let plate = spawn_chip(commands, fonts, chip, em, CHIP_ROUND * w);
            commands.entity(row).add_child(plate);
        }
        commands.entity(band).add_child(row);
    }
    commands.entity(card).add_child(band);
}

/// One keyword chip: its word in light ink on a slate plate, `em` pixels,
/// its ends `round` pixels round.
fn spawn_chip(commands: &mut Commands, fonts: &UiFonts, chip: &str, em: f32, round: f32) -> Entity {
    let text = commands
        .spawn((
            Pickable::IGNORE,
            Text::new(chip.to_owned()),
            TextFont {
                font: bevy::text::FontSource::Handle(fonts.medium.clone()),
                font_size: bevy::text::FontSize::Px(em),
                ..default()
            },
            TextColor(Color::srgb_from_array(textface::LIGHT_INK)),
            TextLayout::new(Justify::Left, bevy::text::LineBreak::NoWrap),
        ))
        .id();
    commands
        .spawn((
            Pickable::IGNORE,
            Node {
                padding: UiRect::axes(Val::Px(CHIP_PAD.0 * em), Val::Px(CHIP_PAD.1 * em)),
                border_radius: BorderRadius::all(Val::Px(round)),
                ..default()
            },
            BackgroundColor(Color::srgb_from_array(textface::CHIP_PAPER)),
        ))
        .add_child(text)
        .id()
}

/// The rules text, set at `em`, in a box that scrolls past the floor, and
/// the box's scrollbar and its `▾`.
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
    spawn_scrollbar(commands, card, text_box, laid, fonts);
}

/// The text box's scrollbar and the `▾` at its foot, shown from the first
/// frame where the fit's model says the text runs over (WP6) and kept true
/// to bevy's layout from then on by [`show_scrollbars`].
fn spawn_scrollbar(
    commands: &mut Commands,
    card: Entity,
    text_box: Entity,
    laid: &UiFace,
    fonts: &UiFonts,
) {
    let w = laid.width;
    let [_, top, x1, foot] = laid.regions.text_box;
    let foot = foot - stats_line(laid.stats, &laid.sizes);
    let hue = textface::Hue::from_code(laid.word >> textface::FACE_BARS_SHIFT);
    let [r, g, b] = textface::paper(hue).map(|c| c * TRACK_SHADE);
    let [tr, tg, tb] = hue.tone();
    let round = BorderRadius::all(Val::Px(TRACK_ROUND * w));
    // The model's share of the text the box shows, which bevy's layout will
    // correct by a pixel or so a frame later.
    let shown = laid
        .overflow
        .and_then(|(room, depth)| thumb(room, depth, 0.0));
    let display = if shown.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    let (thumb_top, thumb_length) = shown.unwrap_or((0.0, 1.0));
    let thumb = commands
        .spawn((
            Pickable::IGNORE,
            FaceScrollThumb,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                top: Val::Percent(thumb_top * 100.0),
                height: Val::Percent(thumb_length * 100.0),
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
                display,
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

    // The `▾`: centred on the box's foot, in the bars' colour, while more
    // of the text is below.
    let size = laid.sizes.small * w;
    let more = commands
        .spawn((
            Pickable::IGNORE,
            FaceMore { text_box },
            Text::new(crate::hud::glyph::CARET_DOWN.to_string()),
            TextFont {
                font: bevy::text::FontSource::Handle(fonts.icons.clone()),
                font_size: bevy::text::FontSize::Px(size),
                ..default()
            },
            TextColor(Color::linear_rgb(tr, tg, tb)),
            Node {
                display,
                position_type: PositionType::Absolute,
                left: Val::Px(0.5 * w - 0.5 * size),
                top: Val::Px(foot * w - LINE_BOX * size),
                ..default()
            },
        ))
        .id();
    commands.entity(card).add_child(more);
}

/// The `▾` at a text box's foot, naming its box: shown while more of the
/// text is below what the box shows ([`show_scrollbars`]).
#[derive(Component, Clone, Copy, Debug)]
pub struct FaceMore {
    /// The box whose offset it follows.
    pub text_box: Entity,
}

/// How many faces [`spawn_ui`] has built in this app: a counter the dev
/// harness reads (`/state`'s `face_builds`) and a test holds still over idle
/// frames, so a face rebuilt per frame shows up as a number that moves.
///
/// A resource and not a process-wide count, so two apps in one test binary
/// do not count each other's faces. An app without it counts nothing.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceBuilds(pub u64);

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

/// What a `▾` is, for [`show_scrollbars`]: a node that is neither a track
/// nor a thumb, which the borrow checker needs said.
type NeitherBarNorThumb = (Without<FaceScrollThumb>, Without<FaceScrollbar>);

/// Shows each face's scrollbar while its text runs over, and stands the
/// thumb where the box has scrolled to; and its `▾` while more of the text
/// is below ([`FaceMore`]).
///
/// Written only on a change, because a `Node` written every frame is a
/// layout every frame. A box bevy has not laid out yet (a size of zero)
/// leaves what the fit's model showed at spawn alone.
pub fn show_scrollbars(
    boxes: Query<(&ScrollPosition, &ComputedNode), With<FaceTextBox>>,
    mut tracks: Query<(&FaceScrollbar, &mut Node, &Children)>,
    mut thumbs: Query<&mut Node, (With<FaceScrollThumb>, Without<FaceScrollbar>)>,
    mut mores: Query<(&FaceMore, &mut Node), NeitherBarNorThumb>,
) {
    let measured = |text_box: Entity| {
        let (position, computed) = boxes.get(text_box).ok()?;
        let scale = computed.inverse_scale_factor();
        let view = computed.size().y * scale;
        (view > 0.0).then(|| (view, computed.content_size().y * scale, position.y))
    };
    for (more, mut node) in &mut mores {
        let Some((view, content, offset)) = measured(more.text_box) else {
            continue;
        };
        let display = if content - offset - view > 0.5 {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    for (bar, mut node, children) in &mut tracks {
        let Some((view, content, offset)) = measured(bar.text_box) else {
            continue;
        };
        let shown = thumb(view, content, offset);
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
///
/// It carries the interface's text step too ([`textface::Step`]), because a
/// face is fitted by the two together: the step moves the sizes the widths
/// are multiplied by. The table's faces ignore it.
pub struct Widths<'a> {
    font: Option<swash::FontRef<'a>>,
    step: textface::Step,
}

impl<'a> Widths<'a> {
    /// Widths read off `font`, or the average without one, at the default
    /// step.
    #[must_use]
    pub fn of(font: Option<&'a Font>) -> Self {
        Self {
            font: font.and_then(|font| swash::FontRef::from_index(font.data.data(), 0)),
            step: textface::Step::DEFAULT,
        }
    }

    /// The same widths, for faces set at text step `step`.
    #[must_use]
    pub fn at(self, step: textface::Step) -> Self {
        Self { step, ..self }
    }

    /// The text step an interface face is set at.
    #[must_use]
    pub fn step(&self) -> textface::Step {
        self.step
    }

    /// How wide a line of rules is at an em of one, its symbols (`{T}`)
    /// each taken as a Mana-font glyph a full em wide.
    #[must_use]
    pub fn marked(&self, text: &str) -> f32 {
        use baylee_client_core::manapip::{Segment, segments};
        segments(text)
            .iter()
            .map(|segment| match segment {
                Segment::Text(words) => self.width(words),
                Segment::Symbol(_) => 1.0,
            })
            .sum()
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
    /// The first sentence of the rules, on its lines
    /// ([`textface::SENTENCE_LINES`] at most), or none.
    pub sentence: Vec<String>,
    /// Whether the widths were the font's ([`Widths::measured`]).
    pub measured: bool,
}

impl WorldFit {
    /// `face`'s name, type line and first sentence, fitted by `widths`.
    #[must_use]
    pub fn of(face: &CardFace, widths: &Widths<'_>) -> Self {
        use textface::{fit_name, fit_type};
        let sentence = face.rules().next().map_or_else(Vec::new, |rules| {
            textface::fit_lines(
                textface::first_sentence(rules),
                textface::SENTENCE_EM,
                textface::line_width(),
                textface::SENTENCE_LINES,
                |s| widths.marked(s),
            )
        });
        Self {
            name: fit_name(&face.name, |s| widths.width(s)),
            kind: fit_type(&face.type_line, |s| widths.width(s)),
            sentence,
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
/// special case. `plate` is what the card's plate already says, so the face
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
    let mut cost_root = None;
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
            entity
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
            let [_, top, _, _] = regions.band;
            // The printed symbols in the Mana font, one span each so each
            // falls back on its own (`manaui::ink_the_marks`); a hybrid is
            // its two colours' glyphs, and a number past the font, digits.
            let cost = line(
                String::new(),
                textface::SMALL_EM,
                Color::srgb_from_array(textface::cost_ink(word).srgb()),
                on_the_card(x1 - TEXT_INSET, top + BAR_PAD),
                Anchor::TOP_RIGHT,
            );
            cost_root = Some(cost);
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
    if let Some(cost) = cost_root {
        let ink = TextColor(Color::srgb_from_array(textface::cost_ink(word).srgb()));
        spawn_cost_spans(commands, cost, &face.cost, ink, fonts);
    }
    texts.extend(spawn_discs(commands, card, word, &regions, fonts));
    texts.extend(spawn_sentence(
        commands,
        card,
        &fit.sentence,
        &regions,
        fonts,
    ));
    texts
}

/// A text face's cost line as spans under `root`: each symbol's glyph in the
/// Mana font, one span each so each falls back on its own
/// (`manaui::ink_the_marks`), a thin space between them.
fn spawn_cost_spans(
    commands: &mut Commands,
    root: Entity,
    cost: &[ManaSymbol],
    ink: TextColor,
    fonts: &UiFonts,
) {
    let size = textface::SMALL_EM * PX_PER_UNIT;
    let face = |font: &Handle<Font>| TextFont {
        font: bevy::text::FontSource::Handle(font.clone()),
        font_size: bevy::text::FontSize::Px(size),
        ..default()
    };
    for (n, (text, mark)) in cost_spans(cost).into_iter().enumerate() {
        if n > 0 {
            commands.spawn((
                TextSpan::new("\u{2009}"),
                face(&fonts.text),
                ink,
                ChildOf(root),
            ));
        }
        let font = if mark.is_some() {
            &fonts.mana
        } else {
            &fonts.text
        };
        let span = commands
            .spawn((TextSpan::new(text), face(font), ink, ChildOf(root)))
            .id();
        if let Some(mark) = mark {
            commands
                .entity(span)
                .insert(crate::manaui::ManaInk::in_line(mark, size));
        }
    }
}

/// The symbol band (WP6): each disc the shader draws wears its colour's
/// symbol, in the Mana font, as a cost's pip does.
fn spawn_discs(
    commands: &mut Commands,
    card: Entity,
    word: u32,
    regions: &Regions,
    fonts: &UiFonts,
) -> Vec<Entity> {
    use baylee_client_core::manapip::{Pip, of_color};
    use bevy::sprite::Anchor;
    textface::discs(word, regions)
        .into_iter()
        .filter_map(|([x, y], hue)| {
            let Pip::Solid { glyph, .. } = of_color(hue_color(hue)?) else {
                return None;
            };
            let entity = commands
                .spawn((
                    WorldFace,
                    Text2d::new(glyph.to_string()),
                    TextFont {
                        font: bevy::text::FontSource::Handle(fonts.mana.clone()),
                        font_size: bevy::text::FontSize::Px(DISC_GLYPH_EM * PX_PER_UNIT),
                        ..default()
                    },
                    TextColor(FACE_INKS.0),
                    crate::manaui::ManaInk::in_line(glyph, DISC_GLYPH_EM * PX_PER_UNIT),
                    Anchor::CENTER,
                    Transform::from_translation(on_the_card(x, y).extend(0.002))
                        .with_scale(Vec3::splat(1.0 / PX_PER_UNIT)),
                    ChildOf(card),
                ))
                .id();
            Some(entity)
        })
        .collect()
}

/// The first sentence of the rules, in the text box (WP6): what a card does,
/// read off the table without opening its preview. One `Text2d` of spans, a
/// symbol's in the Mana font, on the lines the fit broke it into.
fn spawn_sentence(
    commands: &mut Commands,
    card: Entity,
    lines: &[String],
    regions: &Regions,
    fonts: &UiFonts,
) -> Option<Entity> {
    use bevy::sprite::Anchor;
    use bevy::text::LineBreak;
    if lines.is_empty() {
        return None;
    }
    let [x0, top, _, _] = regions.text_box;
    let size = textface::SENTENCE_EM * PX_PER_UNIT;
    let root = commands
        .spawn((
            WorldFace,
            Text2d::default(),
            TextFont {
                font: bevy::text::FontSource::Handle(fonts.text.clone()),
                font_size: bevy::text::FontSize::Px(size),
                ..default()
            },
            TextColor(FACE_INKS.0),
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
            Anchor::TOP_LEFT,
            Transform::from_translation(on_the_card(x0 + TEXT_INSET, top + BAR_PAD).extend(0.002))
                .with_scale(Vec3::splat(1.0 / PX_PER_UNIT)),
            ChildOf(card),
        ))
        .id();
    for (n, words) in lines.iter().enumerate() {
        let words = if n + 1 < lines.len() {
            format!("{words}\n")
        } else {
            words.clone()
        };
        for (text, mana) in sentence_spans(&words) {
            let font = if mana { &fonts.mana } else { &fonts.text };
            let mark = mana.then(|| text.clone());
            let span = commands
                .spawn((
                    TextSpan::new(text),
                    TextFont {
                        font: bevy::text::FontSource::Handle(font.clone()),
                        font_size: bevy::text::FontSize::Px(size),
                        ..default()
                    },
                    TextColor(FACE_INKS.0),
                    ChildOf(root),
                ))
                .id();
            if let Some(mark) = mark {
                crate::manaui::ink_span(commands, span, &mark, size);
            }
        }
    }
    Some(root)
}

/// The table's colour-disc glyph, as an em in card widths: the disc's
/// diameter less a ring.
const DISC_GLYPH_EM: f32 = 0.11;

/// The colour a hue is the colour of, for the five that are one.
const fn hue_color(hue: textface::Hue) -> Option<MagicColor> {
    use textface::Hue;
    match hue {
        Hue::White => Some(MagicColor::White),
        Hue::Blue => Some(MagicColor::Blue),
        Hue::Black => Some(MagicColor::Black),
        Hue::Red => Some(MagicColor::Red),
        Hue::Green => Some(MagicColor::Green),
        Hue::Gold | Hue::Grey => None,
    }
}

/// A line of rules as spans: its prose, and each symbol as its glyph in the
/// Mana font (`true`). A symbol with no single glyph (a number past the
/// font's range, a loyalty cost) is written as it is printed.
fn sentence_spans(line: &str) -> Vec<(String, bool)> {
    use baylee_client_core::manapip::{Pip, Segment, Tick, segments};
    segments(line)
        .into_iter()
        .map(|segment| match segment {
            Segment::Text(text) => (text, false),
            Segment::Symbol(
                Pip::Solid { glyph, .. }
                | Pip::Split {
                    left: (glyph, _), ..
                },
            ) => (glyph.to_string(), true),
            Segment::Symbol(Pip::Number { value }) => (value.to_string(), false),
            Segment::Symbol(Pip::Loyalty(loyalty)) => {
                let sign = match loyalty.tick {
                    Tick::Up => "+",
                    Tick::Down => "\u{2212}",
                    Tick::Flat => "",
                };
                (format!("{sign}{}", loyalty.caption()), false)
            }
        })
        .collect()
}

/// The body line a text face still has to write: none when its plate
/// already says the same number (#274, #298). `plate` is the plate's kind
/// ([`Plate::kind`]).
///
/// A card on the table showing its text face always shows its plate at its
/// bottom right (`Corner::shows_plate`: there is no print to say the
/// numbers), so a creature drawn as text would say `3/3` twice. What it keeps is the one shape where the two differ — a planeswalker that is
/// also a creature plates its loyalty, and its power and toughness are then
/// said nowhere else on the table — and every card with no plate over it, as
/// a card in hand has.
fn world_stats(stats: Option<Stats>, plate: u32) -> Option<Stats> {
    match (stats?, plate) {
        (Stats::PowerToughness { .. }, cardplate::KIND_FIGHT)
        | (Stats::Loyalty(_), cardplate::KIND_LOYALTY) => None,
        (stats, _) => Some(stats),
    }
}

#[cfg(test)]
pub(crate) mod tests;
