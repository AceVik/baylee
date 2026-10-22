//! `cards/creatures/artifacts/mv_4/brass_talon_chimera.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn brass_talon_chimera() -> CardIndex {
    card_index("2e114d15-4356-4cef-8f89-260ab4c8d4b9")
}

#[test]
fn brass_talon_chimera_sacrifices_to_give_target_chimera_plus_two_counter_and_first_strike() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                brass_talon_chimera(),
            ],
        )
        .hand(0, &[brass_talon_chimera()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let other_chimera = on_battlefield(&engine, p0, brass_talon_chimera()).unwrap();
    cast_from_hand(&mut engine, p0, brass_talon_chimera());
    pass_until(&mut engine, stack_is_empty);

    let new_chimera = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            id != other_chimera
                && engine
                    .state()
                    .object(id)
                    .unwrap()
                    .card
                    .is_some_and(|c| c.index == brass_talon_chimera())
        })
        .unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: new_chimera,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, other_chimera);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, other_chimera), (4, 4));
    assert!(keywords(&engine, other_chimera).contains(KeywordSet::FIRST_STRIKE));

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, other_chimera), (4, 4));
    assert!(keywords(&engine, other_chimera).contains(KeywordSet::FIRST_STRIKE));
}
