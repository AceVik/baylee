//! `cards/creatures/mv_3/xira_arien.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Xira Arien — {B}{R}{G}, legendary 1/2 Insect Wizard with flying, and
/// "{B}{R}{G}, {T}: Target player draws a card."
///
/// The scenario plays both printed halves: the Wizard is cast off the three
/// basics, so its body and its flying are read on a permanent that really
/// arrived, and the ability is activated on the following turn, because a
/// creature that entered this turn cannot pay a {T} cost (CR 302.6). The
/// mana is read twice — absent on an empty pool, floating again before the
/// claim, since `can_afford` reads the pool and not the untapped lands — and
/// the draw lands on the *named* opponent's library and hand while the
/// controller's stay where they were, which is what tells "target player"
/// from "you".
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn xira_arien_taps_for_a_targeted_draw_and_flies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), forest(), mountain()])
        .hand(0, &[xira_arien()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {B}{R}{G} off the three basics, and the Wizard arrives. Every source on
    // the board is spent on the cast, so the pool is empty again after it.
    cast_from_hand(&mut engine, p0, xira_arien());
    pass_until(&mut engine, stack_is_empty);
    let xira = on_battlefield(&engine, p0, xira_arien()).expect("Xira Arien resolved");
    assert_eq!(pt(&engine, xira), (1, 2), "the printed 1/2 body");
    assert!(
        keywords(&engine, xira).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cast spent the three basics"
    );

    // A creature that entered this turn cannot pay a {T} cost (CR 302.6), so
    // the ability is read on the controller's next turn, where it is not sick.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no pool survives the end of a phase (CR 500.5)"
    );

    let offered = |e: &Engine<RegistryLookup>| -> bool {
        matches!(
            e.pending(),
            Pending::Priority { legal, .. } if legal.abilities.contains(&(xira, 0))
        )
    };
    assert!(
        !is_tapped(&engine, xira),
        "the Wizard is standing, so the tap half of the price is payable"
    );
    assert!(
        !offered(&engine),
        "and {{B}}{{R}}{{G}} is an empty pool's nothing: the ability is not \
         offered at all, which is how an unaffordable cost is refused"
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 3,
        "three basics, and the Wizard has no mana ability of its own"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "{{B}}, {{R}} and {{G}} in the pool"
    );
    assert!(
        offered(&engine),
        "with the mana floating the whole price is payable"
    );

    let their_library = library_size(&engine, p1);
    let their_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    let my_library = library_size(&engine, p0);
    let my_hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, xira_arien(), 0);

    // CR 601.2c before CR 601.2h: while the target question stands, the
    // Wizard is untapped and the three mana is still floating.
    assert!(
        !is_tapped(&engine, xira),
        "targets are chosen before costs are paid"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the mana is still in the pool for the same reason"
    );

    let (who, answer) = match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            options,
            player_options,
            ..
        } => {
            assert!(
                options.is_empty(),
                "\"target player\" names no permanent: {options:?}"
            );
            assert!(
                player_options.contains(&p0) && player_options.contains(&p1),
                "\"target player\" is any player, the controller included: {player_options:?}"
            );
            (
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
        }
        Pending::ChoosePlayer { player, options } => {
            assert!(
                options.contains(&p0) && options.contains(&p1),
                "\"target player\" is any player, the controller included: {options:?}"
            );
            (player, PlayerAction::ChoosePlayer(p1))
        }
        other => panic!("\"target player\" asks a question about players, got {other:?}"),
    };
    assert_eq!(who, p0, "the seat that activated names the target");
    engine
        .apply(who, answer)
        .expect("the opponent was one of the options");

    assert!(
        is_tapped(&engine, xira),
        "{{T}} is half the price and is paid last"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and {{B}}{{R}}{{G}} was the other half"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        their_library - 1,
        "\"target player draws a card\": one card off the library of the player \
         that was named"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        their_hand + 1,
        "and it is in that player's hand, not merely gone from the library"
    );
    assert_eq!(
        library_size(&engine, p0),
        my_library,
        "the seat that aimed the draw drew nothing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        my_hand,
        "and its hand is exactly where it was"
    );
}
