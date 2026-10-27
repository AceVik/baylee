//! Oracle scenarios for Nimble Mongoose.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn nimble_mongoose() -> CardIndex {
    card_index("1059f089-2693-4c89-92c5-39fe49beebb3")
}

#[test]
fn nimble_mongoose_has_shroud_and_gets_plus_two_plus_two_with_threshold() {
    let p0 = PlayerId::new(0);
    let mut board = vec![index::FOREST];
    board.extend_from_slice(&[index::PLAINS; 7]);
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &board)
        .hand(0, &[nimble_mongoose()])
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

    cast_from_hand(&mut e, p0, nimble_mongoose());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, nimble_mongoose()).unwrap();
    assert_eq!(pt(&e, id), (1, 1));
    assert!(keywords(&e, id).contains(KeywordSet::SHROUD));

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

    assert_eq!(pt(&e, id), (3, 3));
    assert!(keywords(&e, id).contains(KeywordSet::SHROUD));
}
