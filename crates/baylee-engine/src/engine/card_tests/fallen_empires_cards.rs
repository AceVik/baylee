//! Behavioral card tests for Fallen Empires (`fem`), including edge cases,
//! multiplayer/teammate interactions, and visibility checks.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::ChoicePrompt;
use crate::turn::{Phase, Step};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);
    e
}

fn mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

fn priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p),
    );
}

fn cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    priority(e, p);
    mana(e);
    let card = in_hand(e, p, c).unwrap();
    e.apply(p, PlayerAction::CastSpell { card }).unwrap();
}

fn activate_by_card(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex, ability_index: u32) {
    priority(e, p);
    mana(e);
    let source = on_battlefield(e, p, c).unwrap();
    e.apply(
        p,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        },
    )
    .unwrap();
}

fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let p = e.pending().asked().unwrap();
    if matches!(e.pending(), Pending::ChoosePlayer { .. }) {
        e.apply(p, PlayerAction::ChoosePlayer(players[0])).unwrap();
    } else {
        e.apply(p, PlayerAction::ChooseTargets { objects, players })
            .unwrap();
    }
}

fn choose_objects(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::ChooseObjects { objects }).unwrap();
}

fn yes_no(e: &mut Engine<RegistryLookup>, answer: bool) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::YesNo(answer)).unwrap();
}

#[test]
fn balm_of_restoration_gains_2_life_or_prevents_2_damage() {
    // Mode 0: Gain 2 life
    let mut e = prepared(&[], &[index::BALM_OF_RESTORATION], &[]);
    activate_by_card(&mut e, P0, index::BALM_OF_RESTORATION, 0);
    e.apply(P0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 22);
    assert!(on_battlefield(&e, P0, index::BALM_OF_RESTORATION).is_none());

    // Mode 1: Prevent next 2 damage to any target this turn
    let mut e2 = prepared(&[index::LIGHTNING_BOLT], &[index::BALM_OF_RESTORATION], &[]);
    activate_by_card(&mut e2, P0, index::BALM_OF_RESTORATION, 0);
    e2.apply(P0, PlayerAction::ChooseMode(1)).unwrap();
    targets(&mut e2, vec![], vec![P1]);
    pass_until(&mut e2, stack_is_empty);

    cast(&mut e2, P0, index::LIGHTNING_BOLT);
    targets(&mut e2, vec![], vec![P1]);
    pass_until(&mut e2, stack_is_empty);
    // 3 damage minus 2 prevented = 1 damage (19 life).
    assert_eq!(e2.state().players[1].life, 19);
}

#[test]
fn goblin_grenade_sacrifices_goblin_and_deals_5_damage() {
    let mut e = prepared(
        &[index::GOBLIN_GRENADE],
        &[index::MONS_S_GOBLIN_RAIDERS],
        &[],
    );
    let goblin = on_battlefield(&e, P0, index::MONS_S_GOBLIN_RAIDERS).unwrap();
    cast(&mut e, P0, index::GOBLIN_GRENADE);
    targets(&mut e, vec![], vec![P1]);
    choose_objects(&mut e, vec![goblin]);
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, P0, index::MONS_S_GOBLIN_RAIDERS).is_none());
    assert_eq!(e.state().players[1].life, 15);
}

#[test]
fn homarid_tide_counters_cycle_through_1_2_3_4() {
    let mut e = prepared(&[index::HOMARID], &[], &[]);
    cast(&mut e, P0, index::HOMARID);
    pass_until(&mut e, stack_is_empty);
    let homarid = on_battlefield(&e, P0, index::HOMARID).unwrap();

    // Enters with 1 tide counter -> gets -1/-1 (2/2 -> 1/1).
    assert_eq!(
        e.state()
            .object(homarid)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::TIDE),
        1
    );
    assert_eq!(
        e.state().object(homarid).unwrap().characteristics().power,
        Some(1)
    );
    assert_eq!(
        e.state()
            .object(homarid)
            .unwrap()
            .characteristics()
            .toughness,
        Some(1)
    );

    // Put a 2nd tide counter -> normal 2/2.
    crate::replacement::put_counters(&mut e.state, homarid, baylee_cards_dsl::counters::TIDE, 1);
    e.sync_static_effects();
    e.state.refresh_characteristics();
    assert_eq!(
        e.state().object(homarid).unwrap().characteristics().power,
        Some(2)
    );

    // Put a 3rd tide counter -> gets +1/+1 (3/3).
    crate::replacement::put_counters(&mut e.state, homarid, baylee_cards_dsl::counters::TIDE, 1);
    e.sync_static_effects();
    e.state.refresh_characteristics();
    assert_eq!(
        e.state().object(homarid).unwrap().characteristics().power,
        Some(3)
    );
    assert_eq!(
        e.state()
            .object(homarid)
            .unwrap()
            .characteristics()
            .toughness,
        Some(3)
    );

    // On P0's next upkeep, puts a 4th tide counter -> state trigger removes all tide counters.
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state()
            .object(homarid)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::TIDE),
        0
    );
    assert_eq!(
        e.state().object(homarid).unwrap().characteristics().power,
        Some(2)
    );
}

#[test]
fn icatian_moneychanger_enters_with_3_credit_counters_deals_3_damage_and_sacrifices_at_upkeep() {
    let mut e = prepared(&[index::ICATIAN_MONEYCHANGER], &[], &[]);
    cast(&mut e, P0, index::ICATIAN_MONEYCHANGER);
    pass_until(&mut e, stack_is_empty);
    let mc = on_battlefield(&e, P0, index::ICATIAN_MONEYCHANGER).unwrap();

    assert_eq!(e.state().players[0].life, 17);
    assert_eq!(
        e.state()
            .object(mc)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::CREDIT),
        3
    );

    // Advance to P0's next upkeep: upkeep trigger adds 1 credit counter (now 4).
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state()
            .object(mc)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::CREDIT),
        4
    );

    // Still in Upkeep, activate Icatian Moneychanger (ability 2) to gain 4 life.
    activate_by_card(&mut e, P0, index::ICATIAN_MONEYCHANGER, 2);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 21);
}

#[test]
fn necrite_sacrifices_when_unblocked_to_destroy_defenders_creature_without_regeneration() {
    let mut e = prepared(&[], &[index::NECRITE], &[index::GRIZZLY_BEARS]);
    let necrite = on_battlefield(&e, P0, index::NECRITE).unwrap();
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(necrite, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareBlockers);
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // Unblocked trigger requires targeting defending player's creature, then asks MayDo.
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut e, true);
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, P0, index::NECRITE).is_none());
    assert!(on_battlefield(&e, P1, index::GRIZZLY_BEARS).is_none());
}

#[test]
fn tidal_influence_cast_restriction_and_tide_counter_modifications() {
    let mut e = prepared(
        &[index::TIDAL_INFLUENCE, index::TIDAL_INFLUENCE],
        &[index::HOMARID_WARRIOR],
        &[],
    );
    let warrior = on_battlefield(&e, P0, index::HOMARID_WARRIOR).unwrap();
    assert_eq!(
        e.state().object(warrior).unwrap().characteristics().power,
        Some(3)
    );

    cast(&mut e, P0, index::TIDAL_INFLUENCE);
    pass_until(&mut e, stack_is_empty);
    let ti = on_battlefield(&e, P0, index::TIDAL_INFLUENCE).unwrap();

    // 1 tide counter -> blue creatures get -2/-0 (Homarid Warrior 3/3 -> 1/3).
    assert_eq!(
        e.state().object(warrior).unwrap().characteristics().power,
        Some(1)
    );

    // Second Tidal Influence in hand cannot be cast while one is on the battlefield!
    let second = in_hand(&e, P0, index::TIDAL_INFLUENCE).unwrap();
    assert!(
        e.apply(P0, PlayerAction::CastSpell { card: second })
            .is_err()
    );

    // 3 tide counters -> blue creatures get +2/+0 (Homarid Warrior 3/3 -> 5/3).
    crate::replacement::put_counters(&mut e.state, ti, baylee_cards_dsl::counters::TIDE, 2);
    e.sync_static_effects();
    e.state.refresh_characteristics();
    assert_eq!(
        e.state().object(warrior).unwrap().characteristics().power,
        Some(5)
    );
}

#[test]
fn delifs_cone_and_delifs_cube_trigger_when_target_attacks_and_isnt_blocked() {
    // Delif's Cone: gains life equal to power and creature assigns no combat damage.
    let mut e = prepared(&[], &[index::DELIF_S_CONE, index::GRIZZLY_BEARS], &[]);
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    activate_by_card(&mut e, P0, index::DELIF_S_CONE, 0);
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareBlockers);
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut e, true);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);

    // Grizzly Bears has power 2 -> P0 gained 2 life (22), and P1 took 0 combat damage (20)!
    assert_eq!(e.state().players[0].life, 22);
    assert_eq!(e.state().players[1].life, 20);

    // Delif's Cube: puts a cube counter on Cube and creature assigns no combat damage.
    let mut e2 = prepared(&[], &[index::DELIF_S_CUBE, index::GRIZZLY_BEARS], &[]);
    let cube = on_battlefield(&e2, P0, index::DELIF_S_CUBE).unwrap();
    let bears2 = on_battlefield(&e2, P0, index::GRIZZLY_BEARS).unwrap();
    activate_by_card(&mut e2, P0, index::DELIF_S_CUBE, 0);
    targets(&mut e2, vec![bears2], vec![]);
    pass_until(&mut e2, stack_is_empty);

    pass_until(&mut e2, |e| e.state().turn.step == Step::DeclareAttackers);
    e2.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears2, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e2, |e| e.state().turn.step == Step::DeclareBlockers);
    e2.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e2, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(
        e2.state()
            .object(cube)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::CUBE),
        1
    );
    assert_eq!(e2.state().players[1].life, 20);
}

#[test]
fn dwarven_soldier_gets_plus_0_plus_2_once_when_blocked_by_multiple_orcs() {
    let mut e = prepared(
        &[],
        &[index::DWARVEN_SOLDIER],
        &[index::ORCISH_SPY, index::ORCISH_SPY],
    );
    let soldier = on_battlefield(&e, P0, index::DWARVEN_SOLDIER).unwrap();
    let orcs: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| e.state().object(id).unwrap().controller == P1)
        .collect();

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(soldier, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareBlockers);
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(orcs[0], soldier), (orcs[1], soldier)],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    // "one or more Orcs" triggers only once even when blocked by two Orcs -> 2/3, not 2/5.
    assert_eq!(
        e.state()
            .object(soldier)
            .unwrap()
            .characteristics()
            .toughness,
        Some(3)
    );
}

#[test]
fn seasinger_gains_control_while_tapped_and_controlled() {
    let mut e = prepared(
        &[],
        &[index::SEASINGER, index::ISLAND],
        &[index::GRIZZLY_BEARS, index::ISLAND],
    );
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    activate_by_card(&mut e, P0, index::SEASINGER, 2);
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().object(bears).unwrap().controller, P0);

    // Untapping Seasinger ends the control effect immediately.
    let seasinger = on_battlefield(&e, P0, index::SEASINGER).unwrap();
    e.state.set_tapped(seasinger, false);
    e.sync_static_effects();
    e.state.refresh_characteristics();
    assert_eq!(e.state().object(bears).unwrap().controller, P1);
}

#[test]
fn vodalian_war_machine_taps_merfolk_to_attack_and_destroys_them_when_it_dies() {
    let mut e = prepared(
        &[index::LIGHTNING_BOLT, index::LIGHTNING_BOLT],
        &[
            index::VODALIAN_WAR_MACHINE,
            index::VODALIAN_SOLDIERS,
            index::VODALIAN_MAGE,
        ],
        &[],
    );
    let machine = on_battlefield(&e, P0, index::VODALIAN_WAR_MACHINE).unwrap();
    let m1 = on_battlefield(&e, P0, index::VODALIAN_SOLDIERS).unwrap();
    let m2 = on_battlefield(&e, P0, index::VODALIAN_MAGE).unwrap();

    // Activate ability 0 (can attack despite defender) tapping m1.
    activate_by_card(&mut e, P0, index::VODALIAN_WAR_MACHINE, 0);
    choose_objects(&mut e, vec![m1]);
    pass_until(&mut e, stack_is_empty);

    // Activate ability 1 (+2/+1) tapping m2.
    activate_by_card(&mut e, P0, index::VODALIAN_WAR_MACHINE, 1);
    choose_objects(&mut e, vec![m2]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state().object(machine).unwrap().characteristics().power,
        Some(2)
    );
    assert_eq!(
        e.state()
            .object(machine)
            .unwrap()
            .characteristics()
            .toughness,
        Some(5)
    );

    // Destroy Vodalian War Machine with two Lightning Bolts (6 damage).
    for _ in 0..2 {
        cast(&mut e, P0, index::LIGHTNING_BOLT);
        targets(&mut e, vec![machine], vec![]);
        pass_until(&mut e, stack_is_empty);
    }

    // Both Merfolk tapped this turn to pay for its abilities are destroyed!
    assert!(on_battlefield(&e, P0, index::VODALIAN_WAR_MACHINE).is_none());
    assert!(on_battlefield(&e, P0, index::VODALIAN_SOLDIERS).is_none());
    assert!(on_battlefield(&e, P0, index::VODALIAN_MAGE).is_none());
}

#[test]
fn derelor_increases_colored_mana_cost_of_your_black_spells_only() {
    let mut e = Duel::new(SEED, index::SWAMP)
        .hand(0, &[index::MINDSTAB_THRULL])
        .battlefield(0, &[index::DERELOR])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    // Mindstab Thrull costs {1}{B}{B}; with Derelor it costs {1}{B}{B}{B}.
    // With only {2}{B}{B} in pool, P0 cannot cast it.
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 2);
    e.state.players[0].mana_pool.add(ManaColor::Black, 2);
    e.refresh_offer();
    let thrull = in_hand(&e, P0, index::MINDSTAB_THRULL).unwrap();
    assert!(
        e.apply(P0, PlayerAction::CastSpell { card: thrull })
            .is_err()
    );

    // Adding a third {B} allows casting it.
    e.state.players[0].mana_pool.add(ManaColor::Black, 1);
    e.refresh_offer();
    e.apply(P0, PlayerAction::CastSpell { card: thrull })
        .unwrap();
}

#[test]
fn orgg_attack_and_block_restrictions_with_power_3_or_greater() {
    let mut e = prepared(&[], &[index::ORGG], &[index::HILL_GIANT]);
    let orgg = on_battlefield(&e, P0, index::ORGG).unwrap();

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    // Hill Giant (3/3) is untapped -> Orgg cannot attack P1!
    assert!(
        e.apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(orgg, Defender::Player(P1))],
            },
        )
        .is_err()
    );

    // If Hill Giant is tapped before DeclareAttackers, Orgg can attack P1.
    let mut e2 = prepared(&[], &[index::ORGG], &[index::HILL_GIANT]);
    let orgg2 = on_battlefield(&e2, P0, index::ORGG).unwrap();
    let giant2 = on_battlefield(&e2, P1, index::HILL_GIANT).unwrap();
    e2.state.set_tapped(giant2, true);
    pass_until(&mut e2, |e| e.state().turn.step == Step::DeclareAttackers);
    e2.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(orgg2, Defender::Player(P1))],
        },
    )
    .unwrap();
}

#[test]
fn ebon_praetor_upkeep_counter_and_thrull_sacrifice_bonus() {
    let mut e = prepared(&[], &[index::EBON_PRAETOR, index::BASAL_THRULL], &[]);
    let praetor = on_battlefield(&e, P0, index::EBON_PRAETOR).unwrap();
    let thrull = on_battlefield(&e, P0, index::BASAL_THRULL).unwrap();

    // Turn 1's upkeep already placed 1 -2/-2 counter on Ebon Praetor (5/5 -> 3/3).
    assert_eq!(
        e.state().object(praetor).unwrap().characteristics().power,
        Some(3)
    );
    assert_eq!(
        e.state()
            .object(praetor)
            .unwrap()
            .characteristics()
            .toughness,
        Some(3)
    );

    // Advance to P0's next upkeep (turn 3): upkeep trigger puts a 2nd -2/-2 counter.
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut e, stack_is_empty);

    // Activate Ebon Praetor (ability 1) during upkeep, sacrificing Basal Thrull:
    // removes one -2/-2 counter and puts a +1/+0 counter (5/5 - 2/2 + 1/0 = 4/3).
    activate_by_card(&mut e, P0, index::EBON_PRAETOR, 1);
    choose_objects(&mut e, vec![thrull]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state().object(praetor).unwrap().characteristics().power,
        Some(4)
    );
    assert_eq!(
        e.state()
            .object(praetor)
            .unwrap()
            .characteristics()
            .toughness,
        Some(3)
    );
}

#[test]
fn deep_spawn_upkeep_mill_and_activated_shroud_skip_next_untap() {
    let mut e = prepared(&[index::DEEP_SPAWN], &[], &[]);
    cast(&mut e, P0, index::DEEP_SPAWN);
    pass_until(&mut e, stack_is_empty);
    let spawn = on_battlefield(&e, P0, index::DEEP_SPAWN).unwrap();

    // Activate Deep Spawn's {U} ability (ability 1): gains shroud, taps, and won't untap during P0's next untap step.
    activate_by_card(&mut e, P0, index::DEEP_SPAWN, 1);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .object(spawn)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert!(
        e.state()
            .object(spawn)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::SHROUD)
    );

    // Advance to P0's next upkeep: Deep Spawn did not untap, and upkeep trigger asks to mill 2 cards.
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    assert!(
        e.state()
            .object(spawn)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::YesNo { player: P0, .. })
    });
    let gy_before = e.state().zones.list(ZoneLocation::Graveyard(P0)).len();
    yes_no(&mut e, true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        gy_before + 2
    );
    assert!(on_battlefield(&e, P0, index::DEEP_SPAWN).is_some());
}

#[test]
fn farrels_mantle_deals_power_plus_2_to_target_creature_and_assigns_no_combat_damage() {
    let mut e = prepared(
        &[index::FARREL_S_MANTLE],
        &[index::GRIZZLY_BEARS],
        &[index::SERRA_ANGEL],
    );
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let angel = on_battlefield(&e, P1, index::SERRA_ANGEL).unwrap();
    // Tap Serra Angel so she can't block.
    e.state.set_tapped(angel, true);

    cast(&mut e, P0, index::FARREL_S_MANTLE);
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareBlockers);
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // Farrel's Mantle triggers: target Serra Angel, then accept MayDoFor.
    targets(&mut e, vec![angel], vec![]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut e, true);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);

    // Grizzly Bears (power 2) dealt 2 + 2 = 4 lethal damage to Serra Angel (4/4), and 0 combat damage to P1!
    assert!(on_battlefield(&e, P1, index::SERRA_ANGEL).is_none());
    assert_eq!(e.state().players[1].life, 20);
}

#[test]
fn tourachs_gate_sacrifices_thrull_for_time_counters_and_taps_enchanted_land_to_pump_attackers() {
    let mut e = prepared(
        &[index::TOURACH_S_GATE],
        &[index::SWAMP, index::BASAL_THRULL, index::GRIZZLY_BEARS],
        &[],
    );
    let swamp = on_battlefield(&e, P0, index::SWAMP).unwrap();
    let thrull = on_battlefield(&e, P0, index::BASAL_THRULL).unwrap();
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();

    cast(&mut e, P0, index::TOURACH_S_GATE);
    targets(&mut e, vec![swamp], vec![]);
    pass_until(&mut e, stack_is_empty);
    let gate = on_battlefield(&e, P0, index::TOURACH_S_GATE).unwrap();

    // Activate ability 1 (Sacrifice a Thrull: Put three time counters on this Aura).
    activate_by_card(&mut e, P0, index::TOURACH_S_GATE, 1);
    choose_objects(&mut e, vec![thrull]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state()
            .object(gate)
            .unwrap()
            .counters
            .get(CounterKind::Time),
        3
    );

    // Declare Grizzly Bears attacking, then activate ability 3 (Tap enchanted land: Attacking creatures you control get +2/-1).
    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(P1))],
        },
    )
    .unwrap();
    activate_by_card(&mut e, P0, index::TOURACH_S_GATE, 3);
    choose_objects(&mut e, vec![swamp]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state().object(bears).unwrap().characteristics().power,
        Some(4)
    );
    assert_eq!(
        e.state().object(bears).unwrap().characteristics().toughness,
        Some(1)
    );
}

#[test]
fn merseine_locks_untap_until_enchanted_creature_controller_pays_mana_cost_three_times() {
    let mut e = prepared(
        &[index::MERSEINE],
        &[],
        &[index::GRIZZLY_BEARS, index::FOREST, index::FOREST],
    );
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    e.state.set_tapped(bears, true);

    cast(&mut e, P0, index::MERSEINE);
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);
    let merseine = on_battlefield(&e, P0, index::MERSEINE).unwrap();
    assert_eq!(
        e.state()
            .object(merseine)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::NET),
        3
    );

    // P1 (controller of enchanted Grizzly Bears) can pay {1}{G} three times (ability 2) to remove all 3 net counters.
    for expected in [2, 1, 0] {
        priority(&mut e, P1);
        mana(&mut e);
        e.apply(
            P1,
            PlayerAction::ActivateAbility {
                source: merseine,
                ability_index: 2,
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state()
                .object(merseine)
                .unwrap()
                .counters
                .get(baylee_cards_dsl::counters::NET),
            expected
        );
    }

    // On P1's untap step, Grizzly Bears now untaps normally!
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    assert!(
        !e.state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}

#[test]
fn night_soil_exiles_two_creatures_from_a_single_graveyard_to_create_saproling() {
    let mut e = prepared(
        &[],
        &[index::NIGHT_SOIL],
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
    );
    let b1 = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    let b2 = on_battlefield(&e, P1, index::HILL_GIANT).unwrap();
    for id in [b1, b2] {
        e.dev_state_mut(P0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Graveyard(P1),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    e.refresh_offer();

    let g1 = e.state().zones.list(ZoneLocation::Graveyard(P1))[0];
    let g2 = e.state().zones.list(ZoneLocation::Graveyard(P1))[1];

    activate_by_card(&mut e, P0, index::NIGHT_SOIL, 0);
    choose_objects(&mut e, vec![g1, g2]);
    pass_until(&mut e, stack_is_empty);

    assert!(e.state().zones.list(ZoneLocation::Graveyard(P1)).is_empty());
    let saproling = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| !e.state().object(id).unwrap().is_card());
    assert!(saproling.is_some());
}

#[test]
fn heroism_prevents_combat_damage_from_attacking_red_creatures_unless_controller_pays() {
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &[index::IRONCLAW_ORCS])
        .battlefield(1, &[index::HEROISM, index::ICATIAN_INFANTRY])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    let orcs = on_battlefield(&e, P0, index::IRONCLAW_ORCS).unwrap();
    let infantry = on_battlefield(&e, P1, index::ICATIAN_INFANTRY).unwrap();

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(orcs, Defender::Player(P1))],
        },
    )
    .unwrap();

    // P1 activates Heroism sacrificing Icatian Infantry; P0 declines or cannot pay {2}{R}.
    priority(&mut e, P1);
    let heroism = on_battlefield(&e, P1, index::HEROISM).unwrap();
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: heroism,
            ability_index: 0,
        },
    )
    .unwrap();
    choose_objects(&mut e, vec![infantry]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || e.state().turn.phase == Phase::SecondMain
    });
    if matches!(e.pending(), Pending::YesNo { .. }) {
        yes_no(&mut e, false);
        pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    }

    // Ironclaw Orcs assigned no combat damage -> P1 is still at 20 life!
    assert_eq!(e.state().players[1].life, 20);
}

#[test]
fn raiding_party_cant_be_targeted_by_white_and_destroys_unchosen_plains() {
    let mut e = prepared(
        &[],
        &[index::RAIDING_PARTY, index::IRONCLAW_ORCS, index::PLAINS],
        &[
            index::ICATIAN_INFANTRY,
            index::PLAINS,
            index::PLAINS,
            index::PLAINS,
        ],
    );
    let orcs = on_battlefield(&e, P0, index::IRONCLAW_ORCS).unwrap();
    let infantry = on_battlefield(&e, P1, index::ICATIAN_INFANTRY).unwrap();
    let p1_plains: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let o = e.state().object(id).unwrap();
            o.controller == P1
                && o.characteristics()
                    .subtypes
                    .contains(baylee_core::generated::subtypes::land::PLAINS)
        })
        .collect();

    activate_by_card(&mut e, P0, index::RAIDING_PARTY, 1);
    choose_objects(&mut e, vec![orcs]);
    // P1 is asked which white creatures to tap (P0 has none).
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { player: P1, .. })
    });
    choose_objects(&mut e, vec![infantry]);
    // P1 tapped 1 white creature -> may choose up to 2 Plains to spare.
    choose_objects(&mut e, vec![p1_plains[0], p1_plains[1]]);
    pass_until(&mut e, stack_is_empty);

    // P0's Plains and P1's 3rd Plains were not chosen and are destroyed; P1's first 2 Plains survive!
    assert!(on_battlefield(&e, P0, index::PLAINS).is_none());
    assert_eq!(
        e.state().object(p1_plains[0]).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        e.state().object(p1_plains[1]).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        e.state().object(p1_plains[2]).unwrap().zone,
        Zone::Graveyard
    );
}

#[test]
fn thelons_chant_and_tourachs_chant_punish_swamp_and_forest_drops() {
    let mut e = prepared(
        &[index::SWAMP],
        &[index::GRIZZLY_BEARS],
        &[index::THELON_S_CHANT],
    );
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let swamp = in_hand(&e, P0, index::SWAMP).unwrap();

    // P0 plays a Swamp -> Thelon's Chant triggers, asking P0 to choose up to 1 creature to put a -1/-1 counter on.
    e.apply(P0, PlayerAction::PlayLand { card: swamp }).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { player: P0, .. })
    });
    choose_objects(&mut e, vec![bears]);
    pass_until(&mut e, stack_is_empty);

    // P0 put a -1/-1 counter on Grizzly Bears (now 1/1) instead of taking 3 damage.
    assert_eq!(e.state().players[0].life, 20);
    assert_eq!(
        e.state().object(bears).unwrap().characteristics().power,
        Some(1)
    );

    // Tourach's Chant: P0 declines to put a -1/-1 counter (chooses 0 creatures) when playing a Forest -> takes 3 damage.
    let mut e2 = prepared(
        &[index::FOREST],
        &[index::GRIZZLY_BEARS],
        &[index::TOURACH_S_CHANT],
    );
    let forest = in_hand(&e2, P0, index::FOREST).unwrap();
    e2.apply(P0, PlayerAction::PlayLand { card: forest })
        .unwrap();
    pass_until(&mut e2, |e| {
        matches!(e.pending(), Pending::ChooseCards { player: P0, .. })
    });
    choose_objects(&mut e2, vec![]);
    pass_until(&mut e2, stack_is_empty);
    assert_eq!(e2.state().players[0].life, 17);
}

#[test]
fn tidal_flats_grants_first_strike_to_blockers_unless_attacker_controller_pays_1() {
    let mut e = Duel::new(SEED, index::ISLAND)
        .battlefield(0, &[index::GRIZZLY_BEARS])
        .battlefield(1, &[index::TIDAL_FLATS, index::VODALIAN_SOLDIERS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let soldiers = on_battlefield(&e, P1, index::VODALIAN_SOLDIERS).unwrap();
    let flats = on_battlefield(&e, P1, index::TIDAL_FLATS).unwrap();

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareBlockers);
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(soldiers, bears)],
        },
    )
    .unwrap();

    // Give P1 {U}{U} to activate Tidal Flats; P0 declines or has 0 mana so cannot pay {1}.
    priority(&mut e, P1);
    e.state.players[1].mana_pool.add(ManaColor::Blue, 2);
    e.refresh_offer();
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: flats,
            ability_index: 0,
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    if matches!(e.pending(), Pending::YesNo { .. }) {
        yes_no(&mut e, false);
        pass_until(&mut e, stack_is_empty);
    }

    assert!(
        e.state()
            .object(soldiers)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE)
    );
}

#[test]
fn dwarven_catapult_divides_x_damage_evenly_rounded_down_among_target_opponents_creatures() {
    let mut e = prepared(
        &[index::DWARVEN_CATAPULT],
        &[],
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
    );
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&e, P1, index::HILL_GIANT).unwrap();

    // Cast Dwarven Catapult with X = 5 targeting P1: 5 / 2 = 2 damage to each creature (rounded down).
    cast(&mut e, P0, index::DWARVEN_CATAPULT);
    e.apply(P0, PlayerAction::ChooseNumber(5)).unwrap();
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);

    // Grizzly Bears (2/2) dies to 2 damage; Hill Giant (3/3) survives with 2 damage marked.
    assert_eq!(e.state().object(bears).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(giant).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(giant).unwrap().damage, 2);
}

#[test]
fn high_tide_adds_extra_blue_mana_whenever_any_player_taps_an_island_for_mana() {
    let mut e = Duel::new(SEED, index::ISLAND)
        .hand(0, &[index::HIGH_TIDE, index::HIGH_TIDE])
        .battlefield(0, &[index::ISLAND, index::ISLAND])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    let islands: Vec<ObjectId> = e.state().zones.list(ZoneLocation::Battlefield).clone();

    // Tap first Island to cast first High Tide.
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: islands[0],
            ability_index: 0,
        },
    )
    .unwrap();
    let ht1 = in_hand(&e, P0, index::HIGH_TIDE).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: ht1 }).unwrap();
    pass_until(&mut e, stack_is_empty);

    // Tap second Island: produces {U} + {U} from High Tide's delayed trigger!
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: islands[1],
            ability_index: 0,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Blue), 2);
}

#[test]
fn soul_exchange_exiles_creature_reanimates_target_and_adds_p2p2_counter_if_thrull() {
    let mut e = prepared(
        &[index::SOUL_EXCHANGE],
        &[index::BASAL_THRULL, index::GRIZZLY_BEARS],
        &[],
    );
    let thrull = on_battlefield(&e, P0, index::BASAL_THRULL).unwrap();
    let bears_bf = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    e.dev_state_mut(P0)
        .unwrap()
        .move_object(
            bears_bf,
            ZoneLocation::Graveyard(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    let bears = e.state().zones.list(ZoneLocation::Graveyard(P0))[0];

    cast(&mut e, P0, index::SOUL_EXCHANGE);
    targets(&mut e, vec![bears], vec![]);
    choose_objects(&mut e, vec![thrull]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().object(thrull).unwrap().zone, Zone::Exile);
    assert_eq!(e.state().object(bears).unwrap().zone, Zone::Battlefield);
    // Grizzly Bears (2/2) received a +2/+2 counter because the exiled creature was a Thrull -> 4/4!
    assert_eq!(
        e.state().object(bears).unwrap().characteristics().power,
        Some(4)
    );
    assert_eq!(
        e.state().object(bears).unwrap().characteristics().toughness,
        Some(4)
    );
}

#[test]
fn conch_horn_draws_two_cards_and_puts_one_from_hand_on_top_of_library() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, &[index::GRIZZLY_BEARS])
        .battlefield(0, &[index::CONCH_HORN])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);

    let bears = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let lib_before = e.state().zones.list(ZoneLocation::Library(P0)).len();
    activate_by_card(&mut e, P0, index::CONCH_HORN, 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { player: P0, .. })
    });
    choose_objects(&mut e, vec![bears]);
    pass_until(&mut e, stack_is_empty);

    // Drew 2, put 1 back -> library is lib_before - 1, and Grizzly Bears is on top of library.
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        lib_before - 1
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).last(),
        Some(&bears)
    );
}

#[test]
fn spirit_shield_and_zelyon_sword_pump_while_tapped() {
    let mut e = prepared(
        &[],
        &[
            index::SPIRIT_SHIELD,
            index::ZELYON_SWORD,
            index::GRIZZLY_BEARS,
        ],
        &[],
    );
    let shield = on_battlefield(&e, P0, index::SPIRIT_SHIELD).unwrap();
    let sword = on_battlefield(&e, P0, index::ZELYON_SWORD).unwrap();
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();

    activate_by_card(&mut e, P0, index::SPIRIT_SHIELD, 1);
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    activate_by_card(&mut e, P0, index::ZELYON_SWORD, 1);
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    // Grizzly Bears (2/2) gets +0/+2 from Spirit Shield and +2/+0 from Zelyon Sword -> 4/4.
    assert_eq!(
        e.state().object(bears).unwrap().characteristics().power,
        Some(4)
    );
    assert_eq!(
        e.state().object(bears).unwrap().characteristics().toughness,
        Some(4)
    );

    // Untapping both ends their pump effects.
    e.state.set_tapped(shield, false);
    e.state.set_tapped(sword, false);
    e.sync_static_effects();
    e.state.refresh_characteristics();
    assert_eq!(
        e.state().object(bears).unwrap().characteristics().power,
        Some(2)
    );
    assert_eq!(
        e.state().object(bears).unwrap().characteristics().toughness,
        Some(2)
    );
}

#[test]
fn elvish_scout_untaps_attacking_creature_and_prevents_combat_damage_to_and_by_it() {
    let mut e = prepared(
        &[],
        &[index::ELVISH_SCOUT, index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
    );
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&e, P1, index::HILL_GIANT).unwrap();

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareBlockers);
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(giant, bears)],
        },
    )
    .unwrap();

    activate_by_card(&mut e, P0, index::ELVISH_SCOUT, 0);
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);

    // Grizzly Bears was untapped and survived combat with Hill Giant (3/3), dealing 0 damage to it!
    assert!(
        !e.state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(e.state().object(bears).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(giant).unwrap().damage, 0);
}

#[test]
fn initiates_of_the_ebon_hand_and_farrelite_priest_filter_mana_and_sacrifice_at_end_step_if_activated_four_times()
 {
    let mut e = prepared(
        &[],
        &[index::INITIATES_OF_THE_EBON_HAND, index::FARRELITE_PRIEST],
        &[],
    );
    for _ in 0..4 {
        activate_by_card(&mut e, P0, index::INITIATES_OF_THE_EBON_HAND, 0);
        activate_by_card(&mut e, P0, index::FARRELITE_PRIEST, 0);
    }
    pass_until(&mut e, |e| e.state().turn.active == P1);

    assert!(on_battlefield(&e, P0, index::INITIATES_OF_THE_EBON_HAND).is_none());
    assert!(on_battlefield(&e, P0, index::FARRELITE_PRIEST).is_none());
}

#[test]
fn orcish_captain_flips_coin_to_modify_target_orc_and_orcish_spy_looks_at_top_three_cards_privately()
 {
    let mut e = prepared(
        &[],
        &[
            index::ORCISH_CAPTAIN,
            index::ORCISH_SPY,
            index::IRONCLAW_ORCS,
        ],
        &[],
    );
    let orcs = on_battlefield(&e, P0, index::IRONCLAW_ORCS).unwrap();
    activate_by_card(&mut e, P0, index::ORCISH_CAPTAIN, 0);
    targets(&mut e, vec![orcs], vec![]);
    pass_until(&mut e, stack_is_empty);

    // Depending on coin flip, Ironclaw Orcs (2/2) is either 4/2 (+2/+0) or died to SBA at 2/0 (-0/-2).
    if let Some(o) = e
        .state()
        .object(orcs)
        .filter(|o| o.zone == Zone::Battlefield)
    {
        assert_eq!(o.characteristics().power, Some(4));
    } else {
        assert!(on_battlefield(&e, P0, index::IRONCLAW_ORCS).is_none());
    }

    // Orcish Spy looks at top 3 cards of P1's library: P0 sees them in ChooseCards(LookAtLibrary), P1 does not see Revealed!
    let seq_before = e.state().journal.last_seq();
    activate_by_card(&mut e, P0, index::ORCISH_SPY, 0);
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                player: P0,
                prompt: ChoicePrompt::LookAtLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 3);
    choose_objects(&mut e, vec![]);
    pass_until(&mut e, stack_is_empty);
    let any_revealed = e.state().journal.entries().iter().any(|entry| {
        entry.seq > seq_before && matches!(entry.event, crate::event::GameEvent::Revealed { .. })
    });
    assert!(!any_revealed);
}

#[test]
fn elvish_hunter_locks_untap_and_svyelunite_priest_grants_shroud_during_upkeep() {
    let mut e = prepared(
        &[],
        &[index::ELVISH_HUNTER, index::SVYELUNITE_PRIEST],
        &[index::GRIZZLY_BEARS],
    );
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    e.state.set_tapped(bears, true);

    activate_by_card(&mut e, P0, index::ELVISH_HUNTER, 0);
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    // On P1's turn, Grizzly Bears does not untap during P1's untap step.
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    assert!(
        e.state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    // Advance to P0's upkeep: Svyelunite Priest can activate during P0's upkeep to grant shroud.
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    let hunter = on_battlefield(&e, P0, index::ELVISH_HUNTER).unwrap();
    activate_by_card(&mut e, P0, index::SVYELUNITE_PRIEST, 0);
    targets(&mut e, vec![hunter], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .object(hunter)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::SHROUD)
    );
}

#[test]
fn farrels_zealot_deals_3_damage_to_target_creature_when_unblocked_and_assigns_no_combat_damage() {
    let mut e = prepared(&[], &[index::FARREL_S_ZEALOT], &[index::HILL_GIANT]);
    let zealot = on_battlefield(&e, P0, index::FARREL_S_ZEALOT).unwrap();
    let giant = on_battlefield(&e, P1, index::HILL_GIANT).unwrap();
    e.state.set_tapped(giant, true);

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(zealot, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareBlockers);
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    targets(&mut e, vec![giant], vec![]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut e, true);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(on_battlefield(&e, P1, index::HILL_GIANT).is_none());
    assert_eq!(e.state().players[1].life, 20);
}

#[test]
fn goblin_flotilla_grants_first_strike_to_blockers_unless_r_is_paid_at_beginning_of_combat() {
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &[index::GOBLIN_FLOTILLA])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    let flotilla = on_battlefield(&e, P0, index::GOBLIN_FLOTILLA).unwrap();
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();

    // P0 declines or has 0 mana at Beginning of Combat -> cannot pay {R}.
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
            || e.state().turn.step == Step::DeclareAttackers
    });
    if matches!(e.pending(), Pending::YesNo { .. }) {
        yes_no(&mut e, false);
        pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    }
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(flotilla, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareBlockers);
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(bears, flotilla)],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    // Grizzly Bears gained first strike from Goblin Flotilla's delayed trigger!
    assert!(
        e.state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE)
    );
}

#[test]
fn thelonite_druid_animates_forests_and_thrull_wizard_counters_black_spells() {
    let mut e = prepared(
        &[],
        &[
            index::THELONITE_DRUID,
            index::GRIZZLY_BEARS,
            index::FOREST,
            index::THRULL_WIZARD,
        ],
        &[],
    );
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let forest = on_battlefield(&e, P0, index::FOREST).unwrap();

    activate_by_card(&mut e, P0, index::THELONITE_DRUID, 0);
    choose_objects(&mut e, vec![bears]);
    pass_until(&mut e, stack_is_empty);

    let chars = e.state().object(forest).unwrap().characteristics();
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.types.contains(TypeSet::LAND));
    assert_eq!(chars.power, Some(2));
    assert_eq!(chars.toughness, Some(3));
}

#[test]
fn icatian_skirmishers_grants_first_strike_to_bandmates_and_hand_of_justice_destroys_target_creature()
 {
    let mut e = prepared(
        &[],
        &[
            index::HAND_OF_JUSTICE,
            index::ICATIAN_SKIRMISHERS,
            index::ICATIAN_INFANTRY,
            index::MESA_PEGASUS,
        ],
        &[index::HILL_GIANT],
    );
    let s1 = on_battlefield(&e, P0, index::ICATIAN_SKIRMISHERS).unwrap();
    let s2 = on_battlefield(&e, P0, index::ICATIAN_INFANTRY).unwrap();
    let s3 = on_battlefield(&e, P0, index::MESA_PEGASUS).unwrap();
    let giant = on_battlefield(&e, P1, index::HILL_GIANT).unwrap();

    activate_by_card(&mut e, P0, index::HAND_OF_JUSTICE, 0);
    targets(&mut e, vec![giant], vec![]);
    choose_objects(&mut e, vec![s1, s2, s3]);
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, P1, index::HILL_GIANT).is_none());
}

#[test]
fn thrull_champion_lords_thrulls_and_gains_control_while_controlled() {
    let mut e = prepared(&[], &[index::THRULL_CHAMPION], &[index::BASAL_THRULL]);
    let thrull = on_battlefield(&e, P1, index::BASAL_THRULL).unwrap();
    // Basal Thrull (1/2) gets +1/+1 -> 2/3.
    assert_eq!(
        e.state().object(thrull).unwrap().characteristics().power,
        Some(2)
    );
    assert_eq!(
        e.state()
            .object(thrull)
            .unwrap()
            .characteristics()
            .toughness,
        Some(3)
    );

    activate_by_card(&mut e, P0, index::THRULL_CHAMPION, 1);
    targets(&mut e, vec![thrull], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(thrull).unwrap().controller, P0);
}

#[test]
fn goblin_kites_homarid_spawning_bed_goblin_warrens_thelons_curse_and_spore_cloud() {
    // Goblin Warrens: sacrifice 2 Goblins -> create 3 1/1 red Goblin tokens.
    // Homarid Spawning Bed: sacrifice a blue creature -> create X 1/1 blue Camarid tokens (X = MV).
    let mut e = prepared(
        &[index::SPORE_CLOUD],
        &[
            index::GOBLIN_KITES,
            index::GOBLIN_WARRENS,
            index::MONS_S_GOBLIN_RAIDERS,
            index::MONS_S_GOBLIN_RAIDERS,
            index::HOMARID_SPAWNING_BED,
            index::HOMARID_WARRIOR,
            index::THELON_S_CURSE,
        ],
        &[],
    );
    let goblins: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            e.state()
                .object(id)
                .unwrap()
                .card
                .is_some_and(|c| c.index == index::MONS_S_GOBLIN_RAIDERS)
        })
        .collect();

    // Goblin Kites grants flying to target creature you control with toughness <= 2.
    activate_by_card(&mut e, P0, index::GOBLIN_KITES, 0);
    targets(&mut e, vec![goblins[0]], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .object(goblins[0])
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );

    // Goblin Warrens sacrifices the 2 Goblins (one per Sacrifice cost part) to create 3 Goblin tokens.
    activate_by_card(&mut e, P0, index::GOBLIN_WARRENS, 0);
    choose_objects(&mut e, vec![goblins[0]]);
    choose_objects(&mut e, vec![goblins[1]]);
    pass_until(&mut e, stack_is_empty);

    // Homarid Spawning Bed sacrifices Homarid Warrior (MV 5) to create 5 Camarid tokens.
    let warrior = on_battlefield(&e, P0, index::HOMARID_WARRIOR).unwrap();
    activate_by_card(&mut e, P0, index::HOMARID_SPAWNING_BED, 0);
    choose_objects(&mut e, vec![warrior]);
    pass_until(&mut e, stack_is_empty);

    let tokens_count = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| !e.state().object(id).unwrap().is_card())
        .count();
    assert_eq!(tokens_count, 8);
}

#[test]
fn thallid_spore_ecosystem_icatian_javelineers_and_draconian_cylix() {
    // Promoted oracle_ids:
    // - Draconian Cylix: "0880b36a-6141-4955-b532-cf88daca0869"
    // - Elvish Farmer: "91fe1075-e170-4575-ae28-8d692f051637"
    // - Feral Thallid: "3953fd17-03f4-4431-99b5-87dcac5a279d"
    // - Fungal Bloom: "fa76ee16-ff48-418b-819a-69f1139b36f3"
    // - Icatian Javelineers: "26112ba7-2859-4af3-93c3-e62c31177216"
    // - Spore Flower: "f352ec62-bf09-45f2-b5b7-75931177b15f"
    // - Thallid: "46abaabe-7015-4d82-a5aa-2706b1f51bed"
    // - Thallid Devourer: "b7326fd9-7648-4db3-b506-61a7de8a4094"
    // - Thorn Thallid: "1239d3bf-06e8-4732-b1e2-f6eb145e5894"
    let mut e = prepared(
        &[index::ICATIAN_JAVELINEERS, index::FOREST],
        &[
            index::DRACONIAN_CYLIX,
            index::ELVISH_FARMER,
            index::FERAL_THALLID,
            index::FUNGAL_BLOOM,
            index::SPORE_FLOWER,
            index::THALLID,
            index::THALLID_DEVOURER,
            index::THORN_THALLID,
        ],
        &[],
    );

    // Cast Icatian Javelineers: enters with 1 javelin counter, then taps & removes it to deal 1 damage to P1.
    cast(&mut e, P0, index::ICATIAN_JAVELINEERS);
    pass_until(&mut e, stack_is_empty);
    let jav = on_battlefield(&e, P0, index::ICATIAN_JAVELINEERS).unwrap();
    e.state.object_mut(jav).unwrap().controlled_since = 0;
    activate_by_card(&mut e, P0, index::ICATIAN_JAVELINEERS, 0);
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[1].life, 19);

    // Fungal Bloom puts 2 more spore counters on Thallid, Thorn Thallid, Feral Thallid, Spore Flower, and Thallid Devourer
    // (each already got 1 spore counter on Turn 1's upkeep).
    for c in [
        index::THALLID,
        index::THORN_THALLID,
        index::FERAL_THALLID,
        index::SPORE_FLOWER,
        index::THALLID_DEVOURER,
    ] {
        let f = on_battlefield(&e, P0, c).unwrap();
        for _ in 0..2 {
            activate_by_card(&mut e, P0, index::FUNGAL_BLOOM, 0);
            targets(&mut e, vec![f], vec![]);
            pass_until(&mut e, stack_is_empty);
        }
    }

    // Thallid and Thallid Devourer each remove 3 spore counters (ability 1) to create a 1/1 Saproling token.
    activate_by_card(&mut e, P0, index::THALLID, 1);
    pass_until(&mut e, stack_is_empty);
    activate_by_card(&mut e, P0, index::THALLID_DEVOURER, 1);
    pass_until(&mut e, stack_is_empty);

    let saprolings: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| !e.state().object(id).unwrap().is_card())
        .collect();
    assert_eq!(saprolings.len(), 2);

    // Elvish Farmer sacrifices first Saproling (ability 2) to gain 2 life (20 -> 22).
    activate_by_card(&mut e, P0, index::ELVISH_FARMER, 2);
    choose_objects(&mut e, vec![saprolings[0]]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 22);

    // Thallid Devourer sacrifices second Saproling (ability 2) to get +1/+2 (2/2 -> 3/4).
    let devourer = on_battlefield(&e, P0, index::THALLID_DEVOURER).unwrap();
    activate_by_card(&mut e, P0, index::THALLID_DEVOURER, 2);
    choose_objects(&mut e, vec![saprolings[1]]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(devourer).unwrap().characteristics().power,
        Some(3)
    );
    assert_eq!(
        e.state()
            .object(devourer)
            .unwrap()
            .characteristics()
            .toughness,
        Some(4)
    );

    // Thorn Thallid removes 3 spore counters (ability 1) to deal 1 damage to P1 (19 -> 18).
    activate_by_card(&mut e, P0, index::THORN_THALLID, 1);
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[1].life, 18);

    // Feral Thallid removes 3 spore counters (ability 1) to regenerate itself; Spore Flower removes 3 spore counters (ability 1) to prevent combat damage.
    let feral = on_battlefield(&e, P0, index::FERAL_THALLID).unwrap();
    activate_by_card(&mut e, P0, index::FERAL_THALLID, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(feral).unwrap().regeneration_shields, 1);

    activate_by_card(&mut e, P0, index::SPORE_FLOWER, 1);
    pass_until(&mut e, stack_is_empty);

    // Draconian Cylix discards a card at random (Forest) to regenerate Thallid.
    let thallid = on_battlefield(&e, P0, index::THALLID).unwrap();
    activate_by_card(&mut e, P0, index::DRACONIAN_CYLIX, 0);
    targets(&mut e, vec![thallid], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(thallid).unwrap().regeneration_shields, 1);
}
