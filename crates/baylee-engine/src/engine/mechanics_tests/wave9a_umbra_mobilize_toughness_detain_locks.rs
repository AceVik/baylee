//! Rule tests for Engine Wave 9a:
//! 1. `KeywordSet::UMBRA_ARMOR` (`CR 702.89a–b`)
//! 2. `KeywordSet::MOBILIZE` & `Effect::Mobilize` (`CR 702.181a`, `CR 508.4`)
//! 3. `Modifier::AssignCombatDamageUsingToughness*` & `MayAssignCombatDamageAsThoughUnblocked` (`CR 510.1c`)
//! 4. `Effect::Detain` & `Effect::DetainFilter` (`CR 701.35a`)
//! 5. `Modifier::PlayersCantGainLife` (`CR 119.7`, `CR 614.17c`) & `Modifier::CantBeSacrificed` (`CR 701.21`)

use super::*;
use crate::choice::YesNoPrompt;
use crate::object::Status;
use crate::turn::Step;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{
    Amount, CounterKind, Duration, Filter, Modifier, PlayerRel, TargetSpec, Trigger, chapter, cost,
    spell, static_ability, triggered,
};
use baylee_core::color::Color;
use baylee_core::generated::subtypes::{creature, enchantment};
use baylee_core::ids::Defender;

const C0: u32 = 99_940;
const C1: u32 = 99_941;
const C2: u32 = 99_942;
const C3: u32 = 99_943;
const C4: u32 = 99_944;
const C5: u32 = 99_945;

fn aura_face(name: &'static str) -> FaceDef {
    FaceDef {
        subtypes: &[enchantment::AURA],
        ..face(name, "{0}", TypeSet::ENCHANTMENT)
    }
}

fn is_tapped(engine: &Bench, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(Status::TAPPED)
}

fn target_obj(engine: &mut Bench, seat: PlayerId, target: ObjectId) {
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("choose object target");
}

fn cast_aura_on(engine: &mut Bench, seat: PlayerId, aura: ObjectId, host: ObjectId) {
    engine
        .apply(seat, PlayerAction::CastSpell { card: aura })
        .expect("cast aura");
    target_obj(engine, seat, host);
    settle(engine, seat);
}

// =========================================================================
// 1. Umbra Armor (CR 702.89a–b)
// =========================================================================

#[test]
fn umbra_armor_replaces_lethal_deathtouch_sba_without_tapping_or_leaving_combat() {
    static AURA_AB: &[AbilityDef] = &[spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    let bear = card(
        C0,
        creature_face("Bear", "{0}", 2, 2),
        KeywordSet::VIGILANCE,
        &[],
    );
    let umbra = card(
        C1,
        aura_face("Hyena Umbra"),
        KeywordSet::UMBRA_ARMOR,
        AURA_AB,
    );
    let viper = card(
        C2,
        creature_face("Viper", "{0}", 1, 3),
        KeywordSet::DEATHTOUCH,
        &[],
    );
    let mut engine = bench(
        1,
        vec![bear, umbra, viper],
        [Seat::with(&[C0]).holding(&[C1]), Seat::with(&[C2])],
    );
    to_main(&mut engine, me());
    let bear_id = the(&engine, ZoneLocation::Battlefield, C0);
    let umbra_hand = the(&engine, ZoneLocation::Hand(me()), C1);
    let viper_id = the(&engine, ZoneLocation::Battlefield, C2);
    cast_aura_on(&mut engine, me(), umbra_hand, bear_id);
    let umbra_bf = the(&engine, ZoneLocation::Battlefield, C1);

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear_id, Defender::Player(them()))],
            },
        )
        .expect("declare bear attacker");
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(viper_id, bear_id)],
            },
        )
        .expect("viper blocks bear");
    walk_until(&mut engine, |e| holds_priority_in(e, me(), Step::CombatEnd));

    let bear_obj = engine.state().object(bear_id).unwrap();
    assert_eq!(bear_obj.zone, Zone::Battlefield);
    assert_eq!(bear_obj.damage, 0);
    assert!(!bear_obj.deathtouched);
    assert!(
        !is_tapped(&engine, bear_id),
        "umbra armor must not tap host"
    );
    assert!(
        engine.state().combat.is_attacking(bear_id),
        "umbra armor must not remove host from combat"
    );
    assert_eq!(
        engine.state().object(umbra_bf).unwrap().zone,
        Zone::Graveyard
    );
}

#[test]
fn umbra_armor_consumes_one_aura_and_saves_host_from_simultaneous_destroy_all() {
    static AURA_AB: &[AbilityDef] = &[spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static RIG_AB: &[AbilityDef] = &[
        free(
            &[Effect::Destroy {
                target: TargetSpec::Object(&Filter::CREATURE),
                no_regen: true,
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        free(
            &[Effect::DestroyAll {
                filter: &Filter::NONLAND,
                no_regen: true,
            }],
            None,
        ),
    ];
    let bear = card(
        C0,
        creature_face("Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let umbra_a = card(C1, aura_face("Umbra A"), KeywordSet::UMBRA_ARMOR, AURA_AB);
    let umbra_b = card(C2, aura_face("Umbra B"), KeywordSet::UMBRA_ARMOR, AURA_AB);
    let rig = card(
        C3,
        face("Rig", "{0}", TypeSet::LAND),
        KeywordSet::EMPTY,
        RIG_AB,
    );
    let mut engine = bench(
        2,
        vec![bear, umbra_a, umbra_b, rig],
        [Seat::with(&[C0, C3]).holding(&[C1, C2]), Seat::default()],
    );
    to_main(&mut engine, me());
    let bear_id = the(&engine, ZoneLocation::Battlefield, C0);
    let rig_id = the(&engine, ZoneLocation::Battlefield, C3);
    let a_hand = the(&engine, ZoneLocation::Hand(me()), C1);
    let b_hand = the(&engine, ZoneLocation::Hand(me()), C2);
    cast_aura_on(&mut engine, me(), a_hand, bear_id);
    cast_aura_on(&mut engine, me(), b_hand, bear_id);

    // 1. Terminate-style no_regen Destroy consumes ONLY ONE Umbra Aura.
    activate(&mut engine, me(), rig_id, 0);
    target_obj(&mut engine, me(), bear_id);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(bear_id).unwrap().zone,
        Zone::Battlefield
    );
    let remaining_auras = objects(&engine, ZoneLocation::Battlefield, C1).len()
        + objects(&engine, ZoneLocation::Battlefield, C2).len();
    assert_eq!(remaining_auras, 1, "exactly one umbra aura remains");

    // 2. Simultaneous DestroyAll (Planar Cleansing ruling) destroys remaining Umbra and saves Bear.
    activate(&mut engine, me(), rig_id, 1);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(bear_id).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, C1).len(), 0);
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, C2).len(), 0);
}

#[test]
fn umbra_armor_ignores_indestructible_and_regeneration_saved_hosts() {
    static AURA_AB: &[AbilityDef] = &[spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static RIG_AB: &[AbilityDef] = &[
        free(
            &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        free(
            &[Effect::Regenerate {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
    ];
    let titan = card(
        C0,
        creature_face("Titan", "{0}", 3, 3),
        KeywordSet::INDESTRUCTIBLE,
        &[],
    );
    let troll = card(
        C1,
        creature_face("Troll", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let umbra = card(
        C2,
        aura_face("Spider Umbra"),
        KeywordSet::UMBRA_ARMOR,
        AURA_AB,
    );
    let rig = card(
        C3,
        face("Rig", "{0}", TypeSet::LAND),
        KeywordSet::EMPTY,
        RIG_AB,
    );
    let mut engine = bench(
        3,
        vec![titan, troll, umbra, rig],
        [
            Seat::with(&[C0, C1, C3]).holding(&[C2, C2]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let titan_id = the(&engine, ZoneLocation::Battlefield, C0);
    let troll_id = the(&engine, ZoneLocation::Battlefield, C1);
    let rig_id = the(&engine, ZoneLocation::Battlefield, C3);
    let u_hand = objects(&engine, ZoneLocation::Hand(me()), C2);
    cast_aura_on(&mut engine, me(), u_hand[0], titan_id);
    cast_aura_on(&mut engine, me(), u_hand[1], troll_id);
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, C2).len(), 2);

    // Destroy indestructible Titan -> Umbra stays!
    activate(&mut engine, me(), rig_id, 0);
    target_obj(&mut engine, me(), titan_id);
    settle(&mut engine, me());
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, C2).len(), 2);

    // Regenerate Troll, then destroy it -> Regeneration shield saves & taps Troll; Umbra stays!
    activate(&mut engine, me(), rig_id, 1);
    target_obj(&mut engine, me(), troll_id);
    settle(&mut engine, me());
    activate(&mut engine, me(), rig_id, 0);
    target_obj(&mut engine, me(), troll_id);
    settle(&mut engine, me());
    assert!(is_tapped(&engine, troll_id));
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, C2).len(), 2);
}

// =========================================================================
// 2. Mobilize N (CR 702.181a, CR 508.4)
// =========================================================================

#[test]
fn mobilize_creates_tapped_attacking_red_warriors_and_sacrifices_at_end_step() {
    static MOB_AB: &[AbilityDef] = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::mobilize(2)]
    )];
    let striker = card(
        C0,
        creature_face("Mardu Striker", "{0}", 2, 3),
        KeywordSet::MOBILIZE,
        MOB_AB,
    );
    let mut engine = bench(4, vec![striker], [Seat::with(&[C0]), Seat::default()]);
    to_main(&mut engine, me());
    let striker_id = the(&engine, ZoneLocation::Battlefield, C0);

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(striker_id, Defender::Player(them()))],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    let warriors: Vec<ObjectId> = engine
        .state()
        .battlefield_seen()
        .filter(|&id| id != striker_id && engine.state().object(id).unwrap().token.is_some())
        .collect();
    assert_eq!(warriors.len(), 2);
    for &w in &warriors {
        let obj = engine.state().object(w).unwrap();
        let ch = obj.characteristics();
        assert!(is_tapped(&engine, w));
        assert!(engine.state().combat.is_attacking(w));
        assert_eq!(ch.power, Some(1));
        assert_eq!(ch.toughness, Some(1));
        assert!(ch.colors.contains(Color::Red));
        assert!(ch.subtypes.contains(creature::WARRIOR));
    }

    // Combat damage: 2 (Striker) + 1 + 1 (Warriors) = 4 damage to them().
    to_main(&mut engine, them());
    assert_eq!(life(&engine, them()), 16);
    let surviving_tokens = engine
        .state()
        .battlefield_seen()
        .filter(|&id| engine.state().object(id).unwrap().token.is_some())
        .count();
    assert_eq!(
        surviving_tokens, 0,
        "mobilize tokens sacrificed at next end step"
    );
}

#[test]
fn mobilize_in_multiplayer_chooses_defender_per_token() {
    static MOB_AB: &[AbilityDef] = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::mobilize(2)]
    )];
    let striker = card(
        C0,
        creature_face("Stadium Headliner", "{0}", 1, 1),
        KeywordSet::MOBILIZE,
        MOB_AB,
    );
    let p1 = PlayerId::new(1);
    let p2 = PlayerId::new(2);
    let mut engine = bench(
        5,
        vec![striker],
        [Seat::with(&[C0]), Seat::default(), Seat::default()],
    );
    to_main(&mut engine, me());
    let striker_id = the(&engine, ZoneLocation::Battlefield, C0);

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(striker_id, Defender::Player(p1))],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine
        .apply(me(), PlayerAction::ChoosePlayer(p1))
        .expect("first warrior attacks p1");
    engine
        .apply(me(), PlayerAction::ChoosePlayer(p2))
        .expect("second warrior attacks p2");
    to_main(&mut engine, p1);

    assert_eq!(
        life(&engine, p1),
        18,
        "p1 took 1 from striker + 1 from warrior"
    );
    assert_eq!(life(&engine, p2), 19, "p2 took 1 from second warrior");
}

// =========================================================================
// 3. Toughness Combat Damage & MayAssignCombatDamageAsThoughUnblocked (CR 510.1c)
// =========================================================================

#[test]
fn combat_damage_using_toughness_applies_to_trample_and_low_toughness_attackers() {
    static DORAN_AB: &[AbilityDef] = &[static_ability!(
        Filter::CREATURE,
        Modifier::AssignCombatDamageUsingToughness
    )];
    let doran_ench = card(
        C0,
        face("Assault Form", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        DORAN_AB,
    );
    let ox = card(
        C1,
        creature_face("Ox", "{0}", 0, 5),
        KeywordSet::TRAMPLE,
        &[],
    );
    let glass = card(
        C2,
        creature_face("Glass", "{0}", 4, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let bear = card(
        C3,
        creature_face("Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let mut engine = bench(
        6,
        vec![doran_ench, ox, glass, bear],
        [Seat::with(&[C0, C1, C2]), Seat::with(&[C3])],
    );
    to_main(&mut engine, me());
    let ox_id = the(&engine, ZoneLocation::Battlefield, C1);
    let glass_id = the(&engine, ZoneLocation::Battlefield, C2);
    let bear_id = the(&engine, ZoneLocation::Battlefield, C3);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (ox_id, Defender::Player(them())),
                    (glass_id, Defender::Player(them())),
                ],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear_id, ox_id)],
            },
        )
        .unwrap();
    to_main(&mut engine, them());
    assert_eq!(
        life(&engine, them()),
        16,
        "3 trample from 0/5 + 1 from 4/1 = 4"
    );
}

#[test]
fn combat_damage_using_toughness_if_greater_preserves_higher_power() {
    static RASAAD_AB: &[AbilityDef] = &[static_ability!(
        Filter::ControlledByYou,
        Modifier::AssignCombatDamageUsingToughnessIfToughnessGreater
    )];
    let ox = card(
        C1,
        creature_face("Ox", "{0}", 0, 5),
        KeywordSet::TRAMPLE,
        &[],
    );
    let glass = card(
        C2,
        creature_face("Glass", "{0}", 4, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let rasaad = card(
        C4,
        face("Huetsli", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        RASAAD_AB,
    );
    let mut engine = bench(
        7,
        vec![rasaad, ox, glass],
        [Seat::with(&[C4, C1, C2]), Seat::default()],
    );
    to_main(&mut engine, me());
    let ox_id = the(&engine, ZoneLocation::Battlefield, C1);
    let glass_id = the(&engine, ZoneLocation::Battlefield, C2);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (ox_id, Defender::Player(them())),
                    (glass_id, Defender::Player(them())),
                ],
            },
        )
        .unwrap();
    to_main(&mut engine, them());
    assert_eq!(life(&engine, them()), 11, "5 from 0/5 + 4 from 4/1 = 9");
}

#[test]
fn may_assign_combat_damage_as_though_unblocked_yes_vs_no() {
    static THORN_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::MayAssignCombatDamageAsThoughUnblocked
    )];
    let thorn = card(
        C0,
        creature_face("Thorn Elemental", "{0}", 5, 5),
        KeywordSet::VIGILANCE,
        THORN_AB,
    );
    let wall = card(
        C1,
        creature_face("Wall", "{0}", 2, 6),
        KeywordSet::DEFENDER,
        &[],
    );
    let mut engine = bench(8, vec![thorn, wall], [Seat::with(&[C0]), Seat::with(&[C1])]);
    to_main(&mut engine, me());
    let thorn_id = the(&engine, ZoneLocation::Battlefield, C0);
    let wall_id = the(&engine, ZoneLocation::Battlefield, C1);

    // Combat 1: YesNo(true) -> 5 to defender, 0 to Wall, Wall deals 2 to Thorn.
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(thorn_id, Defender::Player(them()))],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall_id, thorn_id)],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::AssignDamageAsThoughUnblocked { .. },
                ..
            }
        )
    });
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    walk_until(&mut engine, |e| holds_priority_in(e, me(), Step::CombatEnd));
    assert_eq!(life(&engine, them()), 15);
    assert_eq!(engine.state().object(wall_id).unwrap().damage, 0);
    assert_eq!(engine.state().object(thorn_id).unwrap().damage, 2);

    // Combat 2: YesNo(false) -> 5 to Wall, 0 to defender!
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(thorn_id, Defender::Player(them()))],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall_id, thorn_id)],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::AssignDamageAsThoughUnblocked { .. },
                ..
            }
        )
    });
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    walk_until(&mut engine, |e| holds_priority_in(e, me(), Step::CombatEnd));
    assert_eq!(life(&engine, them()), 15);
    assert_eq!(engine.state().object(wall_id).unwrap().damage, 5);
}

#[test]
fn may_assign_combat_damage_as_though_unblocked_after_blocker_sacrificed() {
    static THORN_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::MayAssignCombatDamageAsThoughUnblocked
    )];
    static SAC_SELF: &[AbilityDef] = &[paid(cost!(SacrificeSelf), &[], None)];
    let thorn = card(
        C0,
        creature_face("Thorn Elemental", "{0}", 5, 5),
        KeywordSet::EMPTY,
        THORN_AB,
    );
    let fanatic = card(
        C2,
        creature_face("Fanatic", "{0}", 1, 1),
        KeywordSet::EMPTY,
        SAC_SELF,
    );
    let mut engine = bench(
        15,
        vec![thorn, fanatic],
        [Seat::with(&[C0]), Seat::with(&[C2])],
    );
    to_main(&mut engine, me());
    let thorn_id = the(&engine, ZoneLocation::Battlefield, C0);
    let fanatic_id = the(&engine, ZoneLocation::Battlefield, C2);

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(thorn_id, Defender::Player(them()))],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(fanatic_id, thorn_id)],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), Step::DeclareBlockers)
    });
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(&mut engine, them(), fanatic_id, 0);
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::AssignDamageAsThoughUnblocked { .. },
                ..
            }
        )
    });
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    to_main(&mut engine, them());
    assert_eq!(life(&engine, them()), 15);
}

// =========================================================================
// 4. Detain (CR 701.35a)
// =========================================================================

#[test]
fn detain_locks_attacks_blocks_activated_and_intrinsic_mana_abilities() {
    static DETAIN_ALL_AB: &[AbilityDef] = &[free(
        &[Effect::DetainFilter {
            filter: &Filter::ControlledByOpponent,
        }],
        None,
    )];
    static RELIC_AB: &[AbilityDef] = &[free(&[Effect::gain_life(1)], None)];
    let rig = card(
        C0,
        creature_face("Arrester", "{0}", 2, 2),
        KeywordSet::EMPTY,
        DETAIN_ALL_AB,
    );
    let foe = card(
        C1,
        creature_face("Brute", "{0}", 3, 3),
        KeywordSet::HASTE,
        &[],
    );
    let relic = card(
        C2,
        face("Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        RELIC_AB,
    );
    let forest_idx = forest();
    let mut engine = bench(
        9,
        vec![rig, foe, relic],
        [Seat::with(&[C0]), Seat::with(&[C1, C2, forest_idx])],
    );
    to_main(&mut engine, me());
    let rig_id = the(&engine, ZoneLocation::Battlefield, C0);
    let relic_id = the(&engine, ZoneLocation::Battlefield, C2);
    let opp_forest = the(&engine, ZoneLocation::Battlefield, forest_idx);

    // Detain all opponent permanents (Brute, Relic, Forest).
    activate(&mut engine, me(), rig_id, 0);
    settle(&mut engine, me());

    // On me()'s combat, detained Brute cannot block Arrester.
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(rig_id, Defender::Player(them()))],
            },
        )
        .unwrap();
    to_main(&mut engine, them());
    assert_eq!(life(&engine, them()), 18);

    // On them()'s turn: Relic ability and intrinsic Forest mana are BOTH locked!
    let (_, legal_them) = offer(&engine);
    assert!(!legal_them.abilities.contains(&(relic_id, 0)));
    assert!(!legal_them.mana_abilities.contains(&opp_forest));
}

#[test]
fn detain_locks_later_animated_artifact_and_expires_on_detainers_next_turn() {
    static DETAIN_ONE_AB: &[AbilityDef] = &[free(
        &[Effect::detain_target(&Filter::Any)],
        Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
    )];
    static ANIMATE_AB: &[AbilityDef] = &[free(
        &[
            Effect::continuous(
                &Filter::ARTIFACT,
                Modifier::AnimateNoncreatureArtifact,
                Duration::UntilEndOfTurn,
            ),
            Effect::continuous(
                &Filter::ARTIFACT,
                Modifier::AddKeyword(KeywordSet::HASTE),
                Duration::UntilEndOfTurn,
            ),
        ],
        None,
    )];
    let rig = card(
        C0,
        creature_face("Arrester", "{0}", 2, 2),
        KeywordSet::EMPTY,
        DETAIN_ONE_AB,
    );
    let foe = card(
        C1,
        creature_face("Brute", "{0}", 3, 3),
        KeywordSet::HASTE,
        &[],
    );
    let relic = card(
        C2,
        face("Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let anim = card(
        C3,
        face("Animator", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        ANIMATE_AB,
    );
    let forest_idx = forest();
    let mut engine = bench(
        14,
        vec![rig, foe, relic, anim],
        [Seat::with(&[C0]), Seat::with(&[C1, C2, C3, forest_idx])],
    );
    to_main(&mut engine, me());
    let rig_id = the(&engine, ZoneLocation::Battlefield, C0);
    let foe_id = the(&engine, ZoneLocation::Battlefield, C1);
    let relic_id = the(&engine, ZoneLocation::Battlefield, C2);
    let anim_id = the(&engine, ZoneLocation::Battlefield, C3);
    let opp_forest = the(&engine, ZoneLocation::Battlefield, forest_idx);

    // Detain Forest on Turn 1 -> locked on Turn 2, unlocked once Turn 3 (me()'s next turn) begins.
    activate(&mut engine, me(), rig_id, 0);
    target_obj(&mut engine, me(), opp_forest);
    settle(&mut engine, me());
    to_main(&mut engine, them());
    assert!(!offer(&engine).1.mana_abilities.contains(&opp_forest));

    // Turn 3 (me()'s next turn): Forest is unlocked; now detain noncreature Relic, then animate it on Turn 4!
    to_main(&mut engine, me());
    activate(&mut engine, me(), rig_id, 0);
    target_obj(&mut engine, me(), relic_id);
    settle(&mut engine, me());
    to_main(&mut engine, them());
    assert!(offer(&engine).1.mana_abilities.contains(&opp_forest));
    activate(&mut engine, them(), anim_id, 0);
    settle(&mut engine, them());
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending() else {
        unreachable!();
    };
    assert!(!attackers.contains(&relic_id));
    assert!(attackers.contains(&foe_id));
}

// =========================================================================
// 5. PlayersCantGainLife (CR 119.7, CR 614.17c) & CantBeSacrificed (CR 701.21)
// =========================================================================

#[test]
fn players_cant_gain_life_blocks_gains_lifelink_upward_set_exchange_and_lich_draws() {
    static EREBOS_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::PlayersCantGainLife(PlayerRel::EachOpponent)
    )];
    static LICH_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::LifeGainDrawsInstead {
            who: PlayerRel::You
        }
    )];
    static PROBE_AB: &[AbilityDef] = &[
        free(&[Effect::gain_life(4)], None),
        free(
            &[Effect::SetLifeTotal {
                who: PlayerRel::You,
                amount: Amount::Fixed(25),
            }],
            None,
        ),
        free(
            &[Effect::SetLifeTotal {
                who: PlayerRel::You,
                amount: Amount::Fixed(12),
            }],
            None,
        ),
        free(
            &[Effect::ExchangeLifeTotals {
                with: PlayerRel::Opponent,
            }],
            None,
        ),
        free(
            &[Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::Player(PlayerRel::Opponent),
            }],
            None,
        ),
    ];
    let erebos = card(
        C0,
        face("Erebos", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        EREBOS_AB,
    );
    let lich = card(
        C1,
        face("Lich", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        LICH_AB,
    );
    let probe = card(
        C2,
        creature_face("Lifelinker", "{0}", 3, 3),
        KeywordSet::LIFELINK,
        PROBE_AB,
    );
    let mut engine = bench(
        10,
        vec![erebos, lich, probe],
        [Seat::with(&[C0]).at(20), Seat::with(&[C1, C2]).at(15)],
    );
    to_main(&mut engine, them());
    let probe_id = the(&engine, ZoneLocation::Battlefield, C2);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(them())).len();

    // 1. GainLife(4) under PlayersCantGainLife + Lich -> neither life gained NOR Lich cards drawn!
    activate(&mut engine, them(), probe_id, 0);
    settle(&mut engine, them());
    assert_eq!(life(&engine, them()), 15);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(them())).len(),
        hand_before
    );

    // 2. SetLifeTotal upward (15 -> 25) is blocked; ExchangeLifeTotals (15 <-> 20) is blocked!
    activate(&mut engine, them(), probe_id, 1);
    settle(&mut engine, them());
    assert_eq!(life(&engine, them()), 15);
    activate(&mut engine, them(), probe_id, 3);
    settle(&mut engine, them());
    assert_eq!(life(&engine, them()), 15);
    assert_eq!(life(&engine, me()), 20);

    // 3. SetLifeTotal downward (15 -> 12) STILL works!
    activate(&mut engine, them(), probe_id, 2);
    settle(&mut engine, them());
    assert_eq!(life(&engine, them()), 12);

    // 4. Lifelink damage STILL deals 3 damage to me() (20 -> 17) while them() gains 0 life!
    activate(&mut engine, them(), probe_id, 4);
    settle(&mut engine, them());
    assert_eq!(life(&engine, me()), 17);
    assert_eq!(life(&engine, them()), 12);
}

#[test]
fn cant_be_sacrificed_opponents_only_blocks_opponent_edict_but_allows_own_cost() {
    static SIGARDA_AB: &[AbilityDef] = &[static_ability!(
        Filter::ControlledByYou,
        Modifier::CantBeSacrificed {
            opponents_only: true
        }
    )];
    static EDICT_AB: &[AbilityDef] = &[free(
        &[Effect::SacrificeFilter {
            who: PlayerRel::Opponent,
            filter: &Filter::CREATURE,
        }],
        None,
    )];
    static OUTLET_AB: &[AbilityDef] = &[paid(
        cost!(Sacrifice(&Filter::CREATURE)),
        &[Effect::gain_life(1)],
        None,
    )];
    let sigarda = card(
        C0,
        creature_face("Sigarda", "{0}", 5, 5),
        KeywordSet::EMPTY,
        SIGARDA_AB,
    );
    let opp_edict = card(
        C1,
        face("Edict", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        EDICT_AB,
    );
    let outlet = card(
        C2,
        face("Altar", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        OUTLET_AB,
    );
    let mut engine = bench(
        11,
        vec![sigarda, opp_edict, outlet],
        [Seat::with(&[C0, C2]), Seat::with(&[C1])],
    );
    to_main(&mut engine, them());
    let sig_id = the(&engine, ZoneLocation::Battlefield, C0);
    let edict_id = the(&engine, ZoneLocation::Battlefield, C1);
    let outlet_id = the(&engine, ZoneLocation::Battlefield, C2);
    activate(&mut engine, them(), edict_id, 0);
    settle(&mut engine, them());
    assert_eq!(
        engine.state().object(sig_id).unwrap().zone,
        Zone::Battlefield,
        "opponent edict cannot cause sacrifice under Sigarda"
    );
    to_main(&mut engine, me());
    activate(&mut engine, me(), outlet_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![sig_id],
            },
        )
        .expect("own sacrifice cost allowed under Sigarda");
    settle(&mut engine, me());
    assert_eq!(engine.state().object(sig_id).unwrap().zone, Zone::Graveyard);
}

#[test]
fn cant_be_sacrificed_total_lock_forbids_own_costs_and_own_effects() {
    static SUIT_AB: &[AbilityDef] = &[static_ability!(
        Filter::CREATURE,
        Modifier::CantBeSacrificed {
            opponents_only: false
        }
    )];
    static OUTLET_AB: &[AbilityDef] = &[
        paid(
            cost!(Sacrifice(&Filter::CREATURE)),
            &[Effect::gain_life(1)],
            None,
        ),
        free(
            &[Effect::SacrificeAll {
                filter: &Filter::CREATURE,
            }],
            None,
        ),
    ];
    static SELF_SAC_AB: &[AbilityDef] = &[paid(cost!(SacrificeSelf), &[], None)];
    let outlet = card(
        C2,
        face("Altar", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        OUTLET_AB,
    );
    let suit = card(
        C3,
        face("Suit", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        SUIT_AB,
    );
    let bear = card(
        C4,
        creature_face("Suited Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        SELF_SAC_AB,
    );
    let mut engine = bench(
        12,
        vec![bear, outlet, suit],
        [Seat::with(&[C4, C2, C3]), Seat::default()],
    );
    to_main(&mut engine, me());
    let bear_id = the(&engine, ZoneLocation::Battlefield, C4);
    let outlet_id = the(&engine, ZoneLocation::Battlefield, C2);
    let (_, legal) = offer(&engine);
    assert!(
        !legal.abilities.contains(&(outlet_id, 0)),
        "CostPart::Sacrifice not offered when no creature can be sacrificed"
    );
    assert!(
        !legal.abilities.contains(&(bear_id, 0)),
        "CostPart::SacrificeSelf not offered when permanent can't be sacrificed"
    );
    activate(&mut engine, me(), outlet_id, 1);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(bear_id).unwrap().zone,
        Zone::Battlefield,
        "own SacrificeAll cannot sacrifice permanent with opponents_only: false"
    );
}

#[test]
fn resolution_bound_players_cant_gain_life_and_saga_sba_sacrifice_lock() {
    static SKULLCRACK_AB: &[AbilityDef] = &[free(
        &[Effect::continuous(
            &Filter::Any,
            Modifier::PlayersCantGainLife(PlayerRel::TargetPlayerOrController),
            Duration::UntilEndOfTurn,
        )],
        Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )];
    static HEAL_AB: &[AbilityDef] = &[free(&[Effect::gain_life(3)], None)];
    static SAGA_AB: &[AbilityDef] = &[
        chapter!(1, &[Effect::gain_life(1)]),
        static_ability!(
            Filter::This,
            Modifier::CantBeSacrificed {
                opponents_only: false
            }
        ),
    ];
    let skull = card(
        C0,
        face("Nemesis", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        SKULLCRACK_AB,
    );
    let healer = card(
        C1,
        face("Fountain", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        HEAL_AB,
    );
    let saga_face = FaceDef {
        subtypes: &[enchantment::SAGA],
        ..face("Immortal Saga", "{0}", TypeSet::ENCHANTMENT)
    };
    let saga = card(C5, saga_face, KeywordSet::EMPTY, SAGA_AB);
    let mut engine = bench(
        13,
        vec![skull, healer, saga],
        [Seat::with(&[C0]).holding(&[C5]), Seat::with(&[C1])],
    );
    to_main(&mut engine, me());
    let skull_id = the(&engine, ZoneLocation::Battlefield, C0);
    let healer_id = the(&engine, ZoneLocation::Battlefield, C1);
    let saga_hand = the(&engine, ZoneLocation::Hand(me()), C5);

    // 1. Target them() with one-shot PlayersCantGainLife(TargetPlayerOrController); me() can still gain life!
    activate(&mut engine, me(), skull_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("target opponent");
    settle(&mut engine, me());

    // 2. Cast 1-chapter Saga with CantBeSacrificed(false): gains 1 life for me() AND stays on battlefield past CR 715.4 SBA!
    engine
        .apply(me(), PlayerAction::CastSpell { card: saga_hand })
        .unwrap();
    settle(&mut engine, me());
    let saga_bf = the(&engine, ZoneLocation::Battlefield, C5);
    assert_eq!(
        life(&engine, me()),
        21,
        "me() was not targeted by Skullcrack"
    );
    assert_eq!(
        engine
            .state()
            .object(saga_bf)
            .unwrap()
            .counters
            .get(CounterKind::Lore),
        1
    );
    assert_eq!(
        engine.state().object(saga_bf).unwrap().zone,
        Zone::Battlefield,
        "Saga with CantBeSacrificed(false) survives CR 715.4 SBA"
    );

    // 3. Meanwhile them() activates Fountain this turn -> gains 0 life; next turn -> gains 3 life!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(&mut engine, them(), healer_id, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 20);
    to_main(&mut engine, them());
    activate(&mut engine, them(), healer_id, 0);
    settle(&mut engine, them());
    assert_eq!(life(&engine, them()), 23);
}
