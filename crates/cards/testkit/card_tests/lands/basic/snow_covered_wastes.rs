//! `cards/lands/basic/snow_covered_wastes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Snow-Covered Wastes prints one line — `{T}: Add {C}` — on a **basic** land
/// that has **no basic land type**, which is the only thing separating it from
/// a Forest. CR 305.6 gives mana off a basic land *type*, and that shortcut is
/// what `LegalActions::mana_abilities` carries, so a Wastes has to arrive in
/// `abilities` as an ordinary `(source, index)` entry instead; asserting both
/// halves of that offer is what reads the printed ability rather than a
/// shortcut the card never had. It is also basic and snow, which is the whole
/// reason the printing exists, and the board is one Wastes and nothing else
/// besides the filler Forests in the library — so the single colourless in the
/// pool can have come from nowhere but its own tap, taken by `tap_all_mana`,
/// which reads both lists (#159).
#[test]
fn snow_covered_wastes_taps_for_one_colorless_through_its_printed_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[snow_covered_wastes()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, snow_covered_wastes()).expect("the Wastes was dealt");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.lands.contains(&card),
        "a basic land in hand is a land drop: {:?}",
        legal.lands
    );

    let wastes = play_land(&mut engine, p0, snow_covered_wastes());
    let chars = engine
        .state()
        .object(wastes)
        .expect("the Wastes is an object")
        .characteristics();
    assert!(chars.types.contains(TypeSet::LAND), "it is a land");
    assert!(
        chars.supertypes.contains(SupertypeSet::BASIC)
            && chars.supertypes.contains(SupertypeSet::SNOW),
        "printed basic and snow: {:?}",
        chars.supertypes
    );

    // The offer, read off the permanent that just arrived. A land with a
    // basic land type would be named in `mana_abilities`; this one has none,
    // so its mana is the ability the card prints.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a land drop uses no stack: {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&wastes),
        "no basic land type means no CR 305.6 shortcut: {:?}",
        legal.mana_abilities
    );
    assert!(
        legal.abilities.contains(&(wastes, 0)),
        "its printed mana ability is offered as `(source, index)`: {:?}",
        legal.abilities
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 1, "the Wastes is the only mana source on the board");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "one {{C}}");
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "colorless and not green, though every filler land in the deck is a Forest"
    );
    assert!(
        is_tapped(&engine, wastes),
        "its own {{T}} was the whole price"
    );
}
