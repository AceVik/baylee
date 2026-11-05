//! `cards/creatures/mv_5/krovikan_vampire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn krovikan_vampire() -> CardIndex {
    card_index("8fa93c01-f9ea-4d48-a040-418d1ca6be19")
}

/// Krovikan Vampire (8fa93c01-f9ea-4d48-a040-418d1ca6be19) — {3}{B}{B} 3/3 Creature — Vampire:
/// "At the beginning of each end step, if a creature dealt damage by this creature this turn died, put that card onto the battlefield under your control. Sacrifice it when you lose control of this creature."
#[test]
fn krovikan_vampire_casts_for_three_and_two_black_and_fights_as_a_three_three_vampire() {
    let mut engine = Duel::new(SEED, index::SWAMP)
        .battlefield(
            0,
            &[
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
            ],
        )
        .hand(0, &[krovikan_vampire()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, None);
    cast_from_hand(&mut engine, P0, krovikan_vampire());
    pass_until(&mut engine, stack_is_empty);

    let vampire = on_battlefield(&engine, P0, krovikan_vampire()).expect("Vampire entered");
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).expect("Bears seated");
    assert_eq!(pt(&engine, vampire), (3, 3));

    // Advance to P0's next turn so Krovikan Vampire loses summoning sickness and attacks.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    let _ = lk::attack(&mut engine, &[vampire]);
    lk::block(&mut engine, &[(bears, vampire)]);
    lk::through_combat(&mut engine);

    assert!(
        in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "3/3 Krovikan Vampire kills the blocking 2/2 Grizzly Bears"
    );
    assert!(
        on_battlefield(&engine, P0, krovikan_vampire()).is_some(),
        "3/3 Krovikan Vampire survives 2 combat damage"
    );
}

/// The Vampire blocks and kills an attacking Bears on P1's turn, then returns
/// that exact card under P0's control at the beginning of P1's end step.
fn bears_returned_on_opponents_end_step(
    p0_hand: &[CardIndex],
    p1_hand: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = lk::game(
        SEED,
        &[krovikan_vampire()],
        &[index::GRIZZLY_BEARS],
        p0_hand,
        p1_hand,
    );
    let vampire = lk::obj(&engine, P0, krovikan_vampire());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    reach_their_main_phase(&mut engine, P1);

    let _ = lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[(vampire, bears)]);
    lk::through_combat(&mut engine);
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(lk::zone_of(&engine, vampire), Zone::Battlefield);

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });
    assert!(
        !stack_is_empty(&engine),
        "the death during P1's turn triggers at P1's end step"
    );
    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none(),
        "the return has not resolved yet"
    );
    lk::settle(&mut engine);

    assert_eq!(
        lk::zone_of(&engine, bears),
        Zone::Battlefield,
        "the damaged creature that died returns"
    );
    assert_eq!(
        engine.state().object(bears).unwrap().controller,
        P0,
        "the card returns under the Vampire controller's control, not its owner's"
    );
    (engine, vampire, bears)
}

#[test]
fn krovikan_vampire_returns_a_damaged_dead_creature_at_each_players_end_step() {
    let (engine, _, _) = bears_returned_on_opponents_end_step(&[], &[]);
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_none());
}

#[test]
fn krovikan_vampire_ignores_a_death_it_did_not_damage() {
    let mut engine = lk::game(
        SEED,
        &[krovikan_vampire()],
        &[index::GRIZZLY_BEARS],
        &[index::LIGHTNING_BOLT],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, index::LIGHTNING_BOLT, &[bears], &[]);
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());

    lk::to_step(&mut engine, P0, crate::turn::Step::End);
    assert!(
        stack_is_empty(&engine),
        "a death without damage from the Vampire does not trigger it"
    );
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
}

#[test]
fn krovikan_vampire_forgets_damage_from_an_earlier_turn() {
    let mut engine = lk::game(
        SEED,
        &[krovikan_vampire()],
        &[index::GIANT_SPIDER],
        &[],
        &[index::LIGHTNING_BOLT, index::SHOCK],
    );
    let vampire = lk::obj(&engine, P0, krovikan_vampire());
    let spider = lk::obj(&engine, P1, index::GIANT_SPIDER);
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    let _ = lk::attack(&mut engine, &[vampire]);
    lk::block(&mut engine, &[(spider, vampire)]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::zone_of(&engine, spider), Zone::Battlefield);
    assert_eq!(lk::damage_on(&engine, spider), 3);

    reach_their_main_phase(&mut engine, P1);
    lk::float(&mut engine, P1, &[(ManaColor::Red, 2)]);
    lk::cast_at(&mut engine, P1, index::LIGHTNING_BOLT, &[spider], &[]);
    assert_eq!(lk::zone_of(&engine, spider), Zone::Battlefield);
    lk::cast_at(&mut engine, P1, index::SHOCK, &[spider], &[]);
    assert!(in_graveyard(&engine, P1, index::GIANT_SPIDER).is_some());

    lk::to_step(&mut engine, P1, crate::turn::Step::End);
    assert!(
        stack_is_empty(&engine),
        "last turn's damage does not qualify this turn's death"
    );
    assert!(on_battlefield(&engine, P0, index::GIANT_SPIDER).is_none());
}

#[test]
fn krovikan_vampire_sacrifices_the_returned_creature_when_it_changes_control() {
    let (mut engine, vampire, bears) =
        bears_returned_on_opponents_end_step(&[], &[index::RAY_OF_COMMAND]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    lk::float(
        &mut engine,
        P1,
        &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
    );
    cast_with_floating(&mut engine, P1, index::RAY_OF_COMMAND);
    lk::aim(&mut engine, &[vampire], &[]);
    pass_until(&mut engine, |e| {
        e.state().object(vampire).unwrap().controller == P1 && !stack_is_empty(e)
    });
    assert_eq!(
        lk::zone_of(&engine, bears),
        Zone::Battlefield,
        "the sacrifice waits for its triggered ability"
    );

    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
}

#[test]
fn krovikan_vampire_sacrifices_the_returned_creature_when_it_leaves_play() {
    let (mut engine, vampire, bears) =
        bears_returned_on_opponents_end_step(&[index::LIGHTNING_BOLT], &[]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[vampire], &[]);
    pass_until(&mut engine, |e| {
        lk::zone_of(e, vampire) == Zone::Graveyard && !stack_is_empty(e)
    });
    assert_eq!(
        lk::zone_of(&engine, bears),
        Zone::Battlefield,
        "departure triggers the sacrifice rather than sacrificing immediately"
    );

    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
}

#[test]
fn krovikan_vampire_returns_a_creature_it_killed_on_its_own_turn_at_its_own_end_step() {
    let mut engine = lk::game(
        SEED,
        &[krovikan_vampire()],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let vampire = lk::obj(&engine, P0, krovikan_vampire());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    let _ = lk::attack(&mut engine, &[vampire]);
    lk::block(&mut engine, &[(bears, vampire)]);
    lk::through_combat(&mut engine);
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End && !stack_is_empty(e)
    });
    lk::settle(&mut engine);
    assert_eq!(lk::zone_of(&engine, bears), Zone::Battlefield);
    assert_eq!(engine.state().object(bears).unwrap().controller, P0);
}
