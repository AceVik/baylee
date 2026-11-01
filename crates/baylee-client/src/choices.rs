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
    if let Some(rules) = view.object(object).and_then(|object| object.rules) {
        return Some(rules.card);
    }
    baylee_client_core::decision::known_cards(view)
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
    card_sentence(names.texts, cast_card(names, object)?, line, at)
}

/// [`printed_sentence`] for a card already known: one printed sentence of
/// `card`'s front face, as a row or a log line says it.
fn card_sentence(
    texts: Option<&crate::cardtext::CardTexts>,
    card: CardIndex,
    line: fn(CardIndex, usize, usize) -> Option<baylee_cards::lines::AbilityLine>,
    at: usize,
) -> Option<String> {
    let found = line(card, CAST_FACE, at)?;
    let printed = baylee_view::StackText {
        face: u8::try_from(CAST_FACE).ok()?,
        line: found.line,
        of: found.of,
    };
    let blocks = crate::cardtext::sentence(texts, card, printed)?;
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
    cast_card(names, object)
        .and_then(|card| mode_words(names.texts, card, i))
        .unwrap_or_else(|| Phrase::CastModeNumber.fill(lang, &[&(i + 1).to_string()]))
}

/// What mode `i` of `card` says: its printed sentence, else the card's own
/// words for a choice made inside one sentence; `None` where neither is
/// known. The cast chooser's rows and the game log's mode lines (#23) read
/// it alike.
pub(crate) fn mode_words(
    texts: Option<&crate::cardtext::CardTexts>,
    card: CardIndex,
    i: usize,
) -> Option<String> {
    card_sentence(texts, card, baylee_cards::lines::mode_line, i)
        .or_else(|| baylee_cards::lines::inline_mode_words(card, CAST_FACE, i).map(str::to_string))
}

/// A non-interactive caption before a group of related choice rows.
#[derive(Clone, PartialEq, Debug)]
pub struct ChoiceHeading {
    /// The category and, for a permanent defender, its controller.
    pub label: String,
    /// A Font Awesome mark drawn beside the category.
    pub icon: char,
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
    /// A caption attached to the first row of a visible group.
    pub heading: Option<ChoiceHeading>,
    /// Already chosen: a target picked, the defender or attacker the
    /// declaration aims at, the seat a list is filtered to. Drawn taken, as
    /// a picked row is (the candle's wash and edge), never as words in the
    /// label: `[x] ` read as code beside a card's name.
    pub chosen: bool,
}

impl ChoiceOption {
    /// A row that is only words.
    pub(crate) fn text(index: usize, label: String) -> Self {
        Self {
            index,
            label,
            pip: None,
            cost: None,
            heading: None,
            chosen: false,
        }
    }

    /// The row, chosen or not.
    #[must_use]
    pub(crate) fn chosen(self, chosen: bool) -> Self {
        Self { chosen, ..self }
    }

    /// The row, with the caption of the group it opens.
    #[must_use]
    pub(crate) fn headed(self, heading: ChoiceHeading) -> Self {
        Self {
            heading: Some(heading),
            ..self
        }
    }
}

/// One page of a list shown `size` rows at a time: the rows it holds and
/// where it stands among the pages. The one reading of a page every paged
/// chooser shares (targets, attack, block, damage), so an asked page past
/// the end clamps to the last one everywhere alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Page {
    /// The first row on it.
    pub start: usize,
    /// One past the last row on it.
    pub end: usize,
    /// Which page it is, counted from 0.
    pub index: usize,
    /// How many pages the list has; at least one.
    pub count: usize,
}

impl Page {
    /// Page `asked` of a list of `len` rows, `size` to a page.
    #[must_use]
    pub(crate) fn of(len: usize, asked: usize, size: usize) -> Self {
        let count = len.saturating_sub(1) / size + 1;
        let index = asked.min(count - 1);
        let start = index * size;
        Self {
            start,
            end: (start + size).min(len),
            index,
            count,
        }
    }

    /// Whether a page comes before it.
    #[must_use]
    pub(crate) const fn earlier(self) -> bool {
        self.index > 0
    }

    /// Whether a page comes after it.
    #[must_use]
    pub(crate) const fn later(self) -> bool {
        self.index + 1 < self.count
    }
}

/// The two page arrows under a paged list, each where there is a page to go
/// to, worded alike in every chooser (they were "←", "→" in two and words in
/// two others).
pub(crate) fn page_arrows(rows: &mut Vec<ChoiceOption>, lang: Lang, earlier: bool, later: bool) {
    if earlier {
        rows.push(ChoiceOption::text(
            baylee_client_core::targeting::PREVIOUS,
            Phrase::PageBack.text(lang).into(),
        ));
    }
    if later {
        rows.push(ChoiceOption::text(
            baylee_client_core::targeting::NEXT,
            Phrase::PageMore.text(lang).into(),
        ));
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

/// The creature-type chooser's two lists (the owner, 08.10.2026, item 8):
/// the quick list — the offered types this seat's deck's creatures carry
/// most, each with its count ("Elf · 14") — and the full list under it,
/// alphabetical in the player's language, filtered by what is typed or, with
/// nothing typed, by the letter group `group` (`typechooser::GROUPS`).
///
/// The deck is the seat's own decklist where this client knows it
/// (`Duel::own_deck`, a game it hosts itself), and otherwise the seat's own
/// cards the view has shown it — the hand, its permanents, its graveyard and
/// exile, its command zone: what a networked seat has seen of its deck. Only
/// offered types appear in either list, and every row answers with the
/// offer's own index.
#[must_use]
pub fn type_lists(
    options: &[SubtypeId],
    filter: &str,
    group: usize,
    lang: Lang,
    deck: &[CardIndex],
) -> (Vec<ChoiceOption>, Vec<ChoiceOption>) {
    use baylee_client_core::typechooser::{self, DeckCard};
    let shown = |id: SubtypeId| {
        subtypes::name(id).map(|english| {
            (
                baylee_client_core::type_names::name(english, lang).to_string(),
                english.to_string(),
            )
        })
    };
    let mut counted: Vec<(CardIndex, u32)> = Vec::new();
    for card in deck {
        match counted.iter_mut().find(|(c, _)| c == card) {
            Some((_, n)) => *n += 1,
            None => counted.push((*card, 1)),
        }
    }
    let cards: Vec<DeckCard<'static>> = counted
        .iter()
        .filter_map(|(index, copies)| {
            let def = baylee_cards::by_index(*index)?;
            let face = def.faces.first()?;
            Some(DeckCard {
                copies: *copies,
                creature: face.types.contains(baylee_core::types::TypeSet::CREATURE),
                changeling: def
                    .keywords
                    .contains(baylee_cards_dsl::KeywordSet::CHANGELING)
                    || face
                        .keywords
                        .contains(baylee_cards_dsl::KeywordSet::CHANGELING),
                subtypes: face.subtypes,
            })
        })
        .collect();
    // What is typed narrows the quick list too: the highlight then starts
    // on a type the player is spelling, not on the deck's favourite.
    let needle = filter.trim().to_lowercase();
    let quick = typechooser::quick(options, cards, |id| shown(id).map(|(name, _)| name))
        .into_iter()
        .filter(|row| {
            needle.is_empty()
                || row.name.to_lowercase().starts_with(&needle)
                || subtypes::name(options[row.index])
                    .is_some_and(|english| english.to_lowercase().starts_with(&needle))
        })
        .map(|row| ChoiceOption::text(row.index, format!("{} · {}", row.name, row.count)))
        .collect();
    let typed = !filter.trim().is_empty();
    let full = typechooser::full(options, filter, shown)
        .into_iter()
        .filter(|row| typed || typechooser::group_of(&row.name) == group)
        .take(TYPE_ROWS)
        .map(|row| ChoiceOption::text(row.index, row.name))
        .collect();
    (quick, full)
}

/// The most rows the full type list draws at once: a letter group's types
/// fit, and a filter narrows the rest.
pub const TYPE_ROWS: usize = 60;

/// The seat's own cards this client knows of, for the type chooser's quick
/// list: its decklist where it hosts the game, else what the view has shown
/// it of its own.
#[must_use]
pub fn own_cards(duel: &crate::Duel) -> Vec<CardIndex> {
    if !duel.own_deck.is_empty() {
        return duel.own_deck.clone();
    }
    let Some(view) = duel.view.as_ref() else {
        return Vec::new();
    };
    let me = view.seat;
    let mine = |o: &&baylee_view::PublicObject| o.owner == me;
    let at = me.get() as usize;
    view.hand
        .iter()
        .map(|h| h.card.index)
        .chain(
            view.battlefield
                .iter()
                .filter(mine)
                .chain(view.graveyards.get(at).into_iter().flatten())
                .chain(view.exile.get(at).into_iter().flatten().filter(mine))
                .chain(view.command.get(at).into_iter().flatten())
                .filter(|o| o.token.is_none())
                .filter_map(|o| o.card.map(|c| c.index)),
        )
        .collect()
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
        format!("{name} ({})", Phrase::TargetBeforeZoneChange.text(lang))
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

fn color_options(options: &[baylee_core::mana::ManaColor]) -> Vec<ChoiceOption> {
    options
        .iter()
        .enumerate()
        .map(|(i, c)| ChoiceOption {
            pip: Some(colour_pip(*c)),
            ..ChoiceOption::text(i, String::new())
        })
        .collect()
}

fn mana_options(
    options: &[baylee_engine::choice::ManaAbilityChoice],
    lang: Lang,
    names: FaceNames<'_>,
) -> Vec<ChoiceOption> {
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let name = names
                .of(option.source.object, 0)
                .unwrap_or_else(|| Phrase::SourceUnknown.text(lang).to_string());
            let cost = names
                .view
                .map(|view| crate::abilities::required_mana_label(lang, view, *option))
                .unwrap_or_default();
            ChoiceOption::text(index, format!("{name} — {cost}"))
        })
        .collect()
}

fn text_options(draft: &baylee_client_core::text_choice::Draft, lang: Lang) -> Vec<ChoiceOption> {
    let words = baylee_client_core::text_choice::words(draft.kind, lang);
    (0..10)
        .map(|index| {
            let (phrase, chosen) = if index < 5 {
                (Phrase::TextOldWord, draft.from)
            } else {
                (Phrase::TextNewWord, draft.to)
            };
            let mark = if chosen.map(usize::from) == Some(index % 5) {
                "✓ "
            } else {
                ""
            };
            ChoiceOption::text(
                index,
                format!("{mark}{}: {}", phrase.text(lang), words[index % 5]),
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
        Prompt::TextReplacement { draft } => Some(text_options(draft, lang)),
        Prompt::ChooseManaAbility { options, .. } => Some(mana_options(options, lang, names)),
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
        Prompt::ChooseColor { options } => Some(color_options(options)),
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
                    ..ChoiceOption::text(i, cast_label(desc.kind, lang, *object, names))
                })
                .collect(),
        ),
        Prompt::ChooseSubtype { options } => Some(subtype_rows(options, filter, lang)),
        Prompt::ChooseCardType { options } => Some(
            options
                .iter()
                .enumerate()
                .map(|(index, card_type)| {
                    let label = card_type
                        .words()
                        .map(|word| baylee_client_core::type_names::name(word, lang))
                        .collect::<Vec<_>>()
                        .join(" ");
                    ChoiceOption::text(index, label)
                })
                .collect(),
        ),
        // A pile is its cards: the row names them, in the order the
        // separation gave them, and an empty pile says so rather than
        // drawing a blank row, because it is a pile that may be taken.
        //
        // A Raging River label is the same question about the defender's
        // two piles, and the headline calls them "left" and "right": rows
        // numbered 1 and 2 left the player to work out which was which.
        Prompt::ChoosePile { piles, label } => Some(
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
                    let row = match (label, i) {
                        (Some(_), 0) => Phrase::RiverLeftRow.fill(lang, &[&cards]),
                        (Some(_), 1) => Phrase::RiverRightRow.fill(lang, &[&cards]),
                        _ => Phrase::PileRow.fill(lang, &[&(i + 1).to_string(), &cards]),
                    };
                    ChoiceOption::text(i, row)
                })
                .collect(),
        ),
        Prompt::ChooseCardName {
            types,
            excluded_types,
        } => Some(card_name_rows(filter, *types, *excluded_types)),
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
    if matches!(interaction.prompt(), Prompt::ChooseCardName { .. }) {
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
    types: baylee_core::types::TypeSet,
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
                            types: printed.types,
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
fn card_name_rows(
    filter: &str,
    types: baylee_core::types::TypeSet,
    excluded_types: baylee_core::types::TypeSet,
) -> Vec<ChoiceOption> {
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
            if (!types.is_empty() && !row.types.intersects(types))
                || row.types.intersects(excluded_types)
            {
                return None;
            }
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
        K::Permission(source) => source.and_then(|source| names.of(source, 0)).map_or_else(
            || Phrase::CastPermissionUnlimited.text(lang).to_string(),
            |name| format!("{}: {name}", Phrase::CastPermission.text(lang)),
        ),
        K::Counter(kind) => baylee_client_core::interaction::counter_label(kind, lang),
        K::Colors(colors) => {
            use baylee_core::color::Color;
            [
                (Color::White, Phrase::ColorWhite),
                (Color::Blue, Phrase::ColorBlue),
                (Color::Black, Phrase::ColorBlack),
                (Color::Red, Phrase::ColorRed),
                (Color::Green, Phrase::ColorGreen),
            ]
            .into_iter()
            .filter(|(color, _)| colors.contains(*color))
            .map(|(_, phrase)| phrase.text(lang))
            .collect::<Vec<_>>()
            .join(" + ")
        }
        K::Dungeon(dungeon) => dungeon.name().to_string(),
        K::DungeonRoom { dungeon, room } => dungeon.rooms()[usize::from(room)].to_string(),
        K::MutatePosition(top) => if top {
            Phrase::MutateOnTop
        } else {
            Phrase::MutateUnderneath
        }
        .text(lang)
        .to_string(),
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
        K::Foretell => Phrase::CastForetell.text(lang).to_string(),
        K::BeamMeUp => Phrase::CastBeamMeUp.text(lang).to_string(),
        K::Dash => Phrase::CastDash.text(lang).to_string(),
        K::Mutate => Phrase::CastMutate.text(lang).to_string(),
        K::Escape => Phrase::CastEscape.text(lang).to_string(),
        K::EscapeGranted {
            face,
            modes,
            kicked,
            prototype,
            ..
        } => {
            let mut label = Phrase::CastEscape.text(lang).to_string();
            if prototype {
                append_cast_detail(&mut label, Phrase::CastPrototype.text(lang));
            }
            if kicked {
                append_cast_detail(&mut label, Phrase::CastKicked.text(lang));
            }
            if face != 0
                && let Some(name) = names.of(object, face)
            {
                append_cast_detail(&mut label, &name);
            }
            for mode in 0..8 {
                if modes & (1 << mode) != 0 {
                    append_cast_detail(&mut label, &mode_label(lang, object, names, mode));
                }
            }
            label
        }
    }
}

fn append_cast_detail(label: &mut String, detail: &str) {
    label.push_str(" · ");
    label.push_str(detail);
}

/// The same source and effect explanation on the table and in a zone chooser.
pub(crate) fn target_question(
    interaction: &baylee_client_core::Interaction,
    view: &baylee_view::PlayerView,
    lang: Lang,
    texts: &crate::cardtext::CardTexts,
    statics: Option<&GameStatic>,
) -> Vec<String> {
    let mut lines = base_target_question(interaction, view, lang, texts, statics);
    let subject = baylee_client_core::decision::resource_player(view);
    if subject != view.seat {
        lines.insert(
            0,
            Phrase::DecidingFor.fill(lang, &[&seat_name(lang, statics, subject)]),
        );
    }
    lines
}

fn base_target_question(
    interaction: &baylee_client_core::Interaction,
    view: &baylee_view::PlayerView,
    lang: Lang,
    texts: &crate::cardtext::CardTexts,
    statics: Option<&GameStatic>,
) -> Vec<String> {
    if let baylee_engine::choice::Pending::ChooseCards {
        prompt:
            baylee_engine::choice::ChoicePrompt::CastFaceDown {
                paid, fixed_cost, ..
            },
        ..
    } = interaction.pending()
    {
        let symbols = ["{W}", "{U}", "{B}", "{R}", "{G}", "{C}"];
        let receipt = paid
            .iter()
            .zip(symbols)
            .filter(|(n, _)| **n > 0)
            .map(|(n, symbol)| format!("{n} × {symbol}"))
            .collect::<Vec<_>>()
            .join(", ");
        return vec![Phrase::MaskReceipt.fill(
            lang,
            &[
                if receipt.is_empty() { "0" } else { &receipt },
                &fixed_cost.to_string(),
            ],
        )];
    }
    if let baylee_engine::choice::Pending::ChooseNumber {
        reason: baylee_engine::choice::NumberPrompt::TextReplacement { target, .. },
        ..
    } = interaction.pending()
    {
        let label = target_label(
            lang,
            baylee_view::TargetRef::Object(*target),
            FaceNames {
                view: Some(view),
                texts: Some(texts),
            },
            statics,
        );
        let mut lines = vec![label];
        if let Some(rules) = view.target_object(*target).and_then(|o| o.rules)
            && let Some(text) = crate::cardtext::english(rules.card, usize::from(rules.face))
        {
            lines.push(text.oracle_text);
        }
        return lines;
    }
    if let baylee_engine::choice::Pending::ChooseTargets {
        reason: baylee_engine::choice::TargetPrompt::Retarget { current, index, of },
        ..
    } = interaction.pending()
    {
        let name = target_label(
            lang,
            *current,
            FaceNames {
                view: Some(view),
                texts: Some(texts),
            },
            statics,
        );
        let mut lines = vec![Phrase::RetargetOriginal.fill(
            lang,
            &[&(u64::from(*index) + 1).to_string(), &of.to_string(), &name],
        )];
        if let baylee_view::TargetRef::Object(source) = current
            && view
                .target_object(*source)
                .is_some_and(|object| !object.is_current)
        {
            lines.push(Phrase::TargetReturnedHint.text(lang).to_string());
        }
        return lines;
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
    if let baylee_engine::choice::Pending::ChooseNumber {
        reason: baylee_engine::choice::NumberPrompt::TextReplacement { target, .. },
        ..
    } = interaction.pending()
    {
        return duel
            .view
            .as_ref()?
            .target_object(*target)
            .filter(|o| o.is_current)
            .map(|o| o.source.object);
    }
    if let baylee_engine::choice::Pending::ChooseManaAbility { options, .. } = interaction.pending()
    {
        return options.get(index).map(|o| o.source.object);
    }
    match targeting::options(interaction.pending()).get(index) {
        Some(Target::Object(id)) => return Some(*id),
        Some(Target::Player(_)) => return None,
        None => {}
    }
    if let Some(
        baylee_client_core::interaction::BlockRow::Aim(id)
        | baylee_client_core::interaction::BlockRow::Toggle(id),
    ) = interaction.block_rows().get(index)
    {
        return Some(*id);
    }
    match interaction.attack_options().get(index) {
        Some(
            AttackOption::Toggle(id)
            | AttackOption::Aim(
                baylee_core::ids::Defender::Planeswalker(id)
                | baylee_core::ids::Defender::Battle(id),
            ),
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
            let phrase = match effect.kind {
                baylee_engine::choice::DamageEffectKind::RemoveCounter { .. } => {
                    Phrase::DamageCounterShare
                }
                baylee_engine::choice::DamageEffectKind::RedirectNext { .. } => {
                    Phrase::RedirectionShare
                }
                _ => Phrase::PreventionShare,
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
mod tests;

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

/// Attack defenders and creatures share a page control; the longer group decides
/// how many pages exist, and each group clamps independently to its last page.
pub(crate) fn attack_page_count(
    options: &[baylee_client_core::interaction::AttackOption],
) -> usize {
    use baylee_client_core::{interaction::AttackOption, targeting::PAGE_SIZE};
    let count = |wanted: fn(&AttackOption) -> bool| options.iter().filter(|o| wanted(o)).count();
    let defenders = count(|o| matches!(o, AttackOption::Aim(_)));
    let creatures = count(|o| matches!(o, AttackOption::Toggle(_)));
    (defenders.max(creatures).saturating_sub(1) / PAGE_SIZE) + 1
}

/// Damage rows remain bounded by page size, independent of damage amount.
pub(crate) const DAMAGE_PAGE_SIZE: usize = 4;

/// Limits a damage chooser and adds explicit page controls.
pub(crate) fn damage_page(rows: &mut Vec<ChoiceOption>, page: usize, lang: Lang) {
    let page = Page::of(rows.len(), page, DAMAGE_PAGE_SIZE);
    *rows = rows[page.start..page.end].to_vec();
    page_arrows(rows, lang, page.earlier(), page.later());
}

#[cfg(test)]
mod decision_id_tests;

#[cfg(test)]
mod source_choice_tests;

#[cfg(test)]
mod exact_target_tests;

#[cfg(test)]
mod color_choice_tests;
