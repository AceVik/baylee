//! Oracle checklist: {T} cost; choose any of five colors; exactly one mana of chosen color is added; tapped Myr cannot activate again.
#[allow(clippy::wildcard_imports)]
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
