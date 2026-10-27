//! Oracle scenarios for Metamorphic Wurm.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn metamorphic_wurm() -> CardIndex {
    card_index("512477f0-1083-423b-9675-a8bc852f7d88")
}

#[test]
fn metamorphic_wurm_gets_plus_four_plus_four_with_threshold() {
    let p0 = PlayerId::new(0);
    let mut board = vec![index::FOREST; 5];
    board.extend_from_slice(&[index::PLAINS; 7]);
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &board)
        .hand(0, &[metamorphic_wurm()])
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

    cast_from_hand(&mut e, p0, metamorphic_wurm());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, metamorphic_wurm()).unwrap();
    assert_eq!(pt(&e, id), (3, 3));

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

    assert_eq!(pt(&e, id), (7, 7));
}
