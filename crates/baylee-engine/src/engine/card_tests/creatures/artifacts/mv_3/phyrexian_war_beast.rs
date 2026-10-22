//! `cards/creatures/artifacts/mv_3/phyrexian_war_beast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::life;

fn phyrexian_war_beast() -> CardIndex {
    card_index("0c93c418-e10b-4a6d-bddb-3b786bb6a928")
}

/// Phyrexian War Beast ({3} 3/4 Artifact Creature — Phyrexian Beast):
/// "When this creature leaves the battlefield, sacrifice a land and this creature deals 1 damage to you."
#[test]
fn phyrexian_war_beast_ltb_sacrifices_land_and_deals_one_damage_to_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[phyrexian_war_beast(), mountain(), mountain(), forest()],
        )
        .hand(0, &[shatter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let beast = on_battlefield(&engine, p0, phyrexian_war_beast()).unwrap();
    let fst = on_battlefield(&engine, p0, forest()).unwrap();
    let life_before = life(&engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, shatter());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![beast],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![fst] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, phyrexian_war_beast()).is_some());
    assert!(in_graveyard(&engine, p0, forest()).is_some());
    assert_eq!(life(&engine, p0), life_before - 1);
}
