//! Oracle scenarios for Dirty Wererat.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn dirty_wererat() -> CardIndex {
    card_index("7fbd5f69-f6b4-4ad7-8e9e-47a798324f6c")
}

#[test]
fn dirty_wererat_regenerates_by_discarding_and_gains_threshold_stats_and_cant_block() {
    let p0 = PlayerId::new(0);
    let mut board = vec![index::SWAMP; 5];
    board.extend_from_slice(&[index::FOREST; 6]);
    board.push(index::MOUNTAIN);
    board.push(index::MOUNTAIN);
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(0, &board)
        .hand(
            0,
            &[
                dirty_wererat(),
                index::FOREST,
                index::LIGHTNING_BOLT,
                index::SHOCK,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let forests = all_on_battlefield(&e, p0, index::FOREST);
    for &f in &forests {
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                f,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    e.refresh_offer();

    cast_from_hand(&mut e, p0, dirty_wererat());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, dirty_wererat()).unwrap();
    assert_eq!(pt(&e, id), (2, 3));
    assert!(!keywords(&e, id).contains(KeywordSet::CANT_BLOCK));

    let discard_card = in_hand(&e, p0, index::FOREST).unwrap();
    activate(&mut e, p0, dirty_wererat(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![discard_card],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(pt(&e, id), (4, 5));
    assert!(keywords(&e, id).contains(KeywordSet::CANT_BLOCK));
    assert_eq!(e.state().object(id).unwrap().regeneration_shields, 1);
}
