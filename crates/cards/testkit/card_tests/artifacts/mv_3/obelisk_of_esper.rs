//! Oracle checklist: tap produces one chosen mana from {W,U,B}; each of the three choices is real and color-correct; no red/green choice is offered.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::mana::ManaColor;
fn obelisk_of_esper() -> CardIndex {
    card_index("9da963f6-734b-4d3c-a3be-dd5cca2c7b19")
}
#[test]
fn esper_obelisk_produces_each_listed_color() {
    let p0 = PlayerId::new(0);
    for color in [ManaColor::White, ManaColor::Blue, ManaColor::Black] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[obelisk_of_esper()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, obelisk_of_esper()).unwrap();
        assert!(!is_tapped(&e, source));
        activate(&mut e, p0, obelisk_of_esper(), 0);
        let Pending::ChooseColor { options, .. } = e.pending().clone() else {
            panic!("choose one of the three printed colors")
        };
        assert_eq!(
            options,
            vec![ManaColor::White, ManaColor::Blue, ManaColor::Black]
        );
        e.apply(p0, PlayerAction::ChooseColor(color))
            .expect("listed color is legal");
        assert!(is_tapped(&e, source));
        assert_eq!(e.state().players[0].mana_pool.available(color), 1);
    }
}
