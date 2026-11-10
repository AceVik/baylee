//! Quirion Trailblazer — {3}{G} — Creature — Elf Scout
//! Oracle: When this creature enters, you may search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.

use super::*;

#[test]
fn quirion_trailblazer_searches_basic_land_onto_battlefield_tapped() {
    let trailblazer = card_index("a26c4076-c488-4700-8e32-4ceeeac2c29a");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[trailblazer])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, trailblazer);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    let Pending::ChooseCards { options, .. } = engine.pending() else {
        panic!("expected library search choice");
    };
    let picked = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![picked],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, trailblazer).is_some());
    let plains_id = on_battlefield(&engine, p0, plains()).expect("searched Plains on battlefield");
    assert!(
        engine
            .state()
            .object(plains_id)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}
