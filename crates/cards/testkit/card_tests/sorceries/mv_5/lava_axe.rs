//! `cards/sorceries/mv_5/lava_axe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index::{EVERYBODY_LIVES, TEFERI_S_PROTECTION};
use baylee_engine::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lava_axe() -> CardIndex {
    card_index("387b6b07-a283-412d-94c3-f7f1dc76e858")
}

/// Gives a planeswalker placed on the starting battlefield its loyalty counters
/// before state-based actions run (CR 704.5i).
fn seed_loyalty(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    walker: ObjectId,
    loyalty: u16,
) {
    engine
        .dev_state_mut(seat)
        .expect("the harness may seed loyalty")
        .object_mut(walker)
        .expect("planeswalker exists")
        .counters
        .set(CounterKind::Loyalty, loyalty);
    engine.refresh_offer();
}

/// Lava Axe — {4}{R} Sorcery: "Lava Axe deals 5 damage to target player or
/// planeswalker."
///
/// Exercises the real cast path (`LegalActions` → `CastSpell` → `ChooseTargets`
/// → resolution): cost affordability ({4}{R} paid with 4 generic + 1 red),
/// legal target menu (both players and planeswalkers, excluding creatures,
/// artifacts, and lands), refusal of illegal targets and wrong target counts,
/// CR 601.2c/601.2h ordering, 5 life lost by the targeted opponent (CR 120.3a),
/// bystanders untouched (CR 115.10a), and graveyard landing (CR 608.2n).
#[test]
fn lava_axe_deals_five_damage_to_opponent_and_enforces_target_boundaries() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[plains(), plains(), plains(), plains(), mountain()])
        .hand(0, &[lava_axe()])
        .battlefield(
            1,
            &[
                llanowar_elves(),
                sol_ring(),
                forest(),
                jace_the_mind_sculptor(),
            ],
        )
        .life(0, 20)
        .life(1, 20)
        .start();
    let jace = on_battlefield(&engine, P1, jace_the_mind_sculptor()).expect("Jace is out");
    seed_loyalty(&mut engine, P1, jace, 5);
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let axe = in_hand(&engine, P0, lava_axe()).expect("Lava Axe is in hand");
    let elf = on_battlefield(&engine, P1, llanowar_elves()).expect("their Elf is out");
    let ring = on_battlefield(&engine, P1, sol_ring()).expect("their Sol Ring is out");
    let their_forest = on_battlefield(&engine, P1, forest()).expect("their Forest is out");

    // Empty mana pool cannot pay {4}{R} from the pool alone (`castable`), though
    // the untapped 4 Plains + 1 Mountain make it `payable` via CR 601.2g.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.castable.contains(&axe),
        "empty mana pool cannot pay {{4}}{{R}} from floating mana"
    );
    assert!(
        legal.payable.contains(&axe),
        "four untapped Plains plus an untapped Mountain can pay {{4}}{{R}} in CR 601.2g"
    );

    // Four Plains (4 mana, no red) still cannot cast {4}{R}.
    for source in all_on_battlefield(&engine, P0, plains()) {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source })
            .expect("Plains taps for {W}");
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.castable.contains(&axe),
        "{{W}}{{W}}{{W}}{{W}} cannot pay {{4}}{{R}}"
    );

    // Tapping the Mountain supplies {R} alongside the four {W} for {4}{R}.
    let mtn = on_battlefield(&engine, P0, mountain()).expect("Mountain is out");
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: mtn })
        .expect("Mountain taps for {R}");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.castable.contains(&axe),
        "{{4}}{{R}} floating makes Lava Axe castable"
    );

    engine
        .apply(P0, PlayerAction::CastSpell { card: axe })
        .expect("Lava Axe begins casting");

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
    assert_eq!(player, P0, "caster chooses the target");
    assert_eq!((min, max), (1, 1), "CR 115.1a: exactly one target");
    assert!(
        player_options.contains(&P0) && player_options.contains(&P1),
        "both players are legal targets: {player_options:?}"
    );
    assert!(
        options.contains(&jace),
        "planeswalker on the battlefield is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "creature is NOT a legal target: {options:?}"
    );
    assert!(
        !options.contains(&ring),
        "artifact is NOT a legal target: {options:?}"
    );
    assert!(
        !options.contains(&their_forest),
        "land is NOT a legal target: {options:?}"
    );

    // Illegal targets and wrong target counts are refused.
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![elf],
                    players: vec![],
                },
            )
            .is_err(),
        "creature target must be refused"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![ring],
                    players: vec![],
                },
            )
            .is_err(),
        "artifact target must be refused"
    );
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
                    objects: vec![jace],
                    players: vec![P1],
                },
            )
            .is_err(),
        "choosing both a player and a planeswalker must be refused"
    );

    // CR 601.2c precedes CR 601.2h: mana stays floating until the target is chosen.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "cost is paid after targets are announced"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .expect("opponent is a legal target");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{4}}{{R}} spent once cast completes"
    );
    assert_eq!(
        life_of(&engine, P1),
        20,
        "no damage dealt while Lava Axe is still on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        life_of(&engine, P1),
        15,
        "CR 120.3a: 5 damage to opponent drops life from 20 to 15"
    );
    assert_eq!(
        life_of(&engine, P0),
        20,
        "CR 115.10a: untargeted caster takes no damage"
    );
    assert_eq!(
        counters_on(&engine, jace, CounterKind::Loyalty),
        5,
        "CR 115.10a: untargeted planeswalker keeps all 5 loyalty"
    );
    assert_eq!(
        on_battlefield(&engine, P1, llanowar_elves()),
        Some(elf),
        "creature remains untouched"
    );
    assert!(
        in_graveyard(&engine, P0, lava_axe()).is_some(),
        "CR 608.2n: resolved sorcery goes to its owner's graveyard"
    );
}

/// Lava Axe requires both five total mana (four red mana cannot pay {4}{R})
/// and at least one red mana (five white mana with no red source cannot pay
/// {4}{R}), and "target player" includes its own controller.
#[test]
fn lava_axe_requires_red_mana_and_can_target_caster() {
    // Boundary 1: four red mana (1 short of MV 5) cannot cast Lava Axe.
    let mut short = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[lava_axe()])
        .start();
    keep_mulligans(&mut short);
    reach_main_phase(&mut short, P0);
    tap_all_mana(&mut short, P0);
    let short_axe = in_hand(&short, P0, lava_axe()).expect("Lava Axe is in hand");
    let Pending::Priority { legal, .. } = short.pending().clone() else {
        panic!("expected priority, got {:?}", short.pending());
    };
    assert!(
        !legal.castable.contains(&short_axe) && !legal.payable.contains(&short_axe),
        "four red mana is one short of {{4}}{{R}}"
    );
    assert!(
        short
            .apply(P0, PlayerAction::CastSpell { card: short_axe })
            .is_err(),
        "casting {{4}}{{R}} with only four red mana must be refused"
    );

    // Boundary 2: five white mana with no red source cannot cast Lava Axe.
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[mountain(), lava_axe()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let axe = in_hand(&engine, P0, lava_axe()).expect("Lava Axe is in hand");

    // Five white mana (total 5, but no red source at all) cannot pay {4}{R}.
    for source in all_on_battlefield(&engine, P0, plains()) {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source })
            .expect("Plains taps for {W}");
    }
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.castable.contains(&axe) && !legal.payable.contains(&axe),
        "five non-red mana with no red source cannot pay {{4}}{{R}}"
    );
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: axe })
            .is_err(),
        "casting without {{R}} must be refused"
    );

    // Playing a Mountain and tapping it for {R} makes {4}{R} payable and leaves
    // the extra {W} in the pool.
    let mtn_in_hand = in_hand(&engine, P0, mountain()).expect("Mountain is in hand");
    engine
        .apply(P0, PlayerAction::PlayLand { card: mtn_in_hand })
        .expect("playing a land in Main 1 is legal");
    let mtn = on_battlefield(&engine, P0, mountain()).expect("Mountain is now out");
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: mtn })
        .expect("Mountain taps for {R}");
    cast_with_floating(&mut engine, P0, lava_axe());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .expect("caster is a legal target player");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "five mana spent for {{4}}{{R}}, one {{W}} left floating"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        life_of(&engine, P0),
        15,
        "CR 120.3a: 5 damage to yourself drops life from 20 to 15"
    );
    assert_eq!(life_of(&engine, P1), 20, "opponent takes no damage");
    assert!(
        in_graveyard(&engine, P0, lava_axe()).is_some(),
        "CR 608.2n: Lava Axe goes to graveyard"
    );
}

/// CR 120.3c & CR 704.5i: Lava Axe removes 5 loyalty counters from a targeted
/// planeswalker; a 6-loyalty planeswalker survives at 1 loyalty, while a
/// 5-loyalty planeswalker drops to 0 and dies to the state-based action, and
/// once in the graveyard it is no longer a legal target (CR 115.2).
#[test]
fn lava_axe_damages_planeswalker_and_kills_at_five_or_less_loyalty() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[lava_axe(), lava_axe(), lava_axe()])
        .battlefield(1, &[jace_the_mind_sculptor()])
        .life(0, 20)
        .life(1, 20)
        .start();
    let jace = on_battlefield(&engine, P1, jace_the_mind_sculptor()).expect("Jace is out");
    seed_loyalty(&mut engine, P1, jace, 6);
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // First Lava Axe: 6 loyalty -> 1 loyalty (survives).
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, lava_axe());
    aim_at(&mut engine, P0, jace);
    pass_until(&mut engine, |e| at_rest(e, P0));

    assert_eq!(
        counters_on(&engine, jace, CounterKind::Loyalty),
        1,
        "CR 120.3c: 6 loyalty minus 5 damage leaves 1 loyalty"
    );
    assert_eq!(
        on_battlefield(&engine, P1, jace_the_mind_sculptor()),
        Some(jace),
        "planeswalker with 1 loyalty remains on the battlefield"
    );
    assert_eq!(
        life_of(&engine, P1),
        20,
        "damage to a planeswalker does not reduce its controller's life"
    );

    // Second Lava Axe: 1 loyalty (<= 5) -> 0 loyalty -> dies to SBA (CR 704.5i).
    cast_with_floating(&mut engine, P0, lava_axe());
    aim_at(&mut engine, P0, jace);
    pass_until(&mut engine, |e| at_rest(e, P0));

    assert!(
        on_battlefield(&engine, P1, jace_the_mind_sculptor()).is_none(),
        "CR 704.5i: 0-loyalty planeswalker leaves the battlefield"
    );
    assert_eq!(
        in_graveyard(&engine, P1, jace_the_mind_sculptor()),
        Some(jace),
        "CR 704.5i: 0-loyalty planeswalker is put into its owner's graveyard"
    );
    assert_eq!(life_of(&engine, P1), 20, "controller's life still 20");

    // Third Lava Axe: a planeswalker card in a graveyard is not a legal target (CR 115.2).
    cast_with_floating(&mut engine, P0, lava_axe());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(
        !options.contains(&jace),
        "CR 115.2: planeswalker card in graveyard is not a legal target: {options:?}"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![jace],
                    players: vec![],
                },
            )
            .is_err(),
        "targeting planeswalker in graveyard must be refused"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .expect("can finish casting targeting P1");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, P1), 15);
}

/// Exact lethal loyalty boundary (5 loyalty -> 0 loyalty) plus indestructible
/// interaction: CR 704.5i puts a 0-loyalty planeswalker into its owner's
/// graveyard rather than destroying it, so indestructible (CR 702.12b) does
/// not save it, and hexproof on your own planeswalker does not stop you from
/// targeting it (CR 702.11b).
#[test]
fn lava_axe_kills_exact_five_loyalty_indestructible_own_planeswalker() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                forest(),
                forest(),
                jace_the_mind_sculptor(),
            ],
        )
        .hand(0, &[heroic_intervention(), lava_axe()])
        .start();
    let jace = on_battlefield(&engine, P0, jace_the_mind_sculptor()).expect("own Jace is out");
    seed_loyalty(&mut engine, P0, jace, 5);
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Grant own permanents hexproof and indestructible until end of turn.
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, heroic_intervention());
    pass_until(&mut engine, |e| at_rest(e, P0));

    // CR 702.11b: hexproof only stops opponents, so P0 can still target own Jace.
    cast_with_floating(&mut engine, P0, lava_axe());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&jace),
        "CR 702.11b: hexproof on own planeswalker still allows controller to target it"
    );
    aim_at(&mut engine, P0, jace);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, jace_the_mind_sculptor()).is_none(),
        "CR 704.5i / 702.12b: 5 damage drops 5-loyalty indestructible planeswalker to 0 and puts it in graveyard"
    );
    assert_eq!(
        in_graveyard(&engine, P0, jace_the_mind_sculptor()),
        Some(jace),
        "Jace is in P0's graveyard"
    );
}

/// CR 307.1: As a sorcery, Lava Axe cannot be cast when the stack is non-empty,
/// during combat, or on an opponent's turn — only during its controller's main
/// phase with priority and an empty stack.
#[test]
fn lava_axe_enforces_sorcery_timing() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[lightning_bolt(), lava_axe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let axe = in_hand(&engine, P0, lava_axe()).expect("Lava Axe is in hand");

    // 1. Non-empty stack during P0's own main phase: Lightning Bolt on stack,
    //    5 red still floating — Lava Axe is NOT castable (CR 307.1).
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, lightning_bolt());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .expect("Bolt targets P1");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "Bolt is on the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five red still floating for Lava Axe"
    );
    let Pending::Priority { player, legal, .. } = engine.pending().clone() else {
        panic!("expected P0 priority over Bolt, got {:?}", engine.pending());
    };
    assert_eq!(player, P0);
    assert!(
        !legal.castable.contains(&axe) && !legal.payable.contains(&axe),
        "CR 307.1: sorcery not castable while stack is non-empty"
    );
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: axe })
            .is_err(),
        "CR 307.1: casting sorcery with stack non-empty is refused"
    );
    pass_until(&mut engine, |e| at_rest(e, P0));

    // 2. Combat phase on P0's turn: advance to P0's next turn's Combat phase
    //    (so Mountains have untapped), float 6 red, and verify sorcery refusal.
    let turn_start = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > turn_start
            && e.state().turn.active == P0
            && e.state().turn.phase == Phase::Combat
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut engine, P0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 6);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected P0 priority in combat, got {:?}", engine.pending());
    };
    assert!(
        !legal.castable.contains(&axe) && !legal.payable.contains(&axe),
        "CR 307.1: sorcery not castable during combat"
    );
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: axe })
            .is_err(),
        "CR 307.1: casting sorcery in combat is refused"
    );

    // 3. Opponent's turn: reach P0's next turn, leave Mountains untapped, pass
    //    to P1's first main phase when P0 receives priority, float 6 red.
    let combat_turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > combat_turn && e.state().turn.active == P0
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut engine, P0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 6);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected P0 priority on P1's turn, got {:?}",
            engine.pending()
        );
    };
    assert!(
        !legal.castable.contains(&axe) && !legal.payable.contains(&axe),
        "CR 307.1: sorcery not castable on opponent's turn"
    );
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: axe })
            .is_err(),
        "CR 307.1: casting sorcery on opponent's turn is refused"
    );

    // 4. Control: on P0's own next main phase with empty stack, Lava Axe IS castable.
    assert!(
        walk_to_own_main(&mut engine, P0),
        "P0 reaches its own next main phase"
    );
    tap_all_mana(&mut engine, P0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected P0 priority in own main, got {:?}",
            engine.pending()
        );
    };
    assert!(
        legal.castable.contains(&axe),
        "CR 307.1: sorcery IS castable during own main phase with empty stack"
    );
    cast_with_floating(&mut engine, P0, lava_axe());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .expect("targets P1");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, P1),
        12,
        "20 - 3 (Bolt) - 5 (Lava Axe) = 12"
    );
}

/// CR 608.2b: If Lava Axe's targeted planeswalker leaves the battlefield
/// before resolution (bounced with Boomerang in response), Lava Axe does not
/// resolve, deals no damage, and goes to its owner's graveyard.
#[test]
fn lava_axe_fizzles_when_targeted_planeswalker_leaves_before_resolution() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[lava_axe()])
        .battlefield(1, &[island(), island(), jace_the_mind_sculptor()])
        .hand(1, &[boomerang()])
        .life(0, 20)
        .life(1, 20)
        .start();
    let jace = on_battlefield(&engine, P1, jace_the_mind_sculptor()).expect("Jace is out");
    seed_loyalty(&mut engine, P1, jace, 5);
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // P0 casts Lava Axe targeting P1's Jace and passes priority.
    cast_from_hand(&mut engine, P0, lava_axe());
    aim_at(&mut engine, P0, jace);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    // P1 responds with Boomerang targeting Jace.
    cast_from_hand(&mut engine, P1, boomerang());
    aim_at(&mut engine, P1, jace);

    // Resolve the whole stack: Boomerang bounces Jace, then Lava Axe fizzles.
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P1, jace_the_mind_sculptor()).is_none(),
        "Jace left the battlefield via Boomerang"
    );
    assert!(
        in_hand(&engine, P1, jace_the_mind_sculptor()).is_some(),
        "Jace returned safely to P1's hand"
    );
    assert!(
        in_graveyard(&engine, P1, jace_the_mind_sculptor()).is_none(),
        "Jace was never put into the graveyard"
    );
    assert!(
        in_graveyard(&engine, P0, lava_axe()).is_some(),
        "CR 608.2b: fizzled Lava Axe is removed from stack to its owner's graveyard"
    );
    assert_eq!(life_of(&engine, P1), 20, "no damage spilled over to P1");
    assert_eq!(life_of(&engine, P0), 20, "no damage dealt to P0");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "spent mana is not refunded"
    );
}

/// CR 701.6a & CR 701.6b: Countering Lava Axe cancels it, puts it into its
/// owner's graveyard, prevents its 5 damage, and does not refund {4}{R}.
#[test]
fn lava_axe_countered_deals_no_damage_and_keeps_mana_spent() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[lava_axe()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, lava_axe());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .expect("targets P1");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    counter_the_top_spell(&mut engine, P0, P1);

    assert_eq!(
        life_of(&engine, P1),
        20,
        "CR 701.6a: countered Lava Axe deals no damage"
    );
    assert!(
        in_graveyard(&engine, P0, lava_axe()).is_some(),
        "CR 701.6a: countered Lava Axe is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "CR 701.6b: no refund of {{4}}{{R}}"
    );
}

/// CR 702.11b / 608.2b: A targeted planeswalker gaining hexproof in response
/// makes Lava Axe fizzle without dealing damage, and a planeswalker that
/// already has hexproof cannot be targeted at cast time.
#[test]
fn lava_axe_respects_planeswalker_hexproof_on_cast_and_resolution() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[lava_axe(), lava_axe()])
        .battlefield(1, &[forest(), forest(), jace_the_mind_sculptor()])
        .hand(1, &[heroic_intervention()])
        .life(0, 20)
        .life(1, 20)
        .start();
    let jace = on_battlefield(&engine, P1, jace_the_mind_sculptor()).expect("Jace is out");
    seed_loyalty(&mut engine, P1, jace, 5);
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // P0 targets P1's Jace; P1 responds with Heroic Intervention (hexproof EOT).
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, lava_axe());
    aim_at(&mut engine, P0, jace);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    cast_from_hand(&mut engine, P1, heroic_intervention());
    pass_until(&mut engine, |e| at_rest(e, P0));

    // CR 608.2b / 702.11b: Lava Axe fizzled; Jace still has all 5 loyalty.
    assert_eq!(
        counters_on(&engine, jace, CounterKind::Loyalty),
        5,
        "CR 608.2b / 702.11b: hexproof made Jace illegal before resolution"
    );
    assert!(
        in_graveyard(&engine, P0, lava_axe()).is_some(),
        "fizzled Lava Axe is in graveyard"
    );

    // Casting a second Lava Axe this same turn: hexproof Jace is excluded from targets.
    cast_with_floating(&mut engine, P0, lava_axe());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(
        !options.contains(&jace),
        "CR 702.11b: hexproof planeswalker is not offered as a target: {options:?}"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![jace],
                    players: vec![],
                },
            )
            .is_err(),
        "CR 702.11b: targeting opponent's hexproof planeswalker is refused"
    );
    assert!(
        player_options.contains(&P1),
        "P1 themselves has no hexproof"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .expect("can still target P1");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, P1), 15);
}

/// CR 702.11c & CR 702.16j / 608.2b: Player hexproof stops opponents from
/// targeting that player while still letting the caster target themselves;
/// player protection from everything in response fizzles Lava Axe on resolution.
#[test]
fn lava_axe_respects_player_hexproof_and_protection() {
    // Part 1: Player hexproof (Everybody Lives!) blocks targeting the opponent
    // while keeping self-targeting legal (CR 702.11c). P1 casts Everybody
    // Lives! during P0's upkeep so hexproof is active when P0 reaches Main 1.
    let mut engine_hex = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[lava_axe()])
        .battlefield(1, &[plains(), plains()])
        .hand(1, &[EVERYBODY_LIVES])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine_hex);
    pass_until(&mut engine_hex, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == baylee_engine::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
    });
    cast_from_hand(&mut engine_hex, P1, EVERYBODY_LIVES);
    pass_until(&mut engine_hex, stack_is_empty);
    reach_main_phase(&mut engine_hex, P0);

    cast_from_hand(&mut engine_hex, P0, lava_axe());
    let Pending::ChooseTargets { player_options, .. } = engine_hex.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", engine_hex.pending());
    };
    assert!(
        !player_options.contains(&P1),
        "CR 702.11c: hexproof opponent is excluded from targets: {player_options:?}"
    );
    assert!(
        player_options.contains(&P0),
        "CR 702.11c: caster can still target themselves despite own hexproof: {player_options:?}"
    );
    assert!(
        engine_hex
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P1],
                },
            )
            .is_err(),
        "CR 702.11c: targeting hexproof opponent must be refused"
    );
    // Clean up by finishing the cast on P0 (life total can't drop due to Everybody Lives!).
    engine_hex
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .expect("self target is legal");
    pass_until(&mut engine_hex, stack_is_empty);

    // Part 2: Player gains protection from everything in response (Teferi's Protection).
    let mut engine_prot = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[lava_axe()])
        .battlefield(1, &[plains(), plains(), plains()])
        .hand(1, &[TEFERI_S_PROTECTION])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine_prot);
    reach_main_phase(&mut engine_prot, P0);

    cast_from_hand(&mut engine_prot, P0, lava_axe());
    engine_prot
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .expect("P1 is legal before Teferi's Protection");
    engine_prot.apply(P0, PlayerAction::PassPriority).unwrap();

    cast_from_hand(&mut engine_prot, P1, TEFERI_S_PROTECTION);
    pass_until(&mut engine_prot, stack_is_empty);

    assert_eq!(
        life_of(&engine_prot, P1),
        20,
        "CR 702.16j / 608.2b: protected opponent takes no damage"
    );
    assert!(
        in_graveyard(&engine_prot, P0, lava_axe()).is_some(),
        "CR 608.2b: fizzled Lava Axe is in P0's graveyard"
    );
}
