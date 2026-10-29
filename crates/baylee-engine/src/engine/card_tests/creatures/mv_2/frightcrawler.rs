//! Oracle scenarios for Frightcrawler.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn frightcrawler() -> CardIndex {
    card_index("96b611e9-38cc-4326-bb38-73084e99e6f5")
}

#[test]
fn frightcrawler_has_fear_and_gains_stats_and_cant_block_with_threshold() {
    let p0 = PlayerId::new(0);
    let mut board = vec![index::SWAMP, index::SWAMP];
    board.extend_from_slice(&[index::FOREST; 7]);
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(0, &board)
        .hand(0, &[frightcrawler()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let forests = all_on_battlefield(&e, p0, index::FOREST);
    for &f in &forests[..6] {
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

    cast_from_hand(&mut e, p0, frightcrawler());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, frightcrawler()).unwrap();
    assert_eq!(pt(&e, id), (1, 1));
    assert!(keywords(&e, id).contains(KeywordSet::FEAR));
    assert!(!keywords(&e, id).contains(KeywordSet::CANT_BLOCK));

    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            forests[6],
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.apply(p0, PlayerAction::PassPriority).unwrap();

    assert_eq!(pt(&e, id), (3, 3));
    assert!(keywords(&e, id).contains(KeywordSet::FEAR));
    assert!(keywords(&e, id).contains(KeywordSet::CANT_BLOCK));
}
