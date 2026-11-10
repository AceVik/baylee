//! `cards/lands/utility/dragon_cursed_halls.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dragon-Cursed Halls — Land: "{T}: Add {C}" and "{1}, {T}: Until end of
/// turn, target creature gains 'Whenever this creature deals combat damage to
/// a player, create a Treasure token.'"
///
/// The whole card is the grant, so the board is two Elves — one targeted, one
/// left bare — and both are sent at the opponent. The life total says both
/// connected, and exactly one Treasure says the trigger landed on the creature
/// that was targeted and not on the controller's whole board; a static read as
/// "creatures you control" would have paid twice. The trigger itself is a
/// token and nothing else, so the counter is the entire event and no question
/// stands between the damage and the answer.
#[test]
#[allow(clippy::too_many_lines)] // one land, played through every clause it prints
fn dragon_cursed_halls_grants_its_treasure_trigger_to_the_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let _p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                dragon_cursed_halls(),
                forest(),
                forest(),
                earth_king_s_lieutenant(),
                earth_king_s_lieutenant(),
            ],
        )
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let halls = on_battlefield(&engine, p0, dragon_cursed_halls()).expect("the Halls are out");
    let attackers_of_mine = all_on_battlefield(&engine, p0, earth_king_s_lieutenant());
    assert_eq!(
        attackers_of_mine.len(),
        2,
        "two Lieutenants, one of which stays bare"
    );
    let (granted, bare) = (attackers_of_mine[0], attackers_of_mine[1]);

    // Mana before the claim: the two Forests pay the {1}, and the Halls is
    // named as the one permanent kept back because its own {T} is the other
    // half of the cost about to be paid.
    tap_all_mana_but(&mut engine, p0, Some(dragon_cursed_halls()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests, and only the Forests: the Lieutenants make no mana, so \
         they are still standing when combat comes"
    );

    // Ability 0 is "{T}: Add {C}"; ability 1 is the grant.
    activate(&mut engine, p0, dragon_cursed_halls(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the grant targets a creature, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&granted) && options.contains(&bare),
        "both creatures under this seat are legal targets: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![granted],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, halls),
        "{{T}} was half of the cost, so the land paid it when the ability was activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{1}} came out of the pool the Forests filled"
    );

    // Both Lieutenants attack and neither is blocked, so the damage arrives twice
    // while only one of the two carries the granted trigger.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&granted) && attackers.contains(&bare),
        "a granted trigger does not stop a creature attacking: {attackers:?}"
    );
    assert_eq!(
        defenders.len(),
        1,
        "the opponent is the only thing to attack: {defenders:?}"
    );
    let defender = defenders[0];
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(granted, defender), (bare, defender)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| !tokens_of(e, p0).is_empty());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "both Lieutenants dealt combat damage to the player, not just the granted one"
    );
    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        1,
        "and only the creature that was granted the trigger pays a Treasure"
    );
    let treasure = engine
        .state()
        .object(tokens[0])
        .expect("the Treasure is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(treasure.name, "Treasure");
}
