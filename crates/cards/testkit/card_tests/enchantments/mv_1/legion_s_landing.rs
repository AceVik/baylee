//! `cards/enchantments/mv_1/legion_s_landing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Legion's Landing: "When Legion's Landing enters, create a 1/1 white
/// Vampire creature token with lifelink." The card is `Coverage::Partial`
/// for its transform alone (#206); the enters trigger is written.
///
/// The enchantment is cast and the tokens are counted: one 1/1 with
/// lifelink. The legendary supertype is checked alongside, because a
/// `Partial` is not a licence to get the type line wrong.
#[test]
fn legions_landing_makes_a_lifelink_vampire_as_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[legion_s_landing()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, legion_s_landing());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let landing = on_battlefield(&engine, p0, legion_s_landing()).expect("it resolved");
    assert!(
        engine
            .state()
            .object(landing)
            .expect("it exists")
            .characteristics()
            .supertypes
            .contains(baylee_core::types::SupertypeSet::LEGENDARY),
        "Legion's Landing is a legendary enchantment"
    );
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Vampire");
    assert_eq!(pt(&engine, tokens[0]), (1, 1));
    assert!(keywords(&engine, tokens[0]).contains(KeywordSet::LIFELINK));
}

/// Adanto, the First Fort: "{T}: Add {W}." One white, no more, and the tap
/// is the cost.
#[test]
fn adanto_taps_for_exactly_one_white() {
    let p0 = PlayerId::new(0);
    let (mut engine, adanto) =
        placed_face(legion_s_landing(), 1, &[]).expect("Adanto on the battlefield");
    assert!(!is_tapped(&engine, adanto));
    activate(&mut engine, p0, legion_s_landing(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "{{T}}: Add {{W}}");
    assert_eq!(pool.total(), 1, "and nothing else");
    assert!(is_tapped(&engine, adanto), "{{T}} is part of the cost");
}

/// Adanto's "{2}{W}, {T}: Create a 1/1 white Vampire creature token with
/// lifelink": one token per activation, Adanto taps for it, and a tapped
/// Adanto cannot do it again at once even with the mana to spare.
#[test]
fn adanto_makes_one_lifelink_vampire_and_cannot_activate_while_tapped() {
    let p0 = PlayerId::new(0);
    let (mut engine, adanto) =
        placed_face(legion_s_landing(), 1, &[]).expect("Adanto on the battlefield");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "no token before the ability is used"
    );
    let pool = &mut engine.dev_state_mut(p0).unwrap().players[0].mana_pool;
    pool.add(ManaColor::White, 1);
    pool.add(ManaColor::Colorless, 2);
    engine.refresh_offer();

    activate(&mut engine, p0, legion_s_landing(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, adanto), "{{T}} is part of the cost");
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Vampire per activation");
    assert_eq!(pt(&engine, tokens[0]), (1, 1));
    assert!(keywords(&engine, tokens[0]).contains(KeywordSet::LIFELINK));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}}{{W}} was paid in full"
    );

    // The mana for a second go is there, the untapped Adanto is not.
    let pool = &mut engine.dev_state_mut(p0).unwrap().players[0].mana_pool;
    pool.add(ManaColor::White, 1);
    pool.add(ManaColor::Colorless, 2);
    engine.refresh_offer();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(adanto, 1)),
        "a tapped Adanto is offered no second activation"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: adanto,
                    ability_index: 1,
                },
            )
            .is_err(),
        "and the engine refuses it if asked anyway"
    );
    assert_eq!(tokens_of(&engine, p0).len(), 1, "still one Vampire");
}
