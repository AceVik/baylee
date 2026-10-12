#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn smithy_tap_five_refuses_duplicates_six_and_stale_tapped_selection_atomically() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::THOUSAND_MOONS_SMITHY,
                index::SOL_RING,
                index::SOL_RING,
                index::SOL_RING,
                index::GRIZZLY_BEARS,
                index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 6);
    let mut duplicate = options[..4].to_vec();
    duplicate.push(options[0]);
    for selected in [duplicate, options.clone()] {
        let before = e.snapshot_hash();
        assert!(
            e.apply(P0, PlayerAction::ChooseObjects { objects: selected })
                .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
        assert!(options.iter().all(|id| !is_tapped(&e, *id)));
    }
    // A selection is checked against current state, not just cached options.
    e.state.set_tapped(options[0], true);
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: options[..5].to_vec()
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    e.state.set_tapped(options[0], false);
    let smithy = source(&e, P0, index::THOUSAND_MOONS_SMITHY);
    let bear = source(&e, P0, index::GRIZZLY_BEARS);
    // Transparent sickness fixture: nonsymbol tap cost must accept it.
    let acquired = e.state.next_timestamp();
    e.state.object_mut(bear).unwrap().controlled_since = acquired;
    assert!(crate::combat::summoning_sick(
        &e.state,
        e.state.object(bear).unwrap()
    ));
    let mut selected = vec![smithy, bear];
    selected.extend(
        options
            .iter()
            .copied()
            .filter(|id| *id != smithy && *id != bear)
            .take(3),
    );
    choose(&mut e, selected.clone());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(face(&e, smithy), 1);
    assert!(
        selected.iter().all(|id| is_tapped(&e, *id)),
        "self and sick creature pay together"
    );
}

#[test]
fn smithy_gnome_count_tracks_its_new_controller_after_actual_control_magic() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, &[index::THOUSAND_MOONS_SMITHY])
        .hand(1, &[index::CONTROL_MAGIC])
        .battlefield(0, &[index::SOL_RING])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast(&mut e, index::THOUSAND_MOONS_SMITHY);
    pass_until(&mut e, stack_is_empty);
    let gnome = e
        .state
        .battlefield_seen()
        .find(|id| {
            e.state.object(*id).is_some_and(|o| {
                o.characteristics()
                    .subtypes
                    .contains(subtypes::creature::GNOME)
            })
        })
        .unwrap();
    assert_eq!(pt(&e, gnome), (3, 3));
    let turn = e.state.turn.number;
    pass_until(&mut e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == P1
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player: P1, .. })
    });
    for color in ManaColor::ALL {
        e.state.players[1].mana_pool.add(color, 10);
    }
    e.refresh_offer();
    let aura = in_hand(&e, P1, index::CONTROL_MAGIC).unwrap();
    e.apply(P1, PlayerAction::CastSpell { card: aura }).unwrap();
    targets(&mut e, vec![gnome], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(gnome).unwrap().controller, P1);
    assert_eq!(
        pt(&e, gnome),
        (1, 1),
        "Control Magic is an enchantment, not artifact/creature; count only self"
    );
}
