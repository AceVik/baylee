//! `cards/creatures/mv_8/tunneler_wurm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn tunneler_wurm_discards_a_card_to_regenerate() {
    let wurm = card_index("31370fdf-c666-40d2-a7ff-3ba1dfe350cd");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[wurm, grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tw = cast_and_settle(&mut engine, wurm);
    assert_eq!(pt(&engine, tw), (6, 6));
    let discard_card = in_hand(&engine, p0, grizzly_bears()).expect("Grizzly Bears in hand");

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: tw,
                ability_index: 0,
            },
        )
        .expect("activate Tunneler Wurm regenerate ability");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![discard_card],
            },
        )
        .expect("discard Grizzly Bears to pay cost");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(discard_card).unwrap().zone,
        Zone::Graveyard
    );
    assert!(
        engine.state().object(tw).unwrap().regeneration_shields > 0,
        "Tunneler Wurm gained a regeneration shield"
    );
}
