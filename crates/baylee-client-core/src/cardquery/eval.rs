//! Asking a query about one card.
//!
//! Split from the language for the reason the language exists: parsing and
//! writing are about *strings a player types*, and this is about a card. The
//! filter dialog needs the first half and never the second, and a surface
//! that grows a new fact needs the second and never the first.

use super::{Colors, Flag, Key, Op, Query, Term, Value, folded};

/// What a query says about one card.
///
/// Three-valued, and the third is the point. See the module header of
/// [`super`]: a surface that cannot answer a term must not match on it and
/// must not match on its negation either, or a search would report a fact it
/// never had the data for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Match {
    /// The card matches.
    Yes,
    /// The card does not match.
    No,
    /// This surface cannot answer.
    Unknown,
}

impl Match {
    /// Whether a row is shown, which only [`Match::Yes`] earns.
    #[must_use]
    pub const fn shown(self) -> bool {
        matches!(self, Self::Yes)
    }

    /// The opposite, with [`Match::Unknown`] surviving.
    const fn not(self) -> Self {
        match self {
            Self::Yes => Self::No,
            Self::No => Self::Yes,
            Self::Unknown => Self::Unknown,
        }
    }

    /// A plain yes-or-no.
    const fn of(yes: bool) -> Self {
        if yes { Self::Yes } else { Self::No }
    }
}

/// Which keys and flags a surface can answer.
///
/// Held beside the facts rather than derived from them, because "this card
/// has no rules text" and "this list has no rules text for any card" are
/// different answers and only the second may narrow a search to nothing. It
/// is also what a filter dialog offers from: a control for a key the surface
/// cannot answer is a control that hides every row.
#[derive(Clone, Copy, Debug)]
pub struct Surface {
    /// The keys with a control and an answer here.
    pub keys: &'static [Key],
    /// The flags `is:` may be asked about here.
    pub flags: &'static [Flag],
}

impl Surface {
    /// Everything a compiled card row can be asked.
    pub const POOL: Self = Self {
        keys: &[
            Key::Loose,
            Key::Name,
            Key::ExactName,
            Key::Oracle,
            Key::Type,
            Key::Color,
            Key::Identity,
            Key::Mana,
            Key::ManaValue,
            Key::Power,
            Key::Toughness,
            Key::Loyalty,
            Key::Is,
        ],
        flags: &[
            Flag::Playable,
            Flag::Partial,
            Flag::Stub,
            Flag::Commander,
            Flag::Basic,
            Flag::Dfc,
        ],
    };

    /// What a row of the zone browser knows about itself.
    ///
    /// A projected object and not a printing, and the line between them is
    /// exactly `baylee_view::PublicObject`'s fields: **characteristics** are
    /// all there — the name, the type line, the colours, the mana value, the
    /// power, the toughness and the loyalty — and every one of them is the
    /// projected value, so an animated Dryad Arbor is a creature here and a
    /// pumped bear is a 3/3. What is missing is the card's *prose* and the
    /// pool's bookkeeping: no rules text, so `o:` cannot be asked; no printed
    /// mana cost, only its value, so `m:` cannot; no colour identity, which is
    /// a deck-building question a view never answers; and no coverage.
    ///
    /// The flags follow the same rule. `is:token` reads `token`,
    /// `is:basic` the supertypes and `is:commander` the `commander` bit — all
    /// three projected. `is:playable` is the deckbuilder's word for what this
    /// build implements, and a card already on the table is past caring.
    pub const ZONE: Self = Self {
        keys: &[
            Key::Loose,
            Key::Name,
            Key::ExactName,
            Key::Type,
            Key::Color,
            Key::ManaValue,
            Key::Power,
            Key::Toughness,
            Key::Loyalty,
            Key::Is,
        ],
        flags: &[Flag::Token, Flag::Basic, Flag::Commander],
    };

    /// Whether this surface answers that key.
    #[must_use]
    pub fn answers(self, key: &Key) -> bool {
        self.keys.contains(key)
    }

    /// Whether this surface answers that flag.
    #[must_use]
    pub fn knows(self, flag: Flag) -> bool {
        self.flags.contains(&flag)
    }
}

/// One card, as much of it as the surface has.
///
/// Every field is what this *card* is; what the **surface** could have said
/// is [`Surface`] beside it. A `None` here therefore means "this card has
/// none" — a land has no power — and not "nobody here knows", which is the
/// distinction that makes `-pow>=4` include the lands rather than exclude
/// them along with everything else.
#[derive(Clone, Copy, Debug, Default)]
pub struct Facts<'a> {
    /// The name as the row draws it, in the player's language.
    pub name: &'a str,
    /// The same card's English name, which a deck list is written in.
    ///
    /// Its own field rather than the first of [`Facts::alt_names`], because
    /// a caller holds the two apart and joining them would mean building a
    /// list per card per keystroke.
    pub english_name: &'a str,
    /// Every other name the card is printed under, in every language.
    pub alt_names: &'a [String],
    /// The printed type line, in the player's language.
    pub type_line: &'a str,
    /// The English card types, so `t:creature` answers on a German client.
    pub kinds: &'a [String],
    /// Rules text in the player's language.
    pub oracle: &'a str,
    /// The card's own colours.
    pub colors: Colors,
    /// Its colour identity (CR 903.4).
    pub identity: Colors,
    /// The printed mana cost, in `{1}{W}` notation.
    pub mana_cost: &'a str,
    /// Its mana value.
    pub mana_value: u32,
    /// Printed power, for a card that prints one.
    pub power: Option<i32>,
    /// Printed toughness, for a card that prints one.
    pub toughness: Option<i32>,
    /// Printed starting loyalty, for a card that prints one.
    pub loyalty: Option<i32>,
    /// The flags that are true of this card. A flag the surface knows and
    /// this card does not carry is simply absent.
    pub flags: &'a [Flag],
}

/// Whether one card answers a query.
#[must_use]
pub fn matches(query: &Query, facts: &Facts<'_>, surface: Surface) -> Match {
    match query {
        Query::Anything => Match::Yes,
        Query::Term(term) => term_matches(term, facts, surface),
        Query::Not(inner) => matches(inner, facts, surface).not(),
        Query::All(parts) => {
            let mut unsure = false;
            for part in parts {
                match matches(part, facts, surface) {
                    Match::No => return Match::No,
                    Match::Unknown => unsure = true,
                    Match::Yes => {}
                }
            }
            if unsure { Match::Unknown } else { Match::Yes }
        }
        Query::Any(parts) => {
            let mut unsure = false;
            for part in parts {
                match matches(part, facts, surface) {
                    Match::Yes => return Match::Yes,
                    Match::Unknown => unsure = true,
                    Match::No => {}
                }
            }
            if unsure { Match::Unknown } else { Match::No }
        }
    }
}

fn term_matches(term: &Term, facts: &Facts<'_>, surface: Surface) -> Match {
    if !surface.answers(&term.key) {
        return Match::Unknown;
    }
    match &term.key {
        Key::Loose => Match::of(loose_matches(&term.value, facts)),
        Key::Name => Match::of(name_matches(&term.value, facts)),
        Key::ExactName => Match::of(exact_name(&term.value, facts)),
        Key::Oracle => Match::of(oracle_matches(&term.value, facts)),
        Key::Type => Match::of(type_matches(&term.value, facts)),
        Key::Color => colours(term, facts.colors, Op::Ge),
        Key::Identity => colours(term, facts.identity, Op::Le),
        Key::Mana => cost(term, facts.mana_cost),
        Key::ManaValue => number(
            term,
            Some(i32::try_from(facts.mana_value).unwrap_or(i32::MAX)),
        ),
        Key::Power => number(term, facts.power),
        Key::Toughness => number(term, facts.toughness),
        Key::Loyalty => number(term, facts.loyalty),
        Key::Is => flag(term, facts, surface),
        Key::Unknown(_) => Match::Unknown,
    }
}

/// Everywhere a player would look for a word they half remember.
///
/// The name, the type line and the rules text, which is what this box has
/// always done — see [`Key::Loose`]. A surface with no rules text simply has
/// none to search; that is not an unanswered question, because the other two
/// halves were answered.
fn loose_matches(value: &Value, facts: &Facts<'_>) -> bool {
    let Value::Word(text) = value else {
        return false;
    };
    let needle = folded(text);
    if needle.is_empty() {
        return true;
    }
    name_matches(value, facts)
        || folded(facts.type_line).contains(&needle)
        || facts
            .kinds
            .iter()
            .any(|kind| folded(kind).contains(&needle))
        || folded(facts.oracle).contains(&needle)
}

/// Every name this card answers to, folded.
fn name_matches(value: &Value, facts: &Facts<'_>) -> bool {
    let Value::Word(text) = value else {
        return false;
    };
    let needle = folded(text);
    if needle.is_empty() {
        return true;
    }
    every_name(facts).any(|name| folded(name).contains(&needle))
}

/// Every name this card answers to, in one walk.
fn every_name<'a>(facts: &'a Facts<'a>) -> impl Iterator<Item = &'a str> {
    [facts.name, facts.english_name]
        .into_iter()
        .chain(facts.alt_names.iter().map(String::as_str))
        .filter(|name| !name.is_empty())
}

fn exact_name(value: &Value, facts: &Facts<'_>) -> bool {
    let Value::Word(text) = value else {
        return false;
    };
    let wanted = folded(text);
    // No card is named nothing, so without this an empty value would be the
    // one word key that answers `No` — and a fresh `!` row in the filter
    // dialog would empty the list before anything had been typed into it.
    wanted.is_empty() || every_name(facts).any(|name| folded(name) == wanted)
}

/// The rules text, with `~` standing for the card's own name.
///
/// An empty needle asks nothing and is answered by every card, which is what
/// the other word keys already did — see the test named for it in
/// `cardquery::tests`. This one read the opposite way round until the filter
/// dialog began opening rows on an empty value: a fresh `o:` row hid the
/// whole list until the first letter was typed into it.
fn oracle_matches(value: &Value, facts: &Facts<'_>) -> bool {
    let Value::Word(text) = value else {
        return false;
    };
    let needle = folded(&text.replace('~', facts.name));
    needle.is_empty() || folded(facts.oracle).contains(&needle)
}

/// The printed type line, and the English types beside it.
///
/// Both, because only one of them is in the player's language and only one
/// of them is the spelling a deck list or a piece of advice is written in.
/// A partial word matches, which is Scryfall's own rule — `t:legend` finds
/// the legendary creatures.
fn type_matches(value: &Value, facts: &Facts<'_>) -> bool {
    let Value::Word(text) = value else {
        return false;
    };
    let needle = folded(text);
    if needle.is_empty() {
        return true;
    }
    folded(facts.type_line).contains(&needle)
        || facts
            .kinds
            .iter()
            .any(|kind| folded(kind).contains(&needle))
}

/// A colour comparison, with the key's own reading of a bare colon.
fn colours(term: &Term, card: Colors, colon: Op) -> Match {
    let op = if term.op == Op::Colon { colon } else { term.op };
    match &term.value {
        Value::Colors(want) => Match::of(match op {
            Op::Ge | Op::Colon => card.holds(*want),
            Op::Le => want.holds(card),
            Op::Eq => card == *want,
            Op::Ne => card != *want,
            Op::Gt => card.holds(*want) && card != *want,
            Op::Lt => want.holds(card) && card != *want,
        }),
        // "More than one colour" is a count with a word for it, so an
        // operator other than the two that can be read as a question about
        // that word is refused rather than guessed at.
        Value::Multicolor => match op {
            Op::Ne => Match::of(card.count() < 2),
            Op::Colon | Op::Eq | Op::Ge => Match::of(card.count() >= 2),
            _ => Match::Unknown,
        },
        Value::ColorCount(want) => {
            Match::of(compare(op, i64::from(card.count()), i64::from(*want)))
        }
        _ => Match::Unknown,
    }
}

/// A number the card may or may not print.
///
/// A card with no printed number of that kind answers **no** rather than
/// unknown: a land is not a creature with an unrecorded power, it has no
/// power, and `-pow>=4` should reach it. The `Unknown` arm is reserved for
/// the surface, one level up.
fn number(term: &Term, card: Option<i32>) -> Match {
    let Some(card) = card else {
        return Match::No;
    };
    match &term.value {
        Value::Number(want) => {
            let op = if term.op == Op::Colon {
                Op::Eq
            } else {
                term.op
            };
            Match::of(compare(op, i64::from(card), i64::from(*want)))
        }
        Value::Parity(even) => Match::of((card % 2 == 0) == *even),
        _ => Match::Unknown,
    }
}

/// A mana cost, compared as a multiset of symbols.
///
/// "A mana cost is greater than another if it includes all the same symbols
/// and more, and it is less if it includes only a subset" — Scryfall's own
/// wording, and the reason a bare colon here means *contains*.
fn cost(term: &Term, printed: &str) -> Match {
    let Value::Cost(want) = &term.value else {
        return Match::Unknown;
    };
    let card = super::cost_symbols(printed);
    let op = if term.op == Op::Colon {
        Op::Ge
    } else {
        term.op
    };
    let card_holds_want = holds_all(&card, want);
    let want_holds_card = holds_all(want, &card);
    Match::of(match op {
        Op::Ge | Op::Colon => card_holds_want,
        Op::Le => want_holds_card,
        Op::Eq => card_holds_want && want_holds_card,
        Op::Ne => !(card_holds_want && want_holds_card),
        Op::Gt => card_holds_want && !want_holds_card,
        Op::Lt => want_holds_card && !card_holds_want,
    })
}

/// Whether `bag` contains every symbol of `wanted`, counting repeats.
fn holds_all(bag: &[String], wanted: &[String]) -> bool {
    let mut left: Vec<&String> = bag.iter().collect();
    for symbol in wanted {
        let Some(at) = left.iter().position(|held| *held == symbol) else {
            return false;
        };
        left.remove(at);
    }
    true
}

/// A yes-or-no property.
fn flag(term: &Term, facts: &Facts<'_>, surface: Surface) -> Match {
    let Value::Flag(flag) = &term.value else {
        return Match::Unknown;
    };
    if !surface.knows(*flag) {
        return Match::Unknown;
    }
    let held = facts.flags.contains(flag);
    Match::of(match term.op {
        Op::Ne => !held,
        _ => held,
    })
}

/// One numeric comparison, with the operator spelled out once.
const fn compare(op: Op, card: i64, want: i64) -> bool {
    match op {
        Op::Colon | Op::Eq => card == want,
        Op::Ne => card != want,
        Op::Lt => card < want,
        Op::Le => card <= want,
        Op::Gt => card > want,
        Op::Ge => card >= want,
    }
}
