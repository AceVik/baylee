//! Oracle scenarios for Balshan Griffin.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn balshan_griffin() -> CardIndex {
    card_index("0ef74c49-0825-4201-b23b-dc6c6bbadacb")
}

#[test]
fn balshan_griffin_has_flying_and_activated_bounce_discards_a_card() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, index::ISLAND)
        .battlefield(0, &[index::ISLAND; 7])
        .hand(0, &[balshan_griffin(), index::FOREST])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, balshan_griffin());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, balshan_griffin()).unwrap();
    assert_eq!(pt(&e, id), (3, 2));
    assert!(keywords(&e, id).contains(KeywordSet::FLYING));

    let discard_card = in_hand(&e, p0, index::FOREST).unwrap();
    activate(&mut e, p0, balshan_griffin(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![discard_card],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, p0, balshan_griffin()).is_none());
    assert!(in_hand(&e, p0, balshan_griffin()).is_some());
    assert!(in_graveyard(&e, p0, index::FOREST).is_some());
}
