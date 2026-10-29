//! Oracle scenarios for Mystic Penitent.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn mystic_penitent() -> CardIndex {
    card_index("d3d59704-569d-42a7-8b31-8a31b9c0395f")
}

#[test]
fn mystic_penitent_has_vigilance_and_gains_stats_and_flying_with_threshold() {
    let p0 = PlayerId::new(0);
    let mut board = vec![index::PLAINS];
    board.extend_from_slice(&[index::FOREST; 7]);
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(0, &board)
        .hand(0, &[mystic_penitent()])
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

    cast_from_hand(&mut e, p0, mystic_penitent());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, mystic_penitent()).unwrap();
    assert_eq!(pt(&e, id), (1, 1));
    assert!(keywords(&e, id).contains(KeywordSet::VIGILANCE));
    assert!(!keywords(&e, id).contains(KeywordSet::FLYING));

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

    assert_eq!(pt(&e, id), (2, 2));
    assert!(keywords(&e, id).contains(KeywordSet::VIGILANCE));
    assert!(keywords(&e, id).contains(KeywordSet::FLYING));
}
