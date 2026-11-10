//! `cards/lands/filter/crypt_of_the_eternals.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crypt of the Eternals prints three lines and this one scenario plays all
/// three: the entry trigger gains a life, and the two mana abilities are told
/// apart by the *offer* rather than by their text — with an empty pool only
/// the free `{T}: Add {C}` is listed, because `LegalActions::abilities` is
/// filtered through `can_afford`, which reads the mana pool and not the
/// untapped Forests beside it, so the `{1}` the second line charges is
/// invisible until mana is actually floating. That second line then asks a
/// question with exactly three answers, and it is answered with black — the
/// one colour no land on this board could have produced. Tapping the Forests
/// and keeping the Crypt back is what leaves its own `{T}` for the ability
/// this test presses by index.
#[test]
fn crypt_of_the_eternals_gains_a_life_and_charges_one_for_a_coloured_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[crypt_of_the_eternals()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let life_before = engine.state().players[0].life;
    let card = in_hand(&engine, p0, crypt_of_the_eternals()).expect("the land is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card })
        .expect("an empty board has nothing standing between a land and play");
    // The entry trigger is a trigger: it uses the stack, so the life arrives
    // when it resolves and not the moment the land lands.
    pass_until(&mut engine, |e| {
        e.state().players[0].life == life_before + 1
    });
    let crypt = on_battlefield(&engine, p0, crypt_of_the_eternals())
        .expect("the Crypt is on the battlefield");
    assert!(
        !is_tapped(&engine, crypt),
        "the card prints no enters-tapped line, and nothing tapped it"
    );

    // Nothing floats, so the {1} half is not on the menu while the free
    // {{T}}: Add {{C}} line is.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(crypt, 1)),
        "{{T}}: Add {{C}} costs its own tap symbol and nothing else: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(crypt, 2)),
        "{{1}}, {{T}} has no mana behind it yet — the offer is read off the \
         pool, and the two untapped Forests are not in it: {:?}",
        legal.abilities
    );

    tap_all_mana_but(&mut engine, p0, Some(crypt_of_the_eternals()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests tapped, and the Crypt kept back for the ability this \
         test activates by index"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(crypt, 2)),
        "with two mana floating the paid line is offered at last: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, crypt_of_the_eternals(), 2);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}}, {{B}}, or {{R}}\" is a choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options.len(),
        3,
        "three colours, and the card prints no fourth: {options:?}"
    );
    for color in [ManaColor::Blue, ManaColor::Black, ManaColor::Red] {
        assert!(
            options.contains(&color),
            "{color:?} is one of the three printed: {options:?}"
        );
    }
    assert!(
        !options.contains(&ManaColor::Green) && !options.contains(&ManaColor::White),
        "the two colours the line does not print stay off the menu: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the three it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and nothing on this board could have made it"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the {{1}} came out of the pool, leaving one of the two Forests"
    );
    assert_eq!(pool.total(), 2, "and nothing else joined them");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, crypt), "the Crypt paid its own {{T}}");
}
