//! `cards/sorceries/mv_5/devastate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn devastate() -> CardIndex {
    card_index("90b3e2df-182d-4dad-9934-597a048fefc6")
}

#[test]
fn devastate_destroys_target_land_and_deals_one_damage_to_each_creature_and_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[devastate()])
        .battlefield(0, &[mountain(); 5])
        .battlefield(1, &[forest(), llanowar_elves(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_forest = on_battlefield(&engine, p1, forest()).unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, devastate());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![opp_forest],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, forest()).is_some());
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
    assert_eq!(engine.state().object(bears).unwrap().damage, 1);
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);
}
