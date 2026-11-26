//! Domain tests for Animate and Attach mechanics:
//! - Licid Aura transformation (`Effect::BecomeAura`, `Modifier::GrantEnchant`,
//!   `Modifier::LoseAbility`, `GrantedActionKind::EndContinuousEffects` — CR 116.2a,
//!   CR 303.4, CR 702.61b, CR 704.5m/p, CR 707.2)
//! - Reconfigure + Animate timestamp interactions (CR 613.7m, CR 702.151b, CR 704.5p)
//! - Stack-to-battlefield animation (`CR 400.7b`, `Modifier::GrantBloodthirst`,
//!   `Modifier::GrantSunburst`, `Modifier::SetName`, `Effect::ModifyEvent` + `GrantStatic`)
//! - Dynamic P/T animation (`Effect::SetDynamicPT`, `Amount::SourceToughness`,
//!   `Amount::DiscardedManaValue`, signed offsets, targeted `ThisObject`)
//! - Generalized attachment operations (`Effect::Unattach`, `Effect::AttachObject`,
//!   `Effect::MoveAllAttachments`, `Effect::AttachLtbAttachments`,
//!   `Effect::AttachChosenThen`, `Effect::RelocateAttachment`)
//! - Snapshot hash, loop signature, and `AwaitingOp::search_fingerprint` coverage

#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::{GrantedActionKind, GrantedActionOffer, YesNoPrompt};
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::granted::EndableEffectGroup;
use crate::resolve::AwaitingOp;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::prelude::{spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, AttachmentSource, Condition, CostPart, CounterKind, Duration, Filter, Layer, Modifier,
    PlayerRel, SpecialActionCost, SpecialActionTiming, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{artifact, creature, enchantment};
use baylee_core::ids::{DamageSourceRef, EffectId, GrantedActionId, SubtypeId, TargetRef};
use baylee_core::mana::ManaColor;
use smallvec::smallvec;

const C0: u32 = 99_860;
const C1: u32 = 99_861;
const C2: u32 = 99_862;
const C3: u32 = 99_863;
const C4: u32 = 99_864;
const C5: u32 = 99_865;
const C6: u32 = 99_866;

static LICID_SUBTYPES: &[SubtypeId] = &[creature::LICID];
static AURA_SUBTYPES: &[SubtypeId] = &[enchantment::AURA];
static EQUIP_SUBTYPES: &[SubtypeId] = &[artifact::EQUIPMENT];

fn aim_objects(engine: &mut Bench, seat: PlayerId, objects: Vec<ObjectId>) {
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects,
                players: vec![],
            },
        )
        .expect("object targets accepted");
}

fn float_mana(engine: &mut Bench, seat: PlayerId, color: ManaColor, amount: u32) {
    engine.state.players[seat.get() as usize]
        .mana_pool
        .add(color, amount);
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn cast_card(engine: &mut Bench, seat: PlayerId, card_obj: ObjectId) {
    engine
        .apply(seat, PlayerAction::CastSpell { card: card_obj })
        .expect("cast accepted");
}

fn plain_creature(
    index: u32,
    name: &'static str,
    cost: &'static str,
    p: i16,
    t: i16,
) -> &'static CardDef {
    card(
        index,
        creature_face(name, cost, p, t),
        KeywordSet::EMPTY,
        &[],
    )
}

fn plain_equipment(
    index: u32,
    name: &'static str,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    let f = FaceDef {
        subtypes: EQUIP_SUBTYPES,
        ..face(name, "{2}", TypeSet::ARTIFACT)
    };
    card(index, f, KeywordSet::EMPTY, abilities)
}

fn plain_aura(
    index: u32,
    name: &'static str,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    let f = FaceDef {
        subtypes: AURA_SUBTYPES,
        ..face(name, "{1}{W}", TypeSet::ENCHANTMENT)
    };
    card(index, f, KeywordSet::EMPTY, abilities)
}

// =========================================================================
// 1. Licid / BecomeAura / GrantEnchant / LoseAbility / EndContinuousEffects
// =========================================================================

#[test]
fn licid_become_aura_lifecycle_split_second_cleanup_sba_and_clone() {
    static LICID_EFFECTS: &[Effect] = &[Effect::BecomeAura {
        target: TargetSpec::Object(&Filter::CREATURE),
        enchant: TargetSpec::Object(&Filter::CREATURE),
        lose_this_ability: true,
        revert_cost: Some(ManaCost::parse("{W}")),
    }];
    static LICID_PING_COUNTER: &[Effect] = &[Effect::AddCounterSource {
        kind: CounterKind::P1P1,
        amount: Amount::Fixed(1),
    }];
    static LICID_ABILITIES: &[AbilityDef] = &[
        paid(
            Cost {
                mana: ManaCost::parse("{1}{W}"),
                parts: &[CostPart::TapSelf],
            },
            LICID_EFFECTS,
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddKeyword(KeywordSet::FIRST_STRIKE)
        ),
        free(LICID_PING_COUNTER, None),
    ];
    static DESTROY_TARGET: &[Effect] = &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))];
    static MURDER_ABILITIES: &[AbilityDef] = &[spell!(
        DESTROY_TARGET,
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static SPLIT_SECOND_NOOP: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(1),
    }];
    static SPLIT_SECOND_ABILITIES: &[AbilityDef] = &[spell!(SPLIT_SECOND_NOOP)];

    let licid_face = FaceDef {
        subtypes: LICID_SUBTYPES,
        ..creature_face("Test Licid", "{1}{W}", 1, 1)
    };
    let licid_def = card(C0, licid_face, KeywordSet::EMPTY, LICID_ABILITIES);
    let bear_def = plain_creature(C1, "Host Bear", "{1}{G}", 2, 2);
    let murder_def = card(
        C2,
        face("Kill", "{B}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        MURDER_ABILITIES,
    );
    let split_def = card(
        C3,
        face("Split Spell", "{W}", TypeSet::INSTANT),
        KeywordSet::SPLIT_SECOND,
        SPLIT_SECOND_ABILITIES,
    );

    let mut engine = bench(
        101,
        vec![licid_def, bear_def, murder_def, split_def],
        [Seat::with(&[C0, C1]).holding(&[C2, C3]), Seat::default()],
    );
    to_main(&mut engine, me());

    let licid = the(&engine, ZoneLocation::Battlefield, C0);
    let bear = the(&engine, ZoneLocation::Battlefield, C1);

    // Transform Licid into Aura attached to Host Bear.
    float_mana(&mut engine, me(), ManaColor::White, 2);
    activate(&mut engine, me(), licid, 0);
    aim_objects(&mut engine, me(), vec![bear]);
    settle(&mut engine, me());

    let licid_obj = engine.state().object(licid).unwrap();
    let licid_chars = licid_obj.characteristics();
    assert!(licid_chars.types.contains(TypeSet::ENCHANTMENT));
    assert!(!licid_chars.types.contains(TypeSet::CREATURE));
    assert!(licid_chars.subtypes.contains(enchantment::AURA));
    assert_eq!(licid_obj.attached_to, Some(bear));
    assert!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE),
        "Licid Aura grants first strike to enchanted creature"
    );

    // CR 707.2: copiable values of the transformed Licid remain the printed 1/1 Creature — Licid.
    let copied = crate::layers::copiable_values(engine.state(), licid).unwrap();
    assert!(copied.types.contains(TypeSet::CREATURE));
    assert!(!copied.subtypes.contains(enchantment::AURA));

    // Only ability 0 is lost; ability 2 remains offered and activatable.
    let (_, legal) = offer(&engine);
    assert!(
        !legal.abilities.contains(&(licid, 0)),
        "transform ability 0 is lost while Aura"
    );
    assert!(
        legal.abilities.contains(&(licid, 2)),
        "unrelated printed ability 2 is NOT lost"
    );
    // Special action exists in state (not yet payable without mana, or offered once mana floats).
    float_mana(&mut engine, me(), ManaColor::White, 1);
    let (_, legal) = offer(&engine);
    let end_offer = legal
        .granted_actions
        .iter()
        .find(|g| matches!(g.effect, GrantedActionKind::EndContinuousEffects { target } if target.object == licid))
        .expect("EndContinuousEffects special action offered when payable")
        .clone();

    // Advance past turn cleanup into next turn: special action MUST persist across cleanup!
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    assert_eq!(
        engine.state().object(licid).unwrap().attached_to,
        Some(bear),
        "Licid remains an Aura attached to Bear across turns"
    );

    // Split Second (CR 702.61b): special action to end Licid effect STILL works with Split Second on stack!
    let split_card = the(&engine, ZoneLocation::Hand(me()), C3);
    float_mana(&mut engine, me(), ManaColor::White, 2);
    cast_card(&mut engine, me(), split_card);
    // Active player holds priority with Split Second spell on the stack.
    let (prio, split_legal) = offer(&engine);
    assert_eq!(prio, me());
    assert!(
        split_legal.abilities.is_empty(),
        "Split Second locks out ordinary activated abilities"
    );
    assert!(
        split_legal
            .granted_actions
            .iter()
            .any(|g| g.id == end_offer.id),
        "Licid special action remains legal under Split Second (CR 702.61b)"
    );
    engine
        .apply(me(), PlayerAction::TakeGrantedAction { id: end_offer.id })
        .expect("taking Licid revert special action under Split Second succeeds");
    settle(&mut engine, me());

    // Licid reverted to Creature, unattached per CR 704.5p, stayed on battlefield, regained ability 0.
    let reverted = engine.state().object(licid).unwrap();
    assert_eq!(reverted.zone, Zone::Battlefield);
    assert_eq!(reverted.attached_to, None);
    assert!(reverted.characteristics().types.contains(TypeSet::CREATURE));
    assert!(
        !reverted
            .characteristics()
            .subtypes
            .contains(enchantment::AURA)
    );
    assert!(
        !engine
            .state()
            .object(bear)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE)
    );
    assert!(engine.state().endable_effect_groups.is_empty());
    assert!(engine.state().granted_actions.is_empty());

    // Transform again, then kill Host Bear -> CR 704.5m bins Licid-Aura AND prunes special action.
    float_mana(&mut engine, me(), ManaColor::White, 2);
    activate(&mut engine, me(), licid, 0);
    aim_objects(&mut engine, me(), vec![bear]);
    settle(&mut engine, me());
    assert_eq!(engine.state().endable_effect_groups.len(), 1);

    let kill_card = the(&engine, ZoneLocation::Hand(me()), C2);
    float_mana(&mut engine, me(), ManaColor::Black, 1);
    cast_card(&mut engine, me(), kill_card);
    aim_objects(&mut engine, me(), vec![bear]);
    settle(&mut engine, me());

    assert_eq!(
        engine.state().object(bear).unwrap().zone,
        Zone::Graveyard,
        "Host Bear died"
    );
    assert_eq!(
        engine.state().object(licid).unwrap().zone,
        Zone::Graveyard,
        "Unattached Licid-Aura went to graveyard via CR 704.5m"
    );
    assert!(
        engine.state().endable_effect_groups.is_empty(),
        "EndableEffectGroup pruned on battlefield departure"
    );
    assert!(
        engine.state().granted_actions.is_empty(),
        "EndContinuousEffects grant pruned on battlefield departure"
    );
}

#[test]
fn lose_all_abilities_later_timestamp_wipes_grant_enchant() {
    static LICID_BECOME: &[Effect] = &[Effect::BecomeAura {
        target: TargetSpec::Object(&Filter::CREATURE),
        enchant: TargetSpec::Object(&Filter::CREATURE),
        lose_this_ability: true,
        revert_cost: Some(ManaCost::parse("{W}")),
    }];
    static LICID_AB: &[AbilityDef] = &[free(
        LICID_BECOME,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    let aura_like = plain_creature(C0, "Aura Target", "{1}", 1, 1);
    let mut engine = bench(102, vec![aura_like], [Seat::with(&[C0]), Seat::default()]);
    to_main(&mut engine, me());
    let id = the(&engine, ZoneLocation::Battlefield, C0);
    let filter = EffectFilter::object(engine.state(), id);

    let ts1 = engine.state.next_timestamp();
    engine.state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: Some(id),
        controller: me(),
        origin: EffectOrigin::Resolution,
        layer: Layer::Ability,
        timestamp: ts1,
        duration: Duration::Indefinitely,
        filter,
        modifier: Modifier::GrantEnchant(TargetSpec::Object(&Filter::CREATURE)),
    });
    engine.state.refresh_characteristics();
    assert!(
        crate::aura_bindings::restriction(engine.state(), engine.state().object(id).unwrap())
            .is_some(),
        "GrantEnchant supplies enchant restriction"
    );

    let ts2 = engine.state.next_timestamp();
    engine.state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: Some(id),
        controller: me(),
        origin: EffectOrigin::Resolution,
        layer: Layer::Ability,
        timestamp: ts2,
        duration: Duration::Indefinitely,
        filter,
        modifier: Modifier::LoseAllAbilities,
    });
    engine.state.refresh_characteristics();
    assert!(
        crate::aura_bindings::restriction(engine.state(), engine.state().object(id).unwrap())
            .is_none(),
        "Later LoseAllAbilities removes earlier Layer 6 GrantEnchant"
    );

    // Part B: CR 704.5m — a Licid transformed into an Aura via BecomeAura whose
    // Layer 6 GrantEnchant is stripped by a later LoseAllAbilities is binned by SBAs.
    let licid = card(
        C1,
        creature_face("Stripped Licid", "{1}{W}", 1, 1),
        KeywordSet::EMPTY,
        LICID_AB,
    );
    let host = plain_creature(C2, "Host Bear", "{1}{G}", 2, 2);
    let mut e2 = bench(
        1021,
        vec![licid, host],
        [Seat::with(&[C1, C2]), Seat::default()],
    );
    to_main(&mut e2, me());
    let l_id = the(&e2, ZoneLocation::Battlefield, C1);
    let h_id = the(&e2, ZoneLocation::Battlefield, C2);

    activate(&mut e2, me(), l_id, 0);
    aim_objects(&mut e2, me(), vec![h_id]);
    settle(&mut e2, me());
    assert_eq!(e2.state().object(l_id).unwrap().attached_to, Some(h_id));

    let l_filter = EffectFilter::object(e2.state(), l_id);
    let ts_strip = e2.state.next_timestamp();
    e2.state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: me(),
        origin: EffectOrigin::Resolution,
        layer: Layer::Ability,
        timestamp: ts_strip,
        duration: Duration::Indefinitely,
        filter: l_filter,
        modifier: Modifier::LoseAllAbilities,
    });
    e2.state.refresh_characteristics();
    let _ = crate::sba::run(&mut e2.state, &e2.lookup);
    assert_eq!(
        e2.state().object(l_id).unwrap().zone,
        Zone::Graveyard,
        "CR 704.5m: Licid-Aura whose GrantEnchant was stripped by later LoseAllAbilities goes to graveyard"
    );
    assert!(
        e2.state().endable_effect_groups.is_empty(),
        "Endable effect group pruned when stripped Licid-Aura goes to graveyard"
    );
}

// =========================================================================
// 2. Reconfigure + Animate Timestamp Interactions (CR 613.7m, 702.151b, 704.5p)
// =========================================================================

#[test]
fn reconfigure_and_animate_equipment_timestamp_interactions() {
    static RECONF_ATTACH: &[Effect] = &[Effect::ReconfigureAttach];
    static RECONF_ABILITIES: &[AbilityDef] = &[
        free(
            RECONF_ATTACH,
            Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        ),
        static_ability!(
            Filter::This,
            Modifier::RemoveType(TypeSet::CREATURE),
            condition = Some(Condition::SourceMatches(&Filter::IsAttached))
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(1, 1)),
    ];
    let reconf_face = FaceDef {
        subtypes: EQUIP_SUBTYPES,
        power: Some(1),
        toughness: Some(1),
        ..face(
            "Test Reconfigure",
            "{1}",
            TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        )
    };
    let reconf_def = card(C0, reconf_face, KeywordSet::EMPTY, RECONF_ABILITIES);
    let bear_def = plain_creature(C1, "Bear", "{1}{G}", 2, 2);

    let mut engine = bench(
        103,
        vec![reconf_def, bear_def],
        [Seat::with(&[C0, C1]), Seat::default()],
    );
    to_main(&mut engine, me());

    let eq = the(&engine, ZoneLocation::Battlefield, C0);
    let bear = the(&engine, ZoneLocation::Battlefield, C1);

    // Case A: Earlier AddType(CREATURE) applied BEFORE attachment is overridden by
    // Reconfigure's attachment timestamp (CR 613.7m, CR 702.151b).
    let ts_early = engine.state.next_timestamp();
    let eq_filter = EffectFilter::object(engine.state(), eq);
    engine.state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: Some(eq),
        controller: me(),
        origin: EffectOrigin::Resolution,
        layer: Layer::Type,
        timestamp: ts_early,
        duration: Duration::UntilEndOfTurn,
        filter: eq_filter,
        modifier: Modifier::AddType(TypeSet::CREATURE),
    });
    activate(&mut engine, me(), eq, 0);
    aim_objects(&mut engine, me(), vec![bear]);
    settle(&mut engine, me());

    assert_eq!(
        engine.state().object(eq).unwrap().attached_to,
        Some(bear),
        "Earlier AddType(CREATURE) is overridden by Reconfigure attachment timestamp"
    );
    assert!(
        !engine
            .state()
            .object(eq)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    assert_eq!(
        engine.state().object(bear).unwrap().characteristics().power,
        Some(3)
    );

    // Case B: Later AddType(CREATURE) applied AFTER attachment overrides Reconfigure
    // -> becomes a creature -> SBA CR 704.5p unattaches it on the battlefield.
    let ts_late = engine.state.next_timestamp();
    engine.state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: Some(eq),
        controller: me(),
        origin: EffectOrigin::Resolution,
        layer: Layer::Type,
        timestamp: ts_late,
        duration: Duration::UntilEndOfTurn,
        filter: eq_filter,
        modifier: Modifier::AddType(TypeSet::CREATURE),
    });
    engine.state.refresh_characteristics();
    crate::sba::run(&mut engine.state, &engine.lookup);
    assert_eq!(
        engine.state().object(eq).unwrap().attached_to,
        None,
        "Later AddType(CREATURE) makes attached Reconfigure Equipment a creature -> unattaches (CR 704.5p)"
    );
    assert_eq!(engine.state().object(eq).unwrap().zone, Zone::Battlefield);
}

// =========================================================================
// 3. Stack-to-Battlefield Animate (CR 400.7b), Bloodthirst, Sunburst, SetName, ModifyEvent
// =========================================================================

#[test]
fn stack_to_battlefield_bloodthirst_sunburst_set_name_and_grant_static() {
    static GRANT_HASTE_STATIC: &[Effect] = &[Effect::ModifyEvent {
        modifier: Modifier::GrantStatic {
            filter: &Filter::YOUR_CREATURE,
            modifier: &Modifier::AddKeyword(KeywordSet::HASTE),
        },
        duration: Duration::UntilEndOfTurn,
    }];
    static HASTE_GRANTER: &[AbilityDef] = &[triggered!(
        Trigger::EntersBattlefield(&Filter::CREATURE),
        GRANT_HASTE_STATIC
    )];
    static GRANT_BLOODTHIRST: &[Effect] = &[Effect::ModifyEvent {
        modifier: Modifier::GrantBloodthirst(3),
        duration: Duration::Indefinitely,
    }];
    static BLOODLORD_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::SpellCast(&Filter::CREATURE),
        GRANT_BLOODTHIRST
    )];

    static GRANT_SUNBURST: &[Effect] = &[Effect::ModifyEvent {
        modifier: Modifier::GrantSunburst,
        duration: Duration::Indefinitely,
    }];
    static ARTILLERY_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::SpellCast(&Filter::ARTIFACT),
        GRANT_SUNBURST
    )];

    static GRANT_CREATURE_AND_SUNBURST: &[Effect] = &[
        Effect::ModifyEvent {
            modifier: Modifier::AddType(TypeSet::CREATURE),
            duration: Duration::Indefinitely,
        },
        Effect::ModifyEvent {
            modifier: Modifier::SetPT(1, 1),
            duration: Duration::Indefinitely,
        },
        Effect::ModifyEvent {
            modifier: Modifier::GrantSunburst,
            duration: Duration::Indefinitely,
        },
    ];
    static ANIM_ARTILLERY_ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::SpellCast(&Filter::ARTIFACT),
        GRANT_CREATURE_AND_SUNBURST
    )];

    static PING_OPP: &[Effect] = &[Effect::DealDamage {
        amount: Amount::Fixed(1),
        target: TargetSpec::AnyOpponent,
    }];
    static PINGER_ABILITIES: &[AbilityDef] = &[free(
        PING_OPP,
        Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )];

    let bloodlord = card(
        C0,
        creature_face("Bloodlord", "{5}", 3, 3),
        KeywordSet::EMPTY,
        BLOODLORD_ABILITIES,
    );
    let bear = plain_creature(C1, "Vamp Bear", "{1}", 2, 2);
    let pinger = card(
        C2,
        creature_face("Pinger", "{1}", 1, 1),
        KeywordSet::EMPTY,
        PINGER_ABILITIES,
    );

    // 3a. Bloodthirst negative (no damage) & positive (damage dealt)
    let mut e1 = bench(
        104,
        vec![bloodlord, bear, pinger],
        [Seat::with(&[C0, C2]).holding(&[C1, C1]), Seat::default()],
    );
    to_main(&mut e1, me());
    let bears_in_hand = objects(&e1, ZoneLocation::Hand(me()), C1);
    float_mana(&mut e1, me(), ManaColor::Colorless, 1);
    cast_card(&mut e1, me(), bears_in_hand[0]);
    settle(&mut e1, me());
    assert_eq!(
        e1.state()
            .object(bears_in_hand[0])
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0,
        "Bloodthirst adds 0 counters when no opponent took damage this turn"
    );

    // Deal 1 damage to opponent, then cast second bear -> enters with 3 +1/+1 counters.
    let ping_id = the(&e1, ZoneLocation::Battlefield, C2);
    activate(&mut e1, me(), ping_id, 0);
    e1.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![them()],
        },
    )
    .expect("target opponent");
    settle(&mut e1, me());

    float_mana(&mut e1, me(), ManaColor::Colorless, 1);
    cast_card(&mut e1, me(), bears_in_hand[1]);
    settle(&mut e1, me());
    assert_eq!(
        e1.state()
            .object(bears_in_hand[1])
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        3,
        "Stack-granted Bloodthirst(3) puts 3 +1/+1 counters on entering creature"
    );

    // 3b. Sunburst noncreature artifact (charge counters) vs artifact creature (+1/+1 counters)
    // vs stack-animated noncreature artifact (+1/+1 counters because Layer 4 makes it a creature!)
    let artillery = card(
        C3,
        face("Artillery", "{4}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ARTILLERY_ABILITIES,
    );
    let trinket = card(
        C4,
        face("Trinket", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let golem = card(
        C5,
        FaceDef {
            power: Some(1),
            toughness: Some(1),
            ..face("Golem", "{3}", TypeSet::ARTIFACT.union(TypeSet::CREATURE))
        },
        KeywordSet::EMPTY,
        &[],
    );
    let mut e2 = bench(
        105,
        vec![artillery, trinket, golem],
        [Seat::with(&[C3]).holding(&[C4, C5]), Seat::default()],
    );
    to_main(&mut e2, me());
    let trinket_id = the(&e2, ZoneLocation::Hand(me()), C4);
    float_mana(&mut e2, me(), ManaColor::White, 1);
    float_mana(&mut e2, me(), ManaColor::Blue, 1);
    cast_card(&mut e2, me(), trinket_id);
    settle(&mut e2, me());
    assert_eq!(
        e2.state()
            .object(trinket_id)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        2,
        "Noncreature artifact with GrantSunburst paid with W+U enters with 2 charge counters"
    );

    let golem_id = the(&e2, ZoneLocation::Hand(me()), C5);
    float_mana(&mut e2, me(), ManaColor::White, 1);
    float_mana(&mut e2, me(), ManaColor::Blue, 1);
    float_mana(&mut e2, me(), ManaColor::Black, 1);
    cast_card(&mut e2, me(), golem_id);
    settle(&mut e2, me());
    assert_eq!(
        e2.state()
            .object(golem_id)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        3,
        "Artifact creature with GrantSunburst paid with W+U+B enters with 3 +1/+1 counters"
    );

    // Layer 4 animation + GrantSunburst on stack -> enters as creature with +1/+1 counters!
    let anim_artillery = card(
        C6,
        face("Anim Artillery", "{4}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ANIM_ARTILLERY_ABILITIES,
    );
    let mut e3 = bench(
        106,
        vec![anim_artillery, trinket],
        [Seat::with(&[C6]).holding(&[C4]), Seat::default()],
    );
    to_main(&mut e3, me());
    let t_id = the(&e3, ZoneLocation::Hand(me()), C4);
    float_mana(&mut e3, me(), ManaColor::Red, 1);
    float_mana(&mut e3, me(), ManaColor::Green, 1);
    cast_card(&mut e3, me(), t_id);
    settle(&mut e3, me());
    assert_eq!(
        e3.state()
            .object(t_id)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2,
        "Artifact animated to creature on stack gets +1/+1 counters from GrantSunburst"
    );
    assert_eq!(
        e3.state()
            .object(t_id)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        0
    );

    // 3c. Modifier::SetName (Layer 3, CR 613.1c, non-copiable per CR 707.2) + ModifyEvent GrantStatic sync
    let granter = card(
        C0,
        face("Granter", "{2}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        HASTE_GRANTER,
    );
    let mut e4 = bench(
        107,
        vec![granter, bear],
        [Seat::with(&[C0]).holding(&[C1]), Seat::default()],
    );
    to_main(&mut e4, me());
    let b_id = the(&e4, ZoneLocation::Hand(me()), C1);
    float_mana(&mut e4, me(), ManaColor::Colorless, 1);
    cast_card(&mut e4, me(), b_id);
    settle(&mut e4, me());
    assert!(
        e4.state()
            .object(b_id)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE),
        "ModifyEvent granting Modifier::GrantStatic immediately syncs granted static"
    );

    // Modifier::SetName("Moon")
    let filter = EffectFilter::object(e4.state(), b_id);
    let ts = e4.state.next_timestamp();
    e4.state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: Some(b_id),
        controller: me(),
        origin: EffectOrigin::Resolution,
        layer: Layer::Type,
        timestamp: ts,
        duration: Duration::Indefinitely,
        filter,
        modifier: Modifier::SetName("Moon"),
    });
    e4.state.refresh_characteristics();
    let obj = e4.state().object(b_id).unwrap();
    assert_eq!(e4.state().names.get(obj.characteristics().name), "Moon");
    assert!(crate::eval::matches(
        &Filter::Named("Moon"),
        e4.state(),
        obj,
        me(),
        b_id
    ));
    assert!(!crate::eval::matches(
        &Filter::Named("Vamp Bear"),
        e4.state(),
        obj,
        me(),
        b_id
    ));
    let copied = crate::layers::copiable_values(e4.state(), b_id).unwrap();
    assert_eq!(
        e4.state().names.get(copied.name),
        "Vamp Bear",
        "SetName is non-copiable (CR 707.2)"
    );
}

// =========================================================================
// 4. SetDynamicPT, Amount::SourceToughness, Amount::DiscardedManaValue
// =========================================================================

#[test]
fn set_dynamic_pt_targeted_this_object_offsets_source_toughness_and_discarded_mv() {
    // Mangler/Sworn Defender: targeted ability setting SOURCE P/T with signed offsets
    static SET_POWER_ONLY: &[Effect] = &[Effect::SetDynamicPT {
        subject: TargetSpec::ThisObject,
        power: Some(Amount::TargetPower),
        toughness: None,
        power_offset: 0,
        toughness_offset: 0,
        duration: Duration::Indefinitely,
    }];
    static SET_OFFSET_PT: &[Effect] = &[Effect::SetDynamicPT {
        subject: TargetSpec::ThisObject,
        power: Some(Amount::TargetToughness),
        toughness: Some(Amount::TargetPower),
        power_offset: -1,
        toughness_offset: 1,
        duration: Duration::UntilEndOfTurn,
    }];
    static MANGLER_ABILITIES: &[AbilityDef] = &[
        free(
            SET_POWER_ONLY,
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        free(
            SET_OFFSET_PT,
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
    ];

    // Wolfbear: sets target's P/T to source's power and SourceToughness
    static WOLFB_EFFECT: &[Effect] = &[Effect::SetDynamicPT {
        subject: TargetSpec::Object(&Filter::CREATURE),
        power: Some(Amount::SourcePower),
        toughness: Some(Amount::SourceToughness),
        power_offset: 0,
        toughness_offset: 0,
        duration: Duration::UntilEndOfTurn,
    }];
    static WOLFB_ABILITIES: &[AbilityDef] = &[free(
        WOLFB_EFFECT,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    // Slumbering Tora: discard a card as cost, set source P/T to DiscardedManaValue
    static TORA_EFFECT: &[Effect] = &[Effect::SetDynamicPT {
        subject: TargetSpec::ThisObject,
        power: Some(Amount::DiscardedManaValue),
        toughness: Some(Amount::DiscardedManaValue),
        power_offset: 0,
        toughness_offset: 0,
        duration: Duration::UntilEndOfTurn,
    }];
    static TORA_ABILITIES: &[AbilityDef] = &[paid(
        Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::Discard(&Filter::Any)],
        },
        TORA_EFFECT,
        None,
    )];

    let mangler_def = card(
        C0,
        creature_face("Mangler", "{2}", 0, 3),
        KeywordSet::EMPTY,
        MANGLER_ABILITIES,
    );
    let giant_def = plain_creature(C1, "Giant", "{4}", 4, 6);
    let wolfbear_def = card(
        C2,
        creature_face("Wolfbear", "{3}", 2, 7),
        KeywordSet::EMPTY,
        WOLFB_ABILITIES,
    );
    let tiny_def = plain_creature(C3, "Tiny", "{1}", 1, 1);
    let tora_def = card(
        C4,
        creature_face("Tora", "{3}", 1, 1),
        KeywordSet::EMPTY,
        TORA_ABILITIES,
    );

    let mut engine = bench(
        108,
        vec![mangler_def, giant_def, wolfbear_def, tiny_def, tora_def],
        [
            Seat::with(&[C0, C1, C2, C3, C4]).holding(&[C1]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let mangler = the(&engine, ZoneLocation::Battlefield, C0);
    let giant = the(&engine, ZoneLocation::Battlefield, C1);
    let wolfbear = the(&engine, ZoneLocation::Battlefield, C2);
    let tiny = the(&engine, ZoneLocation::Battlefield, C3);
    let tora = the(&engine, ZoneLocation::Battlefield, C4);
    let card_mv4 = the(&engine, ZoneLocation::Hand(me()), C1);

    // 1) Mangler ability 0 (power only, ThisObject even though targeted): 0/3 -> 4/3
    activate(&mut engine, me(), mangler, 0);
    aim_objects(&mut engine, me(), vec![giant]);
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(mangler)
            .unwrap()
            .characteristics()
            .power,
        Some(4)
    );
    assert_eq!(
        engine
            .state()
            .object(mangler)
            .unwrap()
            .characteristics()
            .toughness,
        Some(3)
    );
    assert_eq!(
        engine
            .state()
            .object(giant)
            .unwrap()
            .characteristics()
            .power,
        Some(4)
    );

    // 2) Mangler ability 1 (signed offsets: TargetToughness - 1 / TargetPower + 1 = 5/5)
    activate(&mut engine, me(), mangler, 1);
    aim_objects(&mut engine, me(), vec![giant]);
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(mangler)
            .unwrap()
            .characteristics()
            .power,
        Some(5)
    );
    assert_eq!(
        engine
            .state()
            .object(mangler)
            .unwrap()
            .characteristics()
            .toughness,
        Some(5)
    );

    // 3) Wolfbear sets Tiny (1/1) to SourcePower (2) / SourceToughness (7)
    activate(&mut engine, me(), wolfbear, 0);
    aim_objects(&mut engine, me(), vec![tiny]);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(tiny).unwrap().characteristics().power,
        Some(2)
    );
    assert_eq!(
        engine
            .state()
            .object(tiny)
            .unwrap()
            .characteristics()
            .toughness,
        Some(7)
    );

    // 4) Tora discards Giant (MV 4) -> Tora becomes 4/4
    activate(&mut engine, me(), tora, 0);
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![card_mv4],
                },
            )
            .expect("discard Giant");
    }
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(tora).unwrap().characteristics().power,
        Some(4)
    );
    assert_eq!(
        engine
            .state()
            .object(tora)
            .unwrap()
            .characteristics()
            .toughness,
        Some(4)
    );
}

// =========================================================================
// 5. Attach Engine Ops: Unattach, AttachObject, MoveAll, AttachLtb,
//    AttachChosenThen, RelocateAttachment (incl. GrantedSource & opponent chooser)
// =========================================================================

#[test]
fn unattach_attach_object_and_attach_ltb_attachments() {
    static EQUIP_BUFF: &[AbilityDef] = &[static_ability!(
        Filter::AttachedToBySource,
        Modifier::ModifyPT(3, 0)
    )];
    static UNATTACH_TARGET: &[Effect] = &[Effect::Unattach {
        target: TargetSpec::Object(&Filter::ARTIFACT),
    }];
    static ATTACH_MULTI_TO_SELF: &[Effect] = &[Effect::AttachObject {
        object: TargetSpec::Object(&Filter::ARTIFACT),
        to: TargetSpec::ThisObject,
        optional: false,
    }];
    static ATTACH_EVENT_OPT: &[Effect] = &[Effect::AttachObject {
        object: TargetSpec::EventObject,
        to: TargetSpec::Object(&Filter::YOUR_CREATURE),
        optional: true,
    }];
    static RHUK_EFFECT: &[Effect] = &[Effect::AttachLtbAttachments {
        of: TargetSpec::EventObject,
        filter: &Filter::HasSubtype(artifact::EQUIPMENT),
        to: TargetSpec::ThisObject,
        optional: false,
    }];
    static DESTROY_TARGET: &[Effect] = &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))];

    static TOOL_ABILITIES: &[AbilityDef] = &[
        free(
            UNATTACH_TARGET,
            Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))),
        ),
        free(
            ATTACH_MULTI_TO_SELF,
            Some(TargetReq::exactly(TargetSpec::Object(&Filter::ARTIFACT), 2)),
        ),
        triggered!(
            Trigger::EntersBattlefield(&Filter::HasSubtype(artifact::EQUIPMENT)),
            ATTACH_EVENT_OPT,
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE)))
        ),
        triggered!(Trigger::Dies(&Filter::CREATURE), RHUK_EFFECT),
        free(
            DESTROY_TARGET,
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
    ];

    let tool_def = card(
        C0,
        creature_face("Rhuk Tool", "{3}", 2, 2),
        KeywordSet::EMPTY,
        TOOL_ABILITIES,
    );
    let bear_def = plain_creature(C1, "Bear", "{1}", 2, 2);
    let sword_def = plain_equipment(C2, "Sword", EQUIP_BUFF);

    let mut engine = bench(
        109,
        vec![tool_def, bear_def, sword_def],
        [
            Seat::with(&[C0, C1, C2, C2]).holding(&[C2]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let tool = the(&engine, ZoneLocation::Battlefield, C0);
    let bear = the(&engine, ZoneLocation::Battlefield, C1);
    let swords = objects(&engine, ZoneLocation::Battlefield, C2);
    let sword_hand = the(&engine, ZoneLocation::Hand(me()), C2);

    // 1) AttachObject multi-target -> ThisObject (attaches both swords to tool: 2+3+3 = 8 power)
    activate(&mut engine, me(), tool, 1);
    aim_objects(&mut engine, me(), vec![swords[0], swords[1]]);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(swords[0]).unwrap().attached_to,
        Some(tool)
    );
    assert_eq!(
        engine.state().object(swords[1]).unwrap().attached_to,
        Some(tool)
    );
    assert_eq!(
        engine.state().object(tool).unwrap().characteristics().power,
        Some(8)
    );

    // 2) Unattach target Equipment (CR 701.3d)
    activate(&mut engine, me(), tool, 0);
    aim_objects(&mut engine, me(), vec![swords[0]]);
    settle(&mut engine, me());
    assert_eq!(engine.state().object(swords[0]).unwrap().attached_to, None);
    assert_eq!(
        engine.state().object(tool).unwrap().characteristics().power,
        Some(5)
    );

    // 3) Hammer of Nazahn optional AttachObject (EventObject -> targeted creature): accept attaches!
    float_mana(&mut engine, me(), ManaColor::Colorless, 2);
    cast_card(&mut engine, me(), sword_hand);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_objects(&mut engine, me(), vec![bear]);
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("accept attach");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(sword_hand).unwrap().attached_to,
        Some(bear)
    );
    assert_eq!(
        engine.state().object(bear).unwrap().characteristics().power,
        Some(5)
    );

    // 4) Rhuk: Bear wearing sword_hand dies -> AttachLtbAttachments reads ltb_attachments and moves sword_hand to tool!
    activate(&mut engine, me(), tool, 4);
    aim_objects(&mut engine, me(), vec![bear]);
    settle(&mut engine, me());
    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state().object(sword_hand).unwrap().attached_to,
        Some(tool),
        "AttachLtbAttachments moved dying Bear's Equipment to Rhuk via ltb_attachments"
    );
}

#[test]
fn move_all_attachments_moves_legal_and_leaves_illegal_behind() {
    static ENCHANT_CREATURE: &[AbilityDef] = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(1, 1)),
    ];
    static RED_CREATURE_FILTER: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::HasColor(ColorSet::from_slice(&[Color::Red])),
    ]);
    static ENCHANT_RED_CREATURE: &[AbilityDef] = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&RED_CREATURE_FILTER),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&RED_CREATURE_FILTER)))
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 0)),
    ];
    static SAME_CTRL_CREATURE: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::ControlledBy(PlayerRel::ControllerOfTarget),
    ]);
    static GLAMER_EFFECT: &[Effect] = &[Effect::MoveAllAttachments {
        attachments: &Filter::HasSubtype(enchantment::AURA),
        from: TargetSpec::Object(&Filter::CREATURE),
        recipient_filter: &SAME_CTRL_CREATURE,
        must_change_recipient: true,
        optional: false,
    }];
    static GLAMER_ABILITIES: &[AbilityDef] = &[free(
        GLAMER_EFFECT,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let red_bear = plain_creature(C0, "Red Bear", "{1}{R}", 2, 2);
    let green_bear = plain_creature(C1, "Green Bear", "{1}{G}", 2, 2);
    let any_aura = plain_aura(C2, "Any Aura", ENCHANT_CREATURE);
    let red_aura = plain_aura(C3, "Red Aura", ENCHANT_RED_CREATURE);
    let spinner = card(
        C4,
        creature_face("Spinner", "{4}", 2, 2),
        KeywordSet::EMPTY,
        GLAMER_ABILITIES,
    );

    let mut engine = bench(
        110,
        vec![red_bear, green_bear, any_aura, red_aura, spinner],
        [
            Seat::with(&[C0, C1, C4]).holding(&[C2, C3]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let r_bear = the(&engine, ZoneLocation::Battlefield, C0);
    let g_bear = the(&engine, ZoneLocation::Battlefield, C1);
    let spin = the(&engine, ZoneLocation::Battlefield, C4);
    let a_any = the(&engine, ZoneLocation::Hand(me()), C2);
    let a_red = the(&engine, ZoneLocation::Hand(me()), C3);

    float_mana(&mut engine, me(), ManaColor::White, 4);
    cast_card(&mut engine, me(), a_any);
    aim_objects(&mut engine, me(), vec![r_bear]);
    settle(&mut engine, me());
    cast_card(&mut engine, me(), a_red);
    aim_objects(&mut engine, me(), vec![r_bear]);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(a_any).unwrap().attached_to,
        Some(r_bear)
    );
    assert_eq!(
        engine.state().object(a_red).unwrap().attached_to,
        Some(r_bear)
    );

    activate(&mut engine, me(), spin, 0);
    aim_objects(&mut engine, me(), vec![r_bear]);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(
        !options.contains(&r_bear),
        "must_change_recipient excludes old host"
    );
    assert!(options.contains(&g_bear));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![g_bear],
            },
        )
        .expect("choose Green Bear");
    settle(&mut engine, me());

    assert_eq!(
        engine.state().object(a_any).unwrap().attached_to,
        Some(g_bear),
        "Legal Aura moves to Green Bear"
    );
    assert_eq!(
        engine.state().object(a_red).unwrap().attached_to,
        Some(r_bear),
        "Illegal Red-only Aura stays attached to Red Bear"
    );
}

#[test]
fn attach_chosen_then_gates_followup_on_actual_host_change() {
    static GRANT_INDESTRUCTIBLE_TO_ATTACHED_EQUIP: &[Effect] = &[Effect::ModifyEvent {
        modifier: Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE),
        duration: Duration::UntilEndOfTurn,
    }];
    static TRIGGER_ATTACH_CHOSEN_THEN: &[Effect] = &[Effect::AttachChosenThen {
        filter: &Filter::HasSubtype(artifact::EQUIPMENT),
        to: TargetSpec::EventObject,
        max: Amount::Fixed(1),
        optional: true,
        then_if_attached: GRANT_INDESTRUCTIBLE_TO_ATTACHED_EQUIP,
    }];
    static ETB_SCYTHE_WATCHER: &[AbilityDef] = &[triggered!(
        Trigger::EntersBattlefield(&Filter::CREATURE),
        TRIGGER_ATTACH_CHOSEN_THEN
    )];
    static EQUIP_BUFF: &[AbilityDef] = &[static_ability!(
        Filter::AttachedToBySource,
        Modifier::ModifyPT(2, 0)
    )];
    static THEN_DRAW: &[Effect] = &[Effect::DrawCards {
        amount: Amount::Fixed(1),
    }];
    static ATTACH_CHOSEN_THEN: &[Effect] = &[Effect::AttachChosenThen {
        filter: &Filter::HasSubtype(artifact::EQUIPMENT),
        to: TargetSpec::Object(&Filter::CREATURE),
        max: Amount::Fixed(1),
        optional: true,
        then_if_attached: THEN_DRAW,
    }];
    static HELPER_ABILITIES: &[AbilityDef] = &[free(
        ATTACH_CHOSEN_THEN,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let helper = card(
        C0,
        creature_face("Helper", "{1}", 1, 1),
        KeywordSet::EMPTY,
        HELPER_ABILITIES,
    );
    let bear = plain_creature(C1, "Bear", "{1}", 2, 2);
    let sword = plain_equipment(C2, "Sword", EQUIP_BUFF);

    let mut engine = bench(
        111,
        vec![helper, bear, sword],
        [Seat::with(&[C0, C1, C2]), Seat::default()],
    );
    to_main(&mut engine, me());

    let h = the(&engine, ZoneLocation::Battlefield, C0);
    let b = the(&engine, ZoneLocation::Battlefield, C1);
    let s = the(&engine, ZoneLocation::Battlefield, C2);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();

    // 1) Decline -> no draw, sword stays unattached.
    activate(&mut engine, me(), h, 0);
    aim_objects(&mut engine, me(), vec![b]);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .expect("decline");
    settle(&mut engine, me());
    assert_eq!(engine.state().object(s).unwrap().attached_to, None);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before
    );

    // 2) Choose unattached sword -> attaches AND draws 1 card!
    activate(&mut engine, me(), h, 0);
    aim_objects(&mut engine, me(), vec![b]);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![s] })
        .expect("attach sword");
    settle(&mut engine, me());
    assert_eq!(engine.state().object(s).unwrap().attached_to, Some(b));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before + 1,
        "then_if_attached ran when Equipment changed host"
    );

    // 3) Choose sword AGAIN when ALREADY attached to Bear (CR 701.3b) -> NO draw!
    activate(&mut engine, me(), h, 0);
    aim_objects(&mut engine, me(), vec![b]);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![s] })
        .expect("re-select already-attached sword");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before + 1,
        "CR 701.3b: choosing an Equipment already attached to the host does NOT trigger then_if_attached"
    );

    // 4) Inside a triggered ability (where res.subject.event_identity is pre-populated
    //    with the triggering creature), AttachChosenThen attaches to that creature AND
    //    rebinds event_identity to the newly attached Equipment for then_if_attached.
    let watcher = card(
        C3,
        face("Scythe Watcher", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        ETB_SCYTHE_WATCHER,
    );
    let mut e2 = bench(
        1112,
        vec![watcher, bear, sword],
        [Seat::with(&[C3, C2]).holding(&[C1]), Seat::default()],
    );
    to_main(&mut e2, me());
    let b2 = the(&e2, ZoneLocation::Hand(me()), C1);
    let s2 = the(&e2, ZoneLocation::Battlefield, C2);
    float_mana(&mut e2, me(), ManaColor::Green, 2);
    cast_card(&mut e2, me(), b2);
    walk_until(&mut e2, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    e2.apply(me(), PlayerAction::ChooseObjects { objects: vec![s2] })
        .expect("attach sword to entering bear");
    settle(&mut e2, me());

    assert_eq!(e2.state().object(s2).unwrap().attached_to, Some(b2));
    assert!(
        e2.state()
            .object(s2)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::INDESTRUCTIBLE),
        "then_if_attached EventObject must resolve to newly attached Equipment, not old triggering creature"
    );
    assert!(
        !e2.state()
            .object(b2)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::INDESTRUCTIBLE),
        "Triggering creature must NOT receive effect meant for newly attached Equipment"
    );
}

#[test]
fn relocate_attachment_opponent_chooser_and_granted_source() {
    static THEN_GAIN_LIFE: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(2),
    }];
    static RELOCATE_BY_OPP: &[Effect] = &[Effect::RelocateAttachment {
        attachment: AttachmentSource::Spec(TargetSpec::ThisObject),
        chooser: PlayerRel::Opponent,
        recipient_filter: &Filter::CREATURE,
        must_change_recipient: true,
        optional: false,
        then_if_moved: THEN_GAIN_LIFE,
    }];
    static VINE_AURA_ABILITIES: &[AbilityDef] = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        free(RELOCATE_BY_OPP, None),
    ];

    // Nettlevine Blight shape: Aura grants activated/triggered ability to enchanted permanent;
    // resolving ability relocates AttachmentSource::GrantedSource!
    static RELOCATE_GRANTED_AURA: &[Effect] = &[Effect::RelocateAttachment {
        attachment: AttachmentSource::GrantedSource,
        chooser: PlayerRel::You,
        recipient_filter: &Filter::CREATURE,
        must_change_recipient: true,
        optional: false,
        then_if_moved: &[],
    }];
    static NETTLEVINE_ABILITIES: &[AbilityDef] = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::GrantActivated {
                cost: Cost::FREE,
                effects: RELOCATE_GRANTED_AURA,
                mana_ability: false,
            }
        ),
    ];

    let bear_a = plain_creature(C0, "Bear A", "{1}", 2, 2);
    let bear_b = plain_creature(C1, "Bear B", "{1}", 2, 2);
    let vine = plain_aura(C2, "Vine Aura", VINE_AURA_ABILITIES);
    let blight = plain_aura(C3, "Blight Aura", NETTLEVINE_ABILITIES);

    let mut engine = bench(
        112,
        vec![bear_a, bear_b, vine, blight],
        [Seat::with(&[C0, C1]).holding(&[C2, C3]), Seat::default()],
    );
    to_main(&mut engine, me());

    let a = the(&engine, ZoneLocation::Battlefield, C0);
    let b = the(&engine, ZoneLocation::Battlefield, C1);
    let v = the(&engine, ZoneLocation::Hand(me()), C2);
    let bl = the(&engine, ZoneLocation::Hand(me()), C3);

    float_mana(&mut engine, me(), ManaColor::White, 4);
    cast_card(&mut engine, me(), v);
    aim_objects(&mut engine, me(), vec![a]);
    settle(&mut engine, me());
    cast_card(&mut engine, me(), bl);
    aim_objects(&mut engine, me(), vec![a]);
    settle(&mut engine, me());
    assert_eq!(engine.state().object(v).unwrap().attached_to, Some(a));
    assert_eq!(engine.state().object(bl).unwrap().attached_to, Some(a));

    // 1) Opponent chooser moves Vine Aura from Bear A -> Bear B and fires then_if_moved (+2 life to me)
    let life_before = life(&engine, me());
    activate(&mut engine, me(), v, 1);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, them(), "opponent makes the relocation choice");
    assert_eq!(min, 1, "mandatory relocation");
    assert_eq!(options, vec![b], "must_change_recipient excludes Bear A");
    engine
        .apply(them(), PlayerAction::ChooseObjects { objects: vec![b] })
        .expect("opponent relocates Vine Aura");
    settle(&mut engine, me());
    assert_eq!(engine.state().object(v).unwrap().attached_to, Some(b));
    assert_eq!(life(&engine, me()), life_before + 2);

    // 2) AttachmentSource::GrantedSource: activate granted ability on Bear A -> moves Blight Aura to Bear B!
    let (_, legal) = offer(&engine);
    let granted_idx = legal
        .abilities
        .iter()
        .find_map(|&(src, idx)| {
            (src == a && crate::choice::granted_slot(idx).is_some()).then_some(idx)
        })
        .expect("Blight Aura granted relocation ability to Bear A");
    activate(&mut engine, me(), a, granted_idx);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![b] })
        .expect("relocate GrantedSource Aura to Bear B");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(bl).unwrap().attached_to,
        Some(b),
        "AttachmentSource::GrantedSource resolved granting Aura via Rider::GrantedSource"
    );
}

// =========================================================================
// 6. Hash & Fingerprint Sensitivity (Tags 535..=539) + Replay Invariance
// =========================================================================

#[test]
fn hash_and_fingerprint_tags_535_to_539_sensitivity_and_invariance() {
    let bear = plain_creature(C0, "Bear", "{1}", 2, 2);
    let mut engine = bench(113, vec![bear], [Seat::with(&[C0]), Seat::default()]);
    to_main(&mut engine, me());
    let id = the(&engine, ZoneLocation::Battlefield, C0);
    let target_ref = DamageSourceRef {
        object: id,
        version: engine.state().object(id).unwrap().version,
    };

    // Baseline state with empty endable_effect_groups
    let base_snap = engine.state().snapshot_hash();
    let base_loop = engine.state().loop_signature();

    // Add EndContinuousEffects grant (tag 535) + EndableEffectGroup (tag 536)
    let mut with_grant = engine.state().clone();
    let action_id = with_grant.grant_action(
        me(),
        GrantedActionOffer {
            id: GrantedActionId::new(0),
            source: target_ref,
            ability: None,
            timing: SpecialActionTiming::Priority,
            cost: SpecialActionCost::Mana(ManaCost::parse("{W}")),
            effect: GrantedActionKind::EndContinuousEffects { target: target_ref },
        },
    );
    let snap_grant_only = with_grant.snapshot_hash();
    let loop_grant_only = with_grant.loop_signature();
    assert_ne!(base_snap, snap_grant_only, "tag 535 moves snapshot_hash");
    assert_ne!(base_loop, loop_grant_only, "tag 535 moves loop_signature");

    let mut with_group = with_grant.clone();
    with_group.endable_effect_groups.push(EndableEffectGroup {
        action: action_id,
        target: target_ref,
        effects: smallvec![EffectId::new(1)],
    });
    assert_ne!(
        snap_grant_only,
        with_group.snapshot_hash(),
        "tag 536 moves snapshot_hash"
    );
    assert_ne!(
        loop_grant_only,
        with_group.loop_signature(),
        "tag 536 moves loop_signature"
    );

    // AwaitingOp search_fingerprint uniqueness for tags 537, 538, 539
    let fp_move_all = AwaitingOp::MoveAllAttachments {
        from: target_ref,
        attachments: &Filter::Any,
    }
    .search_fingerprint();
    let fp_chosen_then = AwaitingOp::AttachChosenThen {
        host: TargetRef::Object(target_ref),
        then_if_attached: &[],
    }
    .search_fingerprint();
    let fp_relocate = AwaitingOp::RelocateAttachment {
        attachment: target_ref,
        then_if_moved: &[],
    }
    .search_fingerprint();
    assert_ne!(fp_move_all, 0);
    assert_ne!(fp_chosen_then, 0);
    assert_ne!(fp_relocate, 0);
    assert_ne!(fp_move_all, fp_chosen_then);
    assert_ne!(fp_move_all, fp_relocate);
    assert_ne!(fp_chosen_then, fp_relocate);
}
