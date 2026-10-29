//! Oracle scenarios for Springing Tiger.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn springing_tiger_gets_plus_2_plus_2_with_threshold() {
    let p0 = PlayerId::new(0);
    let card = card_index("2f8c4805-694d-4408-8eb5-9fb7ce278ded");
    let mut board = vec![card];
    board.extend_from_slice(&[index::PLAINS; 7]);
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &board)
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

    let tiger = on_battlefield(&e, p0, card).unwrap();
    assert_eq!(pt(&e, tiger), (3, 3));

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

    assert_eq!(pt(&e, tiger), (5, 5));
}
