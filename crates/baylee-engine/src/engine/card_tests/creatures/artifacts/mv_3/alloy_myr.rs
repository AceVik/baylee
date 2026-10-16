//! `cards/creatures/artifacts/mv_3/alloy_myr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn alloy_myr() -> CardIndex {
    card_index("efb0394c-2a45-4dd8-bca3-08704056fa31")
}

#[test]
fn alloy_myr_taps_for_one_mana_of_each_chosen_color() {
    let p0 = PlayerId::new(0);
    for color in ManaColor::ALL[..5].iter().copied() {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[alloy_myr()])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let myr = on_battlefield(&e, p0, alloy_myr()).unwrap();
        let Pending::Priority { legal, .. } = e.pending().clone() else {
            panic!("expected priority");
        };
        assert!(
            legal.abilities.contains(&(myr, 0)),
            "the printed mana ability is offered by its ability index"
        );
        activate(&mut e, p0, alloy_myr(), 0);
        let Pending::ChooseColor { player, options } = e.pending().clone() else {
            panic!("expected mana color choice: {:?}", e.pending());
        };
        assert_eq!(player, p0);
        assert_eq!(options, ManaColor::ALL[..5].to_vec());
        e.apply(p0, PlayerAction::ChooseColor(color)).unwrap();
        assert!(
            stack_is_empty(&e),
            "a mana ability resolves without using the stack"
        );
        assert!(is_tapped(&e, myr), "{{T}} is paid");
        assert_eq!(e.state().players[0].mana_pool.available(color), 1);
        assert_eq!(e.state().players[0].mana_pool.total(), 1);
        assert_eq!(
            tap_all_mana(&mut e, p0),
            0,
            "the tapped Myr cannot produce a second mana"
        );
        assert_eq!(e.state().players[0].mana_pool.total(), 1);
    }
}

/// Alloy Myr — "{T}: Add one mana of any color": the ability asks which color
/// and gives exactly one mana of it.
#[test]
fn alloy_myr_taps_for_one_mana_of_the_color_named() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[alloy_myr()]);
    let myr = on_battlefield(&engine, p0, alloy_myr()).expect("seated");
    assert_eq!(pt(&engine, myr), (2, 2));

    tap_for_any_color(&mut engine, myr, ManaColor::Red);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(stack_is_empty(&engine), "a mana ability does not use the stack");
}
