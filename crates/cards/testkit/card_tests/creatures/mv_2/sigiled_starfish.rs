//! Oracle checklist: tap to scry exactly 1; choose bottoming and keeping in separate runs; no card is drawn and Starfish stays tapped.
#[allow(clippy::wildcard_imports)]
use super::*;
fn sigiled_starfish() -> CardIndex {
    card_index("94a4f70a-8aac-412a-99f0-933e3ff87fe0")
}
#[test]
fn starfish_scry_one_can_keep_or_bottom_the_looked_card() {
    let p0 = PlayerId::new(0);
    for bottom in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[sigiled_starfish()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, sigiled_starfish()).unwrap();
        let top = *e
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .unwrap();
        let library_size = e.state().zones.list(ZoneLocation::Library(p0)).len();
        let hand = e.state().zones.list(ZoneLocation::Hand(p0)).len();
        activate(&mut e, p0, sigiled_starfish(), 0);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
        let Pending::Arrange {
            cards,
            piles,
            prompt,
            ..
        } = e.pending().clone()
        else {
            panic!("scry asks for a library arrangement")
        };
        assert_eq!(prompt, ArrangePrompt::Scry);
        assert_eq!(piles, scry_piles(1));
        assert_eq!(cards, vec![top]);
        let away = if bottom { vec![top] } else { vec![] };
        e.apply(p0, look_answer(&cards, &away))
            .expect("valid scry arrangement");
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), hand);
        let cards = e.state().zones.list(ZoneLocation::Library(p0));
        assert_eq!(cards.len(), library_size, "scry does not draw or mill");
        if bottom {
            assert_eq!(cards.first(), Some(&top), "the looked card is now bottom");
        } else {
            assert_eq!(cards.last(), Some(&top), "the looked card stays on top");
        }
        assert!(is_tapped(&e, source));
    }
}
