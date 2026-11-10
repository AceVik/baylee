//! `cards/creatures/mv_4/crazed_armodon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn crazed_armodon() -> CardIndex {
    card_index("ce7d672a-7cb9-40e5-a8ad-dd3ce0f8ecd3")
}

/// p0 has a Crazed Armodon and two Forests, in their first main phase.
fn an_armodon_with_mana() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[crazed_armodon(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let armodon = on_battlefield(&engine, P0, crazed_armodon()).unwrap();
    (engine, armodon)
}

/// Crazed Armodon (ce7d672a-7cb9-40e5-a8ad-dd3ce0f8ecd3) — {2}{G}{G} Creature — Elephant:
/// "{G}: This creature gets +3/+0 and gains trample until end of turn. Destroy this creature at the beginning of the next end step. Activate only once each turn."
#[test]
fn crazed_armodon_gets_plus_three_and_trample_for_one_green() {
    let (mut engine, armodon) = an_armodon_with_mana();
    assert_eq!(pt(&engine, armodon), (3, 3));
    assert!(!keywords(&engine, armodon).contains(KeywordSet::TRAMPLE));

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, crazed_armodon(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, armodon),
        (6, 3),
        "+3/+0: toughness is untouched"
    );
    assert!(keywords(&engine, armodon).contains(KeywordSet::TRAMPLE));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the ability cost {{G}}, not both Forests"
    );
}

#[test]
fn crazed_armodon_can_be_activated_only_once_each_turn() {
    let (mut engine, armodon) = an_armodon_with_mana();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, crazed_armodon(), 0);
    pass_until(&mut engine, stack_is_empty);

    // {G} is still floating, so mana is not what stops the second try.
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: armodon,
                    ability_index: 0,
                },
            )
            .is_err(),
        "Crazed Armodon can only be activated once each turn"
    );
    assert_eq!(pt(&engine, armodon), (6, 3), "not +6/+0");
}

#[test]
fn crazed_armodon_is_destroyed_at_the_next_end_step_and_not_before() {
    let (mut engine, _) = an_armodon_with_mana();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, crazed_armodon(), 0);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        on_battlefield(&engine, P0, crazed_armodon()).is_some(),
        "still alive through the turn's second main phase"
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(
        in_graveyard(&engine, P0, crazed_armodon()).is_some(),
        "destroyed at the beginning of the end step"
    );
}

#[test]
fn crazed_armodon_that_is_never_activated_is_never_destroyed() {
    let (mut engine, _) = an_armodon_with_mana();

    reach_their_main_phase(&mut engine, P1);

    assert!(on_battlefield(&engine, P0, crazed_armodon()).is_some());
}
