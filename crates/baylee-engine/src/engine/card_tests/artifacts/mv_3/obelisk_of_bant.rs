//! Oracle checklist: tap produces one chosen mana from {G,W,U}; each listed color works and black/red are absent.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::mana::ManaColor;
fn obelisk_of_bant() -> CardIndex {
    card_index("3f96dfd3-975a-4595-95c1-d2f04a86cb37")
}
#[test]
fn bant_obelisk_produces_each_listed_color() {
    let p0 = PlayerId::new(0);
    for color in [ManaColor::Green, ManaColor::White, ManaColor::Blue] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[obelisk_of_bant()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, obelisk_of_bant()).unwrap();
        assert!(!is_tapped(&e, source));
        activate(&mut e, p0, obelisk_of_bant(), 0);
        let Pending::ChooseColor { options, .. } = e.pending().clone() else {
            panic!("choose one of the three printed colors")
        };
        assert_eq!(
            options,
            vec![ManaColor::Green, ManaColor::White, ManaColor::Blue]
        );
        e.apply(p0, PlayerAction::ChooseColor(color))
            .expect("listed color is legal");
        assert!(is_tapped(&e, source));
        assert_eq!(e.state().players[0].mana_pool.available(color), 1);
    }
}
