//! Oracle scenarios for Amugaba.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn amugaba() -> CardIndex {
    card_index("73a1552e-bbfa-445e-8306-b347354c0364")
}

#[test]
fn amugaba_has_flying_and_activated_bounce_discards_a_card() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, index::ISLAND)
        .battlefield(0, &[index::ISLAND; 10])
        .hand(0, &[amugaba(), index::FOREST])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, amugaba());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, amugaba()).unwrap();
    assert_eq!(pt(&e, id), (6, 6));
    assert!(keywords(&e, id).contains(KeywordSet::FLYING));

    let discard_card = in_hand(&e, p0, index::FOREST).unwrap();
    activate(&mut e, p0, amugaba(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![discard_card],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, p0, amugaba()).is_none());
    assert!(in_hand(&e, p0, amugaba()).is_some());
    assert!(in_graveyard(&e, p0, index::FOREST).is_some());
}
