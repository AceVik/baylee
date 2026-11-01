//! Oracle scenarios for Childhood Horror.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn childhood_horror() -> CardIndex {
    card_index("29bebbaf-d47c-4369-8ff9-81816758da86")
}

#[test]
fn childhood_horror_gets_plus_two_plus_two_and_cant_block_with_threshold() {
    let p0 = PlayerId::new(0);
    let mut board = vec![index::SWAMP; 4];
    board.extend_from_slice(&[index::FOREST; 7]);
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(0, &board)
        .hand(0, &[childhood_horror()])
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

    cast_from_hand(&mut e, p0, childhood_horror());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, childhood_horror()).unwrap();
    assert_eq!(pt(&e, id), (2, 2));
    assert!(keywords(&e, id).contains(KeywordSet::FLYING));
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

    assert_eq!(pt(&e, id), (4, 4));
    assert!(keywords(&e, id).contains(KeywordSet::FLYING));
    assert!(keywords(&e, id).contains(KeywordSet::CANT_BLOCK));
}
