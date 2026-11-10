//! `cards/lands/cycling/umbral_expanse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Umbral Expanse is a `Land — Plains Swamp` that enters tapped and prints
/// two lines besides its types: `{T}: Add {W} or {B}` off those types, and
/// cycling `{2}`. Both halves are played here, and each is the other's
/// control — a permanent that entered tapped can pay no `{T}`, so the two
/// untapped Forests beside it float exactly two and the Expanse contributes
/// nothing, while the second copy is spent off that same pool, out of the
/// hand, for exactly the card that was on top of the library. The turn after
/// is where the mana line comes back, and the question it cannot be resolved
/// without names both of the types the land prints and no third.
#[test]
#[allow(clippy::too_many_lines)] // one card, and both of its printed halves
fn umbral_expanse_enters_tapped_cycles_from_hand_and_taps_for_either_type() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[umbral_expanse(), umbral_expanse()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The land drop is a real entry: "enters tapped" is a replacement effect,
    // so a board seeded through `starting_battlefield` — which places a
    // permanent rather than playing one — could not show it.
    let land = play_land(&mut engine, p0, umbral_expanse());
    assert!(entered_tapped(&engine, land), "this land enters tapped");
    assert_eq!(
        lands_of(&engine, p0).len(),
        3,
        "two Forests and the Expanse"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        deeds(&legal, &[land]).is_empty(),
        "nothing a permanent prints can be paid with a {{T}} it does not have, \
         whichever list the engine files its mana in: {:?}",
        legal.abilities
    );

    // Money first, then the claim: an offer is read off the pool, and the
    // land that entered tapped is not in it.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests, and nothing at all from the land that entered tapped"
    );

    // Cycling {2} — the second printed line, activated out of the hand and
    // not off the battlefield.
    let copy = in_hand(&engine, p0, umbral_expanse()).expect("the second copy is still in hand");
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("p0 has a library");
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let hand_routes = deeds(&legal, &[copy]);
    assert!(
        matches!(hand_routes[..], [(0, Deed::Ability(1))]),
        "the card in hand offers cycling and not its {{T}}, which is the whole \
         of what `ActivationZone::Hand` buys it: {:?}",
        legal.abilities
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: copy,
                ability_index: 1,
            },
        )
        .expect("the ability the offer named is the ability that runs");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, umbral_expanse()).is_some(),
        "\"Discard this card\" is half the price, and the cycled copy lands in \
         the graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and {{2}} was the other half"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" takes exactly one off the top"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and it is the card that was on top of the library, not merely a card \
         of the same name"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card discarded and one drawn"
    );

    // A turn cycle, because the untap step is the only thing that gives a
    // {{T}} back.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "p0's untap step stood the Expanse back up"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let routes = deeds(&legal, &[land]);
    assert!(
        !routes.is_empty(),
        "an untapped Plains Swamp taps for one of the two colours it prints: {legal:?}"
    );
    engine
        .apply(p0, routes[0].1.action(land))
        .expect("the route the offer named is the route that runs");
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`{{T}}: Add {{W}} or {{B}}` is a choice of two colours and cannot \
             be resolved without being asked, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped names the colour");
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "both of the types the land prints: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and no third — the Forest beside it is a different land: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap: the two Forests are still standing"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the Expanse paid its own {{T}}");
}
