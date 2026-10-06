//! `cards/lands/utility/dungeon_descent.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dungeon Descent and Howltooth Hollow are the two remaining shelf taplands
/// with one colour between them, and they are here for the same pair the
/// other three were: entering tapped and then making the colour printed
/// beside it, neither being worth asserting alone.
///
/// Both carry a `Coverage::Partial` for a mechanic with no vocabulary at all
/// — venturing into the dungeon, and hideaway — and both of those hang off a
/// **second** activated ability. So the ability list is read: one entry
/// each, which is the gap made countable rather than described.
///
/// The board is what makes that count mean something. `can_afford` gates
/// every entry in the offer, so a count taken where the second ability could
/// not have been paid for would keep reading one after the ability was
/// written. Dungeon Descent's is "{4}, {T}, Tap an untapped legendary
/// creature you control", so four Forests and Jin-Gitaxias stand beside it;
/// Howltooth Hollow's is "{B}, {T}", and its own tap is part of that price,
/// so the black comes from a Swamp. Both are asked in a main phase with an
/// empty stack, which is the sorcery timing one of them needs.
#[test]
fn the_last_two_shelf_taplands_enter_tapped_and_offer_one_ability_each() {
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
            1,
            "the mana ability, and not the venture or hideaway one beside it \
             — whose price is floating in this pool"
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
