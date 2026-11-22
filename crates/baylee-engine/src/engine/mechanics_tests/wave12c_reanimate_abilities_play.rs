//! Beat 18 / Engine Wave 12c deterministic rule tests:
//! 1. `Effect::MoveAndModify` (`ChangeZone.StaticEffect`, `CR 611.2e`, `CR 303.4f`)
//!    across Aura reanimation (*Animate Dead* / *Necromancy* attach + LTB sacrifice +
//!    source-removed-in-response), self-return type replacement (*Enduring Curiosity*),
//!    `X` counters + animation (*Abuelo's Awakening*), and mass `LinkedExile` return
//!    with `Finality` counters (*Ghost Vacuum*).
//! 2. `Modifier::GainActivatedAbilitiesOf` (`CR 613.1f`, `CR 613.8`) across mutual
//!    `Battlefield` cycles (*Experiment Kraj*), transitive chains (*Myr Welder*),
//!    `LoseAllAbilities` donor suppression, `Graveyard(EachPlayer)` (*Necrotic Ooze*),
//!    `LinkedExile` (*Dark Impostor*), `TopOfYourLibrary` (*Conspicuous Snoop*), and
//!    synthesized intrinsic basic-land mana abilities (*Mirran Safehouse*).
//! 3. `Modifier::MayCastWithAltCost` (`CR 118.9`, `CR 601.3e`) across `TopOfLibrary`
//!    land play + `PayLifeEqualToManaValue` (*Bolas's Citadel*), `AnyPermitted` with
//!    `grant_zone_permission: false` + `once_per_turn: true` + `CancelCast` rollback +
//!    external graveyard permission (*As Foretold* + *Muldrotha*), `EachPlayer` flash
//!    (*Aluren*), and `{W}{U}{B}{R}{G}` alt-cost option selection (*Fist of Suns*).
//! 4. `TargetConstraint::SameController` (`CR 115.1`, `CR 115.7`, `CR 608.2b`) across
//!    reachability, single-group auto-narrowing, multi-group `ChoosePlayer` prompt,
//!    mid-stack control change partial illegality, retarget validation, and activated
//!    / triggered abilities.
//! 5. `ThisWay::ExcessDamage` (`CR 120.10`) across normal creatures, exact lethal,
//!    pre-marked damage, indestructible with lethal marked, deathtouch, planeswalkers,
//!    battles, and reflexive triggers.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::CastModeKind;
use crate::event::Cause;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{
    AbilityDonorZone, AltCastPrice, AltCastZones, Amount, CounterKind, Duration, Filter, Layer,
    Modifier, MoveSubject, PlayerRel, ReflexiveWhen, SourceLeaveAction, TargetConstraint,
    TargetReq, TargetSpec, ThisWay, Trigger, TriggerRemember, TriggerZone, ZoneRef, ZoneSel, spell,
    static_ability, triggered,
};
use baylee_core::generated::subtypes::creature;
use baylee_core::mana::ManaColor;

const C0: u32 = 99_910;
const C1: u32 = 99_911;
const C2: u32 = 99_912;
const C3: u32 = 99_913;
const C4: u32 = 99_914;
const C5: u32 = 99_915;
const C6: u32 = 99_916;
const C7: u32 = 99_917;
const C8: u32 = 99_918;

fn refresh_offer(engine: &mut Bench) {
    engine.state.invalidate_projections();
    engine.sync_static_effects();
    engine.state.refresh_characteristics();
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn add_pool(engine: &mut Bench, player: PlayerId, color: ManaColor, amount: u32) {
    engine.state.players[usize::from(player.get())]
        .mana_pool
        .add(color, amount);
    refresh_offer(engine);
}

fn move_to_graveyard(engine: &mut Bench, id: ObjectId, owner: PlayerId) {
    engine
        .state
        .move_object(
            id,
            ZoneLocation::Graveyard(owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("move test object to graveyard");
    refresh_offer(engine);
}

// =========================================================================
// 1. Effect::MoveAndModify
// =========================================================================

#[test]
fn move_and_modify_aura_reanimate_attaches_modifies_sacrifices_on_leave_and_aborts_if_gone() {
    static ANIMATE_MODS: &[Modifier] = &[Modifier::ModifyPT(-1, 0)];
    static ANIMATE_FX: &[Effect] = &[Effect::MoveAndModify {
        subject: MoveSubject::Target(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::EachPlayer,
        )),
        expected_origin: Some(ZoneRef::Graveyard),
        blink: false,
        owner_control: false,
        tapped: false,
        face: None,
        counters: None,
        modifiers: ANIMATE_MODS,
        duration: Duration::WhileSourceOnBattlefield,
        attach_source: true,
        when_source_leaves: Some(SourceLeaveAction::Sacrifice),
    }];
    static ANIMATE_ABILITIES: &[AbilityDef] = &[free(
        ANIMATE_FX,
        Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::EachPlayer,
        ))),
    )];
    static DESTROY_ENCH: &[AbilityDef] = &[free(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::ENCHANTMENT),
            no_regen: false,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::ENCHANTMENT))),
    )];

    let anim_a = card(
        C0,
        face("Test Necromancy A", "{2}{B}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        ANIMATE_ABILITIES,
    );
    let anim_b = card(
        C1,
        face("Test Necromancy B", "{2}{B}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        ANIMATE_ABILITIES,
    );
    let disenchant = card(
        C2,
        face("Disenchant Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        DESTROY_ENCH,
    );
    let wurm = card(
        C3,
        creature_face("Grave Wurm", "{4}{G}", 4, 4),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1201,
        vec![anim_a, anim_b, disenchant, wurm],
        [Seat::with(&[C0, C1, C2]), Seat::with(&[C3])],
    );
    to_main(&mut engine, me());
    let anim_a_id = the(&engine, ZoneLocation::Battlefield, C0);
    let anim_b_id = the(&engine, ZoneLocation::Battlefield, C1);
    let disenchant_id = the(&engine, ZoneLocation::Battlefield, C2);
    let wurm_initial = the(&engine, ZoneLocation::Battlefield, C3);
    move_to_graveyard(&mut engine, wurm_initial, them());
    let wurm_gy = the(&engine, ZoneLocation::Graveyard(them()), C3);

    // 1. Activate anim_a targeting wurm in opponent's graveyard -> enters under
    //    me()'s control as a 3/4 with anim_a attached to it.
    activate(&mut engine, me(), anim_a_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![wurm_gy],
                players: vec![],
            },
        )
        .expect("target wurm in graveyard");
    settle(&mut engine, me());

    let wurm_bf = the(&engine, ZoneLocation::Battlefield, C3);
    let wurm_obj = engine.state().object(wurm_bf).unwrap();
    assert_eq!(wurm_obj.controller, me(), "stolen under caster's control");
    assert_eq!(
        (
            wurm_obj.characteristics().power,
            wurm_obj.characteristics().toughness
        ),
        (Some(3), Some(4)),
        "-1/-0 modifier applies while source remains on battlefield"
    );
    assert_eq!(
        engine.state().object(anim_a_id).unwrap().attached_to,
        Some(wurm_bf),
        "attach_source attaches the Aura to the reanimated creature"
    );

    // 2. Destroy anim_a -> when_source_leaves sacrifices the reanimated Wurm back
    //    to them()'s graveyard.
    activate(&mut engine, me(), disenchant_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![anim_a_id],
                players: vec![],
            },
        )
        .expect("destroy anim_a");
    settle(&mut engine, me());

    assert!(objects(&engine, ZoneLocation::Battlefield, C3).is_empty());
    let wurm_gy2 = the(&engine, ZoneLocation::Graveyard(them()), C3);

    // 3. Activate anim_b targeting wurm_gy2, then destroy anim_b in response before
    //    the reanimation ability resolves -> attach_source aborts the return when
    //    the Aura source is no longer on the battlefield!
    activate(&mut engine, me(), anim_b_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![wurm_gy2],
                players: vec![],
            },
        )
        .expect("target wurm_gy2");
    activate(&mut engine, me(), disenchant_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![anim_b_id],
                players: vec![],
            },
        )
        .expect("destroy anim_b in response");
    settle(&mut engine, me());

    assert!(
        objects(&engine, ZoneLocation::Battlefield, C3).is_empty(),
        "when attach_source Aura leaves in response, card stays in graveyard"
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(them()), C3).len(),
        1
    );
}

#[test]
fn move_and_modify_enduring_self_enchantment_abuelo_x_counters_and_ghost_vacuum_mass() {
    static THIS_CREATURE: Filter = Filter::And(&[Filter::This, Filter::CREATURE]);
    static ENDURING_MODS: &[Modifier] = &[Modifier::BecomeTypes {
        types: TypeSet::ENCHANTMENT,
        subtypes: &[],
    }];
    static ENDURING_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::Dies(&THIS_CREATURE),
        &[Effect::MoveAndModify {
            subject: MoveSubject::Target(TargetSpec::ThisObject),
            expected_origin: Some(ZoneRef::Graveyard),
            blink: false,
            owner_control: true,
            tapped: false,
            face: None,
            counters: None,
            modifiers: ENDURING_MODS,
            duration: Duration::Indefinitely,
            attach_source: false,
            when_source_leaves: None,
        }]
    )];

    static ABUELO_MODS: &[Modifier] = &[
        Modifier::AddType(TypeSet::CREATURE),
        Modifier::AddSubtype(creature::SPIRIT),
        Modifier::SetPT(1, 1),
        Modifier::AddKeyword(KeywordSet::FLYING),
    ];
    static ABUELO_SPELL: &[AbilityDef] = &[spell!(
        &[Effect::MoveAndModify {
            subject: MoveSubject::Target(TargetSpec::CardInGraveyard(
                &Filter::ARTIFACT,
                PlayerRel::You
            )),
            expected_origin: Some(ZoneRef::Graveyard),
            blink: false,
            owner_control: false,
            tapped: false,
            face: None,
            counters: Some((CounterKind::P1P1, Amount::X)),
            modifiers: ABUELO_MODS,
            duration: Duration::Indefinitely,
            attach_source: false,
            when_source_leaves: None,
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::ARTIFACT,
            PlayerRel::You
        )))
    )];

    static VACUUM_MODS: &[Modifier] = &[
        Modifier::SetPT(1, 1),
        Modifier::AddKeyword(KeywordSet::FLYING),
        Modifier::AddSubtype(creature::SPIRIT),
    ];
    static VACUUM_ABILITIES: &[AbilityDef] = &[
        free(
            &[Effect::ExileTargetsWithSource],
            Some(TargetReq::one(TargetSpec::CardInGraveyard(
                &Filter::CREATURE,
                PlayerRel::EachPlayer,
            ))),
        ),
        free(
            &[Effect::MoveAndModify {
                subject: MoveSubject::AllInZone {
                    zone: ZoneSel::LinkedExile,
                    filter: &Filter::CREATURE,
                },
                expected_origin: Some(ZoneRef::Exile),
                blink: false,
                owner_control: false,
                tapped: false,
                face: None,
                counters: Some((CounterKind::Finality, Amount::Fixed(1))),
                modifiers: VACUUM_MODS,
                duration: Duration::Indefinitely,
                attach_source: false,
                when_source_leaves: None,
            }],
            None,
        ),
    ];
    static KILL_PROBE: &[AbilityDef] = &[free(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::Any),
            no_regen: false,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
    )];

    let mut glimmer_face = creature_face("Enduring Cat", "{2}{U}", 4, 3);
    glimmer_face.types = TypeSet::CREATURE.union(TypeSet::ENCHANTMENT);

    let enduring = card(C0, glimmer_face, KeywordSet::EMPTY, ENDURING_ABILITIES);
    let abuelo = card(
        C1,
        face("Abuelo Probe", "{X}{W}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        ABUELO_SPELL,
    );
    let relic = card(
        C2,
        face("Sunken Relic", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let vacuum = card(
        C3,
        face("Ghost Vacuum Probe", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        VACUUM_ABILITIES,
    );
    let kill = card(
        C4,
        face("Destroy Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        KILL_PROBE,
    );
    let ogre = card(
        C5,
        creature_face("Exiled Ogre", "{3}{R}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1202,
        vec![enduring, abuelo, relic, vacuum, kill, ogre],
        [
            Seat::with(&[C0, C2, C3, C4]).holding(&[C1]),
            Seat::with(&[C5]),
        ],
    );
    to_main(&mut engine, me());
    let enduring_id = the(&engine, ZoneLocation::Battlefield, C0);
    let relic_id = the(&engine, ZoneLocation::Battlefield, C2);
    let vacuum_id = the(&engine, ZoneLocation::Battlefield, C3);
    let kill_id = the(&engine, ZoneLocation::Battlefield, C4);
    let ogre_id = the(&engine, ZoneLocation::Battlefield, C5);
    move_to_graveyard(&mut engine, relic_id, me());
    move_to_graveyard(&mut engine, ogre_id, them());

    // A) Enduring Cat dies as a creature -> returns as a noncreature Enchantment!
    activate(&mut engine, me(), kill_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![enduring_id],
                players: vec![],
            },
        )
        .expect("destroy Enduring Cat");
    settle(&mut engine, me());

    let returned_cat = the(&engine, ZoneLocation::Battlefield, C0);
    let cat_chars = engine
        .state()
        .object(returned_cat)
        .unwrap()
        .characteristics();
    assert!(cat_chars.types.contains(TypeSet::ENCHANTMENT));
    assert!(
        !cat_chars.types.contains(TypeSet::CREATURE),
        "BecomeTypes strips CREATURE so it returns solely as an Enchantment"
    );
    // Destroying the returned noncreature Enchantment does NOT re-trigger THIS_CREATURE dies!
    activate(&mut engine, me(), kill_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![returned_cat],
                players: vec![],
            },
        )
        .expect("destroy returned Enchantment");
    settle(&mut engine, me());
    assert!(objects(&engine, ZoneLocation::Battlefield, C0).is_empty());
    assert_eq!(objects(&engine, ZoneLocation::Graveyard(me()), C0).len(), 1);

    // B) Cast Abuelo Probe with X = 3 targeting Sunken Relic in graveyard -> enters
    //    as a 4/4 (1/1 base + 3 +1/+1 counters) flying Spirit artifact creature!
    add_pool(&mut engine, me(), ManaColor::White, 4);
    let abuelo_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let relic_gy = the(&engine, ZoneLocation::Graveyard(me()), C2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: abuelo_id })
        .expect("cast Abuelo Probe");
    engine
        .apply(me(), PlayerAction::ChooseNumber(3))
        .expect("choose X = 3");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![relic_gy],
                players: vec![],
            },
        )
        .expect("target Sunken Relic");
    settle(&mut engine, me());

    let relic_bf = the(&engine, ZoneLocation::Battlefield, C2);
    let relic_obj = engine.state().object(relic_bf).unwrap();
    assert_eq!(relic_obj.counters.get(CounterKind::P1P1), 3);
    assert!(
        relic_obj
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    assert!(
        relic_obj
            .characteristics()
            .types
            .contains(TypeSet::ARTIFACT)
    );
    assert!(
        relic_obj
            .characteristics()
            .subtypes
            .contains(creature::SPIRIT)
    );
    assert!(
        relic_obj
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );
    assert_eq!(
        (
            relic_obj.characteristics().power,
            relic_obj.characteristics().toughness
        ),
        (Some(4), Some(4))
    );

    // C) Ghost Vacuum exiles Enduring Cat (from me()'s GY) and Exiled Ogre (from
    //    them()'s GY) into LinkedExile, then returns all linked creatures as 1/1
    //    flying Spirits with Finality counters under me()'s control!
    let cat_gy = the(&engine, ZoneLocation::Graveyard(me()), C0);
    let ogre_gy = the(&engine, ZoneLocation::Graveyard(them()), C5);
    for target in [cat_gy, ogre_gy] {
        activate(&mut engine, me(), vacuum_id, 0);
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![target],
                    players: vec![],
                },
            )
            .expect("exile creature with Ghost Vacuum");
        settle(&mut engine, me());
    }
    activate(&mut engine, me(), vacuum_id, 1);
    settle(&mut engine, me());

    let cat_spirit = the(&engine, ZoneLocation::Battlefield, C0);
    let ogre_spirit = the(&engine, ZoneLocation::Battlefield, C5);
    for id in [cat_spirit, ogre_spirit] {
        let o = engine.state().object(id).unwrap();
        assert_eq!(o.controller, me());
        assert_eq!(o.counters.get(CounterKind::Finality), 1);
        assert_eq!(
            (o.characteristics().power, o.characteristics().toughness),
            (Some(1), Some(1))
        );
        assert!(o.characteristics().keywords.contains(KeywordSet::FLYING));
        assert!(o.characteristics().subtypes.contains(creature::SPIRIT));
    }

    // Destroying ogre_spirit with a Finality counter exiles it instead of dying!
    activate(&mut engine, me(), kill_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![ogre_spirit],
                players: vec![],
            },
        )
        .expect("destroy ogre_spirit");
    settle(&mut engine, me());
    assert!(objects(&engine, ZoneLocation::Graveyard(them()), C5).is_empty());
    assert_eq!(objects(&engine, ZoneLocation::Exile(them()), C5).len(), 1);
}

// =========================================================================
// 2. Modifier::GainActivatedAbilitiesOf
// =========================================================================

#[test]
fn gain_activated_abilities_mutual_kraj_cycle_transitive_welder_and_ability_loss() {
    static OTHER_P1P1_CREATURE: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::Not(&Filter::This),
        Filter::HasCounter(CounterKind::P1P1),
    ]);
    static KRAJ_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::GainActivatedAbilitiesOf {
            filter: &OTHER_P1P1_CREATURE,
            zone: AbilityDonorZone::Battlefield,
        }
    )];
    static PING_ABILITIES: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost::FREE,
        effects: &[Effect::gain_life(2)],
        targets: None,
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::PerTurn(1),
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    }];
    static HUMBLE_FX: &[AbilityDef] = &[free(
        &[Effect::CreateContinuousEffect {
            layer: Layer::Ability,
            filter: &Filter::This,
            modifier: Modifier::LoseAllAbilities,
            duration: Duration::UntilEndOfTurn,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let kraj_a = card(
        C0,
        creature_face("Kraj Alpha", "{2}{G}{U}", 4, 4),
        KeywordSet::EMPTY,
        KRAJ_ABILITIES,
    );
    let kraj_b = card(
        C1,
        creature_face("Kraj Beta", "{2}{G}{U}", 4, 4),
        KeywordSet::EMPTY,
        KRAJ_ABILITIES,
    );
    // Welder has NO +1/+1 counter of its own, so it is NOT matched by Kraj B,
    // testing 2-hop transitive propagation: Ping Mage -> Kraj A -> Welder!
    let welder = card(
        C2,
        creature_face("Transitive Welder", "{3}", 2, 2),
        KeywordSet::EMPTY,
        KRAJ_ABILITIES,
    );
    let pinger = card(
        C3,
        creature_face("Life Pinger", "{1}{W}", 1, 1),
        KeywordSet::EMPTY,
        PING_ABILITIES,
    );
    let humbler = card(
        C4,
        face("Humbler Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        HUMBLE_FX,
    );

    let mut engine = bench(
        1203,
        vec![kraj_a, kraj_b, welder, pinger, humbler],
        [Seat::with(&[C0, C1, C2, C4]), Seat::with(&[C3])],
    );
    to_main(&mut engine, me());
    let kraj_a_id = the(&engine, ZoneLocation::Battlefield, C0);
    let kraj_b_id = the(&engine, ZoneLocation::Battlefield, C1);
    let welder_id = the(&engine, ZoneLocation::Battlefield, C2);
    let pinger_id = the(&engine, ZoneLocation::Battlefield, C3);
    let humbler_id = the(&engine, ZoneLocation::Battlefield, C4);

    // Put +1/+1 counters on Kraj A, Kraj B (creating a mutual cycle!) and on
    // opponent's Life Pinger.
    for id in [kraj_a_id, kraj_b_id, pinger_id] {
        engine
            .state
            .object_mut(id)
            .unwrap()
            .counters
            .add(CounterKind::P1P1, 1);
    }
    refresh_offer(&mut engine);

    // Kraj A, Kraj B, and Welder each gain Life Pinger's once-per-turn ability
    // cleanly without infinite recursion, and each gets its own once-per-turn tally!
    let start_life = life(&engine, me());
    for host in [kraj_a_id, kraj_b_id, welder_id] {
        let (_, legal) = offer(&engine);
        let &(_, granted_idx) = legal
            .abilities
            .iter()
            .find(|(src, _)| *src == host)
            .expect("host gained Life Pinger's activated ability");
        activate(&mut engine, me(), host, granted_idx);
        settle(&mut engine, me());
        let (_, after) = offer(&engine);
        assert!(
            !after.abilities.iter().any(|(src, _)| *src == host),
            "host's once-per-turn granted ability is spent for this host"
        );
    }
    assert_eq!(life(&engine, me()), start_life + 6);

    // Stripping all abilities from Life Pinger (CR 613.1f) removes the donated
    // ability from recipients immediately!
    activate(&mut engine, me(), humbler_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![pinger_id],
                players: vec![],
            },
        )
        .expect("strip Life Pinger's abilities");
    settle(&mut engine, me());
    assert_eq!(
        crate::effects::granted_activated(engine.state(), kraj_a_id).count(),
        0,
        "donor with LoseAllAbilities donates zero activated abilities"
    );
}

#[test]
fn gain_activated_abilities_graveyard_linked_exile_library_top_and_basic_land_mana() {
    static OOZE_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::GainActivatedAbilitiesOf {
            filter: &Filter::CREATURE,
            zone: AbilityDonorZone::Graveyard(PlayerRel::EachPlayer),
        }
    )];
    static IMPOSTOR_ABILITIES: &[AbilityDef] = &[
        free(
            &[Effect::ExileTargetsWithSource],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(
            Filter::This,
            Modifier::GainActivatedAbilitiesOf {
                filter: &Filter::CREATURE,
                zone: AbilityDonorZone::LinkedExile,
            }
        ),
    ];
    static GOBLIN_FILTER: Filter = Filter::HasSubtype(creature::GOBLIN);
    static SNOOP_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::GainActivatedAbilitiesOf {
            filter: &GOBLIN_FILTER,
            zone: AbilityDonorZone::TopOfYourLibrary,
        }
    )];
    static SAFEHOUSE_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::GainActivatedAbilitiesOf {
            filter: &Filter::LAND,
            zone: AbilityDonorZone::Graveyard(PlayerRel::You),
        }
    )];
    static DONOR_DRAW: &[AbilityDef] = &[free(&[Effect::draw(1)], None)];
    static DONOR_BOLT: &[AbilityDef] = &[free(&[Effect::gain_life(3)], None)];

    let ooze = card(
        C0,
        creature_face("Ooze Probe", "{2}{B}{B}", 4, 3),
        KeywordSet::EMPTY,
        OOZE_ABILITIES,
    );
    let impostor = card(
        C1,
        creature_face("Impostor Probe", "{4}{B}{B}", 2, 2),
        KeywordSet::EMPTY,
        IMPOSTOR_ABILITIES,
    );
    let mut snoop_face = creature_face("Snoop Probe", "{R}{R}", 2, 2);
    snoop_face.subtypes = Box::leak(Box::new([creature::GOBLIN]));
    let snoop = card(C2, snoop_face, KeywordSet::EMPTY, SNOOP_ABILITIES);
    let safehouse = card(
        C3,
        face("Safehouse Probe", "{3}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        SAFEHOUSE_ABILITIES,
    );
    let mut goblin_donor_face = creature_face("Topdeck Goblin", "{R}", 1, 1);
    goblin_donor_face.subtypes = Box::leak(Box::new([creature::GOBLIN]));
    let goblin_donor = card(C4, goblin_donor_face, KeywordSet::EMPTY, DONOR_DRAW);
    let enemy_donor = card(
        C5,
        creature_face("Enemy Donor", "{1}{B}", 2, 2),
        KeywordSet::EMPTY,
        DONOR_BOLT,
    );

    let mut engine = bench(
        1204,
        vec![ooze, impostor, snoop, safehouse, goblin_donor, enemy_donor],
        [
            Seat::with(&[C0, C1, C2, C3]).holding(&[C4]),
            Seat::with(&[C5]),
        ],
    );
    to_main(&mut engine, me());
    let ooze_id = the(&engine, ZoneLocation::Battlefield, C0);
    let impostor_id = the(&engine, ZoneLocation::Battlefield, C1);
    let snoop_id = the(&engine, ZoneLocation::Battlefield, C2);
    let safehouse_id = the(&engine, ZoneLocation::Battlefield, C3);
    let goblin_hand = the(&engine, ZoneLocation::Hand(me()), C4);
    let enemy_bf = the(&engine, ZoneLocation::Battlefield, C5);

    // 1. Put Topdeck Goblin onto top of me()'s library -> Snoop gains its draw ability!
    //    Drawing that Goblin moves a Forest to top of library -> Snoop immediately loses it!
    engine
        .state
        .move_object(
            goblin_hand,
            ZoneLocation::Library(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    refresh_offer(&mut engine);
    let (_, legal) = offer(&engine);
    let &(_, snoop_idx) = legal
        .abilities
        .iter()
        .find(|(src, _)| *src == snoop_id)
        .expect("Snoop gained Topdeck Goblin's ability");
    activate(&mut engine, me(), snoop_id, snoop_idx);
    settle(&mut engine, me());
    let (_, after_snoop) = offer(&engine);
    assert!(
        !after_snoop
            .abilities
            .iter()
            .any(|(src, _)| *src == snoop_id),
        "Snoop loses the ability once Goblin is no longer on top of library"
    );

    // 2. Move Topdeck Goblin to me()'s graveyard -> Ooze gains its draw ability!
    let goblin_drawn = the(&engine, ZoneLocation::Hand(me()), C4);
    move_to_graveyard(&mut engine, goblin_drawn, me());
    let (_, legal_ooze) = offer(&engine);
    assert!(
        legal_ooze.abilities.iter().any(|(src, _)| *src == ooze_id),
        "Necrotic Ooze gains activated ability from graveyard"
    );

    // 3. Impostor exiles Enemy Donor into LinkedExile -> Impostor gains +3 life ability
    //    while Ooze does NOT see cards in exile!
    activate(&mut engine, me(), impostor_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![enemy_bf],
                players: vec![],
            },
        )
        .expect("exile Enemy Donor with Impostor");
    settle(&mut engine, me());
    let before_life = life(&engine, me());
    let (_, legal_imp) = offer(&engine);
    let &(_, imp_granted_idx) = legal_imp
        .abilities
        .iter()
        .find(|(src, idx)| *src == impostor_id && *idx != 0)
        .expect("Impostor gained linked exile creature's activated ability");
    activate(&mut engine, me(), impostor_id, imp_granted_idx);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), before_life + 3);

    // 4. Move a basic Forest from me()'s library into me()'s graveyard -> non-land
    //    Safehouse synthesizes and activates tap-for-green mana ability!
    let top_forest = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .unwrap();
    move_to_graveyard(&mut engine, top_forest, me());
    let (_, legal_safe) = offer(&engine);
    let &(_, safe_idx) = legal_safe
        .abilities
        .iter()
        .find(|(src, _)| *src == safehouse_id)
        .expect("Mirran Safehouse synthesizes green mana ability from basic Forest in graveyard");
    let green_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Green);
    activate(&mut engine, me(), safehouse_id, safe_idx);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        green_before + 1
    );
}

// =========================================================================
// 3. Modifier::MayCastWithAltCost
// =========================================================================

#[test]
fn may_cast_with_alt_cost_bolas_citadel_top_land_and_pay_life_spell() {
    static CITADEL_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::MayCastWithAltCost {
            who: PlayerRel::You,
            zones: AltCastZones::TopOfLibrary,
            grant_zone_permission: true,
            allow_lands: true,
            with_flash: false,
            max_mana_value: None,
            once_per_turn: false,
            price: AltCastPrice::PayLifeEqualToManaValue,
        }
    )];

    let citadel = card(
        C0,
        face("Citadel Probe", "{3}{B}{B}{B}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        CITADEL_ABILITIES,
    );
    let wurm = card(
        C1,
        creature_face("Citadel Wurm", "{3}{G}{G}", 5, 5),
        KeywordSet::EMPTY,
        &[],
    );
    let wurm2 = card(
        C2,
        creature_face("Too Expensive Wurm", "{3}{G}{G}", 5, 5),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1205,
        vec![citadel, wurm, wurm2],
        [Seat::with(&[C0]).holding(&[C1, C2]).at(6), Seat::default()],
    );
    to_main(&mut engine, me());

    // 1. Top of library is a Forest -> PlayLand plays it from top of library!
    let top_forest = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .unwrap();
    let (_, legal) = offer(&engine);
    assert!(
        legal.lands.contains(&top_forest),
        "Citadel with allow_lands: true lets controller play land from top of library"
    );
    engine
        .apply(me(), PlayerAction::PlayLand { card: top_forest })
        .expect("play top-of-library Forest");
    let next_forest = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .unwrap();
    let (_, after_land) = offer(&engine);
    assert!(
        !after_land.lands.contains(&next_forest),
        "normal land-play limit is still respected"
    );

    // 2. Put MV=5 wurm2 then MV=5 wurm on top of library while me() is at 6 life
    //    with ZERO mana -> wurm casts by paying 5 life (dropping me() to 1 life)!
    let wurm_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let wurm2_id = the(&engine, ZoneLocation::Hand(me()), C2);
    for id in [wurm2_id, wurm_id] {
        engine
            .state
            .move_object(
                id,
                ZoneLocation::Library(me()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    refresh_offer(&mut engine);

    let (_, legal_cast) = offer(&engine);
    assert!(
        legal_cast.castable.contains(&wurm_id),
        "MV=5 spell on top of library is castable at 6 life with zero mana"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: wurm_id })
        .expect("cast wurm via Citadel");
    settle(&mut engine, me());

    assert_eq!(life(&engine, me()), 1, "paid 5 life equal to mana value");
    let _ = the(&engine, ZoneLocation::Battlefield, C1);

    // 3. Now wurm2 (MV=5) is on top of library, and me() is at 1 life -> NOT castable!
    let (_, legal_low_life) = offer(&engine);
    assert!(
        !legal_low_life.castable.contains(&wurm2_id),
        "cannot pay 5 life when at 1 life"
    );
}

#[test]
fn may_cast_with_alt_cost_as_foretold_aluren_flash_and_fist_of_suns() {
    static AS_FORETOLD_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::MayCastWithAltCost {
            who: PlayerRel::You,
            zones: AltCastZones::AnyPermitted,
            grant_zone_permission: false,
            allow_lands: false,
            with_flash: false,
            max_mana_value: Some(Amount::CountersOnSource(CounterKind::Time)),
            once_per_turn: true,
            price: AltCastPrice::Mana(ManaCost::ZERO),
        }
    )];
    static GRAVE_PERM_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::CastPermanentSpellsFromGraveyard
    )];
    static ALUREN_ZONE: &[ZoneRef] = &[ZoneRef::Hand];
    static ALUREN_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::CREATURE,
        Modifier::MayCastWithAltCost {
            who: PlayerRel::EachPlayer,
            zones: AltCastZones::Zones(ALUREN_ZONE),
            grant_zone_permission: false,
            allow_lands: false,
            with_flash: true,
            max_mana_value: Some(Amount::Fixed(3)),
            once_per_turn: false,
            price: AltCastPrice::Mana(ManaCost::ZERO),
        }
    )];
    static FIST_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::MayCastWithAltCost {
            who: PlayerRel::You,
            zones: AltCastZones::AnyPermitted,
            grant_zone_permission: false,
            allow_lands: false,
            with_flash: false,
            max_mana_value: None,
            once_per_turn: false,
            price: AltCastPrice::Mana(ManaCost::parse("{W}{U}{B}{R}{G}")),
        }
    )];
    static TARGETED_BEAR_SPELL: &[AbilityDef] = &[spell!(
        &[Effect::TapTarget],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any)))
    )];

    let as_foretold = card(
        C0,
        face("As Foretold Probe", "{2}{U}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        AS_FORETOLD_ABILITIES,
    );
    let muldrotha = card(
        C1,
        creature_face("Grave Conduit", "{3}{B}{G}{U}", 6, 6),
        KeywordSet::EMPTY,
        GRAVE_PERM_ABILITIES,
    );
    let mv2_tap = card(
        C2,
        face("MV2 Tap", "{1}{U}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        TARGETED_BEAR_SPELL,
    );
    let mv2_bear = card(
        C3,
        creature_face("MV2 Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let mv4_giant = card(
        C4,
        creature_face("MV4 Giant", "{3}{R}", 4, 4),
        KeywordSet::EMPTY,
        &[],
    );
    let aluren = card(
        C5,
        face("Aluren Probe", "{2}{G}{G}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        ALUREN_ABILITIES,
    );
    let fist = card(
        C6,
        face("Fist Probe", "{3}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        FIST_ABILITIES,
    );
    let titan = card(
        C7,
        creature_face("Ten Drop Titan", "{8}{G}{G}", 10, 10),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1206,
        vec![
            as_foretold,
            muldrotha,
            mv2_tap,
            mv2_bear,
            mv4_giant,
            aluren,
            fist,
            titan,
        ],
        [
            Seat::with(&[C0]).holding(&[C1, C2, C3, C4, C5, C6, C7]),
            Seat::default().holding(&[C3, C4]),
        ],
    );
    to_main(&mut engine, me());
    let af_id = the(&engine, ZoneLocation::Battlefield, C0);
    let muldrotha_hand = the(&engine, ZoneLocation::Hand(me()), C1);
    let mv2_tap_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let mv2_bear_me = the(&engine, ZoneLocation::Hand(me()), C3);
    let mv4_giant_me = the(&engine, ZoneLocation::Hand(me()), C4);

    // Put 2 Time counters on As Foretold, and put MV2 Bear into me()'s graveyard.
    engine
        .state
        .object_mut(af_id)
        .unwrap()
        .counters
        .add(CounterKind::Time, 2);
    move_to_graveyard(&mut engine, mv2_bear_me, me());

    // 1. With 2 Time counters and zero mana:
    //    - mv2_tap in hand IS castable;
    //    - mv4_giant in hand is NOT castable (MV 4 > 2);
    //    - mv2_bear in graveyard is NOT castable (grant_zone_permission: false!).
    let mv2_bear_gy = the(&engine, ZoneLocation::Graveyard(me()), C3);
    let (_, legal1) = offer(&engine);
    assert!(legal1.castable.contains(&mv2_tap_id));
    assert!(!legal1.castable.contains(&mv4_giant_me));
    assert!(
        !legal1.castable.contains(&mv2_bear_gy),
        "grant_zone_permission: false must not unlock graveyard without another permission"
    );

    // 2. Start casting mv2_tap via As Foretold, then CancelCast -> once-per-turn
    //    is NOT spent!
    engine
        .apply(me(), PlayerAction::CastSpell { card: mv2_tap_id })
        .expect("begin casting mv2_tap");
    assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
    engine
        .apply(me(), PlayerAction::CancelCast)
        .expect("cancel cast rolls back As Foretold once-per-turn");

    // 3. Put Grave Conduit onto battlefield -> now mv2_bear in graveyard IS castable
    //    for zero mana via As Foretold! Cast it, then verify mv2_tap can no longer
    //    use As Foretold this turn!
    engine
        .state
        .move_object(
            muldrotha_hand,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    refresh_offer(&mut engine);
    let (_, legal_grave) = offer(&engine);
    assert!(
        legal_grave.castable.contains(&mv2_bear_gy),
        "As Foretold combines with external graveyard permission to cast for zero mana"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: mv2_bear_gy })
        .expect("cast mv2_bear from graveyard for zero mana");
    settle(&mut engine, me());
    let _ = the(&engine, ZoneLocation::Battlefield, C3);
    let (_, legal_after_once) = offer(&engine);
    assert!(
        !legal_after_once.castable.contains(&mv2_tap_id),
        "As Foretold once_per_turn: true is spent after one spell cast"
    );

    // 4. Put Aluren onto battlefield -> while a spell is on the stack on me()'s turn,
    //    them() holds priority and CAN flash-cast their MV=2 Bear for zero mana, but
    //    NOT their MV=4 Giant!
    let aluren_hand = the(&engine, ZoneLocation::Hand(me()), C5);
    engine
        .state
        .move_object(
            aluren_hand,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: mv2_tap_id })
        .expect("cast mv2_tap paying {1}{U}");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![af_id],
                players: vec![],
            },
        )
        .expect("target af_id");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass priority to them() with stack non-empty");

    let (holder, them_legal) = offer(&engine);
    assert_eq!(holder, them());
    let them_bear = the(&engine, ZoneLocation::Hand(them()), C3);
    let them_giant = the(&engine, ZoneLocation::Hand(them()), C4);
    assert!(
        them_legal.castable.contains(&them_bear),
        "Aluren grants EachPlayer flash + zero mana for MV <= 3 creatures"
    );
    assert!(!them_legal.castable.contains(&them_giant));
    engine
        .apply(them(), PlayerAction::CastSpell { card: them_bear })
        .expect("them() flash-casts Bear via Aluren");
    settle(&mut engine, me());
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, C3).len(), 2);

    // 5. Put Fist of Suns onto battlefield and give me() enough mana for BOTH normal
    //    {8}{G}{G} and Fist's {W}{U}{B}{R}{G} -> ChooseMode exposes
    //    CastModeKind::Permission(Some(fist_id)) and selecting it pays only 5 mana!
    let fist_hand = the(&engine, ZoneLocation::Hand(me()), C6);
    let titan_hand = the(&engine, ZoneLocation::Hand(me()), C7);
    engine
        .state
        .move_object(
            fist_hand,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let fist_id = the(&engine, ZoneLocation::Battlefield, C6);
    for c in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
    ] {
        add_pool(&mut engine, me(), c, 1);
    }
    add_pool(&mut engine, me(), ManaColor::Green, 7);

    engine
        .apply(me(), PlayerAction::CastSpell { card: titan_hand })
        .expect("begin casting Titan");
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseCastMode between Normal and Fist of Suns, got {:?}",
            engine.pending()
        );
    };
    let fist_opt = options
        .iter()
        .find(|o| o.kind == CastModeKind::Permission(Some(fist_id)))
        .expect("Fist of Suns option present")
        .index;
    engine
        .apply(me(), PlayerAction::ChooseMode(fist_opt as usize))
        .expect("select Fist of Suns WUBRG alt cost");
    settle(&mut engine, me());
    let _ = the(&engine, ZoneLocation::Battlefield, C7);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        6,
        "spent 1 green for WUBRG instead of 10 mana"
    );
}

// =========================================================================
// 4. TargetConstraint::SameController
// =========================================================================

#[test]
fn same_controller_reachability_group_prompt_midstack_control_and_retarget() {
    const SAME_TWO_CREATURES: TargetReq =
        TargetReq::exactly(TargetSpec::Object(&Filter::CREATURE), 2)
            .with_constraint(TargetConstraint::SameController);

    static BIOSHIFT_SPELL: &[AbilityDef] = &[spell!(
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        targets = Some(SAME_TWO_CREATURES)
    )];
    static STEAL_CREATURE: &[AbilityDef] = &[free(
        &[Effect::ChangeController {
            new_controller: PlayerRel::You,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static RETARGET_SPELL: &[AbilityDef] = &[free(
        &[Effect::ChooseNewTargets],
        Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )];
    static SAME_TRIG: &[AbilityDef] = &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        effects: &[Effect::TapTarget],
        targets: Some(SAME_TWO_CREATURES),
        second_targets: None,
        condition: None,
        zone: TriggerZone::Battlefield,
        once_per_turn: false,
        resolves_if_targets_illegal: false,
    }];

    let bioshift = card(
        C0,
        face("Bioshift Probe", "{G}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BIOSHIFT_SPELL,
    );
    let bear_a1 = card(
        C1,
        creature_face("Bear A1", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_a2 = card(
        C2,
        creature_face("Bear A2", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_b1 = card(
        C3,
        creature_face("Bear B1", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_b2 = card(
        C4,
        creature_face("Bear B2", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let stealer = card(
        C5,
        face("Stealer Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        STEAL_CREATURE,
    );
    let retargeter = card(
        C6,
        face("Retarget Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        RETARGET_SPELL,
    );
    let etb_probe = card(
        C7,
        creature_face("SameCtrl ETB", "{1}{W}", 1, 1),
        KeywordSet::EMPTY,
        SAME_TRIG,
    );

    let mut engine = bench(
        1207,
        vec![
            bioshift, bear_a1, bear_a2, bear_b1, bear_b2, stealer, retargeter, etb_probe,
        ],
        [
            Seat::with(&[C1, C6]).holding(&[C0, C2, C7]),
            Seat::with(&[C3, C5]).holding(&[C4]),
        ],
    );
    to_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Green, 4);
    add_pool(&mut engine, me(), ManaColor::White, 4);

    let bioshift_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let a1_id = the(&engine, ZoneLocation::Battlefield, C1);
    let a2_hand = the(&engine, ZoneLocation::Hand(me()), C2);
    let b1_id = the(&engine, ZoneLocation::Battlefield, C3);
    let b2_hand = the(&engine, ZoneLocation::Hand(them()), C4);
    let stealer_id = the(&engine, ZoneLocation::Battlefield, C5);
    let retarget_id = the(&engine, ZoneLocation::Battlefield, C6);
    let etb_hand = the(&engine, ZoneLocation::Hand(me()), C7);

    // 1. Only 1 creature controlled by each player -> SameController min=2 is NOT
    //    reachable!
    let (_, legal1) = offer(&engine);
    assert!(
        !legal1.castable.contains(&bioshift_id),
        "1 creature on each side must not satisfy SameController min=2"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: etb_hand })
        .expect("cast SameCtrl ETB creature");
    // When SameCtrl ETB enters, me() now has 2 creatures (Bear A1 + SameCtrl ETB),
    // while them() has 1 -> single eligible group auto-narrows ChooseTargets to me()'s 2!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let etb_bf = the(&engine, ZoneLocation::Battlefield, C7);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 2);
    assert!(options.contains(&a1_id) && options.contains(&etb_bf));
    assert!(!options.contains(&b1_id));
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![a1_id, etb_bf],
                players: vec![],
            },
        )
        .expect("answer auto-narrowed SameController ETB trigger");
    settle(&mut engine, me());

    // 2. Put Bear A2 and Bear B2 onto battlefield so BOTH seats control >= 2 creatures ->
    //    casting Bioshift prompts Pending::ChoosePlayer { options: [me(), them()] }!
    for (id, ctrl) in [(a2_hand, me()), (b2_hand, them())] {
        engine.state.object_mut(id).unwrap().controller = ctrl;
        engine.state.object_mut(id).unwrap().base_controller = ctrl;
        engine
            .state
            .move_object(
                id,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    refresh_offer(&mut engine);
    let a2_id = the(&engine, ZoneLocation::Battlefield, C2);
    let b2_id = the(&engine, ZoneLocation::Battlefield, C4);

    engine
        .apply(me(), PlayerAction::CastSpell { card: bioshift_id })
        .expect("begin casting Bioshift");
    let Pending::ChoosePlayer {
        options: group_opts,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChoosePlayer group disambiguation, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(group_opts, vec![me(), them()]);
    engine
        .apply(me(), PlayerAction::ChoosePlayer(me()))
        .expect("choose me() as controller group");

    let Pending::ChooseTargets {
        options: narrowed_opts,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected narrowed ChooseTargets, got {:?}",
            engine.pending()
        );
    };
    assert!(narrowed_opts.contains(&a1_id) && narrowed_opts.contains(&a2_id));
    assert!(
        !narrowed_opts.contains(&b1_id) && !narrowed_opts.contains(&b2_id),
        "opposing creatures are excluded after choosing me()'s group"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![a1_id, a2_id],
                players: vec![],
            },
        )
        .expect("target [a1_id, a2_id]");

    // 3. CR 115.7d retarget check: activate Retarget Probe targeting Bioshift on the
    //    stack -> keeping slot 0 as a1_id (controlled by me()) restricts slot 1 strictly
    //    to me()'s other creatures (etb_bf) and rejects cross-controller b1_id!
    let bioshift_stack = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();
    activate(&mut engine, me(), retarget_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bioshift_stack],
                players: vec![],
            },
        )
        .expect("target Bioshift on stack");
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    // Keep slot 0 unchanged (a1_id, controlled by me()).
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .expect("keep slot 0 unchanged");
    let Pending::ChooseTargets {
        options: slot1_opts,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected slot 1 retarget ChooseTargets, got {:?}",
            engine.pending()
        );
    };
    assert!(slot1_opts.contains(&etb_bf));
    assert!(
        !slot1_opts.contains(&b1_id) && !slot1_opts.contains(&b2_id),
        "CR 115.7d must exclude cross-controller creatures when slot 0 stays on me()"
    );
    assert!(
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![b1_id],
                    players: vec![],
                },
            )
            .is_err(),
        "CR 115.7d must reject cross-controller retarget [a1_id, b1_id]"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .expect("keep slot 1 unchanged as a2_id");
    // Pass priority after retarget finishes, leaving Bioshift on the stack for them()!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();

    // 4. CR 608.2b mid-stack control change: them() steals a2_id before Bioshift
    //    resolves -> a1_id stays legal and gets +1/+1, while a2_id (now controlled
    //    by them() != controller(a1_id)) is illegal and gets NO counter!
    let (holder, _) = offer(&engine);
    assert_eq!(holder, them());
    activate(&mut engine, them(), stealer_id, 0);
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![a2_id],
                players: vec![],
            },
        )
        .expect("them() steals a2_id in response");
    settle(&mut engine, me());

    assert_eq!(
        engine
            .state()
            .object(a1_id)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "first target remains legal"
    );
    assert_eq!(
        engine
            .state()
            .object(a2_id)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0,
        "CR 608.2b: stolen second target violates SameController at resolution"
    );
}

// =========================================================================
// 5. ThisWay::ExcessDamage (CR 120.10)
// =========================================================================

#[test]
fn excess_damage_cr_120_10_normal_marked_indestructible_deathtouch_pw_battle_and_reflexive() {
    static EXCESS_BURN_FX: &[Effect] = &[
        Effect::DealDamage {
            amount: Amount::X,
            target: TargetSpec::Object(&Filter::Any),
        },
        Effect::GainLife {
            amount: Amount::ThisWay(ThisWay::ExcessDamage),
        },
        Effect::ReflexiveTrigger {
            when: ReflexiveWhen::ThisWay(ThisWay::ExcessDamage),
            once: true,
            condition: None,
            targets: None,
            remember: TriggerRemember::None,
            effects: &[Effect::LoseLife {
                amount: Amount::EventAmount,
                target: PlayerRel::Opponent,
            }],
        },
    ];
    static EXCESS_PROBE_ABILITIES: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::parse("{X}"),
            parts: &[],
        },
        effects: EXCESS_BURN_FX,
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    }];

    let probe = card(
        C0,
        face("Excess Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        EXCESS_PROBE_ABILITIES,
    );
    let dt_probe = card(
        C1,
        face("Deathtouch Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::DEATHTOUCH,
        EXCESS_PROBE_ABILITIES,
    );
    let hill_giant = card(
        C2,
        creature_face("Three Three", "{2}{R}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );
    let exact_giant = card(
        C3,
        creature_face("Exact Three", "{2}{R}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );
    let marked_giant = card(
        C4,
        creature_face("Marked Three", "{2}{R}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );
    let indest_giant = card(
        C5,
        creature_face("Indestructible Three", "{2}{W}", 3, 3),
        KeywordSet::INDESTRUCTIBLE,
        &[],
    );
    let colossus = card(
        C6,
        creature_face("Ten Ten", "{8}", 10, 10),
        KeywordSet::EMPTY,
        &[],
    );
    let mut pw_face = face("Three Loyalty PW", "{2}{U}", TypeSet::PLANESWALKER);
    pw_face.loyalty = Some(3);
    let walker = card(C7, pw_face, KeywordSet::EMPTY, &[]);
    let mut battle_face = face("Three Defense Battle", "{2}{G}", TypeSet::BATTLE);
    battle_face.defense = Some(3);
    let battle = card(C8, battle_face, KeywordSet::EMPTY, &[]);

    let mut engine = bench(
        1208,
        vec![
            probe,
            dt_probe,
            hill_giant,
            exact_giant,
            marked_giant,
            indest_giant,
            colossus,
            walker,
            battle,
        ],
        [
            Seat::with(&[C0, C1]).at(20),
            Seat::with(&[C2, C3, C4, C5, C6, C7, C8]).at(40),
        ],
    );
    // Answer the opening battle protector prompt if needed, then walk to main.
    if let Pending::ChoosePlayer { player, options } = engine.pending().clone() {
        engine
            .apply(player, PlayerAction::ChoosePlayer(options[0]))
            .unwrap();
    }
    to_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Red, 50);

    let probe_id = the(&engine, ZoneLocation::Battlefield, C0);
    let dt_probe_id = the(&engine, ZoneLocation::Battlefield, C1);
    let giant_id = the(&engine, ZoneLocation::Battlefield, C2);
    let exact_id = the(&engine, ZoneLocation::Battlefield, C3);
    let marked_id = the(&engine, ZoneLocation::Battlefield, C4);
    let indest_id = the(&engine, ZoneLocation::Battlefield, C5);
    let colossus_id = the(&engine, ZoneLocation::Battlefield, C6);
    let walker_id = the(&engine, ZoneLocation::Battlefield, C7);
    let battle_id = the(&engine, ZoneLocation::Battlefield, C8);

    // Pre-mark 2 damage on marked_giant and 3 (already lethal) damage on indest_giant!
    engine.state.object_mut(marked_id).unwrap().damage = 2;
    engine.state.object_mut(indest_id).unwrap().damage = 3;

    let mut fire = |source: ObjectId, x: u32, target: ObjectId| -> (i32, i32) {
        let me_before = life(&engine, me());
        let them_before = life(&engine, them());
        activate(&mut engine, me(), source, 0);
        engine
            .apply(me(), PlayerAction::ChooseNumber(x))
            .expect("choose X");
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![target],
                    players: vec![],
                },
            )
            .expect("choose target");
        settle(&mut engine, me());
        (
            life(&engine, me()) - me_before,
            them_before - life(&engine, them()),
        )
    };

    // 1. 5 damage to fresh 3/3 -> 2 excess (GainLife 2, reflexive LoseLife 2).
    assert_eq!(fire(probe_id, 5, giant_id), (2, 2));
    // 2. 3 damage to fresh 3/3 -> 0 excess.
    assert_eq!(fire(probe_id, 3, exact_id), (0, 0));
    // 3. 5 damage to 3/3 with 2 damage already marked -> 4 excess!
    assert_eq!(fire(probe_id, 5, marked_id), (4, 4));
    // 4. 5 damage to indestructible 3/3 with 3 damage already marked -> all 5 is excess!
    assert_eq!(fire(probe_id, 5, indest_id), (5, 5));
    // 5. 5 deathtouch damage to fresh 10/10 -> 1 is lethal, 4 is excess!
    assert_eq!(fire(dt_probe_id, 5, colossus_id), (4, 4));
    // 6. 5 damage to 3-loyalty planeswalker -> 2 excess!
    assert_eq!(fire(probe_id, 5, walker_id), (2, 2));
    // 7. 6 damage to 3-defense battle -> 3 excess!
    assert_eq!(fire(probe_id, 6, battle_id), (3, 3));
}
