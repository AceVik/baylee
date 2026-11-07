//! `cards/enchantments/mv_4/dreams_of_the_dead.rs`, played.
//!
//! Oracle:
//! - "{1}{U}: Return target white or black creature card from your graveyard to the battlefield. That creature gains 'Cumulative upkeep {2}.' If the creature would leave the battlefield, exile it instead of putting it anywhere else."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dreams_of_the_dead() -> CardIndex {
    card_index("3b6cf935-3b99-4371-a797-6f40b89adeeb")
}

/// A main-phase game with one white, one black and one green creature in P0's
/// graveyard, a white creature in P1's, and two Islands for the granted upkeep.
fn target_game() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, index::ISLAND)
        .battlefield(
            0,
            &[
                dreams_of_the_dead(),
                index::SAVANNAH_LIONS,
                index::BLACK_KNIGHT,
                index::GRIZZLY_BEARS,
                index::ISLAND,
                index::ISLAND,
            ],
        )
        .battlefield(1, &[index::SAVANNAH_LIONS])
        .hand(0, &[index::UNSUMMON])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let knight = lk::obj(&engine, P0, index::BLACK_KNIGHT);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let theirs = lk::obj(&engine, P1, index::SAVANNAH_LIONS);
    bury(&mut engine, &[lions, knight, bears, theirs]);
    engine
}

/// Activates Dreams for `card` and returns its graveyard object and target menu.
fn begin_reanimation(
    engine: &mut Engine<RegistryLookup>,
    card: CardIndex,
) -> (ObjectId, Vec<ObjectId>) {
    let target = in_graveyard(engine, P0, card).expect("the intended target is dead");
    lk::float(
        engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    activate(engine, P0, dreams_of_the_dead(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets for Dreams of the Dead");
    };
    (target, options)
}

#[track_caller]
fn finish_reanimation(engine: &mut Engine<RegistryLookup>, target: ObjectId) -> ObjectId {
    lk::aim(engine, &[target], &[]);
    pass_until(engine, stack_is_empty);
    assert_eq!(
        engine.state().object(target).map(|object| object.zone),
        Some(Zone::Battlefield),
        "the exact targeted card returned to the battlefield"
    );
    target
}

#[track_caller]
fn reanimate(engine: &mut Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    let (target, options) = begin_reanimation(engine, card);
    assert!(options.contains(&target));
    finish_reanimation(engine, target)
}

#[track_caller]
fn upkeep_question(engine: &mut Engine<RegistryLookup>, turn: u32) -> u16 {
    pass_until(engine, |e| {
        e.state().turn.number == turn && matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        panic!("the granted cumulative upkeep asks for mana");
    };
    assert_eq!(player, P0);
    mana
}

#[test]
fn dreams_of_the_dead_returns_only_a_white_or_black_creature_from_your_graveyard() {
    let mut engine = target_game();
    let lions = in_graveyard(&engine, P0, index::SAVANNAH_LIONS).expect("white target");
    let bears = in_graveyard(&engine, P0, index::GRIZZLY_BEARS).expect("green non-target");
    let theirs = in_graveyard(&engine, P1, index::SAVANNAH_LIONS).expect("opponent's non-target");

    let (knight, options) = begin_reanimation(&mut engine, index::BLACK_KNIGHT);
    assert!(options.contains(&lions), "a white creature is legal");
    assert!(options.contains(&knight), "a black creature is legal");
    assert!(!options.contains(&bears), "a green creature is not legal");
    assert!(
        !options.contains(&theirs),
        "an opponent's graveyard is not your graveyard"
    );
    assert_eq!(finish_reanimation(&mut engine, knight), knight);
    assert!(in_graveyard(&engine, P0, index::BLACK_KNIGHT).is_none());
    assert_eq!(
        engine.state().object(lions).map(|object| object.zone),
        Some(Zone::Graveyard),
        "the unchosen card stays in the graveyard"
    );
}

#[test]
fn dreams_of_the_dead_grants_cumulative_upkeep_two_and_exiles_an_unpaid_sacrifice() {
    let mut engine = target_game();
    let creature = reanimate(&mut engine, index::SAVANNAH_LIONS);
    assert_eq!(counters_on(&engine, creature, counters::AGE), 0);

    assert_eq!(upkeep_question(&mut engine, 3), 2);
    assert_eq!(counters_on(&engine, creature, counters::AGE), 1);
    engine
        .apply(P0, PlayerAction::YesNo(true))
        .expect("pay the first granted upkeep");
    let islands = all_on_battlefield(&engine, P0, index::ISLAND);
    assert_eq!(islands.len(), 2);
    engine
        .apply(
            P0,
            PlayerAction::PayWindow {
                steps: islands
                    .into_iter()
                    .map(|source| crate::choice::PaymentStep::ActivateManaAbility { source })
                    .collect(),
            },
        )
        .expect("two Islands pay {2} for one age counter");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(creature).map(|object| object.zone),
        Some(Zone::Battlefield)
    );

    assert_eq!(upkeep_question(&mut engine, 5), 4);
    assert_eq!(counters_on(&engine, creature, counters::AGE), 2);
    engine
        .apply(P0, PlayerAction::YesNo(false))
        .expect("decline the second granted upkeep");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(creature).map(|object| object.zone),
        Some(Zone::Exile),
        "the upkeep sacrifice is replaced with exile"
    );
    assert!(in_graveyard(&engine, P0, index::SAVANNAH_LIONS).is_none());
}

#[test]
fn dreams_of_the_dead_exiles_the_returned_creature_instead_of_bouncing_it() {
    let mut engine = target_game();
    let creature = reanimate(&mut engine, index::BLACK_KNIGHT);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, index::UNSUMMON);
    lk::aim(&mut engine, &[creature], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(creature).map(|object| object.zone),
        Some(Zone::Exile),
        "the departure replacement is not limited to destruction or sacrifice"
    );
    assert!(in_hand(&engine, P0, index::BLACK_KNIGHT).is_none());
    assert!(in_graveyard(&engine, P0, index::BLACK_KNIGHT).is_none());
}
