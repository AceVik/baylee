//! `cards/lands/crystal_quarry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crystal Quarry prints two mana abilities and no body: `{T}: Add {C}`, and
/// `{5}, {T}: Add {W}{U}{B}{R}{G}`. The second line is the card, and its price
/// is the thing a board can prove — four floating Plains leave the ability out
/// of the offer entirely, a fifth puts it there, and the activation drains the
/// five that paid while returning exactly one mana of each colour. The cheap
/// line is played first on the same board, so the Quarry's own `{T}` is read
/// before it is committed to the expensive one; the two need separate turns
/// only because one permanent pays its tap symbol once.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn crystal_quarry_wants_five_before_handing_over_one_mana_of_every_color() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[crystal_quarry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let quarry = play_land(&mut engine, p0, crystal_quarry());
    assert!(!is_tapped(&engine, quarry), "a land is played untapped");
    let offered = |engine: &Engine<RegistryLookup>| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. } if legal.abilities.contains(&(quarry, 1))
        )
    };
    assert!(
        !offered(&engine),
        "an empty pool cannot pay {{5}}, so the second ability is not offered"
    );

    // `{T}: Add {C}` — ability 0, whose whole price is the tap.
    activate(&mut engine, p0, crystal_quarry(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{T}}: Add {{C}}");
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap, and no land was asked for it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, quarry), "the Quarry paid its own {{T}}");

    // One permanent pays its `{T}` once, so the five-mana line waits for the
    // untap step — which is also the step that empties the pool behind it.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 takes another turn with the Quarry standing back up"
    );
    assert!(
        !is_tapped(&engine, quarry),
        "the untap step stood it back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the colourless went with the step that ended (CR 500.5)"
    );
    assert!(
        !offered(&engine),
        "an untapped Quarry alone is not {{5}}: the offer needs the pool"
    );

    // Four Plains tapped, the fifth and the Quarry kept back. The Quarry is
    // named because `{T}: Add {C}` is a mana route of its own — `tap_all_mana`
    // would have spent the very source this test is about (#159).
    let fields = all_on_battlefield(&engine, p0, plains());
    assert_eq!(fields.len(), 5, "five Plains, none of them tapped yet");
    let keep = fields[0];
    let taken = tap_mana_where(&mut engine, p0, |id| id != quarry && id != keep);
    assert_eq!(taken, 4, "four of the five Plains");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four white mana, which is one short"
    );
    assert!(
        !offered(&engine),
        "four is not five: a card that asked for {{4}} would be offered here"
    );

    // The fifth, by hand, because the helper was told to keep it.
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: keep })
        .expect("the kept Plains is still untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "and now five"
    );
    assert!(
        offered(&engine),
        "`can_afford` reads the pool, so the offer appears with the fifth mana"
    );

    activate(&mut engine, p0, crystal_quarry(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.total(),
        5,
        "the {{5}} left the pool and five mana came back — ten here would mean \
         the cost was never paid"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert_eq!(pool.available(color), 1, "one {color:?} and no more");
    }
    assert_eq!(
        pool.available(ManaColor::Colorless),
        0,
        "and the {{C}} the first line made is gone with the Plains that paid"
    );
    assert!(is_tapped(&engine, quarry), "the Quarry paid the {{T}} half");
    assert!(
        fields.iter().all(|id| is_tapped(&engine, *id)),
        "and the five Plains paid the {{5}}"
    );
    assert!(
        stack_is_empty(&engine),
        "no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}
