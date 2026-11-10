//! `cards/creatures/mv_5/thragtusk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thragtusk: "When this creature enters, you gain 5 life. When this
/// creature leaves the battlefield, create a 3/3 green Beast creature
/// token." Cast from hand, then killed: five life on the way in, and the
/// Beast is its controller's on the way out.
#[test]
fn thragtusk_gains_five_as_it_enters_and_leaves_a_beast_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[thragtusk()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, thragtusk());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 25, "you gain 5 life");
    assert_eq!(engine.state().players[1].life, 20);
    let tusk = on_battlefield(&engine, p0, thragtusk()).expect("Thragtusk resolved");
    assert!(tokens_of(&engine, p0).is_empty(), "no Beast while it stays");

    kill(&mut engine, tusk);
    let beasts = tokens_of(&engine, p0);
    assert_eq!(beasts.len(), 1, "one Beast for one departure");
    assert_eq!(pt(&engine, beasts[0]), (3, 3));
    let beast = engine.state().object(beasts[0]).unwrap().characteristics();
    assert_eq!(
        beast.colors,
        baylee_core::color::ColorSet::of(baylee_core::color::Color::Green)
    );
    assert!(beast.types.contains(TypeSet::CREATURE));
    assert_eq!(engine.state().players[0].life, 25, "leaving gains nothing");
}

/// "Leaves the battlefield" is not "dies" (CR 603.6c): exiled by Swords to
/// Plowshares, Thragtusk still leaves a Beast behind.
#[test]
fn thragtusk_leaves_a_beast_when_it_is_exiled_too() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[thragtusk(), plains()])
        .hand(0, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let tusk = on_battlefield(&engine, p0, thragtusk()).expect("in play");
    cast_from_hand(&mut engine, p0, swords_to_plowshares());
    aim_at(&mut engine, p0, tusk);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, thragtusk()).is_none(), "exiled");
    assert!(in_graveyard(&engine, p0, thragtusk()).is_none(), "not died");
    let beasts = tokens_of(&engine, p0);
    assert_eq!(beasts.len(), 1, "exile is a departure, so the Beast comes");
    assert_eq!(pt(&engine, beasts[0]), (3, 3));
}
