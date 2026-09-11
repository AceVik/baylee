//! What a card says in its own printed sentence, checked against the journal.
//!
//! This is the tier the testing plan calls *deliberately dumb*, and the
//! dumbness is the design rather than a shortcut. Every other sweep in this
//! directory reads the [`CardDef`] — the structure the card's file builds —
//! and asks whether the engine honours it. That is a strong check with one
//! blind spot: a card whose `CardDef` says the wrong thing is a card every
//! one of those sweeps agrees with. So this one never looks at the abilities
//! at all. It reads the **English sentence printed on the card**, turns the
//! handful of phrases it recognises into claims, plays the card, and asks
//! the journal whether the claim happened.
//!
//! Two different programs with two different failure modes is the whole
//! point. `landgen` and `forgegen` read text too, and they are allowed to
//! refuse anything they do not understand in full; this one is allowed to
//! understand almost nothing. Ten phrases, no grammar, no parser — a word
//! list and a number beside it.
//!
//! # Where the text comes from
//!
//! Not from the `CardDef`: a face carries no oracle text, and putting it
//! there would be a field every hand-written card had to restate. It comes
//! off the card **file**, from the `//! Oracle:` lines of the header that
//! `xtask codegen` writes and `xtask validate` fails the build over when it
//! drifts from what Scryfall prints. That is the same B→C argument the rest
//! of this tier rests on, one link further out: a *different* program with a
//! *different* failure mode has already pinned those lines to the printing,
//! so reading them here is not circular.
//!
//! Reading the tree at test time is the price. It is one `read_to_string`
//! per file and no new dependency — the alternative was a generated table of
//! 1365 strings linked into `baylee-cards`, which ships into the wasm client
//! that gets its card text from the catalog instead. [`CARD_TREE`] is
//! absolute at compile time, and the walk **recurses**, because `cards/` is
//! a taxonomy and a flat `read_dir` over it returns an empty worklist as an
//! answer.
//!
//! # The two things it asks
//!
//! **A number a card prints is the number the journal shows.** If a card's
//! text says "deals 3 damage" and the press produced *any* damage at all,
//! one of those `DamageDealt` events has to carry a 3. A press that produced
//! no damage is a skip, not a failure — most sentences are conditional, most
//! conditions are not met on a probe board, and a checker that failed over
//! that would report hundreds of cards and be switched off within a week.
//! What survives is the bug actually worth catching at this price: an
//! off-by-one, a sign, a count read from the wrong field.
//!
//! **A card with one button does what its own sentence promises.** Where the
//! engine offered exactly one thing to press, whatever happened belongs to
//! that one thing, so an *unconditional* sentence on that card has to leave
//! its mark: "Destroy target creature." with no permanent in a graveyard
//! afterwards is a finding and not a skip. The single-button restriction is
//! what makes that safe, and the button count is read off [`LegalActions`]
//! rather than off the card.
//!
//! # What it is allowed to read off the card
//!
//! Four fields, and no ability among them: `index` and `oracle_id` to join a
//! file to the pool, `coverage` to leave the stubs out, and the **type
//! line** — through [`super::testkit::is_permanent`] — for the arm above,
//! because a permanent's one button is "cast it" and putting a creature on
//! the battlefield promises nothing about its sentence. Luminarch Ascension
//! is what that costs and what it buys: its one button is the cast, its
//! sentence is a trigger that fires on someone else's turn, and without the
//! type line it was the checker's first false finding. Everything that
//! decides whether a *claim* was kept comes from the printed text on one
//! side and the journal on the other; a rule this module states and then
//! breaks would be worse than one it never stated.
//!
//! # That it is measuring something
//!
//! Two pieces of evidence, and they are different in kind. The **deliberate**
//! one is the mutant every sweep in this directory owes: `state.rs`'s
//! `CardsDrawn { count: drawn.len() as u16 }` written `+ 1` and the pool swept
//! again reports **42 cards**, every one of them a card whose own sentence
//! says "Draw a card" against a journal reading 2 — the horizon lands, Mind
//! Stone, Commander's Sphere. One character in the engine, forty-two printed
//! sentences that stop agreeing with it.
//!
//! The **found** one is why the tier was worth building. On its first real
//! run the presence arm reported Damn: one button, "Destroy target creature"
//! printed on it, and nothing in the journal that is a destruction. That was
//! not a fault in the vocabulary. `ChooseCastMode` was offering a `Normal`
//! option to a card whose only spell ability is `AbilityDef::ModalSpell`, and
//! resolution finds no effects under a cast with no mode — four cards in the
//! pool went hand → stack → graveyard and did nothing at all. Every other
//! sweep here agreed with them, because every other sweep reads the
//! `CardDef`, and the `CardDef` was right.
//!
//! # What it cannot see
//!
//! `scry` is in the vocabulary and has no event to land on: the journal
//! records no rearrangement of the top of a library, so a replay cannot show
//! one either. Its claims are counted and reported, and closing that is a
//! change to [`crate::event::GameEvent`], not to this module.

use super::testkit::{Rest, arena, drive_to_rest, presses};
use super::*;
use crate::event::{GameEvent, JournalEntry};
use crate::zone::Zone;
use baylee_cards_dsl::{CardDef, Coverage};
use baylee_core::ids::CardIndex;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// The printed text, read off the card files
// ---------------------------------------------------------------------------

/// The card tree, absolute and fixed when this file is compiled.
const CARD_TREE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../baylee-cards/src/cards");

/// Every `.rs` under `dir`, recursively and in a fixed order.
///
/// Recursive because `cards/` is a taxonomy — `<type>/<subtype>/mv_N/` —
/// and the one non-recursive walk this repo ever had reported an empty
/// worklist as a successful answer. Sorted because a sweep that reads the
/// pool in the filesystem's order is a sweep whose skip counts move between
/// machines.
fn card_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut found: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    found.sort();
    for path in found {
        if path.is_dir() {
            card_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The value of a `key` that is immediately followed by a quoted string.
fn quoted(text: &str, key: &str) -> Option<String> {
    let start = text.find(key)? + key.len();
    let rest = text.get(start..)?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Every implemented card in the pool, with the text its own file prints.
///
/// Joined to the pool by `oracle_id` rather than by name: a name is printed
/// on a face and two faces have two of them, while the oracle id is the one
/// field a card file and [`baylee_cards::by_oracle_id`] both agree is a key.
fn printed_text() -> Vec<(CardIndex, String)> {
    let mut files = Vec::new();
    card_files(Path::new(CARD_TREE), &mut files);
    let mut out = Vec::new();
    for path in files {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let printed: Vec<&str> = text
            .lines()
            .filter_map(|l| l.strip_prefix("//! Oracle: "))
            .collect();
        if printed.is_empty() {
            continue;
        }
        let Some(oracle_id) = quoted(&text, "oracle_id: \"") else {
            continue;
        };
        let Some(def) = baylee_cards::by_oracle_id(&oracle_id) else {
            continue;
        };
        if !matches!(def.coverage, Coverage::Implemented) {
            continue;
        }
        out.push((def.index, printed.join("\n")));
    }
    out.sort_by_key(|(index, _)| *index);
    out
}

// ---------------------------------------------------------------------------
// Reading a sentence
// ---------------------------------------------------------------------------

/// The printed text with reminder text taken out.
///
/// Reminder text restates a keyword in words this vocabulary recognises —
/// flying's parenthesis says nothing, but trample's and lifelink's do — and
/// a claim read out of a reminder is a claim about a rule rather than about
/// this card.
fn without_reminders(blob: &str) -> String {
    let mut out = String::with_capacity(blob.len());
    let mut depth = 0usize;
    for ch in blob.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

/// One printed sentence, already judged.
struct Line {
    /// The sentence with an activation cost stripped off the front, which is
    /// what the vocabulary reads. "{2}{B}, {T}: Destroy target creature" has
    /// to look like "Destroy target creature" or every activated ability in
    /// the pool is invisible to a word list anchored at the start of a
    /// sentence.
    body: String,
    /// Whether this sentence promises something only under a condition.
    ///
    /// Decided on the sentence *as printed*, cost prefix and all, before the
    /// prefix is cut off: the prefix is part of the printed sentence, so a
    /// condition word standing in it is a condition on the claim behind it.
    conditional: bool,
    /// Whether the paragraph this sentence sits in prints a *replacement*
    /// for its own number (CR 614).
    ///
    /// "Draw two cards. If this spell's additional cost was paid, instead
    /// shuffle your graveyard into your library, draw seven cards" prints
    /// two Draws for one event, and only one of them ever happens. Which is
    /// not decidable here — the additional cost was paid or it was not, and
    /// nothing in a printed sentence says which — so a paragraph carrying
    /// "instead" or "rather than" has its *numbers* put beyond reach while
    /// everything else about it is still read. It is narrower than
    /// [`Line::conditional`], which also switches off the presence arm, and
    /// wider than it needs to be for exactly the reason the condition list
    /// is: one number not compared costs a check, one card falsely reported
    /// costs the sweep.
    superseded: bool,
}

/// The printed text, cut into sentences.
///
/// A **newline is an ability** — that is how Scryfall separates them and how
/// `stubgen` joins the faces — and the split is nested rather than flat
/// because conditionality is a property of the *ability*, not of the
/// sentence. Two of the three ways that bites were found by running this:
///
/// - Sword of Hearth and Home puts "Whenever equipped creature deals combat
///   damage…" and "Put both cards onto the battlefield, then shuffle." on one
///   line. The second sentence read alone is a flat promise to shuffle.
/// - Spirit Water Revival prints "Draw two cards. If this spell's additional
///   cost was paid, **instead** … draw seven cards …". The condition is in
///   the sentence *after* the claim it cancels, so a rule that only carried
///   forwards would have held the card to a two it never promised — and did,
///   on the first run, against a journal reading seven.
///
/// So a condition word anywhere in the paragraph makes every claim in it
/// conditional, in both directions. It is the blunter rule and it is the
/// right one here: a word too many costs a claim that could have been
/// checked, and a word too few costs a card falsely reported, which is what
/// turns a checker off.
///
/// The third is the **trigger's own condition**, which is a claim-shaped
/// clause that promises nothing: "Whenever you draw a card, put a +1/+1
/// counter on target creature" is not a card that draws. Magic templates a
/// trigger as condition-comma-effect, so the head of such a sentence is cut
/// off before the vocabulary reads it. Wizard Class found that one too.
///
/// A sentence that opens with a bullet is a mode (CR 700.2), and a mode is
/// conditional by construction: the driver picks one of them, and the others
/// were never promised.
///
/// Within a paragraph, split on the two marks Magic's templating uses to end
/// a clause and nothing cleverer — there are no abbreviations in oracle
/// text, so a period is always a full stop.
fn lines_of(blob: &str) -> Vec<Line> {
    let mut out = Vec::new();
    for paragraph in without_reminders(blob).split('\n') {
        let ability_is_conditional = says_if(paragraph);
        let ability_is_superseded = says_instead(paragraph);
        for raw in paragraph.split(['.', ';']) {
            let whole = raw.trim();
            if whole.is_empty() {
                continue;
            }
            let conditional = ability_is_conditional || whole.starts_with('\u{2022}');
            let after_cost = whole
                .split_once(": ")
                .map_or(whole, |(_, rest)| rest.trim());
            out.push(Line {
                body: without_the_trigger(after_cost).to_string(),
                conditional,
                superseded: ability_is_superseded,
            });
        }
    }
    out
}

/// A trigger's effect, with the condition that fires it cut off the front.
///
/// CR 603.1 templates a triggered ability as "when/whenever/at *condition*,
/// *effect*", so the comma is the seam and the head is a description of some
/// other event rather than a promise of this one.
fn without_the_trigger(sentence: &str) -> &str {
    let opens = ["when ", "whenever ", "at "];
    let lower = sentence.to_lowercase();
    if !opens.iter().any(|word| lower.starts_with(word)) {
        return sentence;
    }
    sentence
        .split_once(", ")
        .map_or(sentence, |(_, effect)| effect)
}

/// Words that make a sentence a promise about *some other* game state.
///
/// Deliberately over-broad. A word here costs a claim that could have been
/// checked; a word missing here costs a card falsely reported, which is what
/// turns a sweep off. `each` is absent on purpose — "each opponent" is a
/// filter naming who, not a condition on whether.
const CONDITIONS: &[&str] = &[
    " if ",
    " when ",
    " whenever ",
    " unless ",
    " may ",
    " instead ",
    " until ",
    " up to ",
    " for each ",
    " as long as ",
    " rather than ",
    " could ",
    " choose ",
    " chooses ",
    " x ",
    " at the beginning ",
];

/// Whether the sentence carries one of those words.
fn says_if(sentence: &str) -> bool {
    let padded = format!(" {} ", sentence.to_lowercase());
    CONDITIONS.iter().any(|word| padded.contains(word))
}

/// The two words CR 614 templates a replacement with.
///
/// Both are in [`CONDITIONS`] as well, and that is not a duplication: there
/// they say "this may not happen", here they say "if it happens, it happens
/// with a different number". The second is the stronger claim and switches
/// off a different arm.
fn says_instead(sentence: &str) -> bool {
    let padded = format!(" {} ", sentence.to_lowercase());
    [" instead ", " rather than "]
        .iter()
        .any(|word| padded.contains(word))
}

/// The ten phrases this checker knows, and nothing else.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    /// "deals N damage".
    Damage,
    /// "draw(s) N card(s)".
    Draw,
    /// "gain(s) N life".
    GainLife,
    /// "lose(s) N life".
    LoseLife,
    /// "put(s) N ... counter(s) on".
    Counters,
    /// A sentence that begins "Destroy".
    Destroy,
    /// A sentence that begins "Exile".
    Exile,
    /// "create(s) ... token".
    Token,
    /// "shuffle(s)".
    Shuffle,
    /// "scry N" — in the vocabulary, and witnessed by nothing.
    Scry,
}

impl Kind {
    /// Whether the journal has an event that could show this at all.
    const fn witnessable(self) -> bool {
        !matches!(self, Self::Scry)
    }
}

/// One sentence's claim.
struct Claim {
    kind: Kind,
    /// The number the sentence prints, where it prints one.
    amount: Option<u16>,
    /// Whether the sentence that produced it was conditional.
    conditional: bool,
    /// Whether its paragraph prints a replacement for its own number.
    superseded: bool,
    /// The sentence, kept for the message an offender prints.
    sentence: String,
}

/// The number a word names, for the ten a card ever spells out.
fn number(word: &str) -> Option<u16> {
    Some(match word {
        "a" | "an" | "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        _ => word.parse().ok()?,
    })
}

/// The sentence as lowercase words with punctuation trimmed off each.
fn words(sentence: &str) -> Vec<String> {
    sentence
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric() && c != '+' && c != '-')
                .to_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// `words[at]` if it is there.
fn at(words: &[String], index: usize) -> &str {
    words.get(index).map_or("", String::as_str)
}

/// Every claim one sentence makes.
///
/// Three shapes and no grammar: a verb followed by a number followed by a
/// noun, a verb at the head of the sentence, and a word anywhere in it.
fn claims_in(line: &Line) -> Vec<Claim> {
    let words = words(&line.body);
    let mut out = Vec::new();
    let mut claim = |kind: Kind, amount: Option<u16>| {
        out.push(Claim {
            kind,
            amount,
            conditional: line.conditional,
            superseded: line.superseded,
            sentence: line.body.clone(),
        });
    };
    for i in 0..words.len() {
        let verb = at(&words, i);
        let next = number(at(&words, i + 1));
        let noun = at(&words, i + 2);
        match verb {
            "deals" | "deal" if noun == "damage" => {
                if let Some(n) = next {
                    claim(Kind::Damage, Some(n));
                }
            }
            "draws" | "draw" if noun.starts_with("card") => {
                if let Some(n) = next {
                    claim(Kind::Draw, Some(n));
                }
            }
            "gains" | "gain" if noun == "life" => {
                if let Some(n) = next {
                    claim(Kind::GainLife, Some(n));
                }
            }
            "loses" | "lose" if noun == "life" => {
                if let Some(n) = next {
                    claim(Kind::LoseLife, Some(n));
                }
            }
            "puts" | "put" => {
                // "put two +1/+1 counters on" — the noun sits a word or two
                // past the number, because the counter's own name is in
                // between and is spelled several ways.
                let counters = (2..=4).any(|step| at(&words, i + step).starts_with("counter"));
                if let (Some(n), true) = (next, counters) {
                    claim(Kind::Counters, Some(n));
                }
            }
            "scry" => {
                if let Some(n) = next {
                    claim(Kind::Scry, Some(n));
                }
            }
            "creates" | "create" if words.iter().any(|w| w.starts_with("token")) => {
                claim(Kind::Token, None);
            }
            "shuffle" | "shuffles" => claim(Kind::Shuffle, None),
            _ => {}
        }
    }
    match at(&words, 0) {
        "destroy" => claim(Kind::Destroy, None),
        "exile" => claim(Kind::Exile, None),
        _ => {}
    }
    out
}

/// Every claim one card's printed text makes.
fn claims_of(blob: &str) -> Vec<Claim> {
    lines_of(blob).iter().flat_map(claims_in).collect()
}

// ---------------------------------------------------------------------------
// Reading the journal in the same ten words
// ---------------------------------------------------------------------------

/// What the journal says happened, in the vocabulary the claims are written
/// in.
///
/// Three of the ten are read off `ZoneChanged`, and two of those need the
/// *origin* as well as the destination, which is the difference between a
/// check and a tautology. Every instant and sorcery in the pool ends its own
/// resolution in a graveyard, so "something reached a graveyard" is true of
/// every spell ever cast; "something left the battlefield for a graveyard"
/// is what a card means when it says Destroy. A token is the mirror image:
/// it is created where it lands and `resolve::tokens::arrive` names
/// [`Zone::OutsideGame`] as its origin, the one variant that means no zone
/// at all (CR 400.1), so it is the one arrival that cannot be confused with
/// a permanent moving.
///
/// Exile takes any origin, the stack included, and that is not the same
/// mistake: a resolving spell goes to its owner's *graveyard* (CR 608.2m),
/// so a spell that leaves the stack for exile is a spell that said it would.
/// Temporal Mastery prints "Exile Temporal Mastery." and the first run of
/// this sweep reported it, which is how the exception got measured rather
/// than assumed.
fn witnessed(delta: &[JournalEntry]) -> Vec<(Kind, Option<u16>)> {
    let mut out = Vec::new();
    for entry in delta {
        match &entry.event {
            GameEvent::DamageDealt { amount, .. } => out.push((Kind::Damage, Some(*amount))),
            GameEvent::CardsDrawn { count, .. } => out.push((Kind::Draw, Some(*count))),
            GameEvent::LifeChanged { old, new, .. } => {
                let (kind, moved) = match new.cmp(old) {
                    std::cmp::Ordering::Greater => (Kind::GainLife, new - old),
                    std::cmp::Ordering::Less => (Kind::LoseLife, old - new),
                    std::cmp::Ordering::Equal => continue,
                };
                out.push((kind, u16::try_from(moved).ok()));
            }
            GameEvent::CounterChanged { old, new, .. } if new > old => {
                out.push((Kind::Counters, Some(new - old)));
            }
            GameEvent::Shuffled { .. } => out.push((Kind::Shuffle, None)),
            GameEvent::ZoneChanged { from, to, .. } => match (from, to) {
                (Zone::Battlefield, Zone::Graveyard) => out.push((Kind::Destroy, None)),
                (Zone::OutsideGame, Zone::Battlefield) => out.push((Kind::Token, None)),
                (_, Zone::Exile) => out.push((Kind::Exile, None)),
                _ => {}
            },
            _ => {}
        }
    }
    out
}

/// Whether the delta shows anything of `kind` at all.
fn shows(seen: &[(Kind, Option<u16>)], kind: Kind) -> bool {
    seen.iter().any(|(k, _)| *k == kind)
}

/// Whether the delta shows `kind` carrying exactly `amount`.
fn shows_exactly(seen: &[(Kind, Option<u16>)], kind: Kind, amount: u16) -> bool {
    seen.iter().any(|(k, n)| *k == kind && *n == Some(amount))
}

// ---------------------------------------------------------------------------
// Playing the card
// ---------------------------------------------------------------------------

/// What one pass over the pool managed.
#[derive(Default)]
struct Tally {
    /// Cards whose printed text made at least one claim.
    claimed: usize,
    /// Of those, cards the probe board could actually press.
    played: usize,
    /// Presses driven to rest.
    presses: usize,
    /// Numeric claims whose kind the journal showed, so the number was
    /// compared.
    compared: usize,
    /// Claims whose kind the journal never showed — the condition was not
    /// met, the ability was never offered, or the board had nothing to point
    /// at. The large bucket, and the honest one.
    quiet: usize,
    /// Claims in a vocabulary the journal has no event for.
    unwitnessable: usize,
    /// Numeric claims whose own paragraph prints a replacement for them.
    superseded: usize,
    /// Cards the engine offered exactly one thing to press.
    single: usize,
    /// Unconditional claims held to the presence arm on such a card.
    promised: usize,
    /// Presses that never came back to a quiet priority.
    stalled: usize,
    /// Presses the engine refused mid-drive.
    refused: usize,
    /// Questions the driver had no answer for.
    unanswered: Vec<&'static str>,
}

impl Tally {
    fn absorb(&mut self, other: &Self) {
        self.claimed += other.claimed;
        self.played += other.played;
        self.presses += other.presses;
        self.compared += other.compared;
        self.quiet += other.quiet;
        self.unwitnessable += other.unwitnessable;
        self.superseded += other.superseded;
        self.single += other.single;
        self.promised += other.promised;
        self.stalled += other.stalled;
        self.refused += other.refused;
        self.unanswered.extend(other.unanswered.iter().copied());
    }
}

/// What pressing one card's buttons came to.
struct Played {
    /// Everything the journal recorded across every press, in the claims'
    /// own vocabulary.
    seen: Vec<(Kind, Option<u16>)>,
    /// How many things the engine offered to press.
    buttons: usize,
    /// Whether any of them actually resolved off the stack.
    resolved: bool,
    /// What the driving cost in skips.
    tally: Tally,
}

/// Presses every button the card offers and unions what the journal recorded
/// for each.
///
/// A board per press, for [`target_tests`]' reason: the first press moves the
/// game on, so asking the second question of that state would be asking a
/// different question. The union across presses is deliberately generous —
/// a claim printed on one ability and witnessed by another still counts as
/// witnessed, which loses findings and reports nothing false. For a dumb
/// checker that is the right side to err on.
///
/// [`target_tests`]: super::target_tests
fn watch(card: CardIndex) -> Option<Played> {
    let seat = PlayerId::new(0);
    let mut tally = Tally::default();
    let (engine, objects) = arena(card)?;
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        return None;
    };
    let all = presses(&legal, &objects);
    if all.is_empty() {
        return None;
    }
    let mut seen = Vec::new();
    let mut resolved = false;
    for (slot, press) in &all {
        let Some((mut engine, objects)) = arena(card) else {
            continue;
        };
        let Some(object) = objects.get(*slot).copied() else {
            continue;
        };
        let mark = engine.journal().entries().len();
        if engine.apply(seat, press.action(object)).is_err() {
            continue; // `offer_tests`' finding, not this one's.
        }
        tally.presses += 1;
        match drive_to_rest(&mut engine, seat) {
            Rest::Reached | Rest::Over => {}
            Rest::Unanswered(what) => tally.unanswered.push(what),
            Rest::Stalled => tally.stalled += 1,
            Rest::Refused(_) => tally.refused += 1,
        }
        let delta = &engine.journal().entries()[mark..];
        resolved |= delta
            .iter()
            .any(|e| matches!(e.event, GameEvent::StackObjectResolved { .. }));
        seen.extend(witnessed(delta));
    }
    Some(Played {
        seen,
        buttons: all.len(),
        resolved,
        tally,
    })
}

// ---------------------------------------------------------------------------
// The comparison
// ---------------------------------------------------------------------------

/// Every amount the journal recorded for `kind`, for the message.
fn amounts(seen: &[(Kind, Option<u16>)], kind: Kind) -> Vec<u16> {
    seen.iter()
        .filter(|(k, _)| *k == kind)
        .filter_map(|(_, n)| *n)
        .collect()
}

/// The two arms, with the card's sentences on one side and the journal on
/// the other — separate from both, so the counter-test can hand it a pair
/// that does not belong together.
fn disagreements(
    name: &str,
    claims: &[Claim],
    seen: &[(Kind, Option<u16>)],
    one_button: bool,
    tally: &mut Tally,
) -> Vec<String> {
    let mut out = Vec::new();
    for claim in claims {
        if !claim.kind.witnessable() {
            tally.unwitnessable += 1;
            continue;
        }
        if one_button && !claim.conditional {
            tally.promised += 1;
            if !shows(seen, claim.kind) {
                out.push(format!(
                    "{name} offered one button and printed \"{}\", and nothing in the journal \
                     is a {:?}",
                    claim.sentence, claim.kind
                ));
                continue;
            }
        }
        let Some(printed) = claim.amount else {
            continue;
        };
        if claim.superseded {
            tally.superseded += 1;
            continue;
        }
        if shows(seen, claim.kind) {
            tally.compared += 1;
            if !shows_exactly(seen, claim.kind, printed) {
                out.push(format!(
                    "{name} prints \"{}\" and the journal's {:?} reads {:?}",
                    claim.sentence,
                    claim.kind,
                    amounts(seen, claim.kind)
                ));
            }
        } else {
            tally.quiet += 1;
        }
    }
    out
}

/// One card: read its sentences, press its buttons, compare.
fn examine(card: CardIndex, blob: &str) -> (Vec<String>, Tally) {
    let mut tally = Tally::default();
    let claims = claims_of(blob);
    if claims.is_empty() {
        return (Vec::new(), tally);
    }
    tally.claimed += 1;
    let Some(played) = watch(card) else {
        return (Vec::new(), tally);
    };
    tally.absorb(&played.tally);
    tally.played += 1;
    // The presence arm wants a card whose *whole text* is what the press
    // did, and one button is not enough for that. Luminarch Ascension has
    // one button — casting it — and prints an activated ability that needs
    // four quest counters, so the first run of this sweep held the
    // enchantment to a token nothing could have made. A permanent's printed
    // text is a description of what it will be able to do; a spell's is what
    // happens now.
    let one_button = played.buttons == 1
        && played.resolved
        && baylee_cards::by_index(card).is_some_and(|def| !super::testkit::is_permanent(def));
    if one_button {
        tally.single += 1;
    }
    let name = baylee_cards::by_index(card).map_or("an unknown card", CardDef::name);
    let offenders = disagreements(name, &claims, &played.seen, one_button, &mut tally);
    (offenders, tally)
}

/// The sweep, one chunk per core: a board is rebuilt for every press and
/// there are hundreds of them, and a test that stays cheap is a test that
/// keeps running.
fn sweep() -> (Vec<String>, Tally) {
    let pool = printed_text();
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let chunk = pool.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = pool
            .chunks(chunk)
            .map(|slice| {
                scope.spawn(move || {
                    slice.iter().fold(
                        (Vec::new(), Tally::default()),
                        |(mut all, mut total): (Vec<String>, Tally), (card, blob)| {
                            let (found, one) = examine(*card, blob);
                            all.extend(found);
                            total.absorb(&one);
                            (all, total)
                        },
                    )
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("claim chunk"))
            .fold(
                (Vec::new(), Tally::default()),
                |(mut all, mut total), (found, one)| {
                    all.extend(found);
                    total.absorb(&one);
                    (all, total)
                },
            )
    })
}

/// How small the sweep may get before it is no longer measuring anything.
///
/// The guard the whole tier is written around: a checker that finds nothing
/// is indistinguishable from one that checks nothing, and this one has three
/// separate ways to quietly reach zero — the card tree could move and the
/// walk find no files, the vocabulary could stop matching, or the probe
/// board could stop pressing anything.
///
/// Measured 2026-09-11 over the whole pool: 226 cards claimed something, 223
/// of them were played over 318 presses; 75 numbers were compared against the
/// journal, 67 claims were never shown at all, 22 are unwitnessable and 2 are
/// superseded; 26 cards offered one button and were held to 20 promises. The
/// floors are those less a tenth, on the three counts that are the sweep's
/// own reach — the cards it read, the numbers it compared, the sentences it
/// held a card to. The rest are reported and not floored: they are what the
/// probe board could not arrange, and a board that reaches *further* must not
/// fail a test for it.
const CARD_FLOOR: usize = 203;
const COMPARED_FLOOR: usize = 67;
const PROMISED_FLOOR: usize = 18;

/// The tier's own claim: a number a card prints is the number the journal
/// shows, and a card with one button does what its sentence promises.
#[test]
fn what_a_card_says_in_english_is_what_the_journal_records() {
    let (offenders, tally) = sweep();
    println!(
        "{} cards claimed something, {} of them were played over {} presses; {} numbers \
         compared, {} claims the journal never showed, {} unwitnessable, {} superseded; {} \
         cards had one button and were held to {} promises; {} stalled, {} refused, \
         unanswered: {:?}",
        tally.claimed,
        tally.played,
        tally.presses,
        tally.compared,
        tally.quiet,
        tally.unwitnessable,
        tally.superseded,
        tally.single,
        tally.promised,
        tally.stalled,
        tally.refused,
        tally.unanswered,
    );
    assert!(
        offenders.is_empty(),
        "{} cards did something other than what they print: {offenders:#?}",
        offenders.len()
    );
    assert!(
        tally.claimed >= CARD_FLOOR,
        "only {} cards made a claim at all, under the floor of {CARD_FLOOR} — the card tree \
         moved, or the vocabulary stopped matching English",
        tally.claimed
    );
    assert!(
        tally.compared >= COMPARED_FLOOR,
        "only {} printed numbers were compared against the journal, under the floor of \
         {COMPARED_FLOOR}",
        tally.compared
    );
    assert!(
        tally.promised >= PROMISED_FLOOR,
        "only {} unconditional sentences were held to the presence arm, under the floor of \
         {PROMISED_FLOOR}",
        tally.promised
    );
}

/// The counter-test the sweep is worth nothing without: proof that the
/// comparison reads *two* things.
///
/// A checker that asked the journal what happened and then called that the
/// claim would pass every card in the pool while checking none of them. So
/// the same comparison is handed pairs that do not belong together — a
/// three-damage sentence against a journal that recorded two, an
/// unconditional promise against a journal that recorded nothing — and it
/// has to complain each time, and stay quiet when the pair agrees.
#[test]
fn the_comparison_notices_when_the_sentence_and_the_journal_disagree() {
    let three = |conditional| Claim {
        kind: Kind::Damage,
        amount: Some(3),
        conditional,
        superseded: false,
        sentence: "deals 3 damage to any target".to_string(),
    };
    let mut tally = Tally::default();

    let two = [(Kind::Damage, Some(2))];
    assert_eq!(
        disagreements("a card", &[three(false)], &two, false, &mut tally).len(),
        1,
        "a sentence printing 3 passed against a journal that recorded 2"
    );
    assert_eq!(
        disagreements(
            "a card",
            &[three(false)],
            &[(Kind::Damage, Some(3))],
            false,
            &mut tally
        ),
        Vec::<String>::new(),
        "a sentence printing 3 was reported against a journal that recorded 3"
    );

    // The presence arm: the same sentence, nothing in the journal at all.
    assert_eq!(
        disagreements("a card", &[three(false)], &[], true, &mut tally).len(),
        1,
        "an unconditional sentence passed on a one-button card that did nothing"
    );
    assert_eq!(
        disagreements("a card", &[three(true)], &[], true, &mut tally),
        Vec::<String>::new(),
        "a conditional sentence was held to the presence arm"
    );
    assert_eq!(
        disagreements("a card", &[three(false)], &[], false, &mut tally),
        Vec::<String>::new(),
        "a card with several buttons was held to the presence arm"
    );

    // A number its own paragraph replaces is out of reach, and only the
    // number is: the claim still counts as one the sweep looked at.
    let replaced = Claim {
        kind: Kind::Damage,
        amount: Some(3),
        conditional: true,
        superseded: true,
        sentence: "deals 3 damage to any target".to_string(),
    };
    let before = tally.compared;
    assert_eq!(
        disagreements("a card", &[replaced], &two, false, &mut tally),
        Vec::<String>::new(),
        "a replaced number was compared against the number that replaced it"
    );
    assert_eq!(
        (tally.compared, tally.superseded),
        (before, 1),
        "a superseded claim was counted as a comparison"
    );

    // Scry is in the vocabulary and witnessed by nothing, so it must never
    // reach either arm.
    let scry = Claim {
        kind: Kind::Scry,
        amount: Some(2),
        conditional: false,
        superseded: false,
        sentence: "scry 2".to_string(),
    };
    assert_eq!(
        disagreements("a card", &[scry], &[], true, &mut tally),
        Vec::<String>::new(),
        "a claim the journal cannot witness was reported as an offence"
    );
}

/// The other half of the same worry: proof that the *vocabulary* reads
/// English, rather than matching nothing and reporting a clean sweep.
///
/// Every phrase here is a real printed sentence shape, and the four at the
/// end are the readings that were wrong before they were written down: a
/// claim taken out of reminder text is a claim about a keyword rather than
/// about this card, an activation cost hides the verb behind a colon, the
/// condition often sits *in* that cost, and a resolving spell reaching a
/// graveyard is not a card destroying anything.
#[test]
fn the_vocabulary_reads_the_sentences_it_was_written_for() {
    let found = |text: &str| -> Vec<(Kind, Option<u16>, bool)> {
        claims_of(text)
            .iter()
            .map(|c| (c.kind, c.amount, c.conditional))
            .collect()
    };

    assert_eq!(
        found("Lightning Bolt deals 3 damage to any target."),
        vec![(Kind::Damage, Some(3), false)]
    );
    assert_eq!(found("Draw a card."), vec![(Kind::Draw, Some(1), false)]);
    assert_eq!(
        found("You gain 2 life."),
        vec![(Kind::GainLife, Some(2), false)]
    );
    assert_eq!(
        found("Each opponent loses 3 life."),
        vec![(Kind::LoseLife, Some(3), false)],
        "\"each\" names who, not whether"
    );
    assert_eq!(
        found("Put two +1/+1 counters on target creature."),
        vec![(Kind::Counters, Some(2), false)]
    );
    assert_eq!(
        found("Destroy target creature."),
        vec![(Kind::Destroy, None, false)]
    );
    assert_eq!(
        found("Exile target creature."),
        vec![(Kind::Exile, None, false)]
    );
    assert_eq!(
        found("Create a 1/1 white Soldier creature token."),
        vec![(Kind::Token, None, false)]
    );
    assert_eq!(found("Scry 2."), vec![(Kind::Scry, Some(2), false)]);

    // An activation cost hides the verb behind a colon.
    assert_eq!(
        found("{2}{B}, {T}: Destroy target creature."),
        vec![(Kind::Destroy, None, false)]
    );
    // A keyword-ability word in that prefix names a condition the *engine*
    // enforces before it offers the ability at all, so the claim behind it
    // is unconditional by the time this checker can see it pressed.
    assert_eq!(
        found("Metalcraft \u{2014} {T}: You gain 2 life."),
        vec![(Kind::GainLife, Some(2), false)]
    );
    assert_eq!(
        found("Whenever a creature dies, you gain 2 life."),
        vec![(Kind::GainLife, Some(2), true)]
    );
    // A newline is an ability and a period is not, so the back half of a
    // trigger is still the trigger — and the next paragraph is not.
    assert_eq!(
        found(
            "Whenever this creature attacks, search your library for a card. \
             Put it onto the battlefield, then shuffle.\n\
             Draw a card."
        ),
        vec![(Kind::Shuffle, None, true), (Kind::Draw, Some(1), false)]
    );
    // A condition reaches backwards as well: the sentence that cancels a
    // claim comes after it.
    assert_eq!(
        found("Draw two cards. If its cost was paid, instead draw seven cards."),
        vec![(Kind::Draw, Some(2), true), (Kind::Draw, Some(7), true)]
    );
    // And "instead" says more than "maybe": both numbers describe the same
    // event, so neither is the one to hold the journal to.
    assert!(
        claims_of("Draw two cards. If its cost was paid, instead draw seven cards.")
            .iter()
            .all(|c| c.superseded),
        "a replacement's own numbers are still compared"
    );
    assert!(
        claims_of("Draw two cards.").iter().all(|c| !c.superseded),
        "a paragraph with no replacement in it had its number put out of reach"
    );
    // A trigger's condition is claim-shaped and promises nothing.
    assert_eq!(
        found("Whenever you draw a card, put a +1/+1 counter on target creature."),
        vec![(Kind::Counters, Some(1), true)],
        "the \"draw a card\" that fires the trigger is not a card that draws"
    );
    // A mode is only ever one of several, so it promises nothing on its own.
    assert_eq!(
        found("\u{2022} Draw a card."),
        vec![(Kind::Draw, Some(1), true)]
    );
    // Reminder text restates a rule, not this card.
    assert_eq!(
        found("Lifelink (Damage dealt by this creature also causes you to gain that much life.)"),
        Vec::new()
    );
    // A number the card does not print is not a claim.
    assert_eq!(
        found("This creature deals damage equal to its power."),
        Vec::new()
    );
}
