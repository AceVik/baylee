//! Frenzied Tilling — {3}{R}{G} — Sorcery
//! Oracle: Destroy target land. Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.

use super::*;
use baylee_core::generated::index;

#[test]
fn frenzied_tilling_destroys_target_land_and_searches_basic_land_onto_battlefield_tapped() {
    let tilling = card_index("c6b915b3-b504-42fb-8059-62a3e8e224e1");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[mountain(), forest(), forest(), forest(), forest()])
        .battlefield(1, &[index::CITY_OF_BRASS])
        .hand(0, &[tilling])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let opp_land_id = on_battlefield(&engine, p1, index::CITY_OF_BRASS).unwrap();
    assert!(on_battlefield(&engine, p0, plains()).is_none());

    cast_from_hand(&mut engine, p0, tilling);
    aim_at(&mut engine, p0, opp_land_id);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards, got {:?}", engine.pending());
    };
    let chosen_plains = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen_plains],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::CITY_OF_BRASS).is_none());
    assert!(in_graveyard(&engine, p1, index::CITY_OF_BRASS).is_some());
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
