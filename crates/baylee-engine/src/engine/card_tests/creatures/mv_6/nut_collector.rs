//! Oracle scenarios for Nut Collector.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn nut_collector() -> CardIndex {
    card_index("4e1bc03c-e131-4422-beba-0a45a26fdf43")
}

#[test]
fn nut_collector_creates_squirrel_on_upkeep_and_pumps_squirrels_with_threshold() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![index::FOREST; 6];
    board.extend_from_slice(&[index::PLAINS; 7]);
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &board)
        .hand(0, &[nut_collector()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let plains = all_on_battlefield(&e, p0, index::PLAINS);
    for &p in &plains[..6] {
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

    cast_from_hand(&mut e, p0, nut_collector());
    pass_until(&mut e, stack_is_empty);

    reach_their_main_phase(&mut e, p1);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);

    let squirrels = tokens_of(&e, p0);
    assert_eq!(squirrels.len(), 1);
    let squirrel = squirrels[0];
    assert_eq!(pt(&e, squirrel), (1, 1));

    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            plains[6],
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.apply(p0, PlayerAction::PassPriority).unwrap();

    assert_eq!(pt(&e, squirrel), (3, 3));
}
