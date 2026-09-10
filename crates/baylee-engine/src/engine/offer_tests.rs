//! Every ability the engine offers is one it will accept.
//!
//! `LegalActions` is an *enumeration*, not a hint: the engine lists what a
//! seat may do right now and `apply` validates the answer against that same
//! list, so a pair that appears in `abilities` and is then refused by
//! `apply` is the engine contradicting itself. A client cannot recover from
//! it — the action it was told to offer is the action it is punished for
//! sending — and a player sees a lit permanent that does nothing.
//!
//! One live case of exactly that shape was found by hand: four lands
//! printing "target player", where the offer counted the objects *and* the
//! seats the spec could point at (CR 115.4 makes that one choice over two
//! lists) and the activation counted only the objects. All four were
//! `Coverage::Implemented`, which is the part that matters — that is what
//! the deckbuilder hands a player as playable.
//!
//! The bug found beside it is deliberately *not* this shape, and this module
//! would not catch it: a prepared cast was offered at instant speed and
//! activated at instant speed, the two halves agreeing and both wrong. Two
//! probes that agree can still be wrong about the rules, which is what a
//! behavioural card test is for. This module answers the narrower question
//! of whether they agree at all.
//!
//! The AI-vs-AI soak in `baylee-gamehost` cannot reach this class. Its probe
//! plays a game per card with `HeuristicAgent`, so an ability the heuristic
//! never chooses is never activated: reintroducing the land bug leaves that
//! sweep green. This module takes the agent out of the question — it presses
//! every button the engine draws, whether or not anything would want to.
//!
//! That includes the two offers printed on no card: measured on the pool as
//! it stands, the sweep reaches `choice::GRANTED_ABILITY` on Urza's Saga and
//! `choice::PREPARED_CAST` on Emeritus of Woe. Those are the indices a client
//! cannot look up on a face, so an offer it cannot honour is the one a player
//! has no way to make sense of.

use super::testkit::*;
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};
use baylee_core::types::TypeSet;

/// The seed every probe runs at. One fixed seed rather than the card's own
/// index: what is under test is a question asked of a board, and the board
/// is built by hand, so the only thing a varying seed would move is which
/// seat takes the first turn.
const SEED: u64 = 4_211;

/// The floor under the number of cards that had *any* deed to press,
/// asserted so the sweep cannot quietly become a no-op.
///
/// This is the lesson the gamehost soak taught: a pool-wide test that
/// reaches nothing passes exactly as loudly as one that reaches everything.
/// The number is 446 measured on 2026-09-10, less a tenth, so ordinary pool
/// growth never touches it and a setup change that stops part of the pool
/// from arriving at its main phase fails here instead of silently.
const COVERAGE_FLOOR: usize = 400;

/// One thing an object was offered, in the two lists an offer can live in.
///
/// A mana ability belongs here on the same footing as any other: CR 605.1
/// only says it skips the stack, and `mana_abilities` is an enumeration in
/// exactly the same sense — the client's mana planner sends straight off it.
#[derive(Clone, Copy, Debug)]
enum Deed {
    /// `LegalActions::mana_abilities` named this object.
    Mana,
    /// `LegalActions::abilities` named this object and this index.
    Ability(u32),
}

impl Deed {
    fn action(self, source: ObjectId) -> PlayerAction {
        match self {
            Self::Mana => PlayerAction::ActivateManaAbility { source },
            Self::Ability(ability_index) => PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        }
    }
}

/// Four of each registered basic: twenty mana of every colour, which is
/// enough that affordability never decides whether an ability is offered.
///
/// It has to be enough, because an ability the engine declines to offer for
/// want of mana is an ability this sweep never presses.
fn basics() -> Vec<CardIndex> {
    let mut field = Vec::new();
    for basic in baylee_cards::decks::basic_lands().into_iter().flatten() {
        field.extend(std::iter::repeat_n(basic, 4));
    }
    field
}

/// Whether a card's front face is something that can sit on a battlefield.
///
/// The same five types `decks::probe_preset` asks about, and for the same
/// reason: `starting_battlefield` puts a card there without casting it, so
/// handing it an instant would be building a board no game could reach. A
/// card that fails this is still probed — in hand, where cycling and its
/// relatives live (`ActivationZone::Hand`).
fn is_permanent(def: &baylee_cards_dsl::CardDef) -> bool {
    def.faces.first().is_some_and(|f| {
        f.types.contains(TypeSet::LAND)
            || f.types.contains(TypeSet::CREATURE)
            || f.types.contains(TypeSet::ARTIFACT)
            || f.types.contains(TypeSet::ENCHANTMENT)
            || f.types.contains(TypeSet::PLANESWALKER)
    })
}

/// Answers mulligans, priorities and empty combat declarations until `seat`
/// holds priority in its own first main phase.
///
/// Tolerant where [`reach_main_phase`] panics: this walker is *setup* for
/// hundreds of unrelated cards, and a board that asks a question it does not
/// know how to answer on the way is a card this sweep cannot probe rather
/// than a failure. The distinction is kept honest by [`COVERAGE_FLOOR`],
/// which notices when "cannot probe" starts meaning "most of the pool".
fn walk_to_own_main(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> bool {
    for _ in 0..60 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == seat
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return true;
        }
        let refused = match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).is_err()
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).is_err()
            }
            Pending::ChooseAttackers { player, .. } => engine
                .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                .is_err(),
            Pending::ChooseBlockers { player, .. } => engine
                .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                .is_err(),
            _ => return false,
        };
        if refused {
            return false;
        }
    }
    false
}

/// A board holding `card` in both zones an ability can be activated from —
/// seat 0's hand and, when it is a permanent, seat 0's battlefield — beside
/// twenty basics, walked to seat 0's first main phase with every basic
/// tapped for mana.
///
/// The card's own permanent is deliberately left untapped
/// ([`tap_mana_except`]'s reason): one whose interesting ability costs `{T}`
/// would otherwise have spent itself paying for the mana that pays for it.
///
/// Returns the engine, the card's objects (battlefield first, then hand —
/// a fixed order, so a deed found on one board addresses the same object on
/// the next), and the offer standing at that moment. `None` when the board
/// never got there.
fn probe(card: CardIndex) -> Option<(Engine<RegistryLookup>, Vec<ObjectId>, LegalActions)> {
    let seat = PlayerId::new(0);
    let def = baylee_cards::by_index(card)?;
    let mut field = basics();
    if is_permanent(def) {
        field.insert(0, card);
    }
    let filler = baylee_cards::decks::basic_lands()
        .into_iter()
        .flatten()
        .next()
        .unwrap_or(card);
    let mut engine = Duel::new(SEED, filler)
        .battlefield(0, &field)
        .hand(0, &[card])
        .start();
    if !walk_to_own_main(&mut engine, seat) {
        return None;
    }
    let permanent = on_battlefield(&engine, seat, card);
    let objects: Vec<ObjectId> = permanent
        .into_iter()
        .chain(in_hand(&engine, seat, card))
        .collect();
    if objects.is_empty() {
        return None;
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        return None;
    };
    for source in legal.mana_abilities {
        if Some(source) == permanent {
            continue;
        }
        // A mana ability the engine offered and then refused is the same
        // contradiction this module is about, so the board is abandoned
        // rather than unwrapped through: these are the basics, and a basic
        // that cannot tap for mana is a failure the mana tests own.
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .ok()?;
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        return None;
    };
    Some((engine, objects, *legal))
}

/// Everything `legal` offers to do with `objects`, each named by its
/// *position* in that slice rather than by its `ObjectId`, so the answer
/// survives being carried to a freshly built board.
fn deeds(legal: &LegalActions, objects: &[ObjectId]) -> Vec<(usize, Deed)> {
    let mut found = Vec::new();
    for (slot, object) in objects.iter().enumerate() {
        if legal.mana_abilities.contains(object) {
            found.push((slot, Deed::Mana));
        }
        found.extend(
            legal
                .abilities
                .iter()
                .filter(|(source, _)| source == object)
                .map(|(_, index)| (slot, Deed::Ability(*index))),
        );
    }
    found
}

/// Presses every button the engine draws on one card, one fresh board per
/// button, and names the ones it takes back.
///
/// A board per deed because the first activation moves the game on: a cost
/// is paid, a permanent taps, the stack fills. Asking the second question of
/// that state would be asking a different question.
fn refusals(def: &'static baylee_cards_dsl::CardDef) -> (Vec<String>, bool) {
    let seat = PlayerId::new(0);
    let Some((_, objects, legal)) = probe(def.index) else {
        return (Vec::new(), false);
    };
    let wanted = deeds(&legal, &objects);
    let mut offenders = Vec::new();
    for (slot, deed) in &wanted {
        let Some((mut engine, objects, _)) = probe(def.index) else {
            continue;
        };
        if let Err(err) = engine.apply(seat, deed.action(objects[*slot])) {
            offenders.push(format!(
                "{} — {deed:?} was offered, then: {err:?}",
                def.name()
            ));
        }
    }
    (offenders, !wanted.is_empty())
}

/// The sweep, cut into one chunk per core for the reason the gamehost soak
/// gives: a board is built and walked once per deed and there are hundreds
/// of cards, and a test that is cheap is a test that keeps running on every
/// commit. The chunks share nothing — the registry is a static and every
/// board is built from the same constant seed — so what a chunk finds does
/// not depend on how the pool was divided.
fn sweep() -> (Vec<String>, usize) {
    let cards: Vec<&'static baylee_cards_dsl::CardDef> =
        baylee_cards::all().filter(|d| d.is_implemented()).collect();
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let chunk = cards.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = cards
            .chunks(chunk)
            .map(|slice| {
                scope.spawn(move || {
                    let mut offenders = Vec::new();
                    let mut probed = 0;
                    for def in slice {
                        let (found, reached) = refusals(def);
                        offenders.extend(found);
                        probed += usize::from(reached);
                    }
                    (offenders, probed)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("probe chunk"))
            .fold((Vec::new(), 0), |(mut all, total), (found, probed)| {
                all.extend(found);
                (all, total + probed)
            })
    })
}

#[test]
fn every_offered_ability_can_be_activated() {
    let (offenders, probed) = sweep();
    assert!(
        offenders.is_empty(),
        "the engine offered {} abilit(ies) and then refused them:\n  {}",
        offenders.len(),
        offenders.join("\n  "),
    );
    assert!(
        probed >= COVERAGE_FLOOR,
        "only {probed} implemented cards had any ability offered at all \
         (floor {COVERAGE_FLOOR}) — the sweep is no longer reaching the pool",
    );
}
