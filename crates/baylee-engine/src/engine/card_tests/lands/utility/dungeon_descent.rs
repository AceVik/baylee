//! `cards/lands/utility/dungeon_descent.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Both lands enter tapped and make their printed mana. Howltooth's linked
/// play is now a second activated ability; Dungeon Descent remains Partial.
/// Float each complete activation price so the offer count tests implemented
/// abilities rather than accidentally counting unaffordable ones as absent.
#[test]
fn dungeon_and_howltooth_enter_tapped_and_offer_their_implemented_abilities() {
    let p0 = PlayerId::new(0);

    for (card, colour, payment) in [
        (
            dungeon_descent(),
            ManaColor::Colorless,
            &[forest(), forest(), forest(), forest(), jin_gitaxias()][..],
        ),
        (howltooth_hollow(), ManaColor::Black, &[swamp()][..]),
    ] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, payment)
            .hand(0, &[card])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
        let land = play_land(&mut engine, p0, card);
        assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");

        cross_into_the_next_own_main(&mut engine, p0);
        // The price is **floated**, not merely stood on the board:
        // `can_afford` reads the pool, so an ability with any mana cost at
        // all is absent from an offer taken over untapped lands — which is
        // how this pin first passed for the wrong reason. Everything but the
        // land itself is tapped, because the land is what both the mana
        // ability and the missing one tap.
        tap_mana_except(&mut engine, p0, land);
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        assert_eq!(
            legal.abilities.iter().filter(|(id, _)| *id == land).count(),
            if card == howltooth_hollow() { 2 } else { 1 },
            "the complete activation price is floating in the pool"
        );

        let before = engine.state().players[0].mana_pool.available(colour);
        let before_total = engine.state().players[0].mana_pool.total();
        activate(&mut engine, p0, card, 0);
        assert_eq!(
            engine.state().players[0].mana_pool.available(colour),
            before + 1,
            "the one colour its mana ability prints"
        );
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            before_total + 1
        );
    }
}
