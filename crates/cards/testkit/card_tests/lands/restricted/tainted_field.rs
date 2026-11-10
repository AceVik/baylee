//! `cards/lands/restricted/tainted_field.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The four tainted lands: "{T}: Add {B} or {X}. Activate only if you
/// control a Swamp."
///
/// One ability that makes either of two colours, so what is played here is
/// the choice as well as the clause: the options offered are asserted as a
/// pair, and the answer given is the second of them, because a reader that
/// wrote one colour twice would pass a test that took the first.
#[test]
fn a_tainted_land_offers_both_of_its_colours_and_only_beside_a_swamp() {
    let p0 = PlayerId::new(0);
    for (seed, card, colours) in [
        (930, tainted_field(), [ManaColor::White, ManaColor::Black]),
        (931, tainted_isle(), [ManaColor::Blue, ManaColor::Black]),
        (932, tainted_peak(), [ManaColor::Black, ManaColor::Red]),
        (933, tainted_wood(), [ManaColor::Black, ManaColor::Green]),
    ] {
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &[card])
            .hand(0, &[swamp()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        assert!(
            !offered(&engine, card, 1),
            "seed {seed}: no Swamp, no coloured half"
        );
        play_land(&mut engine, p0, swamp());
        assert!(offered(&engine, card, 1), "seed {seed}: the Swamp arrived");

        activate(&mut engine, p0, card, 1);
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!(
                "seed {seed}: expected a colour to pick, got {:?}",
                engine.pending()
            )
        };
        assert_eq!(
            options,
            colours.to_vec(),
            "seed {seed}: both printed colours"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(colours[1]))
            .expect("the colour offered is a legal answer");
        assert_eq!(
            engine.state().players[0].mana_pool.available(colours[1]),
            1,
            "seed {seed}: the colour chosen is the colour added"
        );
    }
}
