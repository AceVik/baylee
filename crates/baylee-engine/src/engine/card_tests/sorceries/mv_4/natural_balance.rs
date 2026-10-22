//! `cards/sorceries/mv_4/natural_balance.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "69acaa6f-3ba2-420c-9069-02958f844121"
fn natural_balance() -> CardIndex {
    card_index("69acaa6f-3ba2-420c-9069-02958f844121")
}

#[test]
fn natural_balance_lets_each_player_search_for_basic_land_onto_battlefield() {
    let mut engine = lk::game(SEED, &[], &[], &[natural_balance()], &[]);

    let p0_lands_before = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| engine.state().object(id).unwrap().controller == P0)
        .count();
    let p1_lands_before = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| engine.state().object(id).unwrap().controller == P1)
        .count();

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Green, 2)],
    );
    cast_with_floating(&mut engine, P0, natural_balance());

    for _ in 0..2 {
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
        });
        if let Pending::ChooseCards {
            player, options, ..
        } = engine.pending().clone()
        {
            engine
                .apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: vec![options[0]],
                    },
                )
                .unwrap();
        }
    }
    pass_until(&mut engine, stack_is_empty);

    let p0_lands_after = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| engine.state().object(id).unwrap().controller == P0)
        .count();
    let p1_lands_after = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| engine.state().object(id).unwrap().controller == P1)
        .count();

    assert_eq!(p0_lands_after, p0_lands_before + 1);
    assert_eq!(p1_lands_after, p1_lands_before + 1);
}
