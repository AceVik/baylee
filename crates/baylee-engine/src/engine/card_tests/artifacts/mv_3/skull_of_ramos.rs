//! `cards/artifacts/mv_3/skull_of_ramos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skull of Ramos prints two mana abilities and they are the whole card:
/// "{T}: Add {B}" and "Sacrifice this artifact: Add {B}". Both make the same
/// single black mana, so the readings that tell them apart are the price and
/// where the artifact ends up — after the first the Skull is tapped and still
/// standing, and after the second it is in its owner's graveyard. Each line is
/// pressed by index out of the offer rather than guessed at, and the printed
/// "{B}" is fixed, so neither asks a `ChooseColor` on the way.
#[test]
fn skull_of_ramos_offers_both_black_mana_abilities_and_the_second_costs_the_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[skull_of_ramos(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let skull = on_battlefield(&engine, p0, skull_of_ramos()).expect("the Skull is out");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");

    // The Forest is tapped and the Skull kept back: its own `{T}` is ability
    // 0, which this test presses by hand, and `tap_all_mana` would have spent
    // it (#159). One green and no black is the board the first line is read
    // against.
    tap_all_mana_but(&mut engine, p0, Some(skull_of_ramos()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the Forest's one green"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        0,
        "and nothing on this board has produced black yet"
    );
    assert!(
        !is_tapped(&engine, skull),
        "the Skull was the one thing kept back from the tapping"
    );

    // Ability 0, "{T}: Add {B}". A mana ability a card prints has an index to
    // name, so it is an ordinary entry in `legal.abilities` and not the
    // CR 305.6 shortcut a basic land uses.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(skull, 0)),
        "an untapped Skull is a paid {{T}}, so the line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, skull_of_ramos(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "`{{B}}` is fixed, so there is nothing to name on the way (CR 605.1), \
         got {:?}",
        engine.pending()
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "{{T}}: Add {{B}}");
    assert_eq!(
        pool.total(),
        2,
        "one green and one black, and nothing beside them"
    );
    assert!(is_tapped(&engine, skull), "the tap was the price");
    assert!(
        is_tapped(&engine, land),
        "the Forest is down for the green beside it, and green is not black: \
         the Skull is the only source of {{B}} on this board"
    );

    // Ability 1, "Sacrifice this artifact: Add {B}". Its whole price is the
    // artifact and no tap at all, so a Skull that is already tapped is still
    // offered — which is the reading the two lines differ in.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(skull, 1)),
        "the sacrifice costs no tap, so the tapped Skull may still pay it: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, skull_of_ramos(), 1);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: the sacrifice is a mana ability too, so nothing is waiting"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        2,
        "one black from the tap and one from the sacrifice"
    );
    assert_eq!(
        pool.total(),
        3,
        "and the Forest's green is untouched by either"
    );
    assert!(
        on_battlefield(&engine, p0, skull_of_ramos()).is_none(),
        "the sacrifice is the price, so the artifact left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, skull_of_ramos()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
}
