//! Oracle scenarios for Krosan Beast.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn krosan_beast() -> CardIndex {
    card_index("042a665d-2f63-4f7e-ac89-0d560da68085")
}

#[test]
fn krosan_beast_gets_plus_seven_plus_seven_with_threshold() {
    let p0 = PlayerId::new(0);
    let mut board = vec![index::FOREST; 4];
    board.extend_from_slice(&[index::PLAINS; 7]);
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &board)
        .hand(0, &[krosan_beast()])
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

    cast_from_hand(&mut e, p0, krosan_beast());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, krosan_beast()).unwrap();
    assert_eq!(pt(&e, id), (1, 1));

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

    assert_eq!(pt(&e, id), (8, 8));
}
