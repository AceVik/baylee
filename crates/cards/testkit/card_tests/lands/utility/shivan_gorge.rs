//! `cards/lands/utility/shivan_gorge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shivan Gorge is a legendary land printing two lines: "{T}: Add {C}" and
/// "{2}{R}, {T}: Shivan Gorge deals 1 damage to each opponent."
///
/// The second line is the one worth playing, because "each opponent" is a rule
/// that counts *seats*: at a two-seat table it would read the same way as
/// "target opponent", and only a third seat tells the two apart. Both prices
/// land where a zone can show them — three Mountains pay the {2}{R} down to an
/// empty pool and the activation taps the Gorge itself — while the one damage is
/// read on the two seats that are not the controller and on the one that is.
#[test]
fn shivan_gorge_taps_and_two_red_for_one_damage_to_every_opponent() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[shivan_gorge(), mountain(), mountain(), mountain()])
        .life(0, 20)
        .life(1, 20)
        .life(2, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let gorge = on_battlefield(&engine, p0, shivan_gorge()).expect("the Gorge is on the table");
    assert!(!is_tapped(&engine, gorge), "a land enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is floating before anything is tapped"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, and that reads
    // the pool rather than the three untapped Mountains: with nothing floating
    // the {2}{R} is unpayable, so only the mana line is offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(gorge, 0)),
        "{{T}}: Add {{C}} costs its own tap and is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(gorge, 1)),
        "{{2}}{{R}} is not three: the damage line is unpayable and absent from \
         the offer: {:?}",
        legal.abilities
    );

    // Three Mountains, and the Gorge itself named as the printing kept back: it
    // prints its own `{{T}}: Add {{C}}`, so `tap_all_mana` would have spent the
    // very tap the damage line charges (#159).
    tap_all_mana_but(&mut engine, p0, Some(shivan_gorge()));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        3,
        "three Mountains tapped, three red"
    );
    assert_eq!(pool.total(), 3, "and nothing came off the Gorge");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(gorge, 1)),
        "with three red floating the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, shivan_gorge(), 1);
    assert!(is_tapped(&engine, gorge), "{{T}} is half the price");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{2}}{{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the line is waiting on the stack"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and nothing has been dealt while it waits there"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals 1 damage to each opponent\": seat 1"
    );
    assert_eq!(engine.state().players[2].life, 19, "and seat 2 with it");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the damage belongs to the opponents and never to the controller"
    );
}
