//! Oracle scenarios for Divine Sacrament.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn divine_sacrament() -> CardIndex {
    card_index("92bd7fe2-bb70-40fb-984b-26e461347c56")
}

#[test]
fn divine_sacrament_pumps_white_creatures_and_adds_bonus_with_threshold() {
    let p0 = PlayerId::new(0);
    let mut board = vec![index::PLAINS; 3];
    board.extend_from_slice(&[index::FOREST; 7]);
    board.push(index::SAVANNAH_LIONS);
    board.push(index::GRIZZLY_BEARS);
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(0, &board)
        .hand(0, &[divine_sacrament()])
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

    let lions = on_battlefield(&e, p0, index::SAVANNAH_LIONS).unwrap();
    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut e, p0, divine_sacrament());
    pass_until(&mut e, stack_is_empty);

    assert_eq!(pt(&e, lions), (3, 2));
    assert_eq!(pt(&e, bears), (2, 2));

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

    assert_eq!(pt(&e, lions), (4, 3));
    assert_eq!(pt(&e, bears), (2, 2));
}
