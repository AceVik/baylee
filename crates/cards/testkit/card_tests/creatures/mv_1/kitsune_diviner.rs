//! `cards/creatures/mv_1/kitsune_diviner.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kitsune Diviner is a `{W}` 0/1 Fox Cleric whose entire text is
/// "{T}: Tap target Spirit". The board holds a Spirit on each side of the
/// table and a Llanowar Elves beside the Diviner, so the offer the
/// activation produces says exactly which permanents the subtype filter
/// reads: two Spirits, one of which the activating seat does not control,
/// and not the Elf a "target creature" would have offered. The Diviner is
/// cast rather than placed and a whole turn cycle passes before it is
/// pressed, because a creature that just arrived cannot pay a `{T}` cost at
/// all (CR 302.6) — a scenario that activated it in its arrival turn would
/// be refused for a reason that has nothing to do with the Spirit it names.
#[test]
fn kitsune_diviner_taps_a_spirit_on_either_side_of_the_table_and_no_other_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), skyclave_apparition(), quiet_creature()])
        .battlefield(1, &[skyclave_apparition()])
        .hand(0, &[kitsune_diviner()])
        .start();
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {W} off the Plains, and the Diviner arrives as the body it prints.
    cast_from_hand(&mut engine, p0, kitsune_diviner());
    pass_until(&mut engine, stack_is_empty);
    let diviner = on_battlefield(&engine, p0, kitsune_diviner()).expect("the Diviner resolved");
    assert_eq!(pt(&engine, diviner), (0, 1), "a printed 0/1");

    // Away and back, so the `{T}` symbol is payable (CR 302.6).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, skyclave_apparition()).expect("my Spirit is out");
    let theirs = on_battlefield(&engine, p1, skyclave_apparition()).expect("their Spirit is out");
    let elves = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    assert!(
        !is_tapped(&engine, diviner),
        "the untap step stood it back up"
    );

    // `{T}` and nothing else, so there is no mana to float and the offer is
    // read off an empty pool (CR 605.1 is not in play: this is no mana
    // ability, and the price is the Diviner's own tap).
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(diviner, 0)),
        "an untapped Diviner that has been around a turn can pay {{T}}: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, kitsune_diviner(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Spirit\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the Spirit");
    assert_eq!(
        (min, max),
        (1, 1),
        "one Spirit, and the card says target and not up to one"
    );
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target Spirit\" is any Spirit, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "the Elf is a creature and no Spirit, which is the whole filter: {options:?}"
    );
    assert!(
        !options.contains(&diviner),
        "the Diviner is a Fox Cleric, so it cannot tap itself: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    // CR 601.2c picks the target before CR 601.2h pays the tap, so the
    // Diviner is down and the Spirit is not the moment the answer lands.
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Spirit the question offered");
    assert!(is_tapped(&engine, diviner), "{{T}} was the cost");
    assert!(
        !is_tapped(&engine, theirs),
        "and the effect has not resolved yet"
    );
    assert!(
        !stack_is_empty(&engine),
        "tapping a Spirit is no mana ability, so it uses the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, theirs),
        "\"tap target Spirit\" reached across the table"
    );
    assert!(
        !is_tapped(&engine, mine),
        "and it tapped the Spirit it was aimed at and no other"
    );
    assert!(
        !is_tapped(&engine, elves),
        "the Elf is not a Spirit and never became one"
    );
}
