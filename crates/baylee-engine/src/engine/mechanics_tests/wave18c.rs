//! Wave 18c deterministic gameplay tests:
//! 1. `Trigger::TurnedFaceUpMatching(&Filter::CREATURE)`: fires on any matching permanent
//!    turning face up (mine or opponent's) and ignores non-matching permanents.
//! 2. `Effect::ChooseBasicLandType` + `Effect::BecomeChosenBasicLandType` (`CR 305.7`):
//!    targeted land changes basic land type, loses old land subtypes/abilities, gains
//!    the chosen basic mana ability until end of turn.
//! 3. `Effect::AnyPlayerMayPayManaAmount` (dynamic APNAP mana payment with `on_payment`
//!    `true` and `false`) and `Effect::AnyPlayerMayPayCost` (`DamageToYou`,
//!    `Sacrifice { count: 2 }`, `ExileFromGraveyard { count: 3 }`).
//! 4. `Trigger::TargetedByOpponent` on `GameEvent::AbilityActivated` +
//!    `Effect::CounterEventSpellOrAbility`, `Effect::CounterTargetSpellTo`,
//!    `Effect::PutSourceOnBottomOfLibrary`, `Effect::CounterAll` + `ThisWay::Countered`,
//!    and `Effect::DelayedManaAnyColorAtNextMain` (`NextFirstMain`).
//! 5. `Duration::UntilEndOfYourNextTurn`, `PlayerRel::PlayerToYourRight`,
//!    `Effect::GainControlLtbAttached`, and `Remembered` capture via `ControllerChanged`.
//! 6. `Effect::DestroyTargetAndSameName`, `Effect::DestroyAllAttachedToTarget`,
//!    `sweep_filter` `Filter::CmcAtMostX` via `res.x`, and `Effect::BecomesBlocked`.

#![allow(clippy::too_many_lines, clippy::items_after_statements)]

use super::*;
use crate::event::Cause;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::effect::{AnyPlayerCost, CounterDestination, CounterScope};
use baylee_cards_dsl::prelude::triggered;
use baylee_cards_dsl::{
    ActivationLimit, ActivationTiming, ActivationZone, Amount, Cost, CostPart, Duration, Filter,
    KeywordSet, Modifier, PlayCostMode, PlayCount, PlaySource, PlayerRel, TargetReq, TargetSpec,
    ThisWay, Trigger, ZoneRef,
};
use baylee_core::generated::subtypes::{artifact, enchantment, land};
use baylee_core::mana;
use baylee_core::mana::ManaColor;
use baylee_core::preset::{Finish, FormatId, PrintInfo};
use baylee_core::types::TypeSet;

const C0: u32 = 99_880;
const C1: u32 = 99_881;
const C2: u32 = 99_882;
const C3: u32 = 99_883;
const C4: u32 = 99_884;
const C5: u32 = 99_885;
const C6: u32 = 99_886;
const C7: u32 = 99_887;
const C8: u32 = 99_888;
const C9: u32 = 99_889;

static EQUIP_SUBTYPE: Filter = Filter::HasSubtype(artifact::EQUIPMENT);
static AURA_SUBTYPE: Filter = Filter::HasSubtype(enchantment::AURA);
static ARTIFACT_CMCX: Filter = Filter::And(&[Filter::ARTIFACT, Filter::CmcAtMostX]);

const fn spell_def(effects: &'static [Effect], targets: Option<TargetReq>) -> AbilityDef {
    AbilityDef::Spell {
        effects,
        targets,
        second_targets: None,
        third_targets: None,
        condition: None,
    }
}

fn choose_one_target(engine: &mut Bench, target: ObjectId) {
    let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("choose target");
}

#[test]
fn wave18c_turn_face_up_choose_basic_land_and_becomes_blocked() {
    // 1. Trigger::TurnedFaceUpMatching(&Filter::CREATURE): watches any face-down
    //    permanent becoming face up as a creature.
    static FACEUP_WATCHER_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::TurnedFaceUpMatching(&Filter::CREATURE),
        &[Effect::draw(1)]
    )];
    let watcher = card(
        C0,
        creature_face("Aphetto Proxy", "{1}{U}", 2, 3),
        KeywordSet::EMPTY,
        FACEUP_WATCHER_ABILITIES,
    );

    let bear = card(
        C1,
        creature_face("Faceup Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let relic = card(
        C2,
        face("Faceup Relic", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    static TURN_UP_OWN: &[AbilityDef] = &[free(
        &[Effect::TurnFaceUp {
            target: TargetSpec::Object(&Filter::FaceDown),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::FaceDown))),
    )];
    let turner = card(
        C3,
        face("Turn Up Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        TURN_UP_OWN,
    );

    // 2. ChooseBasicLandType + BecomeChosenBasicLandType (Dream Thrush / Shimmering Mirage).
    static CHOOSE_LAND_ABILITIES: &[AbilityDef] = &[free(
        &[
            Effect::ChooseBasicLandType,
            Effect::BecomeChosenBasicLandType {
                target: TargetSpec::Object(&Filter::LAND),
                duration: Duration::UntilEndOfTurn,
            },
        ],
        Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
    )];
    let land_changer = card(
        C4,
        creature_face("Dream Thrush Proxy", "{1}{U}", 1, 1),
        KeywordSet::EMPTY,
        CHOOSE_LAND_ABILITIES,
    );
    let swamp = card(
        C5,
        FaceDef {
            subtypes: &[land::SWAMP],
            ..land_face("Swamp")
        },
        KeywordSet::EMPTY,
        &[],
    );

    // 3. BecomesBlocked (Fog Patch): makes an attacking creature blocked so it deals 0 damage to defender.
    static FOG_PATCH_ABILITIES: &[AbilityDef] = &[spell_def(
        &[Effect::BecomesBlocked(TargetSpec::Object(
            &Filter::ATTACKING_CREATURE,
        ))],
        None,
    )];
    let fog_patch = card(
        C6,
        face("Fog Patch Proxy", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        FOG_PATCH_ABILITIES,
    );
    let attacker = card(
        C7,
        creature_face("Unblocked Raider", "{1}{R}", 4, 4),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1801,
        vec![
            watcher,
            bear,
            relic,
            turner,
            land_changer,
            swamp,
            fog_patch,
            attacker,
        ],
        [
            Seat::with(&[C0, C3, C4, C5, C7]).holding(&[C6]),
            Seat::with(&[]).holding(&[C1, C2]),
        ],
    );

    let bear_id = the(&engine, ZoneLocation::Hand(them()), C1);
    let relic_id = the(&engine, ZoneLocation::Hand(them()), C2);
    for id in [bear_id, relic_id] {
        engine
            .state
            .move_object(
                id,
                ZoneLocation::Library(them()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    assert!(engine.state.manifest_card(them(), relic_id));
    assert!(engine.state.manifest_card(them(), bear_id));
    engine.state.refresh_characteristics();

    to_main(&mut engine, me());
    let hand_before = engine.state.zones.list(ZoneLocation::Hand(me())).len();
    let turner_bf = the(&engine, ZoneLocation::Battlefield, C3);

    // Turn face up the artifact first -> NOT a creature -> watcher must NOT trigger!
    activate(&mut engine, me(), turner_bf, 0);
    choose_one_target(&mut engine, relic_id);
    settle(&mut engine, me());
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Hand(me())).len(),
        hand_before,
        "TurnedFaceUpMatching(&Filter::CREATURE) must ignore non-creature permanents"
    );

    // Turn face up the creature -> watcher DOES trigger and draws 1!
    activate(&mut engine, me(), turner_bf, 0);
    choose_one_target(&mut engine, bear_id);
    settle(&mut engine, me());
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Hand(me())).len(),
        hand_before + 1,
        "TurnedFaceUpMatching(&Filter::CREATURE) must fire when an opponent's creature turns face up"
    );

    // ChooseBasicLandType + BecomeChosenBasicLandType on our Swamp -> Island until end of turn.
    let changer_bf = the(&engine, ZoneLocation::Battlefield, C4);
    let swamp_bf = the(&engine, ZoneLocation::Battlefield, C5);
    activate(&mut engine, me(), changer_bf, 0);
    choose_one_target(&mut engine, swamp_bf);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        panic!("expected basic land type prompt");
    };
    assert_eq!(
        options.len(),
        5,
        "ChooseBasicLandType offers all 5 basic land types"
    );
    assert!(options.contains(&land::ISLAND));
    let before_land_choice = engine.snapshot_hash();
    assert_eq!(
        engine.state.snapshot_hash(),
        engine.state.clone().snapshot_hash()
    );
    engine
        .apply(player, PlayerAction::ChooseSubtype(land::ISLAND))
        .unwrap();
    settle(&mut engine, me());
    assert_ne!(engine.snapshot_hash(), before_land_choice);

    let swamp_obj = engine.state.object(swamp_bf).unwrap();
    assert!(
        swamp_obj.characteristics().subtypes.contains(land::ISLAND)
            && !swamp_obj.characteristics().subtypes.contains(land::SWAMP),
        "CR 305.7: Swamp became an Island and lost Swamp subtype"
    );

    // BecomesBlocked: attack with C7, pass declare blockers with no blockers, cast Fog Patch Proxy.
    let attacker_bf = the(&engine, ZoneLocation::Battlefield, C7);
    let fog_hand = the(&engine, ZoneLocation::Hand(me()), C6);
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker_bf, baylee_core::ids::Defender::Player(them()))],
            },
        )
        .unwrap();
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == them()),
    );
    engine
        .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == me()),
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: fog_hand })
        .unwrap();
    let opp_life_before = life(&engine, them());
    to_main(&mut engine, them());
    assert_eq!(
        life(&engine, them()),
        opp_life_before,
        "CR 509.1h / Fog Patch: creature made blocked deals no combat damage to defending player"
    );
}

#[test]
fn wave18c_any_player_unless_costs() {
    static DRAW_IF_UNPAID: &[Effect] = &[Effect::AnyPlayerMayPayManaAmount {
        mana: Amount::Fixed(2),
        on_payment: false,
        effects: &[Effect::draw(1)],
    }];
    static LOSE_IF_PAID: &[Effect] = &[Effect::AnyPlayerMayPayManaAmount {
        mana: Amount::Fixed(1),
        on_payment: true,
        effects: &[Effect::LoseLife {
            amount: Amount::Fixed(3),
            target: PlayerRel::You,
        }],
    }];
    static BROWBEAT: &[Effect] = &[Effect::AnyPlayerMayPayCost {
        cost: AnyPlayerCost::DamageToYou(5),
        on_payment: false,
        effects: &[Effect::draw(2)],
    }];
    static PANGOLIN: &[Effect] = &[Effect::AnyPlayerMayPayCost {
        cost: AnyPlayerCost::Sacrifice {
            filter: &Filter::CREATURE,
            count: 2,
        },
        on_payment: true,
        effects: &[Effect::SacrificeSelf],
    }];
    static CARRION: &[Effect] = &[Effect::AnyPlayerMayPayCost {
        cost: AnyPlayerCost::ExileFromGraveyard {
            filter: &Filter::Any,
            count: 3,
        },
        on_payment: true,
        effects: &[Effect::draw(1)],
    }];

    static ART_ABILITIES: &[AbilityDef] = &[
        free(DRAW_IF_UNPAID, None),
        free(LOSE_IF_PAID, None),
        free(BROWBEAT, None),
        free(PANGOLIN, None),
        free(CARRION, None),
    ];
    let hub = card(
        C0,
        face("Unless Hub", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ART_ABILITIES,
    );
    let bear = card(
        C1,
        creature_face("Sac Fodder", "{0}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1802,
        vec![hub, bear],
        [
            Seat::with(&[C0]).at(20),
            Seat::with(&[C1, C1]).holding(&[C1, C1, C1]).at(20),
        ],
    );
    for gy_card in objects(&engine, ZoneLocation::Hand(them()), C1) {
        engine
            .state
            .move_object(
                gy_card,
                ZoneLocation::Graveyard(them()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }

    to_main(&mut engine, me());
    let hub_bf = the(&engine, ZoneLocation::Battlefield, C0);

    // 1. AnyPlayerMayPayManaAmount (on_payment: false): both decline -> draws 1!
    let h0 = engine.state.zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), hub_bf, 0);
    let mut prompt_hashes = Vec::new();
    for p in [me(), them()] {
        walk_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p),
        );
        let h = engine.snapshot_hash();
        assert_eq!(h, engine.snapshot_hash());
        assert_eq!(
            engine.state.snapshot_hash(),
            engine.state.clone().snapshot_hash()
        );
        prompt_hashes.push(h);
        engine.apply(p, PlayerAction::YesNo(false)).unwrap();
    }
    assert_ne!(prompt_hashes[0], prompt_hashes[1]);
    settle(&mut engine, me());
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Hand(me())).len(),
        h0 + 1
    );

    // 2. AnyPlayerMayPayManaAmount (on_payment: true): me() floats {1} and pays -> loses 3 life!
    engine.state.players[usize::from(me().get())]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    activate(&mut engine, me(), hub_bf, 1);
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == me()),
    );
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 17);

    // 3. AnyPlayerCost::DamageToYou(5): me declines, them accepts -> them takes 5 damage and me draws 0!
    let h1 = engine.state.zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), hub_bf, 2);
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == me()),
    );
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == them()),
    );
    engine.apply(them(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 15);
    assert_eq!(engine.state.zones.list(ZoneLocation::Hand(me())).len(), h1);

    // 4. AnyPlayerCost::ExileFromGraveyard(3): me has 0 GY cards (skipped automatically), them exiles 3 -> draws 1!
    let h2 = engine.state.zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), hub_bf, 4);
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == them()),
    );
    let pay_cost_yesno_hash = engine.snapshot_hash();
    assert_eq!(pay_cost_yesno_hash, engine.snapshot_hash());
    engine.apply(them(), PlayerAction::YesNo(true)).unwrap();
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == them()),
    );
    let pay_cost_choose_hash = engine.snapshot_hash();
    assert_ne!(pay_cost_yesno_hash, pay_cost_choose_hash);
    assert_eq!(pay_cost_choose_hash, engine.snapshot_hash());
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected graveyard exile choice");
    };
    assert_eq!(options.len(), 3);
    engine
        .apply(player, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Exile(them())).len(),
        3
    );
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Hand(me())).len(),
        h2 + 1
    );

    // 5. AnyPlayerCost::Sacrifice(2 creatures): me has 0 creatures (skipped), them sacrifices 2 -> hub is sacrificed!
    activate(&mut engine, me(), hub_bf, 3);
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == them()),
    );
    let pay_cost_yesno_hash = engine.snapshot_hash();
    assert_eq!(pay_cost_yesno_hash, engine.snapshot_hash());
    engine.apply(them(), PlayerAction::YesNo(true)).unwrap();
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == them()),
    );
    let pay_cost_choose_hash = engine.snapshot_hash();
    assert_ne!(pay_cost_yesno_hash, pay_cost_choose_hash);
    assert_eq!(pay_cost_choose_hash, engine.snapshot_hash());
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected creature sacrifice choice");
    };
    assert_eq!(options.len(), 2);
    engine
        .apply(player, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    settle(&mut engine, me());
    assert!(objects(&engine, ZoneLocation::Battlefield, C0).is_empty());
    assert!(objects(&engine, ZoneLocation::Battlefield, C1).is_empty());
}

#[test]
fn wave18c_counter_control_and_destroy_mechanics() {
    static TITAN_WARD: &[Effect] = &[Effect::PlayerMayPayOr {
        player: PlayerRel::EventPlayer,
        mana: Amount::Fixed(2),
        effect: &Effect::CounterEventSpellOrAbility,
    }];
    static TITAN_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::TargetedByOpponent {
            filter: &Filter::This,
            you: false,
        },
        TITAN_WARD
    )];
    let titan = card(
        C0,
        creature_face("Frost Titan Proxy", "{4}{U}{U}", 6, 6),
        KeywordSet::EMPTY,
        TITAN_ABILITIES,
    );

    static KILL_ABILITY: &[AbilityDef] = &[free(
        &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    let pinger = card(
        C1,
        face("Targeted Ability Source", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        KILL_ABILITY,
    );

    static SWIFT_SILENCE_AB: &[AbilityDef] = &[spell_def(
        &[
            Effect::CounterAll {
                scope: CounterScope::Spells,
                filter: &Filter::Not(&Filter::This),
            },
            Effect::DrawCards {
                amount: Amount::ThisWay(ThisWay::Countered),
            },
        ],
        None,
    )];
    let swift_silence = card(
        C2,
        face("Swift Silence Proxy", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        SWIFT_SILENCE_AB,
    );

    static CRUMPLE_AB: &[AbilityDef] = &[spell_def(
        &[
            Effect::CounterTargetSpellTo(CounterDestination::BottomOfLibrary),
            Effect::DelayedManaAnyColorAtNextMain,
            Effect::PutSourceOnBottomOfLibrary,
        ],
        Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )];
    let crumple = card(
        C3,
        face("Crumple + Plasm Proxy", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        CRUMPLE_AB,
    );

    static SPELL_3_EFX: &[Effect] = &[Effect::gain_life(1)];
    static SPELL_3_AB: &[AbilityDef] = &[spell_def(SPELL_3_EFX, None)];
    let spell_3 = card(
        C4,
        face("Three Mana Instant", "{3}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        SPELL_3_AB,
    );

    static PURSER_ABS: &[AbilityDef] = &[
        free(
            &[Effect::AddTempModifier {
                target: TargetSpec::Object(&Filter::CREATURE),
                modifier: Modifier::GainControl,
                duration: Duration::UntilEndOfYourNextTurn,
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        free(
            &[Effect::GainControlEx {
                target: TargetSpec::ThisObject,
                to: PlayerRel::PlayerToYourRight,
                duration: Duration::Indefinitely,
                untap: false,
                haste_until_eot: false,
            }],
            None,
        ),
    ];
    let purser = card(
        C5,
        face("Purse + Thief Proxy", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PURSER_ABS,
    );

    static SPOILS_AB: &[AbilityDef] = &[free(
        &[
            Effect::Destroy {
                target: TargetSpec::Object(&Filter::CREATURE),
                no_regen: true,
            },
            Effect::GainControlLtbAttached {
                filter: &EQUIP_SUBTYPE,
                duration: Duration::Indefinitely,
            },
        ],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    let spoils = card(
        C6,
        face("Murderous Spoils Proxy", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        SPOILS_AB,
    );

    static SWORD_ABILITIES: &[AbilityDef] = &[free(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    let sword = card(
        C7,
        FaceDef {
            subtypes: &[artifact::EQUIPMENT],
            ..face("Test Equipment", "{0}", TypeSet::ARTIFACT)
        },
        KeywordSet::EMPTY,
        SWORD_ABILITIES,
    );

    static DEST_HUB_ABS: &[AbilityDef] = &[
        free(
            &[Effect::DestroyTargetAndSameName {
                filter: &Filter::ENCHANTMENT,
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::ENCHANTMENT))),
        ),
        free(
            &[Effect::DestroyAllAttachedToTarget {
                filter: &AURA_SUBTYPE,
                destroy_target: false,
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        AbilityDef::Activated {
            cost: Cost {
                mana: mana!("{X}"),
                parts: &[CostPart::SacrificeSelf],
            },
            effects: &[Effect::DestroyAll {
                filter: &ARTIFACT_CMCX,
                no_regen: false,
            }],
            targets: None,
            second_targets: None,
            third_targets: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
            limit: ActivationLimit::Unlimited,
            x_value: baylee_cards_dsl::ActivationX::Any,
            cost_reduction: None,
        },
    ];
    let dest_hub = card(
        C8,
        FaceDef {
            mana_cost: mana!("{2}"),
            ..face("Destroy Hub", "{2}", TypeSet::ARTIFACT)
        },
        KeywordSet::EMPTY,
        DEST_HUB_ABS,
    );
    let ench = card(
        C9,
        FaceDef {
            mana_cost: mana!("{5}"),
            types: TypeSet::ENCHANTMENT,
            ..land_face("Duplicate Ench")
        },
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1803,
        vec![
            titan,
            pinger,
            swift_silence,
            crumple,
            spell_3,
            purser,
            spoils,
            sword,
            dest_hub,
            ench,
        ],
        [
            Seat::with(&[C0, C5, C6, C8, C9, C5]).holding(&[C2, C3, C4]),
            Seat::with(&[C0, C0, C1, C7, C9]).holding(&[C4]),
        ],
    );

    to_main(&mut engine, me());
    let my_titan = objects(&engine, ZoneLocation::Battlefield, C0)
        .into_iter()
        .find(|&id| engine.state.object(id).unwrap().controller == me())
        .unwrap();
    let opp_titans: Vec<ObjectId> = objects(&engine, ZoneLocation::Battlefield, C0)
        .into_iter()
        .filter(|&id| engine.state.object(id).unwrap().controller == them())
        .collect();
    let titan_bf = opp_titans[0];
    let steal_target = opp_titans[1];
    let pinger_bf = the(&engine, ZoneLocation::Battlefield, C1);

    // A) DestroyTargetAndSameName on C9.
    let ench_mine = objects(&engine, ZoneLocation::Battlefield, C9)[0];
    let hub_bf = the(&engine, ZoneLocation::Battlefield, C8);
    activate(&mut engine, me(), hub_bf, 0);
    choose_one_target(&mut engine, ench_mine);
    settle(&mut engine, me());
    assert!(
        objects(&engine, ZoneLocation::Battlefield, C9).is_empty(),
        "DestroyTargetAndSameName destroyed both C9 enchantments"
    );

    // B) CounterTargetSpellTo(BottomOfLibrary) + PutSourceOnBottomOfLibrary + DelayedManaAnyColorAtNextMain.
    let opp_spell_hand = objects(&engine, ZoneLocation::Hand(them()), C4)[0];
    let my_crumple_hand = the(&engine, ZoneLocation::Hand(me()), C3);
    to_main(&mut engine, them());
    engine.state.players[usize::from(them().get())]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    let sword_bf = the(&engine, ZoneLocation::Battlefield, C7);
    activate(&mut engine, them(), sword_bf, 0);
    choose_one_target(&mut engine, titan_bf);
    settle(&mut engine, them());
    assert_eq!(
        engine.state.object(sword_bf).unwrap().attached_to,
        Some(titan_bf)
    );

    engine
        .apply(
            them(),
            PlayerAction::CastSpell {
                card: opp_spell_hand,
            },
        )
        .unwrap();
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: my_crumple_hand,
            },
        )
        .unwrap();
    choose_one_target(&mut engine, opp_spell_hand);
    settle(&mut engine, them());
    assert_eq!(
        engine
            .state
            .zones
            .list(ZoneLocation::Library(them()))
            .first(),
        Some(&opp_spell_hand),
        "CounterTargetSpellTo(BottomOfLibrary) tucked countered spell on bottom"
    );
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Library(me())).first(),
        Some(&my_crumple_hand),
        "PutSourceOnBottomOfLibrary tucked Crumple on bottom"
    );

    for _ in 0..200 {
        if holds_priority_in(&engine, me(), crate::turn::Step::Main)
            && engine.state().turn.phase == Phase::FirstMain
        {
            break;
        }
        let pending = engine.pending().clone();
        if let Pending::ChooseColor { player, .. } = pending {
            let color_h = engine.snapshot_hash();
            assert_eq!(color_h, engine.snapshot_hash());
            engine
                .apply(player, PlayerAction::ChooseColor(ManaColor::Blue))
                .unwrap();
            assert_ne!(engine.snapshot_hash(), color_h);
        } else {
            assert!(
                super::synthetic::walk_past(&mut engine, &pending),
                "unexpected question: {pending:?}"
            );
        }
    }
    settle(&mut engine, me());
    assert_eq!(
        engine.state.players[usize::from(me().get())]
            .mana_pool
            .available(ManaColor::Blue),
        3,
        "Plasm Capture delayed mana trigger added 3 blue mana at me()'s FirstMain"
    );

    // C) GainControlLtbAttached: destroy opp's Titan -> gain control of attached Equipment!
    engine.state.players[usize::from(me().get())]
        .mana_pool
        .add(ManaColor::Colorless, 2);
    let spoils_bf = the(&engine, ZoneLocation::Battlefield, C6);
    activate(&mut engine, me(), spoils_bf, 0);
    choose_one_target(&mut engine, titan_bf);
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == me()),
    );
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());
    assert_ne!(
        engine.state.object(titan_bf).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state.object(sword_bf).unwrap().controller,
        me(),
        "GainControlLtbAttached gained control of Equipment that was attached to destroyed creature"
    );

    // D) PlayerRel::PlayerToYourRight: pass one Purse (C5) to them().
    let my_pursers = objects(&engine, ZoneLocation::Battlefield, C5);
    let purser_bf = my_pursers[0];
    let my_purser = my_pursers[1];
    activate(&mut engine, me(), purser_bf, 1);
    settle(&mut engine, me());
    assert_eq!(
        engine.state.object(purser_bf).unwrap().controller,
        them(),
        "PlayerToYourRight in 2-player gave control to opponent"
    );

    // E) TargetedByOpponent on AbilityActivated: them() activates Pinger targeting my Titan, declines {2} -> countered!
    to_main(&mut engine, them());
    activate(&mut engine, them(), pinger_bf, 0);
    choose_one_target(&mut engine, my_titan);
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == them()),
    );
    engine.apply(them(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, them());
    assert_eq!(
        engine.state.object(my_titan).unwrap().zone,
        Zone::Battlefield,
        "TargetedByOpponent fired on AbilityActivated and CounterEventSpellOrAbility countered the destroy ability"
    );

    // F) Duration::UntilEndOfYourNextTurn
    to_main(&mut engine, me());
    engine.state.players[usize::from(me().get())]
        .mana_pool
        .add(ManaColor::Colorless, 2);
    activate(&mut engine, me(), my_purser, 0);
    choose_one_target(&mut engine, steal_target);
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == me()),
    );
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state.object(steal_target).unwrap().controller, me());
    let steal_active_h = engine.snapshot_hash();
    assert_eq!(steal_active_h, engine.snapshot_hash());
    to_main(&mut engine, them());
    assert_eq!(engine.state.object(steal_target).unwrap().controller, me());
    to_main(&mut engine, me());
    assert_eq!(engine.state.object(steal_target).unwrap().controller, me());
    to_main(&mut engine, them());
    assert_ne!(engine.snapshot_hash(), steal_active_h);
    assert_eq!(
        engine.state.object(steal_target).unwrap().controller,
        them(),
        "UntilEndOfYourNextTurn expires at cleanup of controller's next turn"
    );

    // G) CounterAll + ThisWay::Countered (Swift Silence) and AB$ DestroyAll CmcAtMostX with X=1.
    let my_spell_hand = objects(&engine, ZoneLocation::Hand(me()), C4)[0];
    let silence_hand = the(&engine, ZoneLocation::Hand(me()), C2);
    to_main(&mut engine, me());
    let h_before = engine.state.zones.list(ZoneLocation::Hand(me())).len() - 2;
    engine.state.players[usize::from(me().get())]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    engine.refresh_offer();
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: my_spell_hand,
            },
        )
        .unwrap();
    engine
        .apply(me(), PlayerAction::CastSpell { card: silence_hand })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Hand(me())).len(),
        h_before + 1,
        "Swift Silence countered 1 other spell and drew 1 card via ThisWay::Countered"
    );

    // DestroyAll CmcAtMostX with X=1 (hub_bf sacrificed as cost; my_purser MV 0 is destroyed!).
    activate(&mut engine, me(), hub_bf, 2);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    let Pending::ChooseNumber { player, .. } = engine.pending().clone() else {
        panic!("expected ChooseNumber for X");
    };
    engine.apply(player, PlayerAction::ChooseNumber(1)).unwrap();
    settle(&mut engine, me());
    assert_ne!(
        engine.state.object(my_purser).unwrap().zone,
        Zone::Battlefield,
        "CmcAtMostX with X=1 destroyed MV 0 artifact"
    );
}

#[test]
fn wave18c_target_power_cmc_commander_and_apnap() {
    static FELL_FILTER: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::PowerAtLeastAmount(&Amount::Plus {
            base: &Amount::TargetPower,
            offset: 1,
        }),
    ]);
    static FELL_AB: &[AbilityDef] = &[spell_def(
        &[Effect::destroy_all(&FELL_FILTER)],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    let fell = card(
        C0,
        face("Fell the Mighty Proxy", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        FELL_AB,
    );

    static COUNTERPOINT_FILTER: Filter = Filter::CmcAtMostAmount(&Amount::TargetCmc);
    static COUNTERPOINT_AB: &[AbilityDef] = &[spell_def(
        &[
            Effect::CounterTargetSpell,
            Effect::Play {
                source: PlaySource::Zone {
                    zone: ZoneRef::Hand,
                    owner: PlayerRel::You,
                },
                filter: &COUNTERPOINT_FILTER,
                controller: PlayerRel::You,
                count: PlayCount::UpTo(1),
                cost: PlayCostMode::WithoutPayingManaCost,
                cast_only: false,
                copy: false,
                exile_instead_of_graveyard: false,
                optional: true,
            },
        ],
        Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )];
    let counterpoint = card(
        C1,
        face("Counterpoint Proxy", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        COUNTERPOINT_AB,
    );

    static CRUMPLE_AB: &[AbilityDef] = &[spell_def(
        &[Effect::CounterTargetSpellTo(
            CounterDestination::BottomOfLibrary,
        )],
        Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )];
    let crumple = card(
        C2,
        face("Hinder Proxy", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        CRUMPLE_AB,
    );

    static APNAP_MANA_AB: &[Effect] = &[Effect::AnyPlayerMayPayManaAmount {
        mana: Amount::Fixed(1),
        on_payment: true,
        effects: &[Effect::LoseLife {
            amount: Amount::Fixed(2),
            target: PlayerRel::You,
        }],
    }];
    static APNAP_LIFE_AB: &[Effect] = &[Effect::AnyPlayerMayPayCost {
        cost: AnyPlayerCost::DamageToYou(3),
        on_payment: true,
        effects: &[Effect::gain_life(1)],
    }];
    static APNAP_ABS: &[AbilityDef] = &[free(APNAP_MANA_AB, None), free(APNAP_LIFE_AB, None)];
    let apnap_hub = card(
        C3,
        face("APNAP Hub", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        APNAP_ABS,
    );

    let bear_1 = card(
        C4,
        creature_face("Small Bear", "{1}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_2 = card(
        C5,
        creature_face("Mid Bear", "{2}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_3 = card(
        C6,
        creature_face("Big Bear", "{3}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_4 = card(
        C7,
        creature_face("Huge Bear", "{4}", 4, 4),
        KeywordSet::EMPTY,
        &[],
    );
    let opp_cmdr: &'static baylee_cards_dsl::CardDef =
        Box::leak(Box::new(baylee_cards_dsl::CardDef {
            index: CardIndex::new(C8),
            oracle_id: "test",
            scryfall_id: "test",
            faces: Box::leak(Box::new([creature_face("Opponent Commander", "{0}", 2, 2)])),
            color_identity: baylee_core::color::ColorSet::EMPTY,
            keywords: KeywordSet::EMPTY,
            commander: baylee_cards_dsl::CommanderRule::ExplicitlyAllowed,
            partner: baylee_cards_dsl::PartnerKind::None,
            fuse: false,
            coverage: baylee_cards_dsl::Coverage::Implemented,
            abilities: &[],
        }));

    // Part 1: Fell the Mighty (Amount::TargetPower), Counterpoint (Amount::TargetCmc), and APNAP with them() active.
    let mut engine = bench(
        1804,
        vec![
            fell,
            counterpoint,
            apnap_hub,
            bear_1,
            bear_2,
            bear_3,
            bear_4,
        ],
        [
            Seat::with(&[C3, C4, C5, C6]).holding(&[C0, C1, C5, C7]),
            Seat::with(&[]).holding(&[C6]),
        ],
    );
    to_main(&mut engine, me());

    // 1A) Fell the Mighty targeting 2/2: 1/1 and 2/2 survive, 3/3 dies!
    let fell_hand = the(&engine, ZoneLocation::Hand(me()), C0);
    let b1 = the(&engine, ZoneLocation::Battlefield, C4);
    let b2 = the(&engine, ZoneLocation::Battlefield, C5);
    let b3 = the(&engine, ZoneLocation::Battlefield, C6);
    engine
        .apply(me(), PlayerAction::CastSpell { card: fell_hand })
        .unwrap();
    choose_one_target(&mut engine, b2);
    settle(&mut engine, me());
    assert_eq!(engine.state.object(b1).unwrap().zone, Zone::Battlefield);
    assert_eq!(engine.state.object(b2).unwrap().zone, Zone::Battlefield);
    assert_ne!(
        engine.state.object(b3).unwrap().zone,
        Zone::Battlefield,
        "Fell the Mighty targeting power 2 destroyed 3/3 via TargetPower"
    );

    // 1B) APNAP order with them() active on both AnyPlayerMayPayManaAmount and AnyPlayerMayPayCost!
    to_main(&mut engine, them());
    let hub_bf = the(&engine, ZoneLocation::Battlefield, C3);
    // Let them() pass priority once, then me() activates hub ability 0 (AnyPlayerMayPayManaAmount).
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.state.players[usize::from(them().get())]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    engine.state.players[usize::from(me().get())]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    activate(&mut engine, me(), hub_bf, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(
        matches!(engine.pending(), Pending::YesNo { player, .. } if *player == them()),
        "CR 101.4 APNAP: active player them() must be asked first for AnyPlayerMayPayManaAmount, got {:?}",
        engine.pending()
    );
    let apnap_h = engine.snapshot_hash();
    assert_eq!(apnap_h, engine.snapshot_hash());
    engine.apply(them(), PlayerAction::YesNo(false)).unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(
        matches!(engine.pending(), Pending::YesNo { player, .. } if *player == me()),
        "CR 101.4 APNAP: me() is asked second after them() declines"
    );
    assert_ne!(engine.snapshot_hash(), apnap_h);
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, them());

    // AnyPlayerMayPayCost with them() active also prompts them() first.
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    activate(&mut engine, me(), hub_bf, 1);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(
        matches!(engine.pending(), Pending::YesNo { player, .. } if *player == them()),
        "CR 101.4 APNAP: active player them() must be asked first for AnyPlayerMayPayCost"
    );
    engine.apply(them(), PlayerAction::YesNo(false)).unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, them());

    // 1C) Counterpoint countering opponent's MV 3 spell lets me() cast MV 2 card (C5), excluding MV 4 (C7)!
    let opp_mv3 = the(&engine, ZoneLocation::Hand(them()), C6);
    let my_cp = the(&engine, ZoneLocation::Hand(me()), C1);
    let my_mv2 = the(&engine, ZoneLocation::Hand(me()), C5);
    let my_mv4 = the(&engine, ZoneLocation::Hand(me()), C7);
    engine.state.players[usize::from(them().get())]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    engine.refresh_offer();
    engine
        .apply(them(), PlayerAction::CastSpell { card: opp_mv3 })
        .unwrap();
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == me()),
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: my_cp })
        .unwrap();
    choose_one_target(&mut engine, opp_mv3);
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == me()),
    );
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected Counterpoint free-cast ChooseCards prompt");
    };
    assert!(
        options.contains(&my_mv2) && !options.contains(&my_mv4),
        "Counterpoint MV<=3 allows MV-2 card and excludes MV-4 card: {options:?}"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![my_mv2],
            },
        )
        .unwrap();
    settle(&mut engine, them());
    assert_eq!(
        engine.state.object(my_mv2).unwrap().zone,
        Zone::Battlefield,
        "Counterpoint cast MV-2 permanent for free"
    );

    // Part 2: CR 903.9b Commander replacement in CounterTargetSpellTo(BottomOfLibrary).
    let make_entry = |id: u32| DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: CardIndex::new(id),
        print: baylee_core::ids::PrintRef::new(0),
    };
    let deck: Vec<DeckEntry> = (0..99).map(|_| make_entry(bear_1.index.get())).collect();
    let preset = GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format: FormatId::Commander,
        seed: 1805,
        house_rules: HouseRules {
            starting_player: Some(1),
            ..HouseRules::default()
        },
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![
            SeatSpec {
                controller: SeatController::Ai(baylee_core::preset::AIProfile::default()),
                capabilities: baylee_core::preset::SeatCapabilities {
                    dev_commands: true,
                    see_hidden: false,
                },
                deck: deck.clone(),
                sideboard: vec![],
                commanders: vec![],
                starting_life: None,
                starting_hand: Some(vec![make_entry(crumple.index.get())]),
                starting_battlefield: vec![],
                emblems: vec![],
                team: None,
            },
            SeatSpec {
                controller: SeatController::Ai(baylee_core::preset::AIProfile::default()),
                capabilities: baylee_core::preset::SeatCapabilities {
                    dev_commands: true,
                    see_hidden: false,
                },
                deck,
                sideboard: vec![],
                commanders: vec![make_entry(opp_cmdr.index.get())],
                starting_life: None,
                starting_hand: Some(vec![]),
                starting_battlefield: vec![],
                emblems: vec![],
                team: None,
            },
        ],
    };
    let mut cmd_engine = Engine::new(
        &preset,
        SyntheticLookup::new(vec![crumple, bear_1, opp_cmdr]),
    )
    .expect("commander bench starts");
    while let Pending::Mulligan { player, .. } = cmd_engine.pending().clone() {
        cmd_engine
            .apply(player, PlayerAction::MulliganKeep)
            .unwrap();
    }
    to_main(&mut cmd_engine, them());
    let cmdr_obj = the(&cmd_engine, ZoneLocation::Command(them()), C8);
    let my_hinder = the(&cmd_engine, ZoneLocation::Hand(me()), C2);
    cmd_engine
        .apply(them(), PlayerAction::CastSpell { card: cmdr_obj })
        .unwrap();
    walk_until(
        &mut cmd_engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == me()),
    );
    cmd_engine
        .apply(me(), PlayerAction::CastSpell { card: my_hinder })
        .unwrap();
    choose_one_target(&mut cmd_engine, cmdr_obj);
    walk_until(&mut cmd_engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::CommanderReplace { to_library: true, .. },
                ..
            } if *player == them()
        )
    });
    let cmd_hash = cmd_engine.snapshot_hash();
    assert_eq!(cmd_hash, cmd_engine.snapshot_hash());
    cmd_engine.apply(them(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut cmd_engine, them());
    assert_eq!(
        cmd_engine.state.object(cmdr_obj).unwrap().zone,
        Zone::Command,
        "CR 903.9b redirected countered commander from BottomOfLibrary back to Command zone"
    );
}
