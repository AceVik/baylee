//! An ability that targets touches the target it was pointed at and nothing
//! else, swept over every implemented card in the pool.
//!
//! The fourth CR property sweep, and the **dynamic half of a bug the DSL
//! lints already look for statically**. `lints::target_reuse` reads a card's
//! data and finds an ability that picks a target with a filter and then hands
//! that *same filter* to an effect that sweeps everything matching it — the
//! mistake is invisible in the card's printed text and obvious in the
//! filter's name. This module asks the engine the same question by playing
//! the card: put a legal target on the table beside a **bystander** the same
//! filter would also accept, point the ability at one of them, and look at
//! the other.
//!
//! Two programs, two failure modes, one bug. The lint cannot see an effect
//! that reads a filter the *engine* widened; this sweep cannot see an ability
//! nothing on its board could reach. Neither is redundant.
//!
//! # The direction this module does not look
//!
//! It asks whether an ability reached **more** than its target, and there is
//! an opposite question it is blind to by construction: whether it reached
//! all of them. `Effect::AddCounter` read `this_object`, which answers
//! `res.targets.first()`, so Rishkar, Peema Renegade's "put a +1/+1 counter
//! on each of up to two target creatures" put one counter on the first
//! target and nothing on the second — and every bystander in this sweep came
//! back untouched, which is what it is built to report as clean. The card's
//! own test in `card_tests::creatures` is what caught it, and
//! `Effect::UntapTarget` had been walking `res.targets` since it was
//! written. Two cards in the pool name more than one target today, so a
//! sweep for the shape would carry a population of two; the note is here
//! because the blind spot is the module's, not the pool's.
//!
//! # Why the assertion is read off the object and not the journal
//!
//! The plan called for this one to be asserted from the journal, and the
//! journal is the wrong instrument for the *most likely* shape of the bug: a
//! continuous effect records nothing at all. `PumpFilter`, `SetPTFilter` and
//! `CreateContinuousEffect` are three of the seven filters
//! `lints::swept_filters` watches, and a `+1/+1` handed to every creature
//! instead of to the target produces exactly zero journal entries. So a
//! bystander is measured the way a player measures it — what zone it is in,
//! whether it is tapped, what counters and damage it carries, who controls
//! it, and what its **projected** characteristics are — before the ability is
//! pointed anywhere and again once the stack is empty. A pump that reached it
//! shows up in `power`; a destroy that reached it shows up in `zone`.
//!
//! # What is a bystander, and what is not
//!
//! The set starts as the options `Pending::ChooseTargets` itself enumerated —
//! the engine's own list of what this ability could legally have been pointed
//! at — and three rules take objects back out of it:
//!
//! - **Anything the driver picked**, at that prompt or any later one. A
//!   sacrifice cost asks `ChooseCards`, a second target asks `ChooseTargets`
//!   again, the legend rule asks `LegendChoice`; every answer this module
//!   gives is subtracted, so it never has to reason about which prompt could
//!   have reached a bystander.
//! - **The source's own objects**, both of them. Costs are paid after targets
//!   are chosen, so a `{T}` cost lands inside the measured window and is not
//!   an offence.
//! - **Anything else printed from the same card.** The board holds the card
//!   in hand *and* on the battlefield, so casting the hand copy of a legend
//!   kills the other one under CR 704.5j, which is the rules working.
//!
//! A card that prints a `Static` has the **characteristics** half of the
//! comparison switched off and the rest kept. A static ability does not
//! target (CR 115.1 is about spells and abilities that use the word), so a
//! creature whose anthem lifts every Elf on the table is not failing at
//! target locality — it is doing what it says, and its filter is
//! `lints::target_reuse`'s question rather than this one's. The switch is
//! counted, not silent.
//!
//! # What it cannot reach
//!
//! Only what the board can offer a second of. The bystanders here are
//! Llanowar Elves, Sol Rings and basic lands, so "target creature", "target
//! artifact", "target permanent", "target land" and "target creature card in
//! a graveyard" all find one, and "target enchantment", "target spell" and
//! "target player" find none — the last of those because a seat is not an
//! object and has no state this module knows how to photograph.
//!
//! The board earned its Sol Rings by measurement. Of the 152 implemented
//! cards in the pool that print a `TargetSpec`, 56 were watched here when the
//! bystanders were Elves and basics; the artifacts took that to **60 cards
//! and 377 bystanders** (from 336), which is the whole of what one more
//! permanent type is worth. An enchantment would be the next one and cannot
//! be had: every implemented enchantment in the pool prints a static, a
//! trigger or a replacement effect, so putting one here would put a rule on
//! the board — [`testkit::quiet_artifact`](super::testkit::quiet_artifact)
//! says so where the next person will look.
//!
//! What it *does* hold was measured rather than argued. Replacing the chosen
//! target list with the whole enumeration in `actions.rs` — one line, and
//! exactly the shape of the bug `lints::target_reuse` looks for — fails this
//! sweep with 120 named offenders across the pool.
//!
//! Those 120 are also the evidence for the paragraph above, which would
//! otherwise be an argument about what a journal *would* have missed. They
//! name five fields: 33 `zone`, 26 `status`, 16 `keywords`, 8 `power`, 4
//! `toughness`. The last three are 28 lines a journal-based sweep could not
//! have printed at all — Rogue's Passage handing its keyword to every
//! creature on the table, Skarrg pumping all of them +1/+0 and granting
//! trample — because a continuous effect is registered, not recorded.

use super::testkit::{
    Duel, RegistryLookup, Rest, SEED, answer_one, arena, at_rest, basic_forest, keep_mulligans,
    on_battlefield, presses, quiet_creature,
};
use super::*;
use crate::object::{Characteristics, Status};
use crate::zone::Zone;
use baylee_cards_dsl::{AbilityDef, CardDef, CounterKind};
use baylee_core::ids::{ObjectId, SubtypeId};

/// Everything a player's answer named, so it can be taken out of the
/// bystander set.
///
/// The match is exhaustive for the reason `answer_one`'s is: a new
/// `PlayerAction` that can name an object and is read here through a `_` arm
/// would put that object back among the bystanders and report it as an
/// offence the next time something legitimately picked it.
fn named(action: &PlayerAction) -> Vec<ObjectId> {
    match action {
        PlayerAction::ChooseTargets { objects, .. }
        | PlayerAction::ChooseObjects { objects }
        | PlayerAction::OrderObjects { objects } => objects.clone(),
        PlayerAction::PlayLand { card }
        | PlayerAction::CastSpell { card }
        | PlayerAction::Suspend { card } => vec![*card],
        PlayerAction::ActivateManaAbility { source }
        | PlayerAction::ActivateAbility { source, .. } => vec![*source],
        PlayerAction::DeclareAttackers { attackers } => {
            attackers.iter().map(|(id, _)| *id).collect()
        }
        PlayerAction::DeclareBlockers { blockers } => blockers
            .iter()
            .flat_map(|(blocker, attacker)| [*blocker, *attacker])
            .collect(),
        PlayerAction::MulliganKeep
        | PlayerAction::MulliganTake
        | PlayerAction::PassPriority
        | PlayerAction::ChooseColor(_)
        | PlayerAction::ChooseSubtype(_)
        | PlayerAction::ChooseMode(_)
        | PlayerAction::ChooseNumber(_)
        | PlayerAction::ChoosePlayer(_)
        | PlayerAction::YesNo(_)
        | PlayerAction::Concede
        | PlayerAction::OfferDraw
        // A standing answer is addressed to an `AbilityRef` — a card and an
        // index — and a priority hold to a step. Neither names an object on
        // any board, so neither can pick a bystander.
        | PlayerAction::SetPriorityHold(_)
        | PlayerAction::SetStandingAnswer { .. }
        | PlayerAction::SetAbilityYield { .. }
        | PlayerAction::SetAbilityPolicy { .. } => Vec::new(),
    }
}

/// Everything about one object a player could notice, at one moment.
///
/// `what` is the **projection** and not the base: a continuous effect that
/// reached this object is a difference here and nowhere else, which is the
/// whole reason the journal was not the instrument. `None` for an object the
/// arena no longer has, which is itself a difference.
struct Look {
    zone: Zone,
    zone_owner: Option<PlayerId>,
    controller: PlayerId,
    status: Status,
    counters: Vec<(CounterKind, u16)>,
    damage: u16,
    deathtouched: bool,
    attached_to: Option<ObjectId>,
    face_index: u8,
    what: Characteristics,
}

fn look(engine: &Engine<RegistryLookup>, id: ObjectId) -> Option<Look> {
    let object = engine.state().object(id)?;
    Some(Look {
        zone: object.zone,
        zone_owner: object.zone_owner,
        controller: object.controller,
        status: object.status,
        counters: object.counters.iter().collect(),
        damage: object.damage,
        deathtouched: object.deathtouched,
        attached_to: object.attached_to,
        face_index: object.face_index,
        what: object.characteristics().clone(),
    })
}

/// Every way `after` differs from `before`, each rendered as a sentence.
///
/// One arm per field and each of them renders its two values *after*
/// comparing them, never instead of comparing them: `SubtypeSet`'s `Debug` is
/// `SubtypeSet(..)`, so a comparison of what this function prints would pass
/// an Elf against a Forest. That mistake was made once already, in
/// [`printed_tests`](super::printed_tests), and caught by its counter-test.
#[allow(clippy::too_many_lines)] // one flat arm per field a player can see
fn differences(before: &Look, after: &Look, shield_characteristics: bool) -> Vec<String> {
    fn note(out: &mut Vec<String>, field: &str, was: &str, now: &str) {
        out.push(format!("its {field} was {was} and is {now}"));
    }
    fn status(bits: Status) -> String {
        let named = [
            (Status::TAPPED, "tapped"),
            (Status::FACE_DOWN, "face down"),
            (Status::PHASED_OUT, "phased out"),
            (Status::FLIPPED, "flipped"),
        ];
        let on: Vec<&str> = named
            .iter()
            .filter(|(bit, _)| bits.contains(*bit))
            .map(|(_, word)| *word)
            .collect();
        if on.is_empty() {
            "nothing".to_string()
        } else {
            on.join(" and ")
        }
    }
    fn subtypes(set: baylee_core::types::SubtypeSet) -> String {
        let mut ids: Vec<u16> = set.iter().map(SubtypeId::get).collect();
        ids.sort_unstable();
        format!("{ids:?}")
    }
    let mut out = Vec::new();
    if before.zone != after.zone {
        note(
            &mut out,
            "zone",
            &format!("{:?}", before.zone),
            &format!("{:?}", after.zone),
        );
    }
    if before.zone_owner != after.zone_owner {
        note(
            &mut out,
            "zone owner",
            &format!("{:?}", before.zone_owner),
            &format!("{:?}", after.zone_owner),
        );
    }
    if before.controller != after.controller {
        note(
            &mut out,
            "controller",
            &format!("{:?}", before.controller),
            &format!("{:?}", after.controller),
        );
    }
    if before.status != after.status {
        note(
            &mut out,
            "status",
            &status(before.status),
            &status(after.status),
        );
    }
    if before.counters != after.counters {
        note(
            &mut out,
            "counters",
            &format!("{:?}", before.counters),
            &format!("{:?}", after.counters),
        );
    }
    if before.damage != after.damage {
        note(
            &mut out,
            "damage",
            &before.damage.to_string(),
            &after.damage.to_string(),
        );
    }
    if before.deathtouched != after.deathtouched {
        note(
            &mut out,
            "deathtouched flag",
            &before.deathtouched.to_string(),
            &after.deathtouched.to_string(),
        );
    }
    if before.attached_to != after.attached_to {
        note(
            &mut out,
            "attachment",
            &format!("{:?}", before.attached_to),
            &format!("{:?}", after.attached_to),
        );
    }
    if before.face_index != after.face_index {
        note(
            &mut out,
            "face",
            &before.face_index.to_string(),
            &after.face_index.to_string(),
        );
    }
    if shield_characteristics {
        return out;
    }
    let (was, now) = (&before.what, &after.what);
    if was.name != now.name {
        note(
            &mut out,
            "name",
            &format!("{:?}", was.name),
            &format!("{:?}", now.name),
        );
    }
    if was.colors != now.colors {
        note(
            &mut out,
            "colors",
            &format!("{:?}", was.colors),
            &format!("{:?}", now.colors),
        );
    }
    if was.types != now.types {
        note(
            &mut out,
            "types",
            &format!("{:?}", was.types),
            &format!("{:?}", now.types),
        );
    }
    if was.supertypes != now.supertypes {
        note(
            &mut out,
            "supertypes",
            &format!("{:?}", was.supertypes),
            &format!("{:?}", now.supertypes),
        );
    }
    if was.subtypes != now.subtypes {
        note(
            &mut out,
            "subtypes",
            &subtypes(was.subtypes),
            &subtypes(now.subtypes),
        );
    }
    if was.keywords != now.keywords {
        note(
            &mut out,
            "keywords",
            &format!("{:?}", was.keywords),
            &format!("{:?}", now.keywords),
        );
    }
    if was.power != now.power {
        note(
            &mut out,
            "power",
            &format!("{:?}", was.power),
            &format!("{:?}", now.power),
        );
    }
    if was.toughness != now.toughness {
        note(
            &mut out,
            "toughness",
            &format!("{:?}", was.toughness),
            &format!("{:?}", now.toughness),
        );
    }
    if was.loyalty != now.loyalty {
        note(
            &mut out,
            "loyalty",
            &format!("{:?}", was.loyalty),
            &format!("{:?}", now.loyalty),
        );
    }
    out
}

/// The card a bystanding object was printed from, for the report.
///
/// A bystander is a Llanowar Elves, a Sol Ring or a basic today, so this is
/// one word — but a wider board would make "a bystander" ambiguous, and a
/// report nobody can act on is the failure mode this whole tier is written
/// against.
fn whose(engine: &Engine<RegistryLookup>, id: ObjectId) -> &'static str {
    engine
        .state()
        .object(id)
        .and_then(|o| o.card)
        .and_then(|c| baylee_cards::by_index(c.index))
        .map_or("object", CardDef::name)
}

/// What one press was watched doing.
struct Watch {
    /// The options `ChooseTargets` enumerated at the first prompt that
    /// offered more than one object.
    offered: Vec<ObjectId>,
    /// Each of those objects as it stood the instant before the choice.
    before: Vec<Look>,
}

/// Answers the game to a rest, intercepting the one question under test.
///
/// [`answer_one`] answers everything else, which is the whole point of the
/// split: a new `Pending` variant is decided once, in `testkit`, rather than
/// twice in two sweeps that both thought they were exhaustive.
///
/// The first `ChooseTargets` that offers two or more objects is where the
/// watch is set, and it is answered with the **fewest** targets the prompt
/// allows (one, when "up to" makes zero legal), so as many options as
/// possible are left standing to be measured.
fn drive_watching(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
) -> (Option<Watch>, Vec<ObjectId>, bool, Rest) {
    let mut watch: Option<Watch> = None;
    let mut chosen: Vec<ObjectId> = Vec::new();
    let mut prompted = false;
    for _ in 0..200 {
        if at_rest(engine, seat) {
            return (watch, chosen, prompted, Rest::Reached);
        }
        let (player, action) = match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                min,
                max,
                ..
            } => {
                prompted = true;
                if watch.is_none() && options.len() >= 2 {
                    watch = Some(Watch {
                        before: options.iter().filter_map(|id| look(engine, *id)).collect(),
                        offered: options.clone(),
                    });
                }
                let want = usize::from(min).max(1).min(usize::from(max));
                let objects: Vec<_> = options.into_iter().take(want).collect();
                let players = player_options
                    .into_iter()
                    .take(want.saturating_sub(objects.len()))
                    .collect();
                (player, PlayerAction::ChooseTargets { objects, players })
            }
            _ => match answer_one(engine) {
                Ok(pair) => pair,
                Err(rest) => return (watch, chosen, prompted, rest),
            },
        };
        chosen.extend(named(&action));
        if let Err(err) = engine.apply(player, action.clone()) {
            return (
                watch,
                chosen,
                prompted,
                Rest::Refused(format!(
                    "{action:?} came out of the question, then: {err:?}"
                )),
            );
        }
    }
    (watch, chosen, prompted, Rest::Stalled)
}

/// Cards allowed to reach a bystander, each with the sentence that makes it
/// legal.
///
/// A named list rather than a predicate, for the reason `offer_tests` keeps
/// one: an effect that legitimately touches more than its target is a fact
/// about *that card's* rules text, and a rule inferred from it would excuse
/// the next card by accident. Empty until a run finds one.
const REACHES_FURTHER: &[(&str, &str)] = &[];

/// How small the sweep may get before it has stopped measuring anything.
///
/// The count with the floor on it is *cards*, not presses, because a card is
/// the unit the pool grows in. Measured 2026-09-11 with the Sol Rings on the
/// board: 572 cards probed, 816 presses, 60 of them watched on 60 cards over
/// 377 bystanders. The floors are those less a tenth, so ordinary pool growth
/// never touches them and a setup change that stops part of the pool reaching
/// its main phase fails here instead of shrinking the sweep in silence.
const CARD_FLOOR: usize = 54;
/// The floor under bystanders actually compared, on the same measurement.
const BYSTANDER_FLOOR: usize = 339;

/// What one chunk of the pool managed.
#[derive(Default)]
struct Tally {
    /// Cards whose board came up at all.
    cards: usize,
    /// Presses made.
    presses: usize,
    /// Presses where a `ChooseTargets` offered two or more objects.
    watched: usize,
    /// Cards with at least one such press: the count the floor is on,
    /// because a card is the unit the pool grows in.
    cards_watched: usize,
    /// Bystanders compared, before and after.
    bystanders: usize,
    /// Presses that never asked for a target.
    silent: usize,
    /// Presses that asked for a target and never offered a second option.
    thin: usize,
    /// Presses where every option offered was one the driver had picked or
    /// one of the card's own objects, so none was left to measure.
    excluded: usize,
    /// Presses whose characteristics half was shielded by the card's own
    /// `Static`.
    shielded: usize,
    /// Questions [`answer_one`] has no arm for, by name.
    unanswered: Vec<&'static str>,
    /// Presses still asking when the step cap ran out.
    stalled: usize,
    /// Answers the engine enumerated and then refused. Reported, not
    /// asserted: that contradiction is `offer_tests`' finding.
    refused: usize,
}

impl Tally {
    fn absorb(&mut self, other: &mut Self) {
        self.cards += other.cards;
        self.presses += other.presses;
        self.watched += other.watched;
        self.cards_watched += other.cards_watched;
        self.bystanders += other.bystanders;
        self.silent += other.silent;
        self.thin += other.thin;
        self.excluded += other.excluded;
        self.shielded += other.shielded;
        self.unanswered.append(&mut other.unanswered);
        self.stalled += other.stalled;
        self.refused += other.refused;
    }
}

/// Whether the card prints a continuous effect of its own.
fn prints_a_static(def: &CardDef) -> bool {
    (0..def.faces.len()).any(|face| {
        def.abilities_for_face(face)
            .iter()
            .any(|a| matches!(a, AbilityDef::Static(_)))
    })
}

/// Presses every button on one card that could make it target, and names the
/// bystanders it reached.
fn strays(def: &'static CardDef) -> (Vec<String>, Tally) {
    let seat = PlayerId::new(0);
    let mut offenders = Vec::new();
    let mut tally = Tally::default();
    let Some((engine, objects)) = arena(def.index) else {
        return (offenders, tally);
    };
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        return (offenders, tally);
    };
    let all = presses(&legal, &objects);
    if all.is_empty() {
        return (offenders, tally);
    }
    tally.cards += 1;
    let mut ever_watched = false;
    let shield = prints_a_static(def);
    let excused = REACHES_FURTHER.iter().any(|(name, _)| *name == def.name());
    for (slot, press) in all {
        // A board per press: the first one moves the game on, and asking the
        // second question of that state would be asking a different question.
        let Some((mut engine, objects)) = arena(def.index) else {
            continue;
        };
        let Some(object) = objects.get(slot).copied() else {
            continue;
        };
        if engine.apply(seat, press.action(object)).is_err() {
            continue; // `offer_tests`' finding, not this one's.
        }
        tally.presses += 1;
        let (watch, chosen, prompted, rest) = drive_watching(&mut engine, seat);
        match rest {
            Rest::Reached | Rest::Over => {}
            Rest::Unanswered(what) => tally.unanswered.push(what),
            Rest::Stalled => tally.stalled += 1,
            Rest::Refused(_) => tally.refused += 1,
        }
        let Some(watch) = watch else {
            if prompted {
                tally.thin += 1;
            } else {
                tally.silent += 1;
            }
            continue;
        };
        if shield {
            tally.shielded += 1;
        }
        let mut measured = 0;
        for (id, before) in watch.offered.iter().zip(&watch.before) {
            if chosen.contains(id) || objects.contains(id) {
                continue;
            }
            // Another object printed from the same card: CR 704.5j is
            // entitled to remove it and that is not this sweep's finding.
            if engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == def.index))
            {
                continue;
            }
            measured += 1;
            let Some(after) = look(&engine, *id) else {
                offenders.push(format!(
                    "{}: {press:?} left a bystander the arena no longer has",
                    def.name()
                ));
                continue;
            };
            for said in differences(before, &after, shield) {
                if excused {
                    continue;
                }
                offenders.push(format!(
                    "{}: {press:?} — the bystanding {} {said}",
                    def.name(),
                    whose(&engine, *id)
                ));
            }
        }
        tally.bystanders += measured;
        if measured == 0 {
            tally.excluded += 1;
        } else {
            tally.watched += 1;
            ever_watched = true;
        }
    }
    if ever_watched {
        tally.cards_watched += 1;
    }
    (offenders, tally)
}

/// The sweep, cut into one chunk per core the way the other pool sweeps are.
fn sweep() -> (Vec<String>, Tally) {
    let cards: Vec<&'static CardDef> = baylee_cards::all().filter(|d| d.is_implemented()).collect();
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let chunk = cards.len().div_ceil(threads).max(1);
    let mut offenders = Vec::new();
    let mut tally = Tally::default();
    std::thread::scope(|scope| {
        let handles: Vec<_> = cards
            .chunks(chunk)
            .map(|slice| {
                scope.spawn(move || {
                    let mut found = Vec::new();
                    let mut counted = Tally::default();
                    for def in slice {
                        let (mut theirs, mut count) = strays(def);
                        found.append(&mut theirs);
                        counted.absorb(&mut count);
                    }
                    (found, counted)
                })
            })
            .collect();
        for handle in handles {
            let (mut found, mut counted) = handle.join().expect("a chunk does not panic");
            offenders.append(&mut found);
            tally.absorb(&mut counted);
        }
    });
    (offenders, tally)
}

#[test]
fn an_ability_that_targets_reaches_the_target_and_nothing_else() {
    let (offenders, tally) = sweep();
    println!(
        "{} cards probed, {} presses, {} of them watched on {} cards over {} bystanders; \
         {} never targeted, {} never offered a second option, {} had every option \
         excluded, {} shielded by a Static; \
         {} stalled, {} refused, unanswered: {:?}",
        tally.cards,
        tally.presses,
        tally.watched,
        tally.cards_watched,
        tally.bystanders,
        tally.silent,
        tally.thin,
        tally.excluded,
        tally.shielded,
        tally.stalled,
        tally.refused,
        tally.unanswered
    );
    assert!(
        offenders.is_empty(),
        "{} abilities reached past their target: {offenders:#?}",
        offenders.len()
    );
    assert!(
        tally.cards_watched >= CARD_FLOOR,
        "only {} cards had a press worth watching, under the floor of {CARD_FLOOR}",
        tally.cards_watched
    );
    assert!(
        tally.bystanders >= BYSTANDER_FLOOR,
        "only {} bystanders were compared, under the floor of {BYSTANDER_FLOOR}",
        tally.bystanders
    );
}

/// The counter-test the sweep is worth nothing without: proof that
/// [`differences`] reads **two** objects.
///
/// A comparison that rendered before it compared would agree with itself
/// however wrong it was — the mistake `printed_tests` made once, with
/// `SubtypeSet`'s opaque `Debug`, and caught here before this module ever ran
/// over the pool. So the same `Look` against itself has to say nothing, and a
/// Forest against an Elf has to name every field they genuinely differ in.
#[test]
fn the_comparison_notices_when_the_two_objects_are_not_the_same_card() {
    let seat = PlayerId::new(0);
    let elf = quiet_creature();
    let forest = basic_forest();
    let mut engine = Duel::new(SEED, forest)
        .battlefield(0, &[elf, forest])
        .start();
    keep_mulligans(&mut engine);
    let elf_id = on_battlefield(&engine, seat, elf).expect("the Elf is on the table");
    let forest_id = on_battlefield(&engine, seat, forest).expect("the Forest is on the table");
    let elf_look = look(&engine, elf_id).expect("the Elf can be photographed");
    let forest_look = look(&engine, forest_id).expect("the Forest can be photographed");

    assert!(
        differences(&elf_look, &elf_look, false).is_empty(),
        "an object differs from itself: {:?}",
        differences(&elf_look, &elf_look, false)
    );
    let said = differences(&elf_look, &forest_look, false);
    for field in ["name", "types", "subtypes", "power", "toughness"] {
        assert!(
            said.iter().any(|line| line.contains(field)),
            "an Elf and a Forest agree about {field}: {said:?}"
        );
    }
    // Both are untapped, undamaged and uncountered on the same battlefield,
    // so the shielded reading — everything a static cannot move — has
    // nothing to say about them, and that is the half the shield is allowed
    // to silence.
    assert!(
        differences(&elf_look, &forest_look, true).is_empty(),
        "the shield let a characteristic through: {:?}",
        differences(&elf_look, &forest_look, true)
    );
}

/// And the half the shield must **not** silence.
///
/// Shielding a card that prints a `Static` is this sweep's largest
/// concession, so what survives it has to be nailed down: an object that was
/// tapped, destroyed, moved, given a counter or taken over is a difference
/// whatever the card's own text says. A shield that swallowed those would
/// leave every anthem creature in the pool unmeasured while still counting
/// itself as swept.
#[test]
fn the_shield_hides_characteristics_and_nothing_else() {
    let seat = PlayerId::new(0);
    let elf = quiet_creature();
    let forest = basic_forest();
    let mut engine = Duel::new(SEED, forest).battlefield(0, &[elf]).start();
    keep_mulligans(&mut engine);
    let elf_id = on_battlefield(&engine, seat, elf).expect("the Elf is on the table");
    let before = look(&engine, elf_id).expect("the Elf can be photographed");
    engine
        .dev_state_mut(seat)
        .expect("a dev seat")
        .object_mut(elf_id)
        .expect("the Elf is in the arena")
        .status
        .insert(Status::TAPPED);
    let after = look(&engine, elf_id).expect("the Elf is still there");

    let said = differences(&before, &after, true);
    assert_eq!(
        said.len(),
        1,
        "tapping an object should be exactly one difference: {said:?}"
    );
    assert!(
        said[0].contains("status") && said[0].contains("tapped"),
        "the tap was reported as {said:?}"
    );
}
