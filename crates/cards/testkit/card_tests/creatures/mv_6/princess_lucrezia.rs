//! `cards/creatures/mv_6/princess_lucrezia.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Princess Lucrezia — {3}{U}{U}{B}, a legendary 5/4 Human Wizard whose
/// entire text is one mana ability: "{T}: Add {U}."
///
/// Six lands pay her cost down to an empty pool, so the single blue that
/// appears afterwards can only have come off her own tap — every Island on
/// the board is already spent on the cast, which is what makes "one blue and
/// nothing else" a statement about her and not about a land that happened to
/// be untapped. The colour is *fixed*, so the activation asks nothing on the
/// way and the mana lands with an empty stack (CR 605.3b), and since the
/// whole price is her own `{T}` the line is gone from the offer the moment
/// she is spent.
#[test]
fn princess_lucrezia_taps_for_one_blue_and_offers_nothing_once_she_is_spent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(921, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), swamp(), swamp()],
        )
        .hand(0, &[princess_lucrezia()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {3}{U}{U}{B} is exactly four Islands and two Swamps, so the cast spends
    // every mana source on the board and the pool reads empty once she has
    // resolved.
    cast_from_hand(&mut engine, p0, princess_lucrezia());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let wizard = on_battlefield(&engine, p0, princess_lucrezia()).expect("she resolved");
    assert_eq!(pt(&engine, wizard), (5, 4), "the body the card prints");
    assert!(
        types(&engine, wizard).contains(TypeSet::CREATURE),
        "she is the creature she prints"
    );
    assert!(
        engine
            .state()
            .object(wizard)
            .expect("she is an object")
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY),
        "Princess Lucrezia is legendary"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "six lands paid the {{3}}{{U}}{{U}}{{B}} to the last mana"
    );
    for land in all_on_battlefield(&engine, p0, island()) {
        assert!(
            is_tapped(&engine, land),
            "every Island is spent on the cast"
        );
    }

    // A whole turn cycle: only then is her own {T} the price of anything
    // (CR 302.6). The untap step stands the lands back up as well, which
    // costs the reading nothing — none of them is tapped below.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, wizard), "the untap step stood her up");

    // The whole price of the printed line is her own {T}, so it is offered
    // without a single mana floating. It is a mana ability a card *prints*, so
    // it is an ordinary `(source, index)` entry and not the CR 305.6 shortcut
    // a basic land uses.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(wizard, 0)),
        "an untapped Princess Lucrezia is a paid {{T}}: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, princess_lucrezia(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "`{{U}}` is fixed, so nothing is asked on the way (CR 605.1), got {:?}",
        engine.pending()
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "{{T}}: Add {{U}} — one blue, and nothing else on this board was tapped"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "no Swamp was tapped, so the one mana is hers alone"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, wizard), "she paid her own {{T}}");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(wizard, 0)),
        "a tapped Princess Lucrezia has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
}
