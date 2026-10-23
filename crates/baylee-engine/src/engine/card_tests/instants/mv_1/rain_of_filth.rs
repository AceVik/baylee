#[allow(clippy::wildcard_imports)]
use super::*;

fn rain_of_filth() -> CardIndex {
    card_index("aa77980d-0434-40f0-990f-1c8c50610174")
}

#[test]
fn rain_of_filth_sacrifices_land_to_add_black_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[rain_of_filth()])
        .battlefield(0, &[swamp(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let swamp_id = on_battlefield(&engine, p0, swamp()).expect("swamp on bf");
    cast_from_hand(&mut engine, p0, rain_of_filth());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![swamp_id],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
}
