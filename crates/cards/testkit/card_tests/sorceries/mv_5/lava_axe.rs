//! `cards/sets/1997/src/por/lava_axe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

use baylee_core::generated::index::{EVERYBODY_LIVES, HEALING_SALVE};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lava_axe() -> CardIndex {
    card_index("387b6b07-a283-412d-94c3-f7f1dc76e858")
}

/// Lava Axe ({4}{R} Sorcery) — "Lava Axe deals 5 damage to target player or
/// planeswalker."
///
/// Verifies mana cost enforcement ({4}{R} requires 5 total mana including {R}),
/// target menu boundaries (both players offered, creatures/artifacts/lands
/// excluded and refused on `apply`), 5 life loss to the targeted opponent
/// (CR 115.1a, CR 120.3a), and resolution to the owner's graveyard (CR 307.2).
#[test]
fn lava_axe_deals_five_damage_to_opponent_and_enforces_mana_cost() {
    // Mana-cost boundary check: neither 4 mana total nor 5 white mana (no {R})
    // can pay {4}{R} when no further mana sources remain untapped.
    let mut short = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 4])
        .hand(0, &[lava_axe(), plains()])
        .start();
    keep_mulligans(&mut short);
    assert!(walk_to_own_main(&mut short, P0));
    let short_axe = in_hand(&short, P0, lava_axe()).unwrap();
    tap_all_mana(&mut short, P0);
    assert_eq!(short.state().players[0].mana_pool.total(), 4);
    assert!(
        short
            .apply(P0, PlayerAction::CastSpell { card: short_axe })
            .is_err(),
        "4 mana with no untapped sources cannot pay {{4}}{{R}}"
    );
    let fifth_plains = in_hand(&short, P0, plains()).unwrap();
    short
        .apply(P0, PlayerAction::PlayLand { card: fifth_plains })
        .unwrap();
    short
        .apply(
            P0,
            PlayerAction::ActivateManaAbility {
                source: fifth_plains,
            },
        )
        .unwrap();
    assert_eq!(short.state().players[0].mana_pool.total(), 5);
    let Pending::Priority { legal, .. } = short.pending().clone() else {
        panic!("expected priority, got {:?}", short.pending());
    };
    assert!(
        !legal.castable.contains(&short_axe),
        "five white mana without red cannot pay {{4}}{{R}}"
    );
    assert!(
        short
            .apply(P0, PlayerAction::CastSpell { card: short_axe })
            .is_err(),
        "five white mana is refused for {{4}}{{R}}"
    );

    // Main cast with exact {4}{R} ({W}{W}{W}{W}{R}).
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains(), mountain()])
        .battlefield(1, &[llanowar_elves(), sol_ring(), forest()])
        .hand(0, &[lava_axe()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0));

    let axe = in_hand(&engine, P0, lava_axe()).expect("Lava Axe starts in hand");
    let elf = on_battlefield(&engine, P1, llanowar_elves()).expect("opponent Elf is out");
    let rock = on_battlefield(&engine, P1, sol_ring()).expect("opponent artifact is out");
    let land = on_battlefield(&engine, P1, forest()).expect("opponent land is out");

    // Empty mana pool is not offered in `legal.castable`.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&axe));

    tap_all_mana(&mut engine, P0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(legal.castable.contains(&axe));

    engine
        .apply(P0, PlayerAction::CastSpell { card: axe })
        .expect("cast with {4}{R} available");

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!(player, P0);
    assert_eq!((min, max), (1, 1), "exactly one target required");
    assert!(
        options.is_empty(),
        "creatures, artifacts, and lands are not planeswalkers: {options:?}"
    );
    assert!(player_options.contains(&P0) && player_options.contains(&P1));

    // Illegal target selections are refused by `apply` (CR 115.1a).
    for bad in [elf, rock, land] {
        assert!(
            engine
                .apply(
                    P0,
                    PlayerAction::ChooseTargets {
                        objects: vec![bad],
                        players: vec![],
                    },
                )
                .is_err(),
            "non-planeswalker permanent {bad:?} must be refused"
        );
    }
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![],
                },
            )
            .is_err(),
        "zero targets must be refused"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P0, P1],
                },
            )
            .is_err(),
        "two targets must be refused"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .expect("opponent is a legal player target");

    // On the stack: {4}{R} is paid in full, damage has not happened yet.
    assert_eq!(engine.state().object(axe).unwrap().zone, Zone::Stack);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(life_of(&engine, P1), 20);

    pass_until(&mut engine, |e| at_rest(e, P0));

    assert_eq!(life_of(&engine, P1), 15, "opponent loses 5 life (20 -> 15)");
    assert_eq!(life_of(&engine, P0), 20, "caster life unchanged");
    assert_eq!(engine.state().object(elf).unwrap().damage, 0);
    assert_eq!(
        in_graveyard(&engine, P0, lava_axe()),
        Some(axe),
        "resolved sorcery goes to its owner's graveyard (CR 307.2)"
    );
}

/// "Target player" (not "target opponent") allows targeting yourself, and 5
/// damage to a player at 5 life causes that player to lose to SBA (CR 704.5a).
#[test]
fn lava_axe_targets_self_and_eliminates_player_at_five_life() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 10])
        .hand(0, &[lava_axe(), lava_axe()])
        .life(0, 20)
        .life(1, 5)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0));

    tap_all_mana(&mut engine, P0);

    // First Lava Axe targets P0 (self): life drops 20 -> 15.
    cast_with_floating(&mut engine, P0, lava_axe());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .expect("caster is a legal target for 'target player'");
    pass_until(&mut engine, |e| at_rest(e, P0));

    assert_eq!(life_of(&engine, P0), 15, "self-targeted Lava Axe deals 5");
    assert_eq!(life_of(&engine, P1), 5, "opponent untouched by first cast");
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);

    // Second Lava Axe targets P1 at 5 life: life drops 5 -> 0, P1 loses (CR 704.5a).
    cast_with_floating(&mut engine, P0, lava_axe());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::GameOver(_)));

    assert_eq!(life_of(&engine, P1), 0);
    assert!(engine.state().players[1].has_lost());
}

/// CR 120.3c & CR 704.5i: 5 damage to a planeswalker removes 5 loyalty
/// counters; a 6-loyalty planeswalker survives at 1 loyalty, while a 5-loyalty
/// or 3-loyalty planeswalker drops to 0 loyalty and goes to its owner's
/// graveyard as a state-based action without damaging its controller.
#[test]
fn lava_axe_removes_five_loyalty_from_planeswalker_and_kills_at_five_or_less() {
    // Case 1: Opponent ticks Karn (printed loyalty 5) up to 6 via its +1
    // ability, then P0 hits Karn with Lava Axe -> survives at 1 loyalty.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                mountain(),
                jace_the_mind_sculptor(),
            ],
        )
        .battlefield(1, &[karn_the_great_creator(), sol_ring()])
        .hand(0, &[lava_axe()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P1));

    let karn = on_battlefield(&engine, P1, karn_the_great_creator()).unwrap();
    let own_jace = on_battlefield(&engine, P0, jace_the_mind_sculptor()).unwrap();
    assert_eq!(counters_on(&engine, karn, CounterKind::Loyalty), 5);
    assert_eq!(counters_on(&engine, own_jace, CounterKind::Loyalty), 3);

    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: karn,
                ability_index: 1,
            },
        )
        .expect("P1 activates Karn's +1 loyalty ability");
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(P1, PlayerAction::ChooseObjects { objects: vec![] })
            .expect("up to one target allows choosing zero");
    }
    pass_until(&mut engine, |e| at_rest(e, P1));
    assert_eq!(counters_on(&engine, karn, CounterKind::Loyalty), 6);

    assert!(walk_to_own_main(&mut engine, P0));
    cast_from_hand(&mut engine, P0, lava_axe());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&karn) && options.contains(&own_jace),
        "any planeswalker (opponent's or your own) is legal: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "only the two planeswalkers are offered among permanents: {options:?}"
    );
    assert!(player_options.contains(&P0) && player_options.contains(&P1));

    aim_at(&mut engine, P0, karn);
    pass_until(&mut engine, |e| at_rest(e, P0));

    assert_eq!(
        on_battlefield(&engine, P1, karn_the_great_creator()),
        Some(karn),
        "6-loyalty planeswalker survives 5 damage"
    );
    assert_eq!(
        counters_on(&engine, karn, CounterKind::Loyalty),
        1,
        "loyalty reduced from 6 to 1 (CR 120.3c)"
    );
    assert_eq!(counters_on(&engine, own_jace, CounterKind::Loyalty), 3);
    assert_eq!(
        life_of(&engine, P1),
        20,
        "planeswalker's controller loses no life"
    );

    // Case 2: Exact 5-loyalty planeswalker (Karn at printed 5) and 3-loyalty
    // planeswalker (Jace at printed 3) both die to SBA (CR 704.5i).
    for (walker_card, printed_loyalty) in [
        (karn_the_great_creator(), 5_u16),
        (jace_the_mind_sculptor(), 3_u16),
    ] {
        let mut g = Duel::new(SEED, forest())
            .battlefield(0, &[mountain(); 5])
            .battlefield(1, &[walker_card])
            .hand(0, &[lava_axe()])
            .start();
        keep_mulligans(&mut g);
        assert!(walk_to_own_main(&mut g, P0));

        let walker = on_battlefield(&g, P1, walker_card).unwrap();
        assert_eq!(
            counters_on(&g, walker, CounterKind::Loyalty),
            printed_loyalty
        );

        cast_from_hand(&mut g, P0, lava_axe());
        aim_at(&mut g, P0, walker);
        pass_until(&mut g, |e| at_rest(e, P0));

        assert!(
            on_battlefield(&g, P1, walker_card).is_none(),
            "{printed_loyalty}-loyalty planeswalker dies to SBA after 5 damage (CR 704.5i)"
        );
        assert_eq!(in_graveyard(&g, P1, walker_card), Some(walker));
        assert_eq!(
            life_of(&g, P1),
            20,
            "excess damage to a planeswalker does not spill to its controller"
        );
    }
}

/// Unlike "any target" (CR 115.4), "target player or planeswalker" cannot
/// target a Battle or creature.
#[test]
fn lava_axe_excludes_battles_and_creatures_unlike_any_target() {
    use baylee_card_testkit::card_tests::battle_damage_kit as battle_kit;

    let (mut e, battle) = battle_kit::game(SEED, &[], &[lava_axe()]);
    let creature = on_battlefield(&e, P1, fangren_hunter()).unwrap();
    let artifact = on_battlefield(&e, P1, quiet_artifact()).unwrap();

    battle_kit::float(&mut e, ManaColor::Red, 5);
    cast_with_floating(&mut e, P0, lava_axe());

    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = e.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", e.pending());
    };
    assert!(
        !options.contains(&battle) && !options.contains(&creature) && !options.contains(&artifact),
        "Battles, creatures, and artifacts are excluded: {options:?}"
    );
    assert!(player_options.contains(&P0) && player_options.contains(&P1));

    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![battle],
                players: vec![],
            },
        )
        .is_err(),
        "Battle is not a legal target for 'target player or planeswalker'"
    );
}

/// CR 307.1: Lava Axe is a sorcery and cannot be cast while the stack is
/// non-empty, during combat, or on an opponent's turn — even with {4}{R}
/// floating in the caster's mana pool.
#[test]
fn lava_axe_obeys_sorcery_timing_restrictions() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                mountain(),
                island(),
                island(),
            ],
        )
        .hand(0, &[lava_axe(), boomerang()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0));

    let axe = in_hand(&engine, P0, lava_axe()).unwrap();
    let mtn = on_battlefield(&engine, P0, mountain()).unwrap();

    // Control: in P0's main phase with an empty stack and {4}{R}{U}{U} floating,
    // Lava Axe IS castable.
    tap_all_mana(&mut engine, P0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(legal.castable.contains(&axe));

    // 1. Non-empty stack during P0's main phase: {4}{R} remains floating after
    // Boomerang ({U}{U}), yet Lava Axe is not castable until the stack empties.
    cast_with_floating(&mut engine, P0, boomerang());
    aim_at(&mut engine, P0, mtn);
    assert!(!stack_is_empty(&engine));
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    let Pending::Priority { player, legal, .. } = engine.pending().clone() else {
        panic!(
            "expected priority over Boomerang, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, P0);
    assert!(
        !legal.castable.contains(&axe),
        "sorcery requires empty stack"
    );
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: axe })
            .is_err(),
        "sorcery cast with non-empty stack must fail"
    );

    // Once Boomerang resolves, the stack is empty again in Main1 with {4}{R}
    // still floating, and Lava Axe becomes castable again.
    pass_until(&mut engine, |e| at_rest(e, P0));
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected priority after Boomerang, got {:?}",
            engine.pending()
        );
    };
    assert!(legal.castable.contains(&axe));

    // 2. Combat phase on P0's turn: float {4}{R} in CombatBegin -> refused.
    let mut combat_game = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 5])
        .hand(0, &[lava_axe()])
        .start();
    keep_mulligans(&mut combat_game);
    pass_until(&mut combat_game, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut combat_game, P0);
    assert_eq!(combat_game.state().players[0].mana_pool.total(), 5);
    let axe_combat = in_hand(&combat_game, P0, lava_axe()).unwrap();
    let Pending::Priority { legal, .. } = combat_game.pending().clone() else {
        panic!("expected priority in CombatBegin");
    };
    assert!(!legal.castable.contains(&axe_combat));
    assert!(
        combat_game
            .apply(P0, PlayerAction::CastSpell { card: axe_combat })
            .is_err(),
        "sorcery cannot be cast in combat"
    );

    // 3. Opponent's turn (P1's main phase) when P0 holds priority with {4}{R} floating.
    let mut opp_turn = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 5])
        .hand(0, &[lava_axe()])
        .start();
    keep_mulligans(&mut opp_turn);
    assert!(walk_to_own_main(&mut opp_turn, P1));
    opp_turn.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut opp_turn, P0);
    assert_eq!(opp_turn.state().players[0].mana_pool.total(), 5);
    let axe_opp = in_hand(&opp_turn, P0, lava_axe()).unwrap();
    let Pending::Priority { player, legal, .. } = opp_turn.pending().clone() else {
        panic!("expected P0 priority on P1's turn");
    };
    assert_eq!(player, P0);
    assert!(!legal.castable.contains(&axe_opp));
    assert!(
        opp_turn
            .apply(P0, PlayerAction::CastSpell { card: axe_opp })
            .is_err(),
        "sorcery cannot be cast on opponent's turn"
    );
}

/// CR 608.2b: When Lava Axe's sole target (a planeswalker) leaves the
/// battlefield before resolution, all targets are illegal — Lava Axe does not
/// resolve, goes to its owner's graveyard, and deals no damage.
#[test]
fn lava_axe_fizzles_when_targeted_planeswalker_leaves_before_resolution() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 5])
        .battlefield(1, &[island(), island(), jace_the_mind_sculptor()])
        .hand(0, &[lava_axe()])
        .hand(1, &[boomerang()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0));

    let axe = in_hand(&engine, P0, lava_axe()).unwrap();
    let jace = on_battlefield(&engine, P1, jace_the_mind_sculptor()).unwrap();

    cast_from_hand(&mut engine, P0, lava_axe());
    aim_at(&mut engine, P0, jace);

    // P0 passes priority; P1 bounces Jace to hand in response.
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, P1, boomerang());
    aim_at(&mut engine, P1, jace);

    pass_until(&mut engine, |e| at_rest(e, P0));

    assert_eq!(
        in_hand(&engine, P1, jace_the_mind_sculptor()),
        Some(jace),
        "Jace returned to P1's hand before Lava Axe resolved"
    );
    assert!(on_battlefield(&engine, P1, jace_the_mind_sculptor()).is_none());
    assert_eq!(
        in_graveyard(&engine, P0, lava_axe()),
        Some(axe),
        "fizzled Lava Axe goes to P0's graveyard (CR 608.2b)"
    );
    assert_eq!(life_of(&engine, P0), 20);
    assert_eq!(life_of(&engine, P1), 20);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

/// CR 701.6a & CR 701.6b: Countering Lava Axe puts it into its owner's
/// graveyard with no damage dealt and no refund of {4}{R}.
#[test]
fn lava_axe_countered_deals_no_damage_and_refunds_no_mana() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 5])
        .battlefield(1, &[island(), island()])
        .hand(0, &[lava_axe()])
        .hand(1, &[counterspell()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0));

    let axe = in_hand(&engine, P0, lava_axe()).unwrap();
    cast_from_hand(&mut engine, P0, lava_axe());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();

    counter_the_top_spell(&mut engine, P0, P1);

    assert_eq!(in_graveyard(&engine, P0, lava_axe()), Some(axe));
    assert_eq!(life_of(&engine, P1), 20, "countered spell deals no damage");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "spent mana is not refunded when countered (CR 701.6b)"
    );
}

/// CR 702.11c: Player hexproof prevents an opponent from being chosen as Lava
/// Axe's target while still allowing the caster to target themselves.
#[test]
fn lava_axe_cannot_target_opponent_with_hexproof() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 5])
        .battlefield(1, &[plains(), plains()])
        .hand(0, &[lava_axe()])
        .hand(1, &[EVERYBODY_LIVES])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
    });
    cast_from_hand(&mut engine, P1, EVERYBODY_LIVES);
    assert!(walk_to_own_main(&mut engine, P0));

    cast_from_hand(&mut engine, P0, lava_axe());
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(
        !player_options.contains(&P1),
        "hexproof opponent cannot be targeted (CR 702.11c): {player_options:?}"
    );
    assert!(
        player_options.contains(&P0),
        "hexproof does not stop targeting yourself (CR 702.11c): {player_options:?}"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P1],
                },
            )
            .is_err(),
        "targeting hexproof opponent must fail"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .expect("self-targeting remains legal");
    pass_until(&mut engine, |e| at_rest(e, P0));
    assert!(in_graveyard(&engine, P0, lava_axe()).is_some());
}

/// CR 615.7: Lava Axe deals preventable damage (not direct life loss or direct
/// counter removal): a 3-damage prevention shield reduces 5 damage to 2 on
/// both a player (life 20 -> 18) and a 5-loyalty planeswalker (loyalty 5 -> 3).
#[test]
fn lava_axe_damage_is_partially_prevented_by_prevention_shield() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 10])
        .battlefield(1, &[plains(), plains(), karn_the_great_creator()])
        .hand(0, &[lava_axe(), lava_axe()])
        .hand(1, &[HEALING_SALVE, HEALING_SALVE])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0));

    let karn = on_battlefield(&engine, P1, karn_the_great_creator()).unwrap();
    tap_all_mana(&mut engine, P0);

    // 1. Lava Axe at P1; P1 responds with Healing Salve mode 1 (prevent next 3).
    cast_with_floating(&mut engine, P0, lava_axe());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    let p1_plains = all_on_battlefield(&engine, P1, plains());
    engine
        .apply(
            P1,
            PlayerAction::ActivateManaAbility {
                source: p1_plains[0],
            },
        )
        .unwrap();
    cast_with_floating(&mut engine, P1, HEALING_SALVE);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!(
            "expected Healing Salve mode choice, got {:?}",
            engine.pending()
        );
    };
    let mode_1 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .expect("prevention mode offered");
    engine.apply(P1, PlayerAction::ChooseMode(mode_1)).unwrap();
    engine
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| at_rest(e, P0));
    assert_eq!(
        life_of(&engine, P1),
        18,
        "3 of 5 damage prevented -> P1 loses 2 life (CR 615.7, CR 120.3a)"
    );

    // 2. Lava Axe at 5-loyalty Karn; P1 responds with Healing Salve mode 1 on Karn.
    cast_with_floating(&mut engine, P0, lava_axe());
    aim_at(&mut engine, P0, karn);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    engine
        .apply(
            P1,
            PlayerAction::ActivateManaAbility {
                source: p1_plains[1],
            },
        )
        .unwrap();
    cast_with_floating(&mut engine, P1, HEALING_SALVE);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!(
            "expected Healing Salve mode choice, got {:?}",
            engine.pending()
        );
    };
    let mode_1 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .expect("prevention mode offered");
    engine.apply(P1, PlayerAction::ChooseMode(mode_1)).unwrap();
    aim_at(&mut engine, P1, karn);

    pass_until(&mut engine, |e| at_rest(e, P0));
    assert_eq!(
        on_battlefield(&engine, P1, karn_the_great_creator()),
        Some(karn),
        "3 of 5 damage prevented -> 5-loyalty Karn survives"
    );
    assert_eq!(
        counters_on(&engine, karn, CounterKind::Loyalty),
        3,
        "2 unprevented damage removes 2 loyalty counters (5 -> 3)"
    );
}
