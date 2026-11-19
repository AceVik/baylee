//! `cards/sorceries/mv_5/lava_axe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_engine::event::DamageTarget;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lava_axe() -> CardIndex {
    card_index("387b6b07-a283-412d-94c3-f7f1dc76e858")
}

/// Returns the player-vs-object targeting options currently presented to `caster`.
fn target_menu(
    engine: &Engine<RegistryLookup>,
    caster: PlayerId,
) -> (Vec<ObjectId>, Vec<PlayerId>) {
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending())
    };
    assert_eq!(player, caster, "the caster names the target");
    assert_eq!((min, max), (1, 1), "Lava Axe requires exactly one target");
    (options, player_options)
}

/// Has P1 deploy `walker` (paying `{4}` / `{2}{U}{U}` without tapping `keep`),
/// optionally activates `+1` on Karn, then advances to P0's next main phase.
fn deploy_p1_planeswalker(
    p0_board: &[CardIndex],
    p0_hand: &[CardIndex],
    p1_board: &[CardIndex],
    p1_hand: &[CardIndex],
    walker: CardIndex,
    keep_untapped: Option<CardIndex>,
    activate_plus_one: bool,
) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, p0_board)
        .hand(0, p0_hand)
        .battlefield(1, p1_board)
        .hand(1, p1_hand)
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    tap_all_mana_but(&mut engine, P1, keep_untapped);
    cast_with_floating(&mut engine, P1, walker);
    pass_until(&mut engine, stack_is_empty);

    let pw = on_battlefield(&engine, P1, walker).expect("planeswalker entered battlefield");
    if activate_plus_one {
        activate(&mut engine, P1, walker, 1);
        pass_until(&mut engine, stack_is_empty);
    }

    reach_their_main_phase(&mut engine, P0);
    (engine, pw)
}

/// Lava Axe — "{4}{R} — Sorcery: Lava Axe deals 5 damage to target player or
/// planeswalker."
///
/// Covers CR 115.1 / 115.10 target selection (players offered, creatures and
/// lands excluded, invalid selections refused), generic+red `{4}{R}` mana
/// payment, and CR 120.3a damage to an opponent reducing life 20 -> 15.
#[test]
fn lava_axe_deals_five_damage_to_target_opponent_and_excludes_creatures() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), plains(), plains(), plains(), plains()])
        .hand(0, &[lava_axe()])
        .battlefield(1, &[gray_ogre(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let axe = in_hand(&engine, P0, lava_axe()).expect("Lava Axe is in hand");
    let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("opponent creature is out");
    let their_island = on_battlefield(&engine, P1, island()).expect("opponent land is out");

    // With 0 mana floating, Lava Axe is payable via lands (CR 601.2g) but not yet in the pool.
    let legal = priority_offer(&engine);
    assert!(
        !legal.castable.contains(&axe),
        "empty mana pool cannot pay {{4}}{{R}} without tapping lands"
    );
    assert!(
        legal.payable.contains(&axe),
        "untapped Mountain + 4 Plains can pay {{4}}{{R}}"
    );

    // Tap only 4 Plains (short on total mana in pool and missing {R}).
    tap_all_mana_but(&mut engine, P0, Some(mountain()));
    assert!(
        !priority_offer(&engine).castable.contains(&axe),
        "four white mana floating cannot pay {{4}}{{R}}"
    );

    // Tap the Mountain as well to complete {4}{R} ({W}{W}{W}{W}{R}).
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, lava_axe());

    let (options, player_options) = target_menu(&engine, P0);
    assert!(
        player_options.contains(&P0) && player_options.contains(&P1),
        "any player is a legal target: {player_options:?}"
    );
    assert!(
        !options.contains(&ogre),
        "creatures cannot be targeted by Lava Axe: {options:?}"
    );
    assert!(
        !options.contains(&their_island),
        "lands cannot be targeted by Lava Axe: {options:?}"
    );

    // Illegal target answers: a creature, zero targets, or two targets.
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![ogre],
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
        "more than one target must be refused"
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

    // While still on the stack, no damage has been dealt yet.
    assert_eq!(life_of(&engine, P1), 20);
    let before = engine.journal().len();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, P1), 15, "5 damage drops P1 from 20 to 15");
    assert_eq!(life_of(&engine, P0), 20, "caster's life is unchanged");
    assert_eq!(
        engine.state().object(ogre).map(|o| o.damage),
        Some(0),
        "opponent's creature took no damage"
    );
    assert!(
        in_graveyard(&engine, P0, lava_axe()).is_some(),
        "resolved sorcery goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "all {{4}}{{R}} mana spent"
    );
    assert_eq!(
        damage_events(&engine, before)
            .into_iter()
            .map(|(_, to, amt, combat)| (to, amt, combat))
            .collect::<Vec<_>>(),
        vec![(DamageTarget::Player(P1), 5, false)]
    );
}

/// Lava Axe says "target player", which includes its own controller.
#[test]
fn lava_axe_can_target_its_caster() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 5])
        .hand(0, &[lava_axe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, lava_axe());
    let (_, player_options) = target_menu(&engine, P0);
    assert!(player_options.contains(&P0));

    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .expect("caster may target self");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, P0), 15, "5 damage to self drops 20 -> 15");
    assert_eq!(life_of(&engine, P1), 20);
    assert!(in_graveyard(&engine, P0, lava_axe()).is_some());
}

/// Damage dealt to a planeswalker removes that many loyalty counters (CR 120.3c,
/// CR 306.8); a 6-loyalty planeswalker drops to 1 and stays on the battlefield.
#[test]
fn lava_axe_drops_planeswalker_loyalty_by_five_and_leaves_six_loyalty_alive() {
    let (mut engine, karn) = deploy_p1_planeswalker(
        &[mountain(), plains(), plains(), plains(), plains()],
        &[lava_axe()],
        &[island(); 4],
        &[karn_the_great_creator()],
        karn_the_great_creator(),
        None,
        true,
    );
    assert_eq!(
        counters_on(&engine, karn, CounterKind::Loyalty),
        6,
        "Karn starts at 5 and +1 brings him to 6"
    );

    cast_from_hand(&mut engine, P0, lava_axe());
    let (options, _) = target_menu(&engine, P0);
    assert!(
        options.contains(&karn),
        "planeswalker is a legal target: {options:?}"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![karn],
                    players: vec![P1],
                },
            )
            .is_err(),
        "cannot pick both a planeswalker and a player for one target"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![karn],
                players: vec![],
            },
        )
        .expect("Karn is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, karn, CounterKind::Loyalty),
        1,
        "5 damage removes 5 loyalty counters (6 -> 1)"
    );
    assert_eq!(
        on_battlefield(&engine, P1, karn_the_great_creator()),
        Some(karn),
        "planeswalker with remaining loyalty stays on the battlefield"
    );
    assert_eq!(life_of(&engine, P1), 20);
    assert!(in_graveyard(&engine, P0, lava_axe()).is_some());
}

/// A planeswalker with 5 loyalty or fewer loses all loyalty counters from
/// Lava Axe's 5 damage and is put into its owner's graveyard by SBA CR 704.5i.
#[test]
fn lava_axe_kills_planeswalker_with_five_or_less_loyalty_via_sba() {
    // Exact 5 loyalty (Karn without +1): 5 -> 0 -> graveyard.
    let (mut engine, karn) = deploy_p1_planeswalker(
        &[mountain(); 5],
        &[lava_axe()],
        &[island(); 4],
        &[karn_the_great_creator()],
        karn_the_great_creator(),
        None,
        false,
    );
    assert_eq!(counters_on(&engine, karn, CounterKind::Loyalty), 5);
    cast_from_hand(&mut engine, P0, lava_axe());
    let (options, _) = target_menu(&engine, P0);
    assert!(options.contains(&karn));
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![karn],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P1, karn_the_great_creator()).is_none());
    assert_eq!(
        in_graveyard(&engine, P1, karn_the_great_creator()),
        Some(karn)
    );

    // Fewer than 5 loyalty (Jace at 3 loyalty): 3 -> 0 -> graveyard without underflow.
    let (mut engine, jace) = deploy_p1_planeswalker(
        &[mountain(); 5],
        &[lava_axe()],
        &[island(); 4],
        &[jace_the_mind_sculptor()],
        jace_the_mind_sculptor(),
        None,
        false,
    );
    assert_eq!(counters_on(&engine, jace, CounterKind::Loyalty), 3);
    cast_from_hand(&mut engine, P0, lava_axe());
    let (options, _) = target_menu(&engine, P0);
    assert!(options.contains(&jace));
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![jace],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P1, jace_the_mind_sculptor()).is_none());
    assert_eq!(
        in_graveyard(&engine, P1, jace_the_mind_sculptor()),
        Some(jace)
    );
}

/// Sorcery timing (CR 307.1 / 117.1a) and red-mana requirement: cannot cast
/// with wrong-color mana, while the stack is non-empty, during combat, or on
/// the opponent's turn.
#[test]
fn lava_axe_obeys_sorcery_timing_and_requires_red_mana() {
    // 5 Plains floating: right total mana value (5), wrong color (no {R}).
    let mut wrong_color = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 5])
        .hand(0, &[lava_axe()])
        .start();
    keep_mulligans(&mut wrong_color);
    reach_main_phase(&mut wrong_color, P0);
    tap_all_mana(&mut wrong_color, P0);
    let axe_wc = in_hand(&wrong_color, P0, lava_axe()).unwrap();
    assert!(!priority_offer(&wrong_color).castable.contains(&axe_wc));
    assert!(!priority_offer(&wrong_color).payable.contains(&axe_wc));
    assert!(
        wrong_color
            .apply(P0, PlayerAction::CastSpell { card: axe_wc })
            .is_err()
    );

    // 4 Mountains floating: right color, 1 generic mana short.
    let mut short_mana = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 4])
        .hand(0, &[lava_axe()])
        .start();
    keep_mulligans(&mut short_mana);
    reach_main_phase(&mut short_mana, P0);
    tap_all_mana(&mut short_mana, P0);
    let axe_short = in_hand(&short_mana, P0, lava_axe()).unwrap();
    assert!(!priority_offer(&short_mana).castable.contains(&axe_short));
    assert!(!priority_offer(&short_mana).payable.contains(&axe_short));
    assert!(
        short_mana
            .apply(P0, PlayerAction::CastSpell { card: axe_short })
            .is_err()
    );

    // Sorcery timing boundaries with full {R}{R}{R}{R}{R} floating.
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
            ],
        )
        .hand(0, &[lava_axe(), giant_growth()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);

    let axe = in_hand(&engine, P0, lava_axe()).unwrap();
    let ogre = on_battlefield(&engine, P1, gray_ogre()).unwrap();

    // Opponent's main phase (with P0 holding priority and 5 {R} floating): cannot cast.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut engine, P0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        5
    );
    assert!(
        !priority_offer(&engine).castable.contains(&axe),
        "sorcery cannot be cast on opponent's turn"
    );
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: axe })
            .is_err()
    );

    // Advance to P0's own next main phase so lands untap cleanly.
    reach_their_main_phase(&mut engine, P0);

    // Non-empty stack in own main phase (5 Mountains untapped, Forest pays Giant Growth):
    // sorcery cannot be cast.
    tap_all_mana_but(&mut engine, P0, Some(mountain()));
    cast_with_floating(&mut engine, P0, giant_growth());
    aim_at(&mut engine, P0, ogre);
    assert!(!stack_is_empty(&engine));
    assert!(
        !priority_offer(&engine).castable.contains(&axe),
        "sorcery cannot be cast while the stack is non-empty"
    );
    assert!(
        !priority_offer(&engine).payable.contains(&axe),
        "sorcery is not payable while the stack is non-empty"
    );
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: axe })
            .is_err()
    );
    pass_until(&mut engine, stack_is_empty);

    // Combat phase on own turn with 5 {R} floating: sorcery cannot be cast.
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::Combat);
    tap_all_mana(&mut engine, P0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        5
    );
    assert!(
        !priority_offer(&engine).castable.contains(&axe),
        "sorcery cannot be cast in combat"
    );
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: axe })
            .is_err()
    );
}

/// CR 608.2b: if the sole targeted planeswalker leaves the battlefield before
/// Lava Axe resolves, all targets are illegal, so Lava Axe does not resolve,
/// deals no damage, and goes to its owner's graveyard.
#[test]
fn lava_axe_fizzles_when_target_planeswalker_leaves_before_resolution() {
    let (mut engine, karn) = deploy_p1_planeswalker(
        &[mountain(); 5],
        &[lava_axe()],
        &[plains(), plains(), plains(), plains(), island(), island()],
        &[karn_the_great_creator(), boomerang()],
        karn_the_great_creator(),
        Some(island()),
        false,
    );

    cast_from_hand(&mut engine, P0, lava_axe());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![karn],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    // P1 responds with Boomerang to bounce Karn before Lava Axe resolves.
    cast_from_hand(&mut engine, P1, boomerang());
    aim_at(&mut engine, P1, karn);

    let before = engine.journal().len();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, P1, karn_the_great_creator()).is_some(),
        "Karn was bounced back to P1's hand"
    );
    assert!(on_battlefield(&engine, P1, karn_the_great_creator()).is_none());
    assert!(
        in_graveyard(&engine, P0, lava_axe()).is_some(),
        "fizzled Lava Axe still goes to owner's graveyard"
    );
    assert_eq!(life_of(&engine, P1), 20);
    assert!(
        damage_events(&engine, before).is_empty(),
        "fizzled Lava Axe must deal no damage"
    );
}

/// CR 702.11b / 608.2b: if the targeted planeswalker gains hexproof in response,
/// it becomes an illegal target for P0's Lava Axe and the spell does not resolve.
#[test]
fn lava_axe_fizzles_when_target_planeswalker_gains_hexproof_in_response() {
    let (mut engine, karn) = deploy_p1_planeswalker(
        &[mountain(); 5],
        &[lava_axe()],
        &[plains(), plains(), plains(), plains(), forest(), forest()],
        &[karn_the_great_creator(), heroic_intervention()],
        karn_the_great_creator(),
        Some(forest()),
        false,
    );

    cast_from_hand(&mut engine, P0, lava_axe());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![karn],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    cast_from_hand(&mut engine, P1, heroic_intervention());
    let before = engine.journal().len();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        on_battlefield(&engine, P1, karn_the_great_creator()),
        Some(karn),
        "Karn survives on battlefield"
    );
    assert_eq!(
        counters_on(&engine, karn, CounterKind::Loyalty),
        5,
        "Karn keeps all 5 loyalty counters"
    );
    assert!(in_graveyard(&engine, P0, lava_axe()).is_some());
    assert!(
        damage_events(&engine, before).is_empty(),
        "fizzled Lava Axe deals no damage"
    );
}

/// CR 702.11c: hexproof on an opponent removes them from Lava Axe's target menu
/// while leaving the caster as a legal self-target.
#[test]
fn lava_axe_cannot_target_hexproof_opponent_but_can_still_target_self() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 5])
        .hand(0, &[lava_axe()])
        .battlefield(1, &[plains(), plains()])
        .hand(1, &[everybody_lives()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // P0 passes in main phase; P1 casts Everybody Lives! (players gain hexproof EOT).
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, P1, everybody_lives());
    pass_until(&mut engine, stack_is_empty);

    // Back to P0 in main phase: P1 has hexproof, so only P0 can be targeted.
    cast_from_hand(&mut engine, P0, lava_axe());
    let (_, player_options) = target_menu(&engine, P0);
    assert!(!player_options.contains(&P1), "hexproof opponent excluded");
    assert!(player_options.contains(&P0), "caster still legal");
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
        "targeting hexproof opponent must be refused"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .expect("self-target accepted");
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P0, lava_axe()).is_some());
}

/// CR 702.16b / 608.2b: if the targeted player gains protection from everything
/// before resolution (Teferi's Protection), they become an illegal target and
/// Lava Axe does not resolve.
#[test]
fn lava_axe_fizzles_against_protection_from_everything_in_response() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 5])
        .hand(0, &[lava_axe()])
        .battlefield(1, &[plains(), plains(), island()])
        .hand(1, &[teferis_protection()])
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
        .unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    cast_from_hand(&mut engine, P1, teferis_protection());
    let before = engine.journal().len();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, P1), 20);
    assert!(in_graveyard(&engine, P0, lava_axe()).is_some());
    assert!(
        damage_events(&engine, before).is_empty(),
        "illegal target due to protection causes Lava Axe to fizzle"
    );
}

/// CR 701.6a / 701.6b: countering Lava Axe prevents its damage and keeps the
/// {4}{R} casting cost spent without refund.
#[test]
fn lava_axe_countered_deals_no_damage_and_keeps_mana_spent() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 5])
        .hand(0, &[lava_axe()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
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
        .unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "mana cost paid while casting"
    );

    let before = engine.journal().len();
    counter_the_top_spell(&mut engine, P0, P1);

    assert_eq!(life_of(&engine, P1), 20);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "no refund");
    assert!(in_graveyard(&engine, P0, lava_axe()).is_some());
    assert!(in_graveyard(&engine, P1, counterspell()).is_some());
    assert!(damage_events(&engine, before).is_empty());
}

/// CR 704.5a: dealing 5 damage to an opponent at 5 life drops them to 0 and
/// causes them to lose the game as a state-based action.
#[test]
fn lava_axe_reduces_opponent_at_five_life_to_zero_and_wins() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 5])
        .hand(0, &[lava_axe()])
        .life(1, 5)
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
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, P1), 0);
    assert!(
        engine.state().players[1].has_lost(),
        "0 life triggers SBA CR 704.5a loss"
    );
}

/// "Target player or planeswalker" places no controller restriction on the
/// planeswalker either: the caster's own planeswalker is a legal target.
#[test]
fn lava_axe_can_target_own_planeswalker() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 5])
        .hand(0, &[karn_the_great_creator(), lava_axe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Turn 1: cast Karn (paying {4} out of 5 Mountains), +1 to 6 loyalty.
    cast_from_hand(&mut engine, P0, karn_the_great_creator());
    pass_until(&mut engine, stack_is_empty);
    let karn = on_battlefield(&engine, P0, karn_the_great_creator()).unwrap();
    activate(&mut engine, P0, karn_the_great_creator(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, karn, CounterKind::Loyalty), 6);

    // P0's next main phase: pass through P1's turn so all 5 Mountains untap.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, lava_axe());
    let (options, _) = target_menu(&engine, P0);
    assert!(
        options.contains(&karn),
        "own planeswalker is offered as a target: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![karn],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, karn, CounterKind::Loyalty), 1);
    assert_eq!(
        on_battlefield(&engine, P0, karn_the_great_creator()),
        Some(karn)
    );
}
