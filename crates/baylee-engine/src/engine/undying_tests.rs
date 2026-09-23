//! Undying (CR 702.93a) and persist (CR 702.79a) — two keyword triggered
//! abilities the engine reads off the bit, the way it reads prowess.
//!
//! Four cards in this pool print one of them and every one was
//! `Coverage::Partial` for the same missing sentence. What is tested here is
//! the *rule*, because the thing that can go wrong is not a card: the
//! intervening "if it had no +1/+1 counters on it" is a question about an
//! object that no longer exists, and the obvious wrong answer — asking the
//! card lying in the graveyard, whose counters the move there cleared — is a
//! creature that returns for ever.

use super::testkit::{
    Duel, RegistryLookup, card_index, in_graveyard, keep_mulligans, on_battlefield, pass_until, pt,
    walk_to_own_main,
};
use super::*;
use crate::event::Cause;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::CounterKind;
use baylee_core::ids::{CardIndex, ObjectId};

/// How many counters of one kind are on a permanent.
#[track_caller]
fn counters_on(engine: &Engine<RegistryLookup>, id: ObjectId, kind: CounterKind) -> u16 {
    engine
        .state()
        .object(id)
        .expect("the permanent is still an object")
        .counters
        .get(kind)
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// A 1/1 with undying and nothing else on it.
fn young_wolf() -> CardIndex {
    card_index("8b492764-10b6-4506-be11-22daa9220a91")
}

/// A 2/2 with persist.
fn safehold_elite() -> CardIndex {
    card_index("68ca91ba-31fb-47e0-9b32-e4f3504cbbca")
}

/// Kills `id` and stops before anything that triggered has resolved, which
/// is the window an opponent gets to respond in.
///
/// It does not assert that something *is* on the stack, because the second
/// death of an undying creature is the case where nothing triggers — that
/// assertion belongs to the test that needs the window, not to the door.
#[track_caller]
fn kill_holding_the_trigger(engine: &mut Engine<RegistryLookup>, id: ObjectId) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    crate::sba::destroy(state, id);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    engine
        .apply(player, PlayerAction::PassPriority)
        .expect("passing priority is always legal");
}

/// Kills `id` through the one door every destruction goes through, then
/// lets the game judge the board and resolve whatever triggered.
#[track_caller]
fn kill(engine: &mut Engine<RegistryLookup>, id: ObjectId) {
    kill_holding_the_trigger(engine, id);
    pass_until(engine, super::testkit::stack_is_empty);
}

/// The whole of undying in one board, and the second death is the half that
/// matters.
///
/// A rule that read the counters off the card in the graveyard would pass
/// the first four assertions and fail this one — those counters are cleared
/// by the very move that put it there, so every death would look like a
/// first death and the Wolf would never stay down.
#[test]
fn undying_returns_a_creature_once_and_the_counter_is_why_it_is_only_once() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(91, forest())
        .battlefield(0, &[young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("the Wolf is seated");
    assert_eq!(
        pt(&engine, wolf),
        (1, 1),
        "a printed 1/1 with nothing on it"
    );

    kill(&mut engine, wolf);

    let back = on_battlefield(&engine, p0, young_wolf()).expect("undying brought it back");
    assert_eq!(
        counters_on(&engine, back, CounterKind::P1P1),
        1,
        "with a +1/+1 counter on it"
    );
    assert_eq!(pt(&engine, back), (2, 2), "so the 1/1 is a 2/2");
    assert!(
        in_graveyard(&engine, p0, young_wolf()).is_none(),
        "and it is not in the graveyard as well"
    );

    kill(&mut engine, back);

    assert!(
        on_battlefield(&engine, p0, young_wolf()).is_none(),
        "it had a +1/+1 counter on it, so the intervening `if` fails and \
         nothing returns"
    );
    assert!(
        in_graveyard(&engine, p0, young_wolf()).is_some(),
        "the second death is the one that keeps it"
    );
}

/// Persist is the same rule with the other counter, and the sign is what
/// tells the two apart: the Elf comes back *smaller*.
#[test]
fn persist_returns_a_creature_with_a_minus_counter_and_not_twice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(92, forest())
        .battlefield(0, &[safehold_elite()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, safehold_elite()).expect("the Elf is seated");
    assert_eq!(pt(&engine, elf), (2, 2));

    kill(&mut engine, elf);

    let back = on_battlefield(&engine, p0, safehold_elite()).expect("persist brought it back");
    assert_eq!(counters_on(&engine, back, CounterKind::M1M1), 1);
    assert_eq!(
        counters_on(&engine, back, CounterKind::P1P1),
        0,
        "a -1/-1 counter and not its mirror — persist and undying are the \
         same sentence with different signs, and a rule that used one for \
         both would grow the creature instead"
    );
    assert_eq!(pt(&engine, back), (1, 1), "so the 2/2 is a 1/1");

    kill(&mut engine, back);
    assert!(
        on_battlefield(&engine, p0, safehold_elite()).is_none(),
        "and the -1/-1 counter it wears now is what keeps it down"
    );
}

/// The trigger names the card in the graveyard, and a card that has left it
/// is a different object (CR 400.7).
///
/// This is the half a reanimation *spell* gets for free: it targets, so
/// CR 608.2b throws it out on an illegal target. Undying targets nothing —
/// it reads the creature that died straight off the event — so the graveyard
/// is a question the effect has to ask for itself, and the first version of
/// it did not: the Wolf was pulled out of **exile** onto the battlefield.
#[test]
fn a_card_exiled_in_response_to_the_trigger_does_not_come_back() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(93, forest())
        .battlefield(0, &[young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("the Wolf is seated");

    kill_holding_the_trigger(&mut engine, wolf);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "the undying trigger is on the stack and has not resolved yet"
    );
    assert!(
        in_graveyard(&engine, p0, young_wolf()).is_some(),
        "the card is where the trigger points, so the board would otherwise \
         return it — which is what makes the exile below the only difference"
    );

    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may move cards about");
    state
        .move_object(
            wolf,
            ZoneLocation::Exile(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("a card in a graveyard may be exiled");
    pass_until(&mut engine, super::testkit::stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, young_wolf()).is_none(),
        "the trigger resolved and found nothing to return"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&wolf),
        "and it stayed in exile rather than being pulled out of it"
    );
}

/// What comes back has not been controlled continuously since the turn
/// began, so it may not attack (CR 302.6).
///
/// This engine reuses the `ObjectId` across a zone change rather than minting
/// a fresh one — the arena entry is moved, which is how `Blink` is written
/// too — so "a new object" (CR 400.7) is a claim about *memory* here and not
/// about the handle: what has to be reset is the entered-the-zone timestamp
/// the sickness check reads, and the returned Wolf is the one permanent on
/// the board for which that could have been missed.
///
/// The Elf beside it is the control: it was seated the same way, was never
/// killed, and is on the list — so an empty attacker list is not what this
/// test is reading.
#[test]
fn the_returned_creature_is_a_new_object_and_is_summoning_sick() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(94, forest())
        .battlefield(0, &[young_wolf(), super::testkit::quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("the Wolf is seated");
    let bystander = on_battlefield(&engine, p0, super::testkit::quiet_creature())
        .expect("the Elf is seated beside it");

    kill(&mut engine, wolf);
    let back = on_battlefield(&engine, p0, young_wolf()).expect("undying brought it back");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the attacker declaration")
    };
    assert!(
        attackers.contains(&bystander),
        "the Elf that was never killed attacks, so the turn is not the reason"
    );
    assert!(
        !attackers.contains(&back),
        "and the Wolf that arrived this turn may not: {attackers:?}"
    );
}
