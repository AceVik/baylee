//! Oracle scenarios for Crashing Centaur.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn crashing_centaur() -> CardIndex {
    card_index("d13a9cb1-1c76-4ac1-80d8-3c471753a588")
}

#[test]
fn crashing_centaur_activates_trample_and_gains_stats_and_shroud_with_threshold() {
    let p0 = PlayerId::new(0);
    let mut board = vec![index::FOREST; 7];
    board.extend_from_slice(&[index::PLAINS; 6]);
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &board)
        .hand(0, &[crashing_centaur(), index::FOREST])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let plains = all_on_battlefield(&e, p0, index::PLAINS);
    for &p in &plains {
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                p,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    e.refresh_offer();

    cast_from_hand(&mut e, p0, crashing_centaur());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, crashing_centaur()).unwrap();
    assert_eq!(pt(&e, id), (3, 4));
    assert!(!keywords(&e, id).contains(KeywordSet::TRAMPLE));
    assert!(!keywords(&e, id).contains(KeywordSet::SHROUD));

    let discard_card = in_hand(&e, p0, index::FOREST).unwrap();
    activate(&mut e, p0, crashing_centaur(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![discard_card],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(keywords(&e, id).contains(KeywordSet::TRAMPLE));
    assert_eq!(
        pt(&e, id),
        (5, 6),
        "discarding the 7th card into the graveyard turns on threshold (+2/+2)"
    );
    assert!(
        keywords(&e, id).contains(KeywordSet::SHROUD),
        "discarding the 7th card into the graveyard turns on threshold shroud"
    );
}
