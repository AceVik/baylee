//! The choices a player answers by *position*, and what to call each row.
//!
//! Four of the engine's pending choices are not a click on the board, because
//! what they name is not on it: a colour, a seat, one of several ways to cast
//! a spell, and a creature type. The interaction model has always been able to
//! answer all four — `Interaction::choose_index` and
//! `Interaction::confirm` — and until now nothing in the client called
//! either, so a tapped Underground Sea asked "Choose a colour" and the prompt
//! bar drew that sentence with no buttons under it. The game stopped there
//! for good, the first time such a land was tapped for its mana.
//!
//! This module is the list. It is a sibling of [`crate::abilities`] and for
//! the same reason: the *label* is what needs to know about mana symbols and
//! card faces, which is knowledge `baylee-client-core` does not carry. A
//! seat's name is the one that came back — every line that talks about
//! another chair wants it, so it is [`baylee_client_core::i18n::seat_name`]
//! and this list asks for it like everybody else.
//!
//! [`Prompt`] is what it reads, not [`baylee_engine::choice::Pending`] — the
//! prompt already carries every option the engine offered, in the engine's
//! order. Each row retains that original index even when subtype labels are sorted.
//!
//! **A creature type is the awkward one.** [`Prompt::ChooseSubtype`] offers all
//! three hundred and fifty of them, and three hundred and fifty buttons is not
//! a chooser — so it is the one choice with a `filter` in front of it, and the
//! only one where a row's position in the returned list is *not* its answer.
//! That is what [`ChoiceOption::index`] is for. Cavern of Souls asks this
//! question as it *enters*, so a client that cannot answer it loses the game
//! on a land drop.
//!
//! **A card name is the same shape, larger.** [`Prompt::ChooseCardName`]
//! carries no list, because any card's name may be chosen (CR 201.4) and the
//! pool is the list: this client's own, which is the engine's, since the two
//! ship as one build. So the rows are every face of the pool, in one fixed
//! order ([`card_name_at`]), narrowed by the same box, and a row's index is
//! its place in that order. The model is told the card and face through
//! [`pick`], which is the door every chooser press goes through.

use baylee_client_core::card_face::TextBlock;
use baylee_client_core::i18n::{Lang, Phrase, seat_name};
use baylee_client_core::interaction::{Interaction, Prompt};
use baylee_client_core::manapip::{self, Pip};
use baylee_core::generated::subtypes;
use baylee_core::ids::{CardIndex, ObjectId, SubtypeId};
use baylee_core::mana::{ManaColor, ManaCost, ManaSymbol};
use baylee_view::GameStatic;

/// Where a row that names a card face looks that name up.
///
/// Two steps, and they live in two places: the object says which card and
/// which printing (the view), and the printing says what the face is called
/// in the player's own language (the catalog's text). Both are optional, and
/// each absence is honest rather than a failure — the view has not arrived
/// yet, the gateway serves no card text, or the caller is one of the two that
/// read only the *shape* of the list and never draw a label at all.
///
/// A face that cannot be named falls back to the phrase the row carried
/// before, which still says what kind of option it is.
#[derive(Clone, Copy, Default)]
pub struct FaceNames<'a> {
    /// The seat's view. Without it nothing can be named.
    pub view: Option<&'a baylee_view::PlayerView>,
    /// The catalog's text. Without it a name is the registry's English.
    pub texts: Option<&'a crate::cardtext::CardTexts>,
}

impl FaceNames<'_> {
    /// What face `face` of `object` is called.
    fn of(self, object: ObjectId, face: usize) -> Option<String> {
        crate::face::face_name(object, face, self.view?, self.texts)
    }
}

/// Which face a cast option is a fact about.
///
/// Zero, always, and it is a fact about `cast_wizard` rather than about
/// this list: the engine enumerates modes out of `abilities_for_face(0)`
/// and alternative costs out of `def.faces[0]`, so `Mode(i)` and
/// `Alternative(i)` count the *front* face's however the card is turned.
/// The two options that name a face carry their own index instead, which
/// is what `FaceNames::of` is given.
const CAST_FACE: usize = 0;

/// Which card `object` is, for a cast chooser.
///
/// It is in **hand** when it is a spell being cast, which is the one zone
/// `PlayerView::object` does not answer for, and on the battlefield (or
/// anywhere else the view shows it) when it is the source of a modal
/// trigger.
fn cast_card(names: FaceNames<'_>, object: ObjectId) -> Option<CardIndex> {
    let view = names.view?;
    view.hand
        .iter()
        .find(|c| c.id == object)
        .map(|c| c.card)
        .or_else(|| view.object(object).and_then(|o| o.card))
        .map(|card| card.index)
}

/// One printed sentence of the card `object` is, in the player's own
/// language — the label a row of the cast chooser wants.
///
/// The same steps a stack entry's sentence is drawn through, and two of
/// them may honestly come up empty: the generated table has to know which
/// sentence a mode or an alternative cost is
/// ([`baylee_cards::lines::mode_line`] and its twin), and the card has to be
/// findable — it is in **hand**, which is the one zone `PlayerView::object`
/// does not answer for. A caller then draws the row's cost and no words.
/// The words themselves never come up empty: they
/// are [`crate::cardtext::sentence`]'s, the player's language when its text
/// pairs and the card's English Oracle when it does not or has not arrived.
///
/// Reminder text is dropped. It is the card explaining itself, which is
/// worth a whole line on a card and is not what a button says: evoke's
/// bracketed half alone is longer than the prompt bar.
///
/// `line` is the lookup rather than a flag because the two are the same
/// function twice — a face's modes and its alternative costs are two lists
/// and one shape, exactly as `Mode(i)` and `Alternative(i)` are.
fn printed_sentence(
    names: FaceNames<'_>,
    object: ObjectId,
    line: fn(CardIndex, usize, usize) -> Option<baylee_cards::lines::AbilityLine>,
    at: usize,
) -> Option<String> {
    let card = cast_card(names, object)?;
    let found = line(card, CAST_FACE, at)?;
    let printed = baylee_view::StackText {
        face: u8::try_from(CAST_FACE).ok()?,
        line: found.line,
        of: found.of,
    };
    let blocks = crate::cardtext::sentence(names.texts, card, printed)?;
    let said: Vec<&str> = blocks
        .iter()
        .filter_map(|block| match block {
            TextBlock::Rules(text) => Some(text.as_str()),
            TextBlock::Reminder(_) => None,
        })
        .collect();
    // The bullet a modal card lists its modes under is the list's mark and
    // not the mode's words — the row is already one of several.
    //
    // Four marks, because a printing chooses its own and this is read off
    // the catalog rather than off Scryfall's English: `•` in English,
    // Portuguese and Chinese (which leaves no space after it), `*` in
    // German, French, Spanish and Italian, and `・` in Japanese. The index
    // is computed against the English text and is unaffected; only what the
    // row draws is.
    let said = said.join(" ");
    let said = without_spree_cost(
        said.trim_start_matches(['\u{2022}', '*', '\u{30fb}', ' '])
            .trim(),
    );
    (!said.is_empty()).then(|| said.to_string())
}

/// A spree mode's words without the plus sign and the mode's own cost in
/// front of them: "+ {1} — Destroy all creatures." is drawn "Destroy all
/// creatures.", because the row draws beside it what the whole set of modes
/// costs, and the mode's share of that is not what the player pays.
///
/// Anything that does not have the whole shape — the sign, one or more
/// symbols, a dash — is returned as it was.
fn without_spree_cost(said: &str) -> &str {
    let Some(rest) = said.strip_prefix('+') else {
        return said;
    };
    let mut rest = rest.trim_start();
    let mut symbols = 0;
    while let Some(after) = rest.strip_prefix('{') {
        let Some(end) = after.find('}') else {
            return said;
        };
        rest = &after[end + 1..];
        symbols += 1;
    }
    let rest = rest.trim_start();
    match rest.strip_prefix(['\u{2014}', '\u{2013}', '-']) {
        Some(words) if symbols > 0 => words.trim_start(),
        _ => said,
    }
}

/// What one mode of a modal card says: its printed sentence, the card's own
/// words for a choice made inside one sentence, and failing both its number.
fn mode_label(lang: Lang, object: ObjectId, names: FaceNames<'_>, i: usize) -> String {
    // One-based in the fallback, because the printed card numbers its
    // modes from one and a player reads the card, not the index.
    printed_sentence(names, object, baylee_cards::lines::mode_line, i)
        .or_else(|| {
            let card = cast_card(names, object)?;
            baylee_cards::lines::inline_mode_words(card, CAST_FACE, i).map(str::to_string)
        })
        .unwrap_or_else(|| Phrase::CastModeNumber.fill(lang, &[&(i + 1).to_string()]))
}

/// One row of an indexed chooser.
#[derive(Clone, PartialEq, Debug)]
pub struct ChoiceOption {
    /// Which of the engine's options this row answers.
    ///
    /// The same as the row's position for every choice but a creature type,
    /// where a filter has hidden most of the list. Carrying it means the
    /// filter can never make a button send the wrong answer.
    pub index: usize,
    /// What the button says. Empty when the pip alone carries it, which is
    /// the colour chooser: a `{U}` disc says "blue" in every language there
    /// is, and the word beside it would only be the same claim twice.
    pub label: String,
    /// A mana symbol drawn before the label.
    pub pip: Option<Pip>,
    /// A cost drawn after the label, for the cast options that have one.
    pub cost: Option<ManaCost>,
}

impl ChoiceOption {
    /// A row that is only words.
    fn text(index: usize, label: String) -> Self {
        Self {
            index,
            label,
            pip: None,
            cost: None,
        }
    }
}

/// How many creature types the chooser draws at once.
///
/// The engine offers every type there is, which is not a list anybody reads;
/// it is one they filter. Twelve is what the prompt bar holds, and the hint
/// line under the headline is what says so.
pub const SUBTYPE_ROWS: usize = 12;

/// The creature types matching `filter`, alphabetized in the client language.
///
/// A prefix match and not a substring one, because typing `el` to be offered
/// "Elf" and "Elemental" is the behaviour a player predicts; `Rebel` turning
/// up as well is not. A type the generated table has no name for still gets a
/// row — by its number, the same rule as a seat the statics do not describe —
/// because the alternative is an option the engine offered and nobody can
/// pick.
fn subtype_rows(options: &[SubtypeId], filter: &str, lang: Lang) -> Vec<ChoiceOption> {
    let needle = filter.trim().to_lowercase();
    let mut rows: Vec<_> = options
        .iter()
        .enumerate()
        .filter_map(|(i, id)| match subtypes::name(*id) {
            Some(name) => {
                let translated = baylee_client_core::type_names::name(name, lang);
                (translated.to_lowercase().starts_with(&needle)
                    || name.to_lowercase().starts_with(&needle))
                .then(|| ChoiceOption::text(i, translated.to_string()))
            }
            None => needle
                .is_empty()
                .then(|| ChoiceOption::text(i, format!("#{}", id.get()))),
        })
        .collect();
    rows.sort_by_cached_key(|row| (row.label.starts_with('#'), row.label.to_lowercase()));
    rows.truncate(SUBTYPE_ROWS);
    rows
}

/// The symbol for one colour of mana.
///
/// [`manapip::of_color`] takes a [`baylee_core::color::Color`], which has no
/// colourless arm; the engine's choice list is [`ManaColor`], which does. So
/// the mapping is spelled out here rather than half-converted.
fn colour_pip(color: ManaColor) -> Pip {
    manapip::pip(match color {
        ManaColor::White => ManaSymbol::White,
        ManaColor::Blue => ManaSymbol::Blue,
        ManaColor::Black => ManaSymbol::Black,
        ManaColor::Red => ManaSymbol::Red,
        ManaColor::Green => ManaSymbol::Green,
        ManaColor::Colorless => ManaSymbol::Colorless,
    })
}

/// Localizes only the exact entitled source snapshot, never a newer object.
fn source_label(
    lang: Lang,
    source: baylee_core::ids::DamageSourceRef,
    names: FaceNames<'_>,
    statics: Option<&GameStatic>,
) -> String {
    let Some(projected) = names
        .view
        .and_then(|view| view.damage_sources.iter().find(|s| s.source == source))
    else {
        return Phrase::SourceUnknown.text(lang).to_string();
    };
    let text = projected
        .rules
        .or_else(|| projected.card.map(Into::into))
        .zip(names.texts)
        .and_then(|(face, texts)| texts.face(face.card, face.face));
    let name = baylee_client_core::card_face::shown_name(&projected.name, text.as_ref());
    let mut label = compact_source_label(lang, projected, name, statics);
    let context = source_stack_context(lang, projected, names, statics);
    if !context.is_empty() {
        label.push('\n');
        label.push_str(&context);
    }
    label
}

/// The chooser gives identity and incarnation; concrete stack targets follow below.
fn compact_source_label(
    lang: Lang,
    source: &baylee_view::DamageSourceView,
    name: &str,
    statics: Option<&GameStatic>,
) -> String {
    let name = if source.card.is_none() && source.rules.is_none() && source.token.is_none() {
        match name {
            "Unknown source" => Phrase::SourceUnknown.text(lang),
            "Face-down" => Phrase::SourceFaceDown.text(lang),
            other => other,
        }
    } else {
        name
    };
    let identity = match (source.power, source.toughness) {
        (Some(power), Some(toughness)) => format!("{name} ({power}/{toughness})"),
        _ => name.to_string(),
    };
    let age = if source.is_current {
        Phrase::SourceCurrent.text(lang).to_string()
    } else {
        format!(
            "{} {}",
            Phrase::SourceHistorical.text(lang),
            source.source.version
        )
    };
    format!(
        "{identity} · {} · {age}",
        seat_name(lang, statics, source.controller)
    )
}

/// Names an exact public target without falling back to a newer object identity.
pub(crate) fn target_label(
    lang: Lang,
    target: baylee_view::TargetRef,
    names: FaceNames<'_>,
    statics: Option<&GameStatic>,
) -> String {
    let source = match target {
        baylee_view::TargetRef::Object(source) => source,
        baylee_view::TargetRef::Player(player) => return seat_name(lang, statics, player),
    };
    let Some(projected) = names.view.and_then(|view| view.target_object(source)) else {
        return Phrase::PreviousTarget.text(lang).to_string();
    };
    let text = projected
        .rules
        .or_else(|| projected.card.map(Into::into))
        .zip(names.texts)
        .and_then(|(face, texts)| texts.face(face.card, face.face));
    let name = baylee_client_core::card_face::shown_name(&projected.name, text.as_ref());
    let name = if projected.card.is_none() && projected.rules.is_none() && projected.token.is_none()
    {
        match name {
            "Unknown source" => Phrase::SourceUnknown.text(lang),
            "Face-down" => Phrase::SourceFaceDown.text(lang),
            other => other,
        }
    } else {
        name
    };
    if projected.is_current {
        name.to_string()
    } else {
        format!(
            "{name} · {} {}",
            Phrase::SourceHistorical.text(lang),
            source.version
        )
    }
}

/// References come from the exact snapshot; only their public targets are named.
fn source_stack_context(
    lang: Lang,
    source: &baylee_view::DamageSourceView,
    names: FaceNames<'_>,
    statics: Option<&GameStatic>,
) -> String {
    let Some(view) = names.view else {
        return String::new();
    };
    source
        .referenced_by
        .iter()
        .filter_map(|id| {
            let entry = view.stack.iter().find(|entry| entry.id == *id)?;
            let targets: Vec<_> = entry
                .targets
                .iter()
                .map(|target| target_label(lang, *target, names, statics))
                .collect();
            (!targets.is_empty())
                .then(|| format!("{} → {}", Phrase::StackTitle.text(lang), targets.join(", ")))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn source_options(
    options: &[baylee_core::ids::DamageSourceRef],
    lang: Lang,
    statics: Option<&GameStatic>,
    names: FaceNames<'_>,
) -> Vec<ChoiceOption> {
    options
        .iter()
        .enumerate()
        .map(|(index, &source)| {
            ChoiceOption::text(
                index,
                format!(
                    "{}. {}",
                    index + 1,
                    source_label(lang, source, names, statics)
                ),
            )
        })
        .collect()
}

/// The rows of an indexed choice, or `None` when this prompt is not one.
///
/// [`ChoiceOption::index`] is the answer, and a caller passes *that* to
/// `Interaction::choose_index` — not the row's position, which stops being
/// the same thing as soon as `filter` hides part of the list. `filter` is
/// read for a creature type and ignored by every other choice.
///
/// The overlay and the click handler call this same function, so a button can
/// only ever send an answer the current prompt is still offering.
#[must_use]
pub fn options(
    prompt: &Prompt,
    lang: Lang,
    statics: Option<&GameStatic>,
    filter: &str,
    names: FaceNames<'_>,
) -> Option<Vec<ChoiceOption>> {
    match prompt {
        Prompt::ChooseDamageSource { options } => {
            Some(source_options(options, lang, statics, names))
        }
        Prompt::ChooseDamageEffect { damage, options } => Some(
            options
                .iter()
                .enumerate()
                .map(|(index, effect)| {
                    ChoiceOption::text(
                        index,
                        damage_effect_label(effect, damage, lang, statics, names),
                    )
                })
                .collect(),
        ),
        Prompt::AllocatePrevention {
            effect,
            damage,
            amounts,
            ..
        } => Some(prevention_options(
            effect, damage, amounts, lang, statics, names,
        )),
        Prompt::ChooseColor { options } => Some(
            options
                .iter()
                .enumerate()
                .map(|(i, c)| ChoiceOption {
                    index: i,
                    label: String::new(),
                    pip: Some(colour_pip(*c)),
                    cost: None,
                })
                .collect(),
        ),
        Prompt::ChoosePlayer { options } => Some(
            options
                .iter()
                .enumerate()
                // A seat with no identity is still a seat that has to be
                // pickable, so `seat_name` numbers it rather than dropping
                // the row — in the words a player is shown everywhere else,
                // which this list used to spell `#7`.
                .map(|(i, p)| {
                    let label = names.view.map_or_else(
                        || seat_name(lang, statics, *p),
                        |view| {
                            let role = statics
                                .and_then(|s| s.seats.iter().find(|seat| seat.player == *p))
                                .map_or(
                                    baylee_client_core::board::SeatRole::Present,
                                    baylee_client_core::board::SeatRole::of,
                                );
                            crate::hud::seatbar::called(lang, view, statics, *p, role)
                        },
                    );
                    ChoiceOption::text(i, label)
                })
                .collect(),
        ),
        Prompt::CastMode { object, options } => Some(
            options
                .iter()
                .enumerate()
                .map(|(i, desc)| ChoiceOption {
                    index: i,
                    label: cast_label(desc.kind, lang, *object, names),
                    pip: None,
                    // The cost is the part that actually distinguishes two
                    // alternative costs from each other; the words above it
                    // only say which kind of thing it is. It is *also* what
                    // fails for a pathway, whose two options cost nothing and
                    // differ only in the name they print — which is why the
                    // label names the face rather than the kind.
                    //
                    // The normal way has no words at all, so its cost is the
                    // whole row, and a spell cast for nothing says `{0}`
                    // rather than drawing an empty row.
                    cost: if desc.cost.is_empty() {
                        matches!(desc.kind, baylee_engine::choice::CastModeKind::Normal)
                            .then(|| ManaCost::try_parse("{0}").ok())
                            .flatten()
                    } else {
                        Some(desc.cost)
                    },
                })
                .collect(),
        ),
        Prompt::ChooseSubtype { options } => Some(subtype_rows(options, filter, lang)),
        // A pile is its cards: the row names them, in the order the
        // separation gave them, and an empty pile says so rather than
        // drawing a blank row, because it is a pile that may be taken.
        Prompt::ChoosePile { piles } => Some(
            piles
                .iter()
                .enumerate()
                .map(|(i, pile)| {
                    let cards = if pile.is_empty() {
                        Phrase::EmptyPile.text(lang).to_string()
                    } else {
                        pile.iter()
                            .map(|id| names.of(*id, 0).unwrap_or_else(|| "?".to_string()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    ChoiceOption::text(
                        i,
                        Phrase::PileRow.fill(lang, &[&(i + 1).to_string(), &cards]),
                    )
                })
                .collect(),
        ),
        Prompt::ChooseCardName => Some(card_name_rows(filter)),
        _ => None,
    }
}

/// Picks row `index` of the chooser [`options`] draws for `interaction`'s
/// prompt: by position for an indexed choice, and for a card name by the
/// card and face that row stands for.
///
/// One door for every press, key and click alike, so a row can only ever
/// answer what it says. Returns `false` when the row answers nothing.
pub fn pick(interaction: &mut Interaction, index: usize) -> bool {
    if matches!(interaction.prompt(), Prompt::ChooseCardName) {
        return card_name_at(index)
            .is_some_and(|(card, face)| interaction.choose_card_name(card, face));
    }
    interaction.choose_index(index)
}

/// The row that is picked, in the numbering [`options`] gives its rows.
#[must_use]
pub fn picked(interaction: &Interaction) -> Option<usize> {
    interaction
        .chosen_card_name()
        .and_then(|(card, face)| card_name_row(card, face))
        .or_else(|| interaction.chosen_index())
}

/// One face of the pool, as the card-name chooser lists it.
struct NamedRow {
    card: CardIndex,
    face: u8,
    name: &'static str,
    /// The name as it is matched, lower-cased once.
    lower: String,
}

/// Every name the pool prints, one row per face, alphabetized, a name two
/// faces share listed once.
///
/// Built once: the pool is compiled in, and a chooser that rebuilt three
/// thousand rows would do it every frame the box is open.
fn card_names() -> &'static [NamedRow] {
    static NAMES: std::sync::LazyLock<Vec<NamedRow>> = std::sync::LazyLock::new(|| {
        let mut rows: Vec<NamedRow> = baylee_cards::all()
            .flat_map(|def| {
                def.faces
                    .iter()
                    .enumerate()
                    .filter_map(move |(face, printed)| {
                        Some(NamedRow {
                            card: def.index,
                            face: u8::try_from(face).ok()?,
                            name: printed.name,
                            lower: printed.name.to_lowercase(),
                        })
                    })
            })
            .collect();
        rows.sort_by(|a, b| {
            (&a.lower, a.name, a.card, a.face).cmp(&(&b.lower, b.name, b.card, b.face))
        });
        rows.dedup_by(|a, b| a.name == b.name);
        rows
    });
    &NAMES
}

/// The card and face row `index` of the card-name chooser names.
#[must_use]
pub fn card_name_at(index: usize) -> Option<(CardIndex, u8)> {
    card_names().get(index).map(|row| (row.card, row.face))
}

/// The row that names face `face` of `card`.
fn card_name_row(card: CardIndex, face: u8) -> Option<usize> {
    card_names()
        .iter()
        .position(|row| row.card == card && row.face == face)
}

/// The card names matching `filter`, cut to what the bar holds.
///
/// A name that begins with what was typed first, then one with a word that
/// does, each alphabetized: typing `needle` is how a player looks for Pithing
/// Needle, and a prefix match alone would never offer it. In the pool's
/// English, which is the name's identity in the rules and the one every
/// client has; nothing is matched inside a word.
fn card_name_rows(filter: &str) -> Vec<ChoiceOption> {
    let needle = filter.trim().to_lowercase();
    let word_start = |lower: &str| {
        lower
            .split(|c: char| !c.is_alphanumeric())
            .any(|word| word.starts_with(&needle))
    };
    let mut rows: Vec<(bool, ChoiceOption)> = card_names()
        .iter()
        .enumerate()
        .filter_map(|(i, row)| {
            let first = row.lower.starts_with(&needle);
            (first || word_start(&row.lower))
                .then(|| (!first, ChoiceOption::text(i, row.name.to_string())))
        })
        .collect();
    rows.sort_by_key(|(later, _)| *later);
    rows.truncate(SUBTYPE_ROWS);
    rows.into_iter().map(|(_, row)| row).collect()
}

/// What one cast option is called.
///
/// Two of the six kinds name a **face**, and they say the face's own name
/// where it can be found, because that is the only thing that tells them
/// apart: a pathway's two land faces are the same kind at the same empty cost
/// and differ in nothing else a row draws.
///
/// Two more are a **sentence the card prints**, and they are the reason this
/// function is not a table of six phrases. `Mode(i)` and `Alternative(i)` are
/// the whole of what the engine says about a mode and an alternative cost —
/// there is no label in the protocol, and there could not be, because the
/// engine carries no card text at all. So the row read "Mode 2", which asks a
/// player to pick between two numbers on a card they may never have seen, and
/// the ability sheet beside it had been drawing the printed sentence since
/// it existed. The generated line table is what closes that: it says which
/// sentence a mode is, the catalog says what that sentence is in the player's
/// own language, and the two together make a row that reads like the card.
///
/// The **normal** way is no sentence at all: the card's printed mana cost,
/// which the row draws as pips beside an empty label. It said "Printed
/// cost", which is this client's sentence and not the card's.
///
/// An alternative cost's sentence is missing only where the card itself
/// cannot be named (the generated table knows every one,
/// `lines::every_mode_and_alternative_cost_knows_its_printed_sentence`), and
/// the row then draws its cost and nothing invented beside it: it said
/// "Alternative cost" before.
///
/// A mode that has no sentence of its own is one of three modal triggers
/// that state their choice **inside** one sentence (Derevi, Inspirit,
/// Tireless Provisioner): "put your choice of a +1/+1 counter or two charge
/// counters". Its row says the card's own words for it, "a +1/+1 counter",
/// which [`baylee_cards::lines::inline_mode_words`] holds as a verbatim
/// slice of the English Oracle sentence. English, because nothing can say
/// where inside a translated sentence the choice sits; the player's language
/// falls back to English everywhere the card's text cannot be paired. These
/// rows read "Mode 1" and "Mode 2" until #319.
///
/// Only a mode that is neither keeps its number: one whose card cannot be
/// found, and the effect-less mode a player declines "choose up to one"
/// with, which a card prints nowhere. A trigger's mode costs nothing, so
/// without the number those rows would be blank and identical.
fn cast_label(
    kind: baylee_engine::choice::CastModeKind,
    lang: Lang,
    object: ObjectId,
    names: FaceNames<'_>,
) -> String {
    use baylee_engine::choice::CastModeKind as K;
    match kind {
        K::Normal => String::new(),
        K::Kicked => Phrase::CastKicked.text(lang).to_string(),
        K::Alternative(i) => {
            printed_sentence(names, object, baylee_cards::lines::alternative_line, i)
                .unwrap_or_default()
        }
        K::Mode(i) => mode_label(lang, object, names, i),
        // Several modes at once (Farewell, a spree card): each one's own
        // words, in the order the card prints them, which is the order they
        // happen in (CR 608.2c), joined by the plus a spree card lists them
        // under.
        K::Modes(set) => (0..u8::BITS as usize)
            .filter(|i| set & (1 << i) != 0)
            .map(|i| mode_label(lang, object, names, i))
            .collect::<Vec<_>>()
            .join(" + "),
        K::Face(i) => names
            .of(object, i)
            .unwrap_or_else(|| Phrase::CastBackFace.text(lang).to_string()),
        K::PlayLandFace(i) => names
            .of(object, i)
            .unwrap_or_else(|| Phrase::CastLandFace.text(lang).to_string()),
        K::Disguise => Phrase::CastDisguise.text(lang).to_string(),
        K::Prototype => Phrase::CastPrototype.text(lang).to_string(),
        K::Miracle => Phrase::CastMiracle.text(lang).to_string(),
        K::Flashback => Phrase::CastFlashback.text(lang).to_string(),
        K::Dash => Phrase::CastDash.text(lang).to_string(),
        K::Escape => Phrase::CastEscape.text(lang).to_string(),
    }
}

/// The same source and effect explanation on the table and in a zone chooser.
pub(crate) fn target_question(
    interaction: &baylee_client_core::Interaction,
    view: &baylee_view::PlayerView,
    lang: Lang,
    texts: &crate::cardtext::CardTexts,
    statics: Option<&GameStatic>,
) -> Vec<String> {
    if matches!(
        interaction.pending(),
        baylee_engine::choice::Pending::ChooseTargets {
            reason: baylee_engine::choice::TargetPrompt::Retarget { .. },
            ..
        }
    ) {
        return vec![interaction.prompt().headline_naming_targets(
            lang,
            baylee_client_core::Turn::of(view.active, view.seat),
            statics,
            false,
            &|_| None,
            &|source| {
                Some(target_label(
                    lang,
                    baylee_view::TargetRef::Object(source),
                    FaceNames {
                        view: Some(view),
                        texts: Some(texts),
                    },
                    statics,
                ))
            },
        )];
    }
    target_explanation(view, lang, texts)
}

pub(crate) fn target_explanation(
    view: &baylee_view::PlayerView,
    lang: Lang,
    texts: &crate::cardtext::CardTexts,
) -> Vec<String> {
    let Some(context) = &view.targeting else {
        return Vec::new();
    };
    let mut lines = vec![
        Phrase::TargetingFor.fill(lang, &[&crate::face::name_of(&context.source, view, texts)]),
    ];
    let blocks = context.source.rules.and_then(|rules| {
        if context.whole_spell {
            texts
                .face(rules.card, rules.face)
                .map(|text| baylee_client_core::card_face::split_blocks(&text.oracle_text))
        } else {
            context
                .text
                .and_then(|sentence| crate::cardtext::sentence(Some(texts), rules.card, sentence))
        }
    });
    if let Some(blocks) = blocks {
        let text = blocks
            .iter()
            .filter_map(|b| match b {
                baylee_client_core::card_face::TextBlock::Rules(s) => Some(s.as_str()),
                baylee_client_core::card_face::TextBlock::Reminder(_) => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        if !text.is_empty() {
            lines.push(text);
        }
    }
    lines
}

/// Resolve only card-bearing choices; navigation and filters have no preview.
pub(crate) fn preview_object(
    duel: &crate::Duel,
    index: usize,
) -> Option<baylee_core::ids::ObjectId> {
    use baylee_client_core::{
        interaction::AttackOption,
        targeting::{self, Target},
    };
    if duel.cast_menu.is_some() {
        return None;
    }
    let interaction = duel.interaction.as_ref()?;
    match targeting::options(interaction.pending()).get(index) {
        Some(Target::Object(id)) => return Some(*id),
        Some(Target::Player(_)) => return None,
        None => {}
    }
    match interaction.attack_options().get(index) {
        Some(
            AttackOption::Toggle(id)
            | AttackOption::Aim(baylee_core::ids::Defender::Planeswalker(id)),
        ) => Some(*id),
        _ => None,
    }
}

fn prevention_options(
    effect: &baylee_engine::choice::DamageEffectOption,
    damage: &[baylee_engine::choice::DamagePartView],
    amounts: &[u32],
    lang: Lang,
    statics: Option<&GameStatic>,
    names: FaceNames<'_>,
) -> Vec<ChoiceOption> {
    damage
        .iter()
        .enumerate()
        .map(|(index, part)| {
            let label = baylee_client_core::damage::part_label(lang, part, &|target| {
                damage_target(target, lang, statics, names)
            });
            let phrase = if matches!(
                effect.kind,
                baylee_engine::choice::DamageEffectKind::RemoveCounter { .. }
            ) {
                Phrase::DamageCounterShare
            } else {
                Phrase::PreventionShare
            };
            ChoiceOption::text(
                index,
                phrase.fill(
                    lang,
                    &[
                        &label,
                        &amounts.get(index).copied().unwrap_or(0).to_string(),
                        &part.amount.to_string(),
                    ],
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;
    use baylee_engine::choice::{ArrangePrompt, CastModeDesc, CastModeKind};

    #[test]
    fn a_colour_row_is_a_symbol_and_no_word() {
        let rows = options(
            &Prompt::ChooseColor {
                options: vec![ManaColor::Blue, ManaColor::Black],
            },
            Lang::En,
            None,
            "",
            FaceNames::default(),
        )
        .expect("a colour choice has rows");
        assert_eq!(rows.len(), 2, "one row per offered colour, in engine order");
        assert!(rows.iter().all(|r| r.pip.is_some() && r.label.is_empty()));
        assert_ne!(rows[0].pip, rows[1].pip, "two colours, two symbols");
    }

    #[test]
    fn a_seat_row_says_the_name_the_table_shows() {
        let statics = GameStatic {
            decision_secs: None,
            reconnect_secs: None,
            view_version: baylee_view::VIEW_VERSION,
            game_id: "g".into(),
            your_seat: PlayerId::new(0),
            seats: vec![baylee_view::SeatIdentity {
                player: PlayerId::new(1),
                display_name: "House AI".into(),
                is_ai: true,
                away: false,
                team: None,
            }],
            prints: vec![],
        };
        let rows = options(
            &Prompt::ChoosePlayer {
                options: vec![PlayerId::new(1), PlayerId::new(7)],
            },
            Lang::En,
            Some(&statics),
            "",
            FaceNames::default(),
        )
        .expect("a player choice has rows");
        assert_eq!(rows[0].label, "House AI");
        // A seat the statics do not describe still gets a row: a chooser that
        // dropped it would offer fewer answers than the engine did, and the
        // index of everything after it would name the wrong seat. It is
        // numbered in the words the rest of the interface numbers a seat in,
        // which this row used to spell `#7`.
        assert_eq!(rows[1].label, "Seat 7");
    }

    #[test]
    fn player_choices_use_the_same_localized_house_name_as_the_table() {
        let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        let mut statics = GameStatic {
            decision_secs: None,
            reconnect_secs: None,
            view_version: baylee_view::VIEW_VERSION,
            game_id: "g".into(),
            your_seat: PlayerId::new(0),
            seats: vec![baylee_view::SeatIdentity {
                player: PlayerId::new(1),
                display_name: String::new(),
                is_ai: true,
                away: false,
                team: None,
            }],
            prints: vec![],
        };
        for (name, expected) in [
            ("House AI", "Haus-KI"),
            ("Solide 1", "Solide 1"),
            ("Ada", "Ada"),
        ] {
            statics.seats[0].display_name = name.into();
            let rows = options(
                &Prompt::ChoosePlayer {
                    options: vec![statics.seats[0].player],
                },
                Lang::De,
                Some(&statics),
                "",
                FaceNames {
                    view: Some(&view),
                    texts: None,
                },
            )
            .unwrap();
            assert_eq!(rows[0].label, expected);
            assert_eq!(
                rows[0].label,
                crate::hud::seatbar::called(
                    Lang::De,
                    &view,
                    Some(&statics),
                    statics.seats[0].player,
                    baylee_client_core::board::SeatRole::House
                )
            );
        }
    }

    #[test]
    fn a_cast_option_of_an_unnamed_card_is_its_cost() {
        let desc = |kind, cost: &str| CastModeDesc {
            index: 0,
            kind,
            cost: ManaCost::try_parse(cost).expect("a cost"),
        };
        let rows = options(
            &Prompt::CastMode {
                object: ObjectId::new(1, 0),
                options: vec![
                    desc(CastModeKind::Normal, "{2}{U}"),
                    desc(CastModeKind::Mode(1), "{U}"),
                ],
            },
            Lang::En,
            None,
            "",
            FaceNames::default(),
        )
        .expect("a cast choice has rows");
        // No view names the card, so there is no sentence to draw: each
        // row is its cost, and nothing this client composed stands in.
        assert_eq!(rows[0].label, "", "the normal way is its cost");
        assert_eq!(
            rows[1].label, "Mode 2",
            "a mode with no sentence keeps its number, one-based as printed"
        );
        assert_eq!(rows[0].cost, ManaCost::try_parse("{2}{U}").ok());
        assert_eq!(rows[1].cost, ManaCost::try_parse("{U}").ok());
    }

    /// A normal cast for nothing is a `{0}`, not an empty row: the cost is
    /// all the normal way draws. An alternative cost for no mana keeps no
    /// pips, because its sentence says what it charges.
    #[test]
    fn a_free_normal_cast_says_zero() {
        let rows = options(
            &Prompt::CastMode {
                object: ObjectId::new(1, 0),
                options: vec![
                    CastModeDesc {
                        index: 0,
                        kind: CastModeKind::Normal,
                        cost: ManaCost::ZERO,
                    },
                    CastModeDesc {
                        index: 1,
                        kind: CastModeKind::Alternative(0),
                        cost: ManaCost::ZERO,
                    },
                ],
            },
            Lang::En,
            None,
            "",
            FaceNames::default(),
        )
        .expect("a cast choice has rows");
        let zero = rows[0].cost.expect("a free normal cast still draws a cost");
        assert!(!zero.is_empty());
        assert_eq!(zero.to_string(), "{0}");
        assert_eq!(rows[1].cost, None);
    }

    /// Brightclimb Pathway, in the hand, as one object with two land faces.
    ///
    /// The hand and not the battlefield because that is where the question is
    /// asked from — and because `PlayerView::object` does not look in the
    /// hand, which is the whole reason [`crate::face::face_name`] searches it
    /// first.
    fn pathway_in_hand() -> (baylee_view::PlayerView, ObjectId) {
        let def = baylee_cards::by_oracle_id("1c633e02-95ef-445e-b4e0-fbfbc5ed9cc9")
            .expect("Brightclimb Pathway is in the pool");
        let id = ObjectId::new(11, 0);
        let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        view.hand = vec![baylee_view::HandObject {
            id,
            card: baylee_view::CardIdentity {
                index: def.index,
                print: baylee_core::ids::PrintRef::new(0),
                face: 0,
            },
            name: def.faces[0].name.to_string(),
            mana_value: 0,
            colors: baylee_core::color::ColorSet::default(),
            types: baylee_core::types::TypeSet::LAND,
            commander: false,
        }];
        (view, id)
    }

    /// The owner's AE11: two buttons that said the same thing.
    ///
    /// A pathway's two options are the same kind at the same empty cost, so
    /// every column a row draws was identical — the label, the missing pip,
    /// the missing cost — and the choice was made blind.
    #[test]
    fn a_pathways_two_land_faces_are_two_different_buttons() {
        let (view, object) = pathway_in_hand();
        let desc = |face: usize| CastModeDesc {
            index: u8::try_from(face).expect("a face index"),
            kind: CastModeKind::PlayLandFace(face),
            cost: ManaCost::ZERO,
        };
        let rows = options(
            &Prompt::CastMode {
                object,
                options: vec![desc(0), desc(1)],
            },
            Lang::En,
            None,
            "",
            FaceNames {
                view: Some(&view),
                texts: None,
            },
        )
        .expect("a cast choice has rows");
        assert_eq!(rows[0].label, "Brightclimb Pathway");
        assert_eq!(rows[1].label, "Grimclimb Pathway");
        assert!(
            rows.iter().all(|r| r.cost.is_none()),
            "a land face costs nothing, which is why the label has to carry it"
        );
    }

    /// The rung under it: nothing to look a name up with is not a blank row.
    ///
    /// A view that has not arrived, an object the seat cannot place, a card
    /// the registry does not have — each leaves the phrase that says what kind
    /// of option this is, which is what every row said before.
    #[test]
    fn a_face_that_cannot_be_named_keeps_the_words_it_had() {
        let rows = options(
            &Prompt::CastMode {
                object: ObjectId::new(11, 0),
                options: vec![CastModeDesc {
                    index: 0,
                    kind: CastModeKind::PlayLandFace(1),
                    cost: ManaCost::ZERO,
                }],
            },
            Lang::En,
            None,
            "",
            FaceNames::default(),
        )
        .expect("a cast choice has rows");
        assert_eq!(rows[0].label, Phrase::CastLandFace.text(Lang::En));
    }

    /// The prompts this module does not answer: they are clicks on the board
    /// or the table, and a chooser row drawn for one would be a second, wrong
    /// way to answer.
    #[test]
    fn prompts_that_are_not_indexed_choices_have_no_rows() {
        assert!(
            options(
                &Prompt::Arrange {
                    reason: ArrangePrompt::Order,
                    onto: None,
                },
                Lang::En,
                None,
                "",
                FaceNames::default(),
            )
            .is_none()
        );
        assert!(
            options(
                &Prompt::ChooseTargets {
                    min: 1,
                    max: 1,
                    reason: baylee_engine::choice::TargetPrompt::Targets,
                },
                Lang::En,
                None,
                "",
                FaceNames::default(),
            )
            .is_none()
        );
    }

    /// Every subtype the generated table knows, which is what the engine
    /// hands over for a Cavern of Souls.
    fn all_subtypes() -> Vec<SubtypeId> {
        (0..400).map(SubtypeId::new).collect()
    }

    #[test]
    fn an_unfiltered_type_list_is_cut_to_what_the_bar_holds() {
        let rows = options(
            &Prompt::ChooseSubtype {
                options: all_subtypes(),
            },
            Lang::En,
            None,
            "",
            FaceNames::default(),
        )
        .expect("a subtype choice has rows");
        assert_eq!(
            rows.len(),
            SUBTYPE_ROWS,
            "the bar holds twelve, not four hundred"
        );
        assert!(
            rows.windows(2)
                .all(|pair| pair[0].label.to_lowercase() <= pair[1].label.to_lowercase())
        );
        for row in rows {
            assert_eq!(
                subtypes::name(all_subtypes()[row.index]),
                Some(row.label.as_str())
            );
        }
    }

    /// The property the whole `index` field exists for: once a filter has
    /// hidden part of the list, a row's position is no longer its answer.
    #[test]
    fn a_filtered_row_still_carries_the_engine_s_own_index() {
        let all = all_subtypes();
        let elf = all
            .iter()
            .position(|id| subtypes::name(*id) == Some("Elf"))
            .expect("the generated table knows about elves");
        let rows = options(
            &Prompt::ChooseSubtype { options: all },
            Lang::En,
            None,
            "elf",
            FaceNames::default(),
        )
        .expect("a subtype choice has rows");
        assert_eq!(rows[0].label, "Elf", "a prefix match, case-insensitively");
        assert_ne!(rows[0].index, 0, "an elf is not the first creature type");
        assert_eq!(rows[0].index, elf, "the row answers the engine's option");
    }

    #[test]
    fn creature_types_search_in_both_languages_and_keep_the_original_answer() {
        let offered = vec![
            subtypes::creature::ELF,
            subtypes::creature::ALLY,
            subtypes::creature::WIZARD,
        ];
        for needle in ["verb", "VERBÜ", "ally"] {
            let rows = subtype_rows(&offered, needle, Lang::De);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].index, 1);
            assert_eq!(rows[0].label, "Verbündeter");
        }
        let rows = subtype_rows(&offered, "zaub", Lang::De);
        assert_eq!((rows[0].index, rows[0].label.as_str()), (2, "Zauberer"));
        assert!(subtype_rows(&offered, "verb", Lang::En).is_empty());
        assert_eq!(subtype_rows(&offered, "ally", Lang::En)[0].label, "Ally");
    }

    /// A card name is answered out of the pool: typing a word of the name
    /// offers it, the row stands for its card and face, and picking it is
    /// what the model sends.
    #[test]
    fn a_card_name_is_found_by_any_word_of_it_and_sent_as_its_card() {
        let needle = baylee_cards::decks::by_name("Pithing Needle").expect("in the pool");
        let rows = options(
            &Prompt::ChooseCardName,
            Lang::En,
            None,
            "needle",
            FaceNames::default(),
        )
        .expect("a card name is a chooser");
        let row = rows
            .iter()
            .find(|row| row.label == "Pithing Needle")
            .expect("a word of the name finds it, not only its start");
        assert_eq!(card_name_at(row.index), Some((needle, 0)));
        assert!(rows.len() <= SUBTYPE_ROWS);

        let pending = baylee_engine::choice::Pending::ChooseCardName {
            player: baylee_core::ids::PlayerId::new(0),
        };
        let mut interaction = Interaction::new(pending, baylee_core::ids::PlayerId::new(0));
        assert!(pick(&mut interaction, row.index));
        assert_eq!(picked(&interaction), Some(row.index));
        assert_eq!(
            interaction.confirm(),
            Some(baylee_engine::choice::PlayerAction::ChooseCardName {
                card: needle,
                face: 0
            })
        );
        assert!(
            !pick(&mut interaction, usize::MAX),
            "a row past the pool answers nothing"
        );
    }

    /// A back face's name is a name of its own (CR 201.4d), and a name that
    /// begins with what was typed comes before one with a later word that does.
    #[test]
    fn a_back_face_is_named_and_a_leading_match_comes_first() {
        let rebirth = baylee_cards::decks::by_name("Malakir Rebirth").expect("in the pool");
        let rows = card_name_rows("malakir mire");
        assert_eq!(
            rows.first().and_then(|row| card_name_at(row.index)),
            Some((rebirth, 1)),
            "Malakir Mire is the back of Malakir Rebirth"
        );
        let rows = card_name_rows("pith");
        assert_eq!(
            rows.first().map(|row| row.label.as_str()),
            Some("Pithing Needle")
        );
        assert!(card_name_rows("zzzzqq").is_empty());
    }

    #[test]
    fn a_filter_that_matches_nothing_offers_nothing() {
        let rows = options(
            &Prompt::ChooseSubtype {
                options: all_subtypes(),
            },
            Lang::En,
            None,
            "zzzz",
            FaceNames::default(),
        )
        .expect("a subtype choice still answers, with no rows");
        assert!(
            rows.is_empty(),
            "nothing may be pickable that does not match"
        );
    }

    /// One card in the hand with one printing's text filed against it.
    ///
    /// The hand for the reason `pathway_in_hand` gives — it is where a cast
    /// question is asked from, and the one zone `PlayerView::object` does
    /// not answer for.
    fn asking_about(
        oracle_id: &str,
        lang: &str,
        oracle: &str,
    ) -> (
        baylee_view::PlayerView,
        crate::cardtext::CardTexts,
        ObjectId,
    ) {
        use baylee_client_core::card_face::{CardTextEntry, FaceText};
        let def = baylee_cards::by_oracle_id(oracle_id).expect("the card is in the pool");
        let print = baylee_core::ids::PrintRef::new(3);
        let id = ObjectId::new(21, 0);
        let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        view.hand = vec![baylee_view::HandObject {
            id,
            card: baylee_view::CardIdentity {
                index: def.index,
                print,
                face: 0,
            },
            name: def.faces[0].name.to_string(),
            mana_value: 0,
            colors: baylee_core::color::ColorSet::default(),
            types: def.faces[0].types,
            commander: false,
        }];
        // Served the way the gateway serves it: `printed` is the asked
        // language's text, and never English.
        let texts = crate::cardtext::CardTexts::filed(CardTextEntry {
            oracle_id: oracle_id.to_string(),
            layout: "normal".to_string(),
            scryfall_id: "x".to_string(),
            lang: lang.to_string(),
            faces: vec![FaceText {
                printed: (lang != "en").then(|| oracle.to_string()),
                name: def.faces[0].name.to_string(),
                english_name: def.faces[0].name.to_string(),
                type_line: String::new(),
                oracle_text: oracle.to_string(),
                mana_cost: String::new(),
            }],
        });
        (view, texts, id)
    }

    /// The rows of a cast chooser over `kinds`, all at the same cost.
    fn cast_rows(
        object: ObjectId,
        kinds: &[CastModeKind],
        names: FaceNames<'_>,
        lang: Lang,
    ) -> Vec<ChoiceOption> {
        options(
            &Prompt::CastMode {
                object,
                options: kinds
                    .iter()
                    .enumerate()
                    .map(|(i, kind)| CastModeDesc {
                        index: u8::try_from(i).expect("a small list"),
                        kind: *kind,
                        cost: ManaCost::ZERO,
                    })
                    .collect(),
            },
            lang,
            None,
            "",
            names,
        )
        .expect("a cast choice has rows")
    }

    /// The owner's AM1: a mode row says what the mode *does*.
    ///
    /// Sheoldred's Edict prints a header and three bullets, and the engine
    /// offers three modes carrying nothing but their own index — so the
    /// chooser drew "Mode 1", "Mode 2", "Mode 3" and the player picked
    /// between three numbers. The bullet is the list's mark and not the
    /// mode's words, so it is cut off the front.
    #[test]
    fn a_mode_row_says_the_bullet_the_card_prints() {
        let (view, texts, object) = asking_about(
            "217062f5-96f1-454c-9507-17f34ef37070",
            "en",
            "Choose one —\n\
             • Each opponent sacrifices a nontoken creature of their choice.\n\
             • Each opponent sacrifices a creature token of their choice.\n\
             • Each opponent sacrifices a planeswalker of their choice.",
        );
        let rows = cast_rows(
            object,
            &[
                CastModeKind::Mode(0),
                CastModeKind::Mode(1),
                CastModeKind::Mode(2),
            ],
            FaceNames {
                view: Some(&view),
                texts: Some(&texts),
            },
            Lang::En,
        );
        assert_eq!(
            rows[0].label,
            "Each opponent sacrifices a nontoken creature of their choice."
        );
        assert_eq!(
            rows[2].label,
            "Each opponent sacrifices a planeswalker of their choice."
        );
    }

    /// And it says it in the player's own language, because the sentence is
    /// the *catalog's* and only the index is the registry's.
    ///
    /// The text is the one the gateway actually serves for this printing,
    /// **including its bullet**: a printing chooses its own mark, and the
    /// German one is `*` where the English is `•`. A trim that knew only
    /// the English mark left every German mode row starting with a star.
    #[test]
    fn a_mode_row_is_read_in_the_language_the_player_chose() {
        let (view, texts, object) = asking_about(
            "217062f5-96f1-454c-9507-17f34ef37070",
            "de",
            "Bestimme eines -\n\
             * Jeder Gegner opfert eine Nichtspielsteinkreatur, die er bestimmt.\n\
             * Jeder Gegner opfert einen Kreaturenspielstein, den er bestimmt.\n\
             * Jeder Gegner opfert einen Planeswalker, den er bestimmt.",
        );
        let rows = cast_rows(
            object,
            &[CastModeKind::Mode(1)],
            FaceNames {
                view: Some(&view),
                texts: Some(&texts),
            },
            Lang::De,
        );
        assert_eq!(
            rows[0].label,
            "Jeder Gegner opfert einen Kreaturenspielstein, den er bestimmt."
        );
    }

    /// A row for several modes says each one's words, in printed order, and
    /// for a spree card without the plus sign and the mode's own cost in
    /// front of them: the row draws beside it what the whole set costs.
    #[test]
    fn a_row_of_several_modes_says_each_without_its_own_cost() {
        let (view, texts, object) = asking_about(
            "7e7ec3d6-a84f-4cc3-93f4-4d181d41e126",
            "en",
            "Spree (Choose one or more additional costs.)\n\
             + {1} — All creatures lose all abilities until end of turn.\n\
             + {1} — Choose a creature you control. It gains indestructible \
             until end of turn.\n\
             + {3}{W}{W} — Destroy all creatures.",
        );
        let rows = cast_rows(
            object,
            &[CastModeKind::Modes(0b101)],
            FaceNames {
                view: Some(&view),
                texts: Some(&texts),
            },
            Lang::En,
        );
        assert_eq!(
            rows[0].label,
            "All creatures lose all abilities until end of turn. + Destroy all creatures."
        );
    }

    /// "Choose two": Cryptic Command's row for a pair says both modes' words,
    /// in printed order, the way a spree row does.
    #[test]
    fn a_row_of_two_chosen_modes_says_both() {
        let (view, texts, object) = asking_about(
            "a3e51a35-09df-4189-b131-08a21e6a557d",
            "en",
            "Choose two —\n\
             • Counter target spell.\n\
             • Return target permanent to its owner's hand.\n\
             • Tap all creatures your opponents control.\n\
             • Draw a card.",
        );
        let rows = cast_rows(
            object,
            &[CastModeKind::Modes(0b1100)],
            FaceNames {
                view: Some(&view),
                texts: Some(&texts),
            },
            Lang::En,
        );
        assert_eq!(
            rows[0].label,
            "Tap all creatures your opponents control. + Draw a card."
        );
    }

    /// Ragavan's dash row names the keyword in the player's language, beside
    /// the printed cost's row, which needs no words.
    #[test]
    fn a_dash_row_names_the_keyword() {
        for (lang, code, dash) in [(Lang::En, "en", "Dash"), (Lang::De, "de", "Sturmangriff")] {
            let (view, texts, object) =
                asking_about("37108cd4-bbab-4ce3-9ed6-f60e8422e703", code, "Dash {1}{R}");
            let rows = cast_rows(
                object,
                &[CastModeKind::Normal, CastModeKind::Dash],
                FaceNames {
                    view: Some(&view),
                    texts: Some(&texts),
                },
                lang,
            );
            assert_eq!(rows[1].label, dash);
        }
    }

    /// Uro's escape row names the keyword in the player's language.
    #[test]
    fn an_escape_row_names_the_keyword() {
        for (lang, code, escape) in [(Lang::En, "en", "Escape"), (Lang::De, "de", "Befreiung")] {
            let (view, texts, object) = asking_about(
                "ee302659-59ed-4eef-babe-451b9ccf7f14",
                code,
                "Escape—{G}{G}{U}{U}, Exile five other cards from your graveyard.",
            );
            let rows = cast_rows(
                object,
                &[CastModeKind::Escape],
                FaceNames {
                    view: Some(&view),
                    texts: Some(&texts),
                },
                lang,
            );
            assert_eq!(rows[0].label, escape);
        }
    }

    /// A printing whose own split is a different length is refused whole,
    /// and the row falls to the card's English Oracle rather than to a number.
    ///
    /// The `of` guard, at the one surface where being off by one is worst:
    /// an index that is *in* range of a printing one mode short names the
    /// mode beside the right one and draws it as the card's own words. So the
    /// printing is not read at all, and the owner's rule for every row drawn
    /// from card text — "Fallback ist immer englisch" — supplies the
    /// sentence the index was counted against.
    #[test]
    fn a_printing_of_a_different_length_falls_to_the_english_oracle() {
        let (view, texts, object) = asking_about(
            "217062f5-96f1-454c-9507-17f34ef37070",
            "en",
            "Choose one —\n\
             • Each opponent sacrifices a nontoken creature of their choice.\n\
             • Each opponent sacrifices a creature token of their choice.",
        );
        let rows = cast_rows(
            object,
            &[CastModeKind::Mode(1)],
            FaceNames {
                view: Some(&view),
                texts: Some(&texts),
            },
            Lang::En,
        );
        assert_eq!(
            rows[0].label,
            "Each opponent sacrifices a creature token of their choice."
        );
    }

    /// The owner's other half of AM1: an alternative cost is a sentence too.
    ///
    /// "Alternative cost" is a category and every card in the pool that has
    /// one drew exactly that word. Solitude's is a keyword line, which is
    /// the shape a sentence-shaped reader would have missed.
    #[test]
    fn an_alternative_cost_row_says_what_it_charges() {
        let (view, texts, object) = asking_about(
            "dcb9c2a7-ae54-4ddc-a567-640bf4bf4366",
            "en",
            "Flash\n\
             Lifelink\n\
             When this creature enters, exile up to one other target creature. \
             That creature's controller gains life equal to its power.\n\
             Evoke—Exile a white card from your hand.",
        );
        let rows = cast_rows(
            object,
            &[CastModeKind::Normal, CastModeKind::Alternative(0)],
            FaceNames {
                view: Some(&view),
                texts: Some(&texts),
            },
            Lang::En,
        );
        assert_eq!(rows[0].label, "", "the normal way has no line");
        assert_eq!(rows[1].label, "Evoke—Exile a white card from your hand.");
    }

    /// With no card text filed at all — the ordinary offline case — a row
    /// says the card's English Oracle sentence, not the phrase it had.
    #[test]
    fn a_row_with_no_text_filed_says_the_english_oracle() {
        let (view, _, object) = asking_about(
            "dcb9c2a7-ae54-4ddc-a567-640bf4bf4366",
            "en",
            "Evoke—Exile a white card from your hand.",
        );
        let rows = cast_rows(
            object,
            &[CastModeKind::Alternative(0)],
            FaceNames {
                view: Some(&view),
                texts: None,
            },
            Lang::En,
        );
        assert_eq!(rows[0].label, "Evoke—Exile a white card from your hand.");
    }

    /// Where the card cannot be found, an alternative cost has no words:
    /// there is nothing printed to say, and "Alternative cost" was this
    /// client's wording. A mode with no sentence keeps its number, the one
    /// thing that tells it from its neighbour.
    ///
    /// Solitude has no modes, so its `Mode(1)` has no line in the table; and
    /// with no view the card in hand cannot be named, so even its evoke cost
    /// has no sentence to draw.
    #[test]
    fn a_row_with_no_printed_sentence_has_no_words_but_a_mode_its_number() {
        let (view, texts, object) = asking_about(
            "dcb9c2a7-ae54-4ddc-a567-640bf4bf4366",
            "en",
            "Evoke—Exile a white card from your hand.",
        );
        let unprinted = cast_rows(
            object,
            &[CastModeKind::Mode(1)],
            FaceNames {
                view: Some(&view),
                texts: Some(&texts),
            },
            Lang::En,
        );
        assert_eq!(
            unprinted[0].label, "Mode 2",
            "one-based, as a card numbers it"
        );
        let unfound = cast_rows(
            object,
            &[CastModeKind::Alternative(0)],
            FaceNames {
                view: None,
                texts: Some(&texts),
            },
            Lang::En,
        );
        assert_eq!(unfound[0].label, "");
    }

    /// The source of a modal trigger, on the battlefield as the view shows
    /// it: the zone a trigger's question is asked from.
    fn on_the_battlefield(name: &str) -> (baylee_view::PlayerView, ObjectId) {
        let object = crate::registry_printed(55, 0, name);
        let id = object.id;
        let view = baylee_client_core::test_support::ViewBuilder::new(2)
            .with_battlefield(0, [object])
            .build();
        (view, id)
    }

    /// The owner's report 01a0e3ec (#319): Inspirit's combat trigger asked
    /// "Modus 1" or "Modus 2".
    ///
    /// Its choice is printed inside one sentence, so neither mode *is* a
    /// sentence and the table of printed lines rightly knows none; the row
    /// fell straight to a number this client had worded. The shape here is
    /// the report's: the trigger's source on the battlefield, a German
    /// interface, and the German printing's text as Scryfall files it — in
    /// English, as it was served for this card.
    #[test]
    fn inspirit_s_two_modes_say_the_card_s_own_words() {
        let (view, object) = on_the_battlefield("Inspirit, Flagship Vessel");
        let (_, texts, _) = asking_about(
            "554df866-3dbb-4811-8573-6033481591aa",
            "de",
            "Station (Tap another creature you control: Put charge counters equal to its \
             power on this Spacecraft. Station only as a sorcery. It's an artifact creature \
             at 8+.)\n\
             1+ | At the beginning of combat on your turn, put your choice of a +1/+1 counter \
             or two charge counters on up to one other target artifact.\n\
             8+ | Flying\n\
             Other artifacts you control have hexproof and indestructible.",
        );
        for texts in [Some(&texts), None] {
            let rows = cast_rows(
                object,
                &[CastModeKind::Mode(0), CastModeKind::Mode(1)],
                FaceNames {
                    view: Some(&view),
                    texts,
                },
                Lang::De,
            );
            assert_eq!(rows[0].label, "a +1/+1 counter");
            assert_eq!(rows[1].label, "two charge counters");
        }
    }

    /// Every mode of every modal ability in the pool is a row that says
    /// something the card prints, and no two modes of one ability say the
    /// same thing.
    ///
    /// Asked the way the report was asked: a German interface with no text
    /// filed, so a row can only come from the compiled English Oracle —
    /// through the printed sentence a mode is, or the words it is inside a
    /// sentence. The one mode allowed its number is the effect-less mode a
    /// player declines "choose up to one" with, which no card prints.
    #[test]
    fn every_mode_in_the_pool_is_a_row_in_the_card_s_words() {
        let mut modes_seen = 0usize;
        let mut faces_seen = 0usize;
        for def in baylee_cards::all() {
            let modes = baylee_cards::lines::face_modes(def.abilities_for_face(CAST_FACE));
            if modes.is_empty() {
                continue;
            }
            faces_seen += 1;
            let (view, object) = on_the_battlefield(def.name());
            let kinds: Vec<CastModeKind> = (0..modes.len()).map(CastModeKind::Mode).collect();
            let rows = cast_rows(
                object,
                &kinds,
                FaceNames {
                    view: Some(&view),
                    texts: None,
                },
                Lang::De,
            );
            let mut said = Vec::new();
            for (at, (mode, row)) in modes.iter().zip(&rows).enumerate() {
                let number = Phrase::CastModeNumber.fill(Lang::De, &[&(at + 1).to_string()]);
                if mode.effects.is_empty() {
                    assert_eq!(
                        row.label,
                        number,
                        "{} mode {at} declines, and is printed nowhere",
                        def.name()
                    );
                    continue;
                }
                modes_seen += 1;
                assert!(
                    !row.label.is_empty() && row.label != number,
                    "{} mode {at} is drawn as {:?}, not in the card's words",
                    def.name(),
                    row.label
                );
                let oracle = baylee_cards::oracle::face(def.index, CAST_FACE)
                    .expect("a pool card has its Oracle text");
                assert!(
                    oracle.contains(row.label.as_str()),
                    "{} mode {at}: {:?} is not what the card prints",
                    def.name(),
                    row.label
                );
                said.push(row.label.clone());
            }
            let count = said.len();
            said.sort_unstable();
            said.dedup();
            assert_eq!(
                said.len(),
                count,
                "{}: two of its modes are drawn alike",
                def.name()
            );
        }
        // The walk's own floor: a pool that lost its modal cards, or a
        // reading of modes that found none, would pass everything above.
        assert!(faces_seen >= 10, "only {faces_seen} modal faces walked");
        assert!(modes_seen >= 25, "only {modes_seen} modes walked");
    }
}

fn damage_target(
    target: baylee_engine::event::DamageTarget,
    lang: Lang,
    statics: Option<&GameStatic>,
    names: FaceNames<'_>,
) -> String {
    match target {
        baylee_engine::event::DamageTarget::Object(id) => {
            names.of(id, 0).unwrap_or_else(|| format!("#{id}"))
        }
        baylee_engine::event::DamageTarget::Player(id) => seat_name(lang, statics, id),
    }
}

fn damage_origin(
    effect: &baylee_engine::choice::DamageEffectOption,
    lang: Lang,
    names: FaceNames<'_>,
) -> String {
    effect
        .source
        .and_then(|id| names.of(id, 0))
        .or_else(|| {
            effect.ability.and_then(|ability| {
                baylee_cards::by_index(ability.card).map(|card| card.faces[0].name.to_string())
            })
        })
        .unwrap_or_else(|| Phrase::DamageRule.text(lang).to_string())
}

/// Structured provenance and mechanics for an offered damage effect.
pub(crate) fn damage_effect_label(
    effect: &baylee_engine::choice::DamageEffectOption,
    damage: &[baylee_engine::choice::DamagePartView],
    lang: Lang,
    statics: Option<&GameStatic>,
    names: FaceNames<'_>,
) -> String {
    let origin = damage_origin(effect, lang, names);
    baylee_client_core::damage::effect_label(lang, effect, damage, &origin, &|target| {
        damage_target(target, lang, statics, names)
    })
}

/// Damage rows remain bounded by page size, independent of damage amount.
pub(crate) const DAMAGE_PAGE_SIZE: usize = 4;

/// Limits a damage chooser and adds explicit page controls.
pub(crate) fn damage_page(rows: &mut Vec<ChoiceOption>, page: usize, lang: Lang) {
    let total = rows.len();
    let page = page.min(total.saturating_sub(1) / DAMAGE_PAGE_SIZE);
    let start = page * DAMAGE_PAGE_SIZE;
    *rows = rows
        .iter()
        .skip(start)
        .take(DAMAGE_PAGE_SIZE)
        .cloned()
        .collect();
    if page > 0 {
        rows.push(ChoiceOption::text(
            baylee_client_core::targeting::PREVIOUS,
            Phrase::PageBack.text(lang).into(),
        ));
    }
    if start + DAMAGE_PAGE_SIZE < total {
        rows.push(ChoiceOption::text(
            baylee_client_core::targeting::NEXT,
            Phrase::PageMore.text(lang).into(),
        ));
    }
}

#[cfg(test)]
mod decision_id_tests {
    use super::*;
    use baylee_core::ids::PlayerId;
    use baylee_engine::choice::{DamageEffectKind, DamageEffectOption, DamagePartView};
    use baylee_engine::event::DamageTarget;

    #[test]
    fn damage_rows_explain_the_offer_and_keep_bounded_pages() {
        let parts: Vec<_> = (0..9)
            .map(|id| DamagePartView {
                id: id + 10,
                source: ObjectId::new(id + 20, 0),
                recipient: DamageTarget::Player(PlayerId::new(0)),
                amount: u32::MAX,
                is_combat: id == 0,
                preventable: id != 1,
            })
            .collect();
        let effect = DamageEffectOption {
            id: 77,
            source: None,
            ability: None,
            controller: PlayerId::new(0),
            kind: DamageEffectKind::RemoveCounter {
                kind: baylee_cards_dsl::CounterKind::P1P1,
                remaining: 9,
            },
            parts: parts.iter().map(|part| part.id).collect(),
        };
        let prompt = Prompt::AllocatePrevention {
            effect,
            damage: parts,
            total: 9,
            amounts: vec![0; 9],
        };
        for lang in [Lang::En, Lang::De] {
            let mut rows = options(&prompt, lang, None, "", FaceNames::default()).unwrap();
            assert_eq!(rows.len(), 9);
            assert!(
                rows[1]
                    .label
                    .contains(Phrase::DamageUnpreventable.text(lang))
            );
            assert!(rows[0].label.contains("4294967295"));
            damage_page(&mut rows, 1, lang);
            assert_eq!(rows.len(), DAMAGE_PAGE_SIZE + 2);
            assert_eq!(rows[0].index, 4);
            assert_eq!(
                rows.last().unwrap().index,
                baylee_client_core::targeting::NEXT
            );
        }
    }
}

#[cfg(test)]
mod source_choice_tests {
    use super::*;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::ids::{DamageSourceRef, PlayerId, SourceChoiceId};
    use baylee_engine::choice::Pending;

    #[test]
    fn opening_a_source_choice_clears_the_previous_card_preview() {
        let source = DamageSourceRef {
            object: ObjectId::new(9, 0),
            version: 1,
        };
        let pending = Pending::ChooseDamageSource {
            player: PlayerId::new(0),
            choice: SourceChoiceId::new(3),
            options: vec![source],
        };
        let mut duel = crate::Duel {
            view: Some(ViewBuilder::new(2).build()),
            hovered: Some(source.object),
            hovered_at: Some(crate::HoverSpot::Point(bevy::prelude::Vec2::ZERO)),
            ..Default::default()
        };
        duel.receive_choice(pending.clone());
        assert_eq!(duel.hovered, None);
        assert!(duel.hovered_at.is_none());
        // A re-sent identical question does not steal a newly requested preview.
        duel.hovered = Some(source.object);
        duel.receive_choice(pending);
        assert_eq!(duel.hovered, Some(source.object));
    }

    #[test]
    fn historical_rows_stay_distinct_and_never_preview_the_current_object() {
        let mut view = ViewBuilder::new(2)
            .with_battlefield(1, vec![token(9, 1, "New secret identity", 8, 8)])
            .build();
        let old = baylee_view::DamageSourceView {
            source: DamageSourceRef {
                object: ObjectId::new(9, 0),
                version: 2,
            },
            name: "Orcish Artillery".into(),
            card: None,
            rules: None,
            token: None,
            controller: PlayerId::new(1),
            zone: baylee_view::LogZone::Battlefield,
            is_current: false,
            referenced_by: vec![ObjectId::new(10, 0)],
            colors: baylee_core::color::ColorSet::default(),
            types: baylee_core::types::TypeSet::default(),
            power: Some(1),
            toughness: Some(3),
            keywords: 0,
        };
        let older = baylee_view::DamageSourceView {
            source: DamageSourceRef {
                version: 1,
                ..old.source
            },
            ..old.clone()
        };
        let refs = vec![old.source, older.source];
        let mut old = old;
        old.referenced_by = vec![ObjectId::new(10, 0)];
        let mut older = older;
        older.referenced_by = vec![ObjectId::new(11, 0)];
        let mut first = token(10, 1, "Ability", 0, 0);
        first.targets = vec![baylee_view::TargetRef::Player(PlayerId::new(0))];
        let mut second = token(11, 1, "Ability", 0, 0);
        second.targets = vec![baylee_client_core::test_support::target(ObjectId::new(
            12, 0,
        ))];
        view.battlefield.push(token(12, 0, "Grizzly Bears", 2, 2));
        view.stack = vec![first, second];
        baylee_client_core::test_support::project_current_targets(&mut view);
        view.damage_sources = vec![old, older];
        let pending = Pending::ChooseDamageSource {
            player: view.seat,
            choice: SourceChoiceId::new(3),
            options: refs,
        };
        let i = Interaction::new(pending, view.seat);
        let rows = options(
            &i.prompt(),
            Lang::De,
            None,
            "",
            FaceNames {
                view: Some(&view),
                texts: None,
            },
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].label.starts_with("1. Orcish Artillery"));
        assert!(
            rows[0].label.contains("früheres Objekt 2")
                && rows[1].label.contains("früheres Objekt 1")
        );
        assert!(rows.iter().all(|r| !r.label.contains("secret")));
        assert!(rows[0].label.contains(&format!(
            "Stapel → {}",
            seat_name(Lang::De, None, PlayerId::new(0))
        )));
        assert!(rows[1].label.contains("Stapel → Grizzly Bears"));
        assert!(!rows[0].label.contains("Grizzly Bears"));
        let duel = crate::Duel {
            view: Some(view),
            interaction: Some(i),
            ..Default::default()
        };
        assert_eq!(preview_object(&duel, 0), None);
        assert_eq!(preview_object(&duel, 1), None);
    }
}

#[cfg(test)]
mod exact_target_tests {
    use super::*;
    use baylee_client_core::test_support::{ViewBuilder, target_snapshot, token};
    use baylee_core::ids::{DamageSourceRef, PlayerId};
    use baylee_engine::choice::{Pending, TargetPrompt};

    #[test]
    fn historical_target_label_and_confirm_never_use_the_returned_incarnation() {
        let original = token(9, 0, "Original Bears", 2, 2);
        let mut historical = target_snapshot(&original, baylee_view::LogZone::Battlefield);
        historical.is_current = false;
        let source = historical.source;
        let mut view = ViewBuilder::new(2)
            .with_battlefield(0, vec![token(9, 0, "Returned card", 4, 4)])
            .build();
        view.target_objects = vec![historical];
        let label = target_label(
            Lang::De,
            baylee_view::TargetRef::Object(source),
            FaceNames {
                view: Some(&view),
                texts: None,
            },
            None,
        );
        assert!(label.contains("Original Bears") && label.contains("früheres Objekt 1"));
        assert!(!label.contains("Returned"));
        let question = |version| Pending::ChooseTargets {
            player: PlayerId::new(0),
            options: vec![source.object],
            player_options: vec![],
            min: 0,
            max: 1,
            reason: TargetPrompt::Retarget {
                current: baylee_view::TargetRef::Object(DamageSourceRef { version, ..source }),
                index: 0,
                of: 1,
            },
        };
        let old = Interaction::new(question(1), view.seat);
        let fresh = Interaction::new(question(3), view.seat);
        let button = crate::hud::PromptButton {
            action: baylee_client_core::ledge::PromptAction::Confirm,
            decision_id: old.decision_id(),
        };
        assert!(!button.matches_decision_id(fresh.decision_id()));
        view.target_objects.clear();
        assert_eq!(
            target_label(
                Lang::De,
                baylee_view::TargetRef::Object(source),
                FaceNames {
                    view: Some(&view),
                    texts: None
                },
                None
            ),
            Phrase::PreviousTarget.text(Lang::De)
        );
    }
}
