//! `cards/enchantments/mv_5/centaur_glade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Centaur Glade` is an enchantment under `Coverage::Implemented` with an activated ability costing `{2}{G}{G}` to create a 3/3 Centaur token.
/// With an empty mana pool, its ability is unpayable and is withheld from legal actions.
/// Tapping four Forests pays the cost without tapping the enchantment itself, resolving a 3/3 green Centaur token onto the battlefield.
#[test]
fn centaur_glade_pays_four_mana_to_create_a_centaur_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), centaur_glade()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let glade = on_battlefield(&engine, p0, centaur_glade()).expect("Centaur Glade deployed");
    assert_eq!(tokens_of(&engine, p0).len(), 0, "no tokens initially");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(glade, 0)),
        "cannot afford {{2}}{{G}}{{G}} with an empty mana pool"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        4,
        "four Forests produce four green mana"
    );

    activate(&mut engine, p0, centaur_glade(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}}{{G}}{{G}} was spent from the pool"
    );
    assert!(
        !is_tapped(&engine, glade),
        "Centaur Glade does not tap to activate"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Centaur token was created");
    let centaur = tokens[0];
    assert_eq!(pt(&engine, centaur), (3, 3), "Centaur token is 3/3");
}
