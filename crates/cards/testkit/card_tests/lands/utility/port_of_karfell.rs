//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn karfell_chooses_after_milling_and_returns_the_chosen_creature_tapped() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[port_of_karfell()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, port_of_karfell()).unwrap();
    let bear = hand_to_library_top(&mut e, p0, index::GRIZZLY_BEARS);
    let before = library_size(&e, p0);
    let state = e.dev_state_mut(p0).unwrap();
    state.set_tapped(land, false);
    for (color, n) in [
        (ManaColor::Colorless, 3),
        (ManaColor::Blue, 1),
        (ManaColor::Black, 2),
    ] {
        state.players[0].mana_pool.add(color, n);
    }
    e.refresh_offer();
    activate(&mut e, p0, port_of_karfell(), 1);
    assert!(
        matches!(e.pending(), Pending::Priority { .. }),
        "no target is chosen on activation"
    );
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Graveyard);
    assert_eq!(library_size(&e, p0), before);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending()
    else {
        panic!("choice")
    };
    assert_eq!(options, &vec![bear]);
    assert_eq!((*min, *max), (1, 1));
    let hash = e.state().snapshot_hash();
    assert!(
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land]
            }
        )
        .is_err()
    );
    assert_eq!(e.state().snapshot_hash(), hash);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bear],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(library_size(&e, p0), before - 4);
    assert!(is_tapped(&e, bear));
    assert_eq!(e.state().object(bear).unwrap().zone, Zone::Battlefield);
}
