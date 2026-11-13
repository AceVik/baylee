//! `cards/lands/tapland/underground_mortuary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn underground_mortuary() -> CardIndex {
    card_index("840119bf-e60f-4ff7-9c9b-d420d09df545")
}

/// A real land drop enters tapped and puts surveil 1 on the stack. After the
/// next untap step, the intrinsic Swamp/Forest ability produces either color.
#[test]
fn underground_mortuary_enters_tapped_surveils_and_taps_for_black_or_green() {
    let p0 = PlayerId::new(0);
    for (seed, color) in [(8_401, ManaColor::Black), (8_402, ManaColor::Green)] {
        let mut engine = Duel::new(seed, forest())
            .hand(0, &[underground_mortuary()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let library_before = library_size(&engine, p0);

        let land = play_land(&mut engine, p0, underground_mortuary());
        assert!(is_tapped(&engine, land), "the printed tapped entry");
        let (cards, _) = surveil_offer(&mut engine, p0);
        assert_eq!(cards.len(), 1, "surveil 1 looks at exactly one card");
        engine.apply(p0, look_answer(&cards, &[])).unwrap();
        assert_eq!(library_size(&engine, p0), library_before);
        assert!(in_graveyard(&engine, p0, forest()).is_none());

        cross_into_the_next_own_main(&mut engine, p0);
        assert!(
            !is_tapped(&engine, land),
            "the next untap step stands it up"
        );
        activate(&mut engine, p0, underground_mortuary(), 0);
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("the two-color ability asks for a color");
        };
        assert_eq!(options, vec![ManaColor::Black, ManaColor::Green]);
        engine.apply(p0, PlayerAction::ChooseColor(color)).unwrap();
        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(pool.total(), 1);
        assert_eq!(pool.available(color), 1);
        assert!(is_tapped(&engine, land));
        assert!(stack_is_empty(&engine), "a mana ability uses no stack");
    }
}
