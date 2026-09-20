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
/// The number is 464 measured on 2026-09-10, less a tenth, so ordinary pool
/// growth never touches it and a setup change that stops part of the pool
/// from arriving at its main phase fails here instead of silently.
///
/// It is now every implemented card the engine draws a button for on this
/// board: no board is abandoned on the way in and none is abandoned tapping
/// its mana. What is left out is *not* simply "cards with no activated
/// ability". An ability `can_afford` refuses on *this* board is a card that
/// arrives, is asked, and answers with nothing — counted here exactly as a
/// vanilla creature is. That used to include every sacrifice and discard cost
/// on every board, because nothing could ask which card; since `cost_wizard`
/// it is an ordinary board reading, and the six cards that carried such a
/// cost are pressed here like any other.
const COVERAGE_FLOOR: usize = 417;

/// The floor under the deeds driven all the way back to a quiet priority.
///
/// Pressing a button and watching the engine refuse the very answer it just
/// enumerated is the point of the drive, and a drive that stops at the first
/// question it cannot answer checks nothing past the press. This is the
/// second half of [`COVERAGE_FLOOR`]: that one says the sweep still reaches
/// the pool, this one says it still gets through it. Every one of the 658
/// deeds rested when this was measured on 2026-09-10, with nothing stalled
/// and no question the driver could not answer; the floor is that less a
/// tenth.
const RESTED_FLOOR: usize = 592;

/// A board holding `card` in both zones an ability can be activated from —
/// seat 0's hand and, when it is a permanent, seat 0's battlefield — beside
/// twenty basics and two [`quiet_creature`]s, one of which is put into the
/// graveyard before anything is asked, walked to seat 0's first main phase
/// with every basic tapped for mana.
///
/// The card's own permanent is deliberately left untapped
/// ([`tap_mana_except`]'s reason): one whose interesting ability costs `{T}`
/// would otherwise have spent itself paying for the mana that pays for it.
/// The surviving Elf is left untapped for a different reason — it is there
/// to be a creature, not to make mana.
///
/// Twenty basics answer only "can this be afforded". The pair of Elves
/// answers "is there anything for it to reach": an Equipment has nothing to
/// equip on a board of lands, and a land that returns a creature card from a
/// graveyard has nothing to return from an empty one. Neither was offered
/// anything to press before, so neither was probed at all.
///
/// Returns the engine, the card's objects (battlefield first, then hand —
/// a fixed order, so a deed found on one board addresses the same object on
/// the next), and the offer standing at that moment. `None` when the board
/// never got there.
fn probe(card: CardIndex) -> Option<(Engine<RegistryLookup>, Vec<ObjectId>, LegalActions)> {
    let seat = PlayerId::new(0);
    let def = baylee_cards::by_index(card)?;
    let elf = quiet_creature();
    let mut field = basics();
    if is_permanent(def) {
        field.insert(0, card);
    }
    field.extend([elf, elf]);
    let filler = baylee_cards::decks::basic_lands()
        .into_iter()
        .flatten()
        .next()
        .unwrap_or(card);
    let mut engine = Duel::new(SEED, filler)
        .battlefield(0, &field)
        .hand(0, &[card])
        .start();
    // One of the two Elves into the graveyard, before anything is asked. It
    // is the *last* of them and not the first because the card under test may
    // itself be Llanowar Elves and `on_battlefield` answers with the first
    // match: the probed permanent has to be the one that survives.
    let doomed = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .rfind(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == elf))
        })?;
    sba::destroy(engine.dev_state_mut(seat)?, doomed);
    if !walk_to_own_main(&mut engine, seat) {
        return None;
    }
    let survivor = on_battlefield(&engine, seat, elf);
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
        if Some(source) == permanent || Some(source) == survivor {
            continue;
        }
        // A mana ability the engine offered and then refused is the same
        // contradiction this module is about, and the board is abandoned
        // rather than unwrapped through — which is tolerance in the same
        // sense as `walk_to_own_main`'s, and cost exactly as much. It used
        // to say that these are the basics and a basic that cannot tap for
        // mana is the mana tests' problem. They are not always the basics:
        // Chromatic Lantern and Great Divide Guide grant every land a second
        // mana ability, the same land came back in this list twice, and the
        // second press was refused. Both cards left the sweep here, silently,
        // and it took reading the delta to notice. See
        // [`a_permanent_that_makes_mana_two_ways_is_offered_once`].
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .ok()?;
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        return None;
    };
    Some((engine, objects, *legal))
}

/// What one chunk of the pool managed, so the sweep can say how far it got
/// rather than only whether it found anything.
#[derive(Default)]
struct Tally {
    /// Cards with at least one deed to press.
    cards: usize,
    /// Deeds pressed.
    deeds: usize,
    /// Deeds driven all the way back to a quiet priority.
    rested: usize,
    /// Questions [`drive_to_rest`] has no arm for, by name rather than by
    /// count: a report that says *which* one stopped it is the difference
    /// between a taxonomy finding and a number nobody can act on.
    unanswered: Vec<&'static str>,
    /// Deeds still asking questions when the step cap ran out.
    stalled: usize,
}

impl Tally {
    fn absorb(&mut self, other: &Self) {
        self.cards += other.cards;
        self.deeds += other.deeds;
        self.rested += other.rested;
        self.unanswered.extend(other.unanswered.iter().copied());
        self.stalled += other.stalled;
    }
}

/// Presses every button the engine draws on one card, drives each press to a
/// rest, and names what the engine took back.
///
/// A board per deed because the first activation moves the game on: a cost is
/// paid, a permanent taps, the stack fills. Asking the second question of that
/// state would be asking a different question.
///
/// A panic inside the rules is named here rather than taking the whole sweep
/// down, for `probe_chunk`'s reason: a run should say *every* card it found,
/// not stop at the first.
fn refusals(def: &'static baylee_cards_dsl::CardDef) -> (Vec<String>, Tally) {
    let seat = PlayerId::new(0);
    let mut tally = Tally::default();
    let Some((_, objects, legal)) = probe(def.index) else {
        return (Vec::new(), tally);
    };
    let wanted = deeds(&legal, &objects);
    if wanted.is_empty() {
        return (Vec::new(), tally);
    }
    tally.cards = 1;
    tally.deeds = wanted.len();
    let mut offenders = Vec::new();
    for (slot, deed) in &wanted {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let (mut engine, objects, _) = probe(def.index)?;
            if let Err(err) = engine.apply(seat, deed.action(objects[*slot])) {
                return Some(Rest::Refused(format!(
                    "{deed:?} was offered, then: {err:?}"
                )));
            }
            Some(drive_to_rest(&mut engine, seat))
        }));
        match outcome {
            Err(_) => offenders.push(format!("{} — {deed:?} panicked the engine", def.name())),
            Ok(None) => {}
            Ok(Some(Rest::Refused(what))) => offenders.push(format!("{} — {what}", def.name())),
            Ok(Some(Rest::Reached | Rest::Over)) => tally.rested += 1,
            Ok(Some(Rest::Unanswered(what))) => tally.unanswered.push(what),
            Ok(Some(Rest::Stalled)) => tally.stalled += 1,
        }
    }
    (offenders, tally)
}

/// The sweep, cut into one chunk per core for the reason the gamehost soak
/// gives: a board is built and walked once per deed and there are hundreds of
/// cards, and a test that is cheap is a test that keeps running on every
/// commit. The chunks share nothing — the registry is a static and every board
/// is built from the same constant seed — so what a chunk finds does not
/// depend on how the pool was divided.
fn sweep() -> (Vec<String>, Tally) {
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
                    let mut tally = Tally::default();
                    for def in slice {
                        let (found, one) = refusals(def);
                        offenders.extend(found);
                        tally.absorb(&one);
                    }
                    (offenders, tally)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("probe chunk"))
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

#[test]
fn every_offered_ability_can_be_activated() {
    let (offenders, tally) = sweep();
    assert!(
        offenders.is_empty(),
        "the engine offered {} thing(s) and then took them back:\n  {}\n\
         (of {} deeds on {} cards: {} rested, {} unanswered, {} stalled)",
        offenders.len(),
        offenders.join("\n  "),
        tally.deeds,
        tally.cards,
        tally.rested,
        tally.unanswered.len(),
        tally.stalled,
    );
    assert!(
        tally.cards >= COVERAGE_FLOOR,
        "only {} implemented cards had any ability offered at all (floor \
         {COVERAGE_FLOOR}) — the sweep is no longer reaching the pool",
        tally.cards,
    );
    assert!(
        tally.rested >= RESTED_FLOOR,
        "only {} of {} deeds resolved all the way back to a quiet priority \
         (floor {RESTED_FLOOR}): {} were still asking when the step cap ran \
         out, and these questions have no arm in the driver: {:?}",
        tally.rested,
        tally.deeds,
        tally.stalled,
        {
            let mut kinds = tally.unanswered.clone();
            kinds.sort_unstable();
            kinds.dedup();
            kinds
        },
    );
}

/// Recurring Nightmare: "Sacrifice a creature, Return this enchantment to
/// its owner's hand: Return target creature card from your graveyard to the
/// battlefield."
///
/// The card this module used to prove the engine was *silent* about. A cost
/// that says "a creature" names none, and `can_afford` refused the part
/// rather than asking, so the only ability on the card was never offered and
/// the guard standing here asserted that absence. `cost_wizard` closed it,
/// and the claim turns over: the ability is offered, pressed, its target
/// chosen, its cost answered, and the creature card comes back.
///
/// Two enumerations, each read with both halves struck, because reading the
/// card cannot tell either list from a wider one. The target list carries
/// the creature card in *my* graveyard and neither the opponent's Elf nor
/// the land of mine lying beside it. The cost list carries the creature I
/// control and neither the opponent's — CR 701.21a lets a player sacrifice
/// only a permanent they control — nor my land, nor the enchantment asking
/// the question, which is mine and is no creature.
///
/// The `prompt` is asserted with them. `ChoicePrompt::CostSacrifice` inside
/// a `Pending::ChooseCards` is the whole of what tells a client that this is
/// a cost being paid and not a search, and choosing what to sacrifice is not
/// targeting (CR 115.1) — the same list arriving as `ChooseTargets` would
/// give a hexproof creature a say in whether its own controller may eat it.
/// A test that read the options alone would pass against
/// `ChoicePrompt::Generic`.
///
/// The last claim is about where the bounce lives, and it is the one a
/// reader of the card file cannot settle. Everything before the colon is the
/// cost (CR 602.1a), so `ReturnSelfToHand` belongs where the card file puts
/// it, and the difference is observable: once the cost is paid the
/// enchantment is in its owner's hand while its ability is still on the
/// stack (CR 113.7a). Written as an effect instead, it would be sitting on
/// the battlefield at that moment.
#[allow(clippy::too_many_lines)] // two enumerations, and a cost paid between them
#[test]
fn an_ability_whose_cost_asks_a_question_is_offered_and_paid() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let nightmare = card_index("a6708b11-1bcd-4208-a967-fe91f2e3313c");
    let elves = card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3");
    let forest = card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
    let mut engine = Duel::new(SEED, forest)
        .battlefield(0, &[nightmare, forest, elves, elves])
        .battlefield(1, &[elves, elves])
        .start();
    // One Elf per seat into its own graveyard, and one card off my library
    // on top of mine. The ability wants a creature to sacrifice *and* a
    // creature card to bring back; the two extra cards are the ones each
    // list has to reject — the one in the wrong graveyard, and the one that
    // is no creature.
    for seat in [p0, p1] {
        let doomed = on_battlefield(&engine, seat, elves).expect("an Elf on the battlefield");
        sba::destroy(
            engine
                .dev_state_mut(seat)
                .expect("the test kit grants dev commands"),
            doomed,
        );
    }
    seed_graveyard(&mut engine, p0, 1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the board never reached seat 0's own main phase"
    );

    let source = on_battlefield(&engine, p0, nightmare).expect("Recurring Nightmare in play");
    let survivor = on_battlefield(&engine, p0, elves).expect("a creature left to sacrifice");
    let land = on_battlefield(&engine, p0, forest).expect("a land of my own");
    let theirs = on_battlefield(&engine, p1, elves).expect("a creature of theirs");
    let mine_dead = in_graveyard(&engine, p0, elves).expect("a creature card in my graveyard");
    let theirs_dead = in_graveyard(&engine, p1, elves).expect("a creature card in theirs");
    let my_land_card = in_graveyard(&engine, p0, forest).expect("a land card in mine");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(source, 0)),
        "a creature to sacrifice and a creature card to return: the card's \
         only ability is offered: {:?}",
        legal.abilities
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .expect("what the engine offers, the engine accepts");

    // CR 601.2c, and the printed word is *your*.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the reanimation asks which card: {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine_dead),
        "the creature card in my own graveyard is the target: {options:?}"
    );
    assert!(
        !options.contains(&theirs_dead),
        "the one lying in theirs is not: {options:?}"
    );
    assert!(
        !options.contains(&my_land_card),
        "nor is a card of mine that is no creature: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![mine_dead],
                players: Vec::new(),
            },
        )
        .expect("the Elf in my graveyard is one of the legal targets");

    // CR 601.2h, and the one step of it the player takes.
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the cost asks which creature to sacrifice: {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "the variant is what says a cost is being paid rather than a \
         graveyard searched"
    );
    assert_eq!((min, max), (1, 1), "one creature, and exactly one");
    assert!(
        options.contains(&survivor),
        "the creature I control is what pays the cost: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "a creature I do not control is not, whatever the filter says: \
         CR 701.21a lets a player sacrifice only a permanent they control: \
         {options:?}"
    );
    assert!(
        !options.contains(&land),
        "nor is a permanent of mine that is no creature: {options:?}"
    );
    assert!(
        !options.contains(&source),
        "and least of all the enchantment asking the question: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![survivor],
            },
        )
        .expect("the Elf is one of the answers the engine listed");

    // Both halves of the cost are paid before the ability is on the stack,
    // and both are visible from here.
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&survivor),
        "the creature the player named is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, nightmare).is_none()
            && in_hand(&engine, p0, nightmare).is_some(),
        "and the enchantment is in its owner's hand: the bounce is printed \
         before the colon, so it is a cost and is paid now"
    );
    assert!(
        !stack_is_empty(&engine),
        "the ability is on the stack all the same, its source in a hand \
         (CR 113.7a)"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&mine_dead),
        "the targeted creature card is back on the battlefield"
    );
    assert_eq!(
        engine.state().object(mine_dead).map(|o| o.controller),
        Some(p0),
        "under the player whose ability returned it"
    );
    assert_eq!(
        in_graveyard(&engine, p1, elves),
        Some(theirs_dead),
        "and the other graveyard was never touched"
    );
}

// The two guards this file used to carry here, and why they are gone.
//
// Until `cost_wizard` existed, `can_afford` refused `CostPart::Sacrifice`
// and `CostPart::Discard` on every board, so an ability carrying one was
// never offered and the sweep above counted the card exactly as it counts a
// vanilla creature — it arrives, is asked, and answers with nothing. Two
// static pool-wide guards stood here for that reason: one requiring every
// implemented card with such a cost to say `Coverage::Partial`, and one
// saying the same of the tokens `cards/` does not hold, with the Blood
// token excused by name because no card in the pool makes one.
//
// Both were about a limitation rather than a rule, and the limitation is
// gone: the sweep presses those abilities now like any other, and Blood's
// `{1}, {T}, Discard a card, Sacrifice this artifact: Draw a card` is live
// the day a card creates one. What is left of their argument is the test
// below, which is about a cost the engine would accept and then *skip* —
// the failure that ends in no error at all.

/// The failure that survived the pair above, and the one no test of an
/// action could fail on.
///
/// Those two were about a cost the engine *refused*. This is about a cost the
/// engine forgets: `ExileFromHand` and `PayLifeX` are paid in the casting
/// wizard, so `can_afford` accepts them and `pay_cost` walks past them —
/// right for Force of Will's pitch and Toxic Deluge's X, and on an activated
/// ability a pitch cost that exiles nothing. The activation succeeds, which
/// is why nothing else can catch it: the only evidence would be the card
/// still in a hand that was supposed to have paid it.
///
/// Both doors in one test, because neither has anything to excuse. No card
/// and no token prints such a cost today, so there is no list of excused
/// tokens to keep honest — the message says which one arrived and that is
/// enough.
///
/// Every card, and deliberately not only the implemented ones — which is the
/// one place this parts company with the pair above. `Coverage::Partial` is a
/// real answer there: a refused cost is never offered, so the ability is
/// inert, which is exactly what `Partial` promises about the clause it names.
/// It is no answer at all here. A partial card *plays* — the deckbuilder
/// offers it, marked — so a `Partial` card carrying `ExileFromHand` on an
/// activation ships a pitch cost that exiles nothing into somebody's deck,
/// and the label would be documenting the cheat rather than preventing it.
/// The ability comes off the card with a `// NOT SUPPORTED:` line instead.
#[test]
fn nothing_in_the_pool_carries_an_activated_cost_the_engine_would_skip() {
    let mut offenders = Vec::new();
    let mut check = |who: &str, ability: &AbilityDef| {
        let (AbilityDef::Activated { cost, .. } | AbilityDef::ActivatedConditional { cost, .. }) =
            ability
        else {
            return;
        };
        for part in cost.parts {
            if crate::engine::abilities::paid_by_the_casting_wizard(part) {
                offenders.push(format!("{who} — {part:?}"));
            }
        }
    };
    for def in baylee_cards::all() {
        for face in 0..def.faces.len() {
            for ability in def.abilities_for_face(face) {
                check(def.name(), ability);
            }
        }
    }
    for token in baylee_cards::tokens::ALL {
        for ability in token.abilities {
            check(token.name, ability);
        }
    }
    assert!(
        offenders.is_empty(),
        "an activated ability carries a cost part `pay_cost` skips, so the \
         engine offers the ability and then never charges for it — and \
         `Coverage::Partial` does not excuse it, because a partial card is \
         dealt into a deck and played: {offenders:?}"
    );
}

/// Every counter-X cost in the pool sits on a **mana** ability, and the one
/// line of `start_activation` that says otherwise has no card behind it.
///
/// `RemoveCounterSelfX` announces a number (CR 601.2b), and the number then
/// has to reach the effects. There are two doors for that and they are not
/// the same code: a mana ability resolves immediately (CR 605.3b) and gets
/// its X straight into the `Resolution`, while an ability that uses the
/// stack has to have it written onto the stacked object as `x_value`. The
/// second door is written and is played by nothing — all seventeen cards
/// that print this cost print `Add …` behind it — so the comment beside it
/// makes a claim about the whole pool, and this is what holds the claim.
///
/// It is written the way the *memory* of the last two says to write one:
/// both directions. The count is a floor rather than an equality, because
/// the population grows with every storage land `landgen` learns to read and
/// an exact number would make a new card fail a test it has nothing to do
/// with — but a floor of six is what stops the scan passing over an empty
/// pool, which is how a claim like this goes quietly vacuous.
///
/// If it ever fails, the fix is not to widen it. A card that pays counters
/// into an ability that uses the stack is a card that has to be *played* in
/// an engine test, because `x_value` is a write nobody has ever read back.
#[test]
fn every_counter_x_cost_in_the_pool_is_on_a_mana_ability() {
    let mut carried = Vec::new();
    let mut offenders = Vec::new();
    let mut check = |who: &str, ability: &AbilityDef| {
        let (AbilityDef::Activated {
            cost, mana_ability, ..
        }
        | AbilityDef::ActivatedConditional {
            cost, mana_ability, ..
        }) = ability
        else {
            return;
        };
        if !cost
            .parts
            .iter()
            .any(|p| matches!(p, baylee_cards_dsl::CostPart::RemoveCounterSelfX { .. }))
        {
            return;
        }
        carried.push(who.to_string());
        if !mana_ability {
            offenders.push(who.to_string());
        }
    };
    for def in baylee_cards::all() {
        for face in 0..def.faces.len() {
            for ability in def.abilities_for_face(face) {
                check(def.name(), ability);
            }
        }
    }
    for token in baylee_cards::tokens::ALL {
        for ability in token.abilities {
            check(token.name, ability);
        }
    }
    assert!(
        offenders.is_empty(),
        "a counter-X cost on an ability that uses the stack: the announced \
         number now travels as `x_value` on the stacked object, which no \
         card has ever played — give it an engine test before trusting it: \
         {offenders:?}"
    );
    assert!(
        carried.len() >= 6,
        "only {} cards carry a counter-X cost, so this scan is close to \
         asserting nothing: {carried:?}",
        carried.len()
    );
}

/// The same class once more, on the side of the line where the parts above
/// *are* paid — because "the casting wizard pays it" turned out to be a claim
/// that has to name **which** of a spell's three cost lists.
///
/// They are paid in three different places and none of them by the same code:
///
/// - `alternative_costs[i].cost.parts` pays `PayLife` and `ExileFromHand` —
///   Force of Will's pitch. It is the only one with a gate: `cast_options`
///   runs `can_afford` over it, so a `Sacrifice(_)` written here is *refused*.
///   That is not safety. It is a refusal at the mode, and it took a second
///   fix for it to be a refusal at all: `casting::can_cast` probed the
///   alternative's *mana* and nothing else, so a pitch with nothing to pitch
///   was listed as castable and the wizard then had no option to offer — a
///   dead offer, the shape the `CommanderControlled` bug had. Both askers now
///   read `casting::pitchable`, and a part neither of them understands is
///   still one this list must never carry.
/// - `mandatory_additional_costs` pays `PayLifeX` and `PayLife` — Toxic
///   Deluge's X. Nothing gates it at all, so anything else there is not
///   refused but skipped, and the spell is cast without paying it.
/// - `additional_costs[i].parts` is paid by **nothing**: `finish_cast`
///   combines `add.mana` and never looks at the parts. The field exists, a
///   card can fill it, and a kicker taken with it costs exactly its mana.
///
/// So the rule is per list, and each half of it is held by the predicate the
/// payment itself reads ([`cast_wizard::paid_as_an_alternative_cost`],
/// [`cast_wizard::paid_as_a_mandatory_additional_cost`]) — the arrangement
/// that stopped the activated-ability twin from drifting.
///
/// Every card, not the implemented ones only, for `93ed903`'s reason: a
/// partial card is dealt into decks and played, and a spell cast for free is
/// not something `Coverage::Partial` should be able to hide. No token half —
/// `TokenDef` carries none of these three lists, having no cost to cast — so
/// the door beside the pool is checked here and found not to exist.
///
/// [`cast_wizard::paid_as_an_alternative_cost`]: super::cast_wizard
/// [`cast_wizard::paid_as_a_mandatory_additional_cost`]: super::cast_wizard
#[test]
fn no_spell_cost_list_carries_a_part_its_payment_walks_past() {
    let mut offenders = Vec::new();
    for def in baylee_cards::all() {
        for face in def.faces {
            for alt in face.alternative_costs {
                for part in alt.cost.parts {
                    if !crate::engine::cast_wizard::paid_as_an_alternative_cost(part) {
                        offenders.push(format!(
                            "{} — alternative cost {part:?}, which `finish_cast` does not pay",
                            face.name,
                        ));
                    }
                }
            }
            for part in face.mandatory_additional_costs {
                if !crate::engine::cast_wizard::paid_as_a_mandatory_additional_cost(part) {
                    offenders.push(format!(
                        "{} — mandatory additional cost {part:?}, which `finish_cast` skips",
                        face.name,
                    ));
                }
            }
            for add in face.additional_costs {
                for part in add.parts {
                    offenders.push(format!(
                        "{} — kicker part {part:?}, and a kicker's parts are read by nothing",
                        face.name,
                    ));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a spell prints a cost part on a list whose payment never reads it, so \
         the spell is cast without paying it (or, on an alternative cost the \
         gate refuses, is offered and then uncastable): {offenders:?}"
    );
}

/// A permanent that makes mana two ways is offered once.
///
/// `mana_abilities` is addressed by `PlayerAction::ActivateManaAbility
/// { source }`, which names no index, so the list is a list of *permanents*:
/// a second entry for one of them is an offer nothing can accept. The first
/// press takes whichever ability the engine prefers, the permanent is
/// tapped, and the duplicate is then refused by the very list that put it
/// there — this module's whole subject, one line lower down than usual.
///
/// Chromatic Lantern is where the pool says it. It grants every land
/// "{T}: Add one mana of any color" on top of the CR 305.6 shortcut a basic
/// already has, and entered each of them in the list twice.
///
/// Every reader had to know that on its own. `testkit::tap_mana_except`
/// unwraps and would have panicked; [`probe`] gave up on the board, which is
/// why Chromatic Lantern and Great Divide Guide were the last two implemented
/// cards the sweep could not reach; and the client survived only because
/// `manasources::sources` dedupes by object id for a reason of its own.
#[test]
fn a_permanent_that_makes_mana_two_ways_is_offered_once() {
    let seat = PlayerId::new(0);
    let lantern = card_index("539f5396-d99a-417d-a84c-dff7930b5900");
    let forest = card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
    let mut engine = Duel::new(SEED, forest)
        .battlefield(0, &[lantern, forest, forest])
        .start();
    assert!(
        walk_to_own_main(&mut engine, seat),
        "the board never reached seat 0's own main phase"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered = legal.mana_abilities.clone();
    let mut once = offered.clone();
    once.sort_unstable();
    once.dedup();
    assert_eq!(
        once.len(),
        offered.len(),
        "a permanent was offered as a mana source more than once: {offered:?}"
    );
    assert_eq!(offered.len(), 2, "two Forests under a Lantern: {offered:?}");
    // The other half of the same claim: every entry in the list is one the
    // action will take. Pressed in the order the engine listed them, which
    // is how `HeuristicAgent` and the client's planner both read it.
    for source in offered {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap_or_else(|err| panic!("{source:?} was offered, then: {err:?}"));
    }
}

/// Suspend is offered on the same two conditions as any other activation.
///
/// Its first ability is an activated one — "rather than cast this card from
/// your hand, **pay {U}** and exile it" — with "activate only as a sorcery"
/// on it (CR 702.62a). The offer used to ask only about the turn, so a
/// suspend card sat lit in an opponent's hand-zone list with an empty pool,
/// and `apply` then answered `cannot pay the suspend cost`. That is the
/// shape this whole module exists for, and it was the one branch in the
/// hand-zone scan that did not go through `can_afford`.
///
/// The pool and not the untapped Island is deliberately what decides it:
/// every activation in this engine is offered against mana that is already
/// floating, which is why the client plans a run of taps first.
#[test]
fn a_suspend_card_is_offered_only_once_its_cost_is_on_the_table() {
    let seat = PlayerId::new(0);
    let vision = card_index("9728dec9-d482-4c7a-8cdc-44d010dc878d");
    let island = card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373");
    let mut engine = Duel::new(SEED, island)
        .hand(0, &[vision])
        .battlefield(0, &[island])
        .start();
    assert!(
        walk_to_own_main(&mut engine, seat),
        "the board never reached seat 0's own main phase"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.suspendable.is_empty(),
        "Suspend 4—{{U}} was offered with nothing in the pool: {:?}",
        legal.suspendable
    );
    // And it is not castable either, at any point in this test. Ancestral
    // Vision prints *no* mana cost, which CR 202.1b says is not a cost of
    // zero: the only way it leaves this hand is the suspend ability. It was
    // in `castable` here — a free "target player draws three cards" for
    // anybody who reached their own main phase.
    assert!(
        legal.castable.is_empty(),
        "a card with no mana cost was castable for nothing: {:?}",
        legal.castable
    );

    // The Island is untapped and the timing is right, so the only thing
    // between the two assertions is the {U} itself.
    let source = legal.mana_abilities[0];
    engine
        .apply(seat, PlayerAction::ActivateManaAbility { source })
        .expect("an untapped Island taps for {U}");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.suspendable.len(),
        1,
        "with {{U}} floating the card is suspendable: {:?}",
        legal.suspendable
    );
    assert!(
        legal.castable.is_empty(),
        "a blank cost is not paid by floating a mana either: {:?}",
        legal.castable
    );

    // The other half of this module's claim: what is offered is accepted.
    engine
        .apply(
            seat,
            PlayerAction::Suspend {
                card: legal.suspendable[0],
            },
        )
        .expect("offered, then refused");
}

/// A cost of `{0}` is still a cost, and still buys a spell.
///
/// The other side of `casting::has_a_printed_cost`, and the reason it is a
/// predicate rather than a comparison against `ManaCost::ZERO`: Mox Opal
/// prints `{0}` and is cast by paying nothing, which is an entirely ordinary
/// thing for a spell to do. A guard that could not tell it from a blank would
/// have made four cards in this pool uncastable instead of two.
#[test]
fn a_printed_zero_is_a_cost_and_the_spell_is_still_castable() {
    let seat = PlayerId::new(0);
    let opal = card_index("de2440de-e948-4811-903c-0bbe376ff64d");
    let island = card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373");
    let mut engine = Duel::new(SEED, island).hand(0, &[opal]).start();
    assert!(
        walk_to_own_main(&mut engine, seat),
        "the board never reached seat 0's own main phase"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.castable.len(),
        1,
        "Mox Opal costs {{0}} and an empty pool pays it: {:?}",
        legal.castable
    );
    engine
        .apply(
            seat,
            PlayerAction::CastSpell {
                card: legal.castable[0],
            },
        )
        .expect("offered, then refused");
}

/// Only a card with another way out of the hand prints no mana cost.
///
/// The pool-wide half of CR 202.1b, and the guard on the data rather than on
/// the rule: `face!`'s default cost is the *blank* one, so a hand-written
/// card that prints `{0}` and simply omits the field used to be
/// indistinguishable from Ancestral Vision — and is now uncastable instead of
/// free, which is the quieter of the two failures and the reason this is
/// asserted rather than commented. Mox Opal and Pact of Negation were both
/// sitting here when it was written.
///
/// What is allowed to be blank is a card that names another way to be cast:
/// suspend (`AbilityDef::Suspend`) or a printed alternative cost. Lands and
/// the backs of transforming cards are not cast from a hand at all.
#[test]
fn a_face_with_no_mana_cost_has_another_way_out_of_the_hand() {
    let mut blank: Vec<&str> = Vec::new();
    for def in baylee_cards::all() {
        let suspends = def
            .abilities
            .iter()
            .any(|a| matches!(a, AbilityDef::Suspend { .. }));
        for face in def.faces {
            if face.mana_cost.symbols().next().is_some()
                || !face.castable_from_hand
                || face.types.contains(TypeSet::LAND)
            {
                continue;
            }
            if !suspends && face.alternative_costs.is_empty() {
                blank.push(face.name);
            }
        }
    }
    assert!(
        blank.is_empty(),
        "these faces print no mana cost and no other way to be cast, so they \
         can never leave a hand — a `{{0}}` card that omitted `mana_cost` \
         looks exactly like this: {blank:?}"
    );
}

/// The suspend offer and the suspend payment agree under Mycosynth Lattice.
///
/// Suspend's cost is a bare `ManaCost` rather than a `Cost`, so it reaches
/// the pool through `can_pay_mana` on the offer side and `casting::pay_with`
/// on the apply side. Both branch on `casting::mana_is_wild`, and that is
/// exactly the split that was there before: the offer probed
/// `can_pay_wild` while the payment called `mana_pay::pay`, so a Lattice on
/// the table made the two halves disagree about which mana counts.
///
/// Nothing else in this module reaches it — the sweep's probe board has no
/// Lattice on it, and a rule that is only ever exercised on one side of a
/// branch is a rule with no test at all.
#[test]
fn wild_mana_pays_a_suspend_cost_the_offer_also_accepts() {
    let seat = PlayerId::new(0);
    let vision = card_index("9728dec9-d482-4c7a-8cdc-44d010dc878d");
    let lattice = card_index("ae1f2ab5-c6a5-4d49-a746-3cb4668bf805");
    let forest = card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
    let mut engine = Duel::new(SEED, forest)
        .hand(0, &[vision])
        .battlefield(0, &[lattice, forest])
        .start();
    assert!(
        walk_to_own_main(&mut engine, seat),
        "the board never reached seat 0's own main phase"
    );

    // A Forest and a {U} cost: without the Lattice this is unpayable, which
    // is what makes the tap the whole of the test.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.suspendable.is_empty(),
        "Suspend 4—{{U}} was offered with nothing in the pool: {:?}",
        legal.suspendable
    );
    // The Lattice makes every permanent an artifact and none of them a mana
    // source, so the Forest is the only entry there is.
    assert_eq!(
        legal.mana_abilities.len(),
        1,
        "the Forest is the board's only mana source: {:?}",
        legal.mana_abilities
    );
    let source = legal.mana_abilities[0];
    engine
        .apply(seat, PlayerAction::ActivateManaAbility { source })
        .expect("an untapped Forest taps for {G}");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.suspendable.len(),
        1,
        "under a Lattice the floating {{G}} pays {{U}}: {:?}",
        legal.suspendable
    );
    // The half that was broken: offered, then refused by the payment.
    engine
        .apply(
            seat,
            PlayerAction::Suspend {
                card: legal.suspendable[0],
            },
        )
        .expect("what the engine offers, the engine accepts");
}

/// A CR 605.3a payment window nests exactly once, and this is what says so.
///
/// `PaymentWindow` carries the resolution it was opened over so that
/// [`Engine::resolution`] stays free for whatever is resolving now (#167).
/// One spare slot is enough only while a *mana* ability cannot open a window
/// of its own — and inside a window nothing else may be activated
/// (`narrow_to_mana`), so a mana ability is the only door to a second level.
/// A window is opened by `Effect::PlayerMayPayOr` and by nothing else.
///
/// "A mana ability" is **two** populations and the second is easy to miss. A
/// granted one carries its own `mana_ability` flag on
/// `Modifier::GrantActivated`, and it lives in `legal.mana_abilities`, which
/// `narrow_to_mana` does not clear — so it is activatable inside a window
/// exactly like a printed one, and it is a door to the same second level.
/// Both are scanned, and the granted half carries its own floor because a
/// population of nought agrees with everything just as loudly as an empty
/// pool does.
///
/// Ward is not scanned because it cannot be the offender by construction:
/// `AbilityDef::Ward` is its own variant and reaches the same effect
/// synthetically, so it is never an ability with `mana_ability: true`.
///
/// Read off `{:?}` rather than a hand-written walk over `Effect`. An effect
/// tree nests through a dozen variants, and a walker that does not know one
/// of them reports "no offenders" for a card it never looked into — which is
/// the shape this repository keeps finding in its own readers. A derived
/// `Debug` cannot go blind on a variant nobody taught it, and a false match
/// here is a finding rather than a silence.
///
/// The floor is what stops it passing over a pool with no taxes in it at
/// all: a scan whose population is nought agrees with everything. Measured
/// at 13 on 2026-09-20 — Rhystic Study, Esper Sentinel and Smothering Tithe,
/// the three Karoo-style bounce lands, Mystic Remora, Mana Leak,
/// Flusterstorm and the rest — and the floor is set well under that rather
/// than at it, because a card leaving the pool is not a reason for this
/// claim to go red.
#[test]
fn no_mana_ability_in_the_pool_opens_a_payment_window() {
    let mut carried = Vec::new();
    let mut grants = Vec::new();
    let mut offenders = Vec::new();
    let mut check = |who: &str, ability: &AbilityDef| {
        let rendered = format!("{ability:?}");
        // The second door, and it is the one a reader forgets. A mana
        // ability need not be printed: `Modifier::GrantActivated` carries
        // its own `mana_ability` flag, and a granted one is offered in
        // `legal.mana_abilities`, which `narrow_to_mana` does **not** clear
        // — so it is activatable inside a window exactly like a printed one.
        // It hides in two shapes, an `AbilityDef::Static` and an
        // `Effect::CreateContinuousEffect` inside an ordinary effect list,
        // which is why this asks the rendering rather than a walk: a walk
        // that knew one shape would report a clean pool having looked at
        // half of it. ai-ec found that exact blind spot in the neighbouring
        // scan in `ai_coverage_guards.rs`.
        if rendered.contains("GrantActivated") {
            grants.push(who.to_string());
        }
        if !rendered.contains("PlayerMayPayOr") {
            return;
        }
        carried.push(who.to_string());
        let printed_mana_ability = matches!(
            ability,
            AbilityDef::Activated {
                mana_ability: true,
                ..
            } | AbilityDef::ActivatedConditional {
                mana_ability: true,
                ..
            }
        );
        // Deliberately conservative on the granted half: an ability that
        // both grants something and mentions the tax is reported without
        // asking which of its parts carries which, because this rendering
        // cannot tell. Over-reporting is the safe direction for a claim
        // that says a thing *cannot* happen, and the failure hands over the
        // card to look at rather than a silence to trust.
        if printed_mana_ability || rendered.contains("GrantActivated") {
            offenders.push(who.to_string());
        }
    };
    for def in baylee_cards::all() {
        for face in 0..def.faces.len() {
            for ability in def.abilities_for_face(face) {
                check(def.name(), ability);
            }
        }
    }
    for token in baylee_cards::tokens::ALL {
        for ability in token.abilities {
            check(token.name, ability);
        }
    }
    assert!(
        offenders.is_empty(),
        "a mana ability that charges a tax opens a payment window from \
         inside one, and `PaymentWindow` holds exactly one suspended \
         resolution — the second would overwrite the first, which is the \
         defect #167 was: {offenders:?}"
    );
    assert!(
        grants.len() >= 3,
        "only {} abilities in the pool grant an activated ability, so the \
         granted half of this scan is agreeing with nothing: {grants:?}",
        grants.len()
    );
    assert!(
        carried.len() >= 8,
        "only {} abilities in the pool charge a tax at all, so this scan is \
         close to agreeing with an empty pool: {carried:?}",
        carried.len()
    );
}
