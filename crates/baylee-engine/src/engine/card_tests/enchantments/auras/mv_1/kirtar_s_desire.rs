//! Oracle scenarios for Kirtar's Desire.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn kirtar_s_desire() -> CardIndex {
    card_index("4f21a664-96e3-49be-ba9f-4af4b6bd7475")
}

#[test]
fn kirtar_s_desire_prevents_attacking_and_prevents_blocking_with_threshold() {
    let p0 = PlayerId::new(0);
    let mut board = vec![index::PLAINS, index::GRIZZLY_BEARS];
    board.extend_from_slice(&[index::FOREST; 7]);
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(0, &board)
        .hand(0, &[kirtar_s_desire()])
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

    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut e, p0, kirtar_s_desire());
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bears],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(keywords(&e, bears).contains(KeywordSet::CANT_ATTACK));
    assert!(!keywords(&e, bears).contains(KeywordSet::CANT_BLOCK));

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

    assert!(keywords(&e, bears).contains(KeywordSet::CANT_ATTACK));
    assert!(keywords(&e, bears).contains(KeywordSet::CANT_BLOCK));
}
