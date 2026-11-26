//! Wave 17c deterministic gameplay tests:
//! 1. `Modifier::SpellsCostLessManaByAmount` (*Demilich* `{U}` per instant/sorcery with generic
//!    spillover per `CR 118.7c` vs `colored_only: true` *Khalni Hydra* per `CR 118.7b`),
//!    `SpellCastMetric::Bargained` (*Hamlet Glutton*, `CR 702.166a`), and
//!    `SpellCastMetric::DistinctTargetsMatching` (*Battlefield Thaumaturge*).
//! 2. `Modifier::ForetellActionCostsLess` & `Modifier::MayForetellAnyTurn` (*Cosmos Charger*,
//!    *Ranar the Ever-Watchful*, `CR 702.143a`), `Modifier::UnlockCostsLess` (*Inquisitive
//!    Glimmer*, `CR 709.5e`), and `Modifier::EquipCostsLessIfTargets` (*Fervent Champion*).
//! 3. `CostPart::BeholdAndExile` + LTB `Effect::ReturnLinkedExile` (*Champion of the Clachan*,
//!    `CR 701.4a`, `CR 607.2a`), `Modifier::AdjustLoyaltyCost` (*Carth the Lion*, `CR 606.4`),
//!    `Modifier::LoyaltyAbilitiesCostMore` (*Eidolon of Obstruction*),
//!    `Modifier::NonManaAbilitiesCostMore` (*Suppression Field*, `CR 605.1`), and
//!    `Modifier::RaiseCostIfTargets` (*Dragon's Prey*, *Terror of the Peaks*, `CR 601.2f`).
//! 4. `AffectedZone$ Graveyard` static grants & permissions: `Modifier::GrantsUnearth`
//!    (*Sedris*, `CR 702.84a`), `Modifier::GrantsScavengeManaCost` (*Varolz*, `CR 702.97a`),
//!    `Modifier::GrantsDredge` (*The Necrobloom*, `CR 702.52a`), `Modifier::LoseAllAbilities`
//!    in graveyards (*Yixlid Jailer*, `CR 613.7` timestamp order),
//!    `Filter::SurveilledByYouThisTurn` + `AltCastPrice::PayLifeEqualToManaValue`
//!    (*Eye of Duskmantle*, `CR 701.25a`), and `Modifier::MayCastFromGraveyardUsingAlt`
//!    (*Tenacious Underdog* `Blitz`, *Timeline Culler* `Warp`).
//! 5. `Trigger::OneOrMoreCardsDiscarded` (`T: Mode$ DiscardedAll` batching, `Amount::EventAmount`,
//!    filter-aware `first_this_turn`, `per_player: true` vs `false`, and *Toluz* linked exile)
//!    and `Effect::DoublePT` (`DoublePtSubject::{Source, Attached, EventObject, Targets, Filter}`
//!    plus negative-power doubling per `CR 701.10b–c`).

#![allow(clippy::too_many_lines, clippy::items_after_statements)]

use super::*;
use crate::choice::{ArrangePlace, CastModeKind, unlock_door};
use crate::event::Cause;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::prelude::{equip, mana_ability, spell, static_ability, triggered};
use baylee_cards_dsl::{
    AltCastPrice, AltCastZones, Amount, ChooseMove, ChooseZone, CostPart, DoublePtSubject,
    Duration, Filter, GraveyardAltKind, Layer, Modifier, PlayerRel, SpellCastMetric, StaticAbility,
    TargetSpec, Trigger, ZoneRef, ZoneSel, cost,
};
use baylee_core::generated::subtypes::{artifact, creature, enchantment};
use baylee_core::mana;
use baylee_core::mana::ManaColor;

const C0: u32 = 99_870;
const C1: u32 = 99_871;
const C2: u32 = 99_872;
const C3: u32 = 99_873;
const C4: u32 = 99_874;
const C5: u32 = 99_875;
const C6: u32 = 99_876;
const C7: u32 = 99_877;

fn refresh(engine: &mut Bench) {
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
    refresh(engine);
}

fn move_to_graveyard(engine: &mut Bench, owner: PlayerId, id: ObjectId) {
    engine
        .state
        .move_object(
            id,
            ZoneLocation::Graveyard(owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("moved to graveyard");
    refresh(engine);
}

fn move_to_top_of_library(engine: &mut Bench, owner: PlayerId, id: ObjectId) {
    engine
        .state
        .move_object(
            id,
            ZoneLocation::Library(owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("moved to top of library");
    refresh(engine);
}

fn choose_cast_mode_kind(engine: &mut Bench, seat: PlayerId, kind: CastModeKind) {
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode, got {:?}", engine.pending());
    };
    let pos = options
        .iter()
        .position(|opt| opt.kind == kind)
        .unwrap_or_else(|| panic!("expected {kind:?} in {options:?}"));
    engine
        .apply(seat, PlayerAction::ChooseMode(pos))
        .expect("cast mode accepted");
}

fn pt_of(engine: &Bench, id: ObjectId) -> (i16, i16) {
    let chars = engine.state().object(id).unwrap().characteristics();
    (chars.power.unwrap(), chars.toughness.unwrap())
}

#[test]
fn wave17c_spells_cost_less_mana_by_amount_bargain_and_distinct_targets() {
    // 1. Demilich (`colored_only: false`, CR 118.7c) vs ColoredOnlyHydra (`colored_only: true`, CR 118.7b).
    static INSTANT_OR_SORCERY: Filter = Filter::Or(&[Filter::INSTANT, Filter::SORCERY]);
    static DEMILICH_AMT: Amount = Amount::SpellsCastMatchingThisTurn {
        filter: &INSTANT_OR_SORCERY,
        caster: PlayerRel::You,
    };
    static DEMILICH_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::This,
        layer: Layer::Text,
        modifier: Modifier::SpellsCostLessManaByAmount {
            who: PlayerRel::You,
            filter: &Filter::This,
            unit: mana!("{U}"),
            colored_only: false,
            amount: &DEMILICH_AMT,
        },
        source_zone: ZoneRef::Stack,
        condition: None,
    })];
    static HYDRA_AMT: Amount = Amount::CountOf {
        filter: &Filter::YOUR_CREATURE,
        zone: ZoneSel::Battlefield,
    };
    static HYDRA_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::This,
        layer: Layer::Text,
        modifier: Modifier::SpellsCostLessManaByAmount {
            who: PlayerRel::You,
            filter: &Filter::This,
            unit: mana!("{G}"),
            colored_only: true,
            amount: &HYDRA_AMT,
        },
        source_zone: ZoneRef::Stack,
        condition: None,
    })];
    static CANTRIP_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];
    static BARGAIN_GIANT_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::This,
        layer: Layer::Text,
        modifier: Modifier::AdjustSpellCostByCast {
            who: PlayerRel::You,
            filter: &Filter::This,
            delta_per_unit: -2,
            metric: SpellCastMetric::Bargained,
        },
        source_zone: ZoneRef::Stack,
        condition: None,
    })];
    static THAUMATURGE_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::AdjustSpellCostByCast {
            who: PlayerRel::You,
            filter: &INSTANT_OR_SORCERY,
            delta_per_unit: -1,
            metric: SpellCastMetric::DistinctTargetsMatching(&Filter::CREATURE),
        }
    )];
    static TWIN_PING_AB: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq {
            min: 1,
            ..TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 2)
        })
    )];

    let demilich = card(
        C0,
        creature_face("Demilich Probe", "{2}{U}{U}", 4, 3),
        KeywordSet::EMPTY,
        DEMILICH_AB,
    );
    let hydra = card(
        C1,
        creature_face("Hydra Probe", "{2}{G}{G}", 5, 5),
        KeywordSet::EMPTY,
        HYDRA_AB,
    );
    let cantrip = card(
        C2,
        face("Zero Ritual", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        CANTRIP_AB,
    );
    let bargain_giant = card(
        C3,
        FaceDef {
            bargain: true,
            ..creature_face("Hamlet Glutton Probe", "{4}{G}", 6, 6)
        },
        KeywordSet::EMPTY,
        BARGAIN_GIANT_AB,
    );
    let relic = card(
        C4,
        face("Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let thaumaturge = card(
        C5,
        creature_face("Thaumaturge Probe", "{1}{U}", 2, 4),
        KeywordSet::EMPTY,
        THAUMATURGE_AB,
    );
    let twin_ping = card(
        C6,
        face("Twin Ping", "{2}{R}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        TWIN_PING_AB,
    );
    let squire = card(
        C7,
        creature_face("Squire", "{W}", 1, 4),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        17_301,
        vec![
            demilich,
            hydra,
            cantrip,
            bargain_giant,
            relic,
            thaumaturge,
            twin_ping,
            squire,
        ],
        [
            Seat::with(&[C4, C5, C7]).holding(&[C0, C1, C2, C2, C2, C3, C3, C6]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    // Demilich ({2}{U}{U}) before any instant/sorcery is NOT castable on {R}.
    let demilich_hand = the(&engine, ZoneLocation::Hand(me()), C0);
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    assert!(
        !offer(&engine).1.castable.contains(&demilich_hand),
        "Demilich requires {{2}}{{U}}{{U}} when 0 instants/sorceries were cast"
    );

    // Cast 3 zero-mana instants -> reduction is {U}{U}{U}: removes {U}{U} AND spills {1} generic -> {1} left!
    let cantrips = objects(&engine, ZoneLocation::Hand(me()), C2);
    assert_eq!(cantrips.len(), 3);
    for c in cantrips {
        engine
            .apply(me(), PlayerAction::CastSpell { card: c })
            .expect("cast cantrip");
        settle(&mut engine, me());
    }
    assert!(
        offer(&engine).1.castable.contains(&demilich_hand),
        "Demilich reduced to {{1}} generic (payable with floating {{R}})"
    );
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: demilich_hand,
            },
        )
        .expect("cast Demilich for {1} paid by {R}");
    settle(&mut engine, me());
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, C0).len(), 1);

    // Now we control 3 creatures (Thaumaturge, Squire, Demilich) -> Hydra ({2}{G}{G}) gets 3x {G}
    // with `colored_only: true`, removing {G}{G} ONLY and leaving {2} generic untouched!
    let hydra_hand = the(&engine, ZoneLocation::Hand(me()), C1);
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    assert!(
        !offer(&engine).1.castable.contains(&hydra_hand),
        "colored_only: true must NOT spill the 3rd {{G}} reduction onto {{2}} generic"
    );
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    assert!(
        offer(&engine).1.castable.contains(&hydra_hand),
        "Hydra casts for {{2}} generic with zero green mana"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: hydra_hand })
        .expect("cast Hydra for {2}");
    settle(&mut engine, me());
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, C1).len(), 1);

    // 2. Bargain reduction: Giant ({4}{G}) with {2}{G} floating + 1 Relic on battlefield.
    let giants = objects(&engine, ZoneLocation::Hand(me()), C3);
    let relic_bf = the(&engine, ZoneLocation::Battlefield, C4);
    add_pool(&mut engine, me(), ManaColor::Green, 3);
    assert!(
        offer(&engine).1.castable.contains(&giants[0]),
        "Bargain Giant castable for {{2}}{{G}} when an artifact can be bargained"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: giants[0] })
        .expect("announce Bargain Giant");
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine
            .apply(me(), PlayerAction::YesNo(true))
            .expect("opt into Bargain");
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![relic_bf],
            },
        )
        .expect("sacrifice Relic for Bargain");
    settle(&mut engine, me());
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, C3).len(), 1);

    // With Relic gone (no artifact/enchantment/token left), the 2nd Giant is NOT castable on {2}{G}!
    add_pool(&mut engine, me(), ManaColor::Green, 3);
    assert!(
        !offer(&engine).1.castable.contains(&giants[1]),
        "without bargain fodder, Giant costs full {{4}}{{G}}"
    );
    engine.state.players[0].mana_pool = baylee_core::mana::ManaPool::new();
    refresh(&mut engine);

    // 3. Battlefield Thaumaturge (`DistinctTargetsMatching(&Filter::CREATURE)`):
    //    Twin Ping ({2}{R}, 1..=2 creature targets) with ONLY {R} floating requires 2 distinct targets!
    let ping_hand = the(&engine, ZoneLocation::Hand(me()), C6);
    let th_bf = the(&engine, ZoneLocation::Battlefield, C5);
    let sq_bf = the(&engine, ZoneLocation::Battlefield, C7);
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    assert!(
        offer(&engine).1.castable.contains(&ping_hand),
        "Twin Ping offered on {{R}} because choosing 2 creature targets reduces it by {{2}}"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: ping_hand })
        .expect("announce Twin Ping");
    let _ = engine.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![th_bf],
        },
    );
    assert_eq!(
        engine.state().object(ping_hand).unwrap().zone,
        crate::zone::Zone::Hand,
        "choosing only 1 creature target leaves cost at {{1}}{{R}}, reversing cast under CR 734.1"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: ping_hand })
        .expect("re-announce Twin Ping");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![th_bf, sq_bf],
            },
        )
        .expect("choosing 2 distinct creature targets reduces cost to {R}");
    settle(&mut engine, me());
    assert_eq!(engine.state().object(th_bf).unwrap().damage, 1);
    assert_eq!(engine.state().object(sq_bf).unwrap().damage, 1);
}

#[test]
fn wave17c_foretell_unlock_and_equip_if_targets_reductions() {
    static CHARGER_AB: &[AbilityDef] = &[
        static_ability!(
            Filter::Any,
            Modifier::MayForetellAnyTurn {
                who: PlayerRel::You
            }
        ),
        static_ability!(
            Filter::Any,
            Modifier::ForetellActionCostsLess {
                who: PlayerRel::You,
                amount: 1,
                first_each_turn_only: false,
            }
        ),
    ];
    static RANAR_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::ForetellActionCostsLess {
            who: PlayerRel::You,
            amount: 2,
            first_each_turn_only: true,
        }
    )];
    static GLIMMER_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::UnlockCostsLess {
            who: PlayerRel::You,
            amount: 1,
        }
    )];
    static CHAMPION_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::EquipCostsLessIfTargets {
            who: PlayerRel::You,
            target_filter: &Filter::This,
            amount: 3,
        }
    )];
    static SWORD_AB: &[AbilityDef] = &[equip!("{3}")];
    static ROOM_SUBS: &[baylee_core::ids::SubtypeId] = &[enchantment::ROOM];
    static EQUIP_SUBS: &[baylee_core::ids::SubtypeId] = &[artifact::EQUIPMENT];

    static FORETELL_SPELL_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(2),
    }])];
    let foretell_card = card(
        C0,
        FaceDef {
            foretell: Some(ManaCost::parse("{U}")),
            ..face("Saw It Coming Probe", "{2}{U}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        FORETELL_SPELL_AB,
    );
    let charger = card(
        C1,
        creature_face("Cosmos Charger Probe", "{3}{U}", 3, 3),
        KeywordSet::EMPTY,
        CHARGER_AB,
    );
    let ranar = card(
        C2,
        creature_face("Ranar Probe", "{2}{W}{U}", 2, 3),
        KeywordSet::EMPTY,
        RANAR_AB,
    );
    let room_card: &'static CardDef = Box::leak(Box::new(CardDef {
        index: CardIndex::new(C3),
        oracle_id: "room-test",
        scryfall_id: "room-test",
        faces: Box::leak(Box::new([
            FaceDef {
                subtypes: ROOM_SUBS,
                ..face("Left Door", "{2}{W}", TypeSet::ENCHANTMENT)
            },
            FaceDef {
                subtypes: ROOM_SUBS,
                ..face("Right Door", "{3}{U}", TypeSet::ENCHANTMENT)
            },
        ])),
        coverage: Coverage::Implemented,
        ..CardDef::DEFAULT
    }));
    let glimmer = card(
        C4,
        creature_face("Inquisitive Glimmer Probe", "{W}{U}", 2, 3),
        KeywordSet::EMPTY,
        GLIMMER_AB,
    );
    let champion = card(
        C5,
        creature_face("Fervent Champion Probe", "{R}", 1, 1),
        KeywordSet::EMPTY,
        CHAMPION_AB,
    );
    let squire = card(
        C6,
        creature_face("Plain Squire", "{W}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let sword = card(
        C7,
        FaceDef {
            subtypes: EQUIP_SUBS,
            ..face("Heavy Greatsword", "{1}", TypeSet::ARTIFACT)
        },
        KeywordSet::EMPTY,
        SWORD_AB,
    );

    // 1. Ranar on our turn: 1st foretell is {0}, 2nd foretell on the same turn costs {2}.
    let mut engine = bench(
        17_302,
        vec![
            foretell_card,
            charger,
            ranar,
            room_card,
            glimmer,
            champion,
            squire,
            sword,
        ],
        [
            Seat::with(&[C2, C3, C4, C5, C6, C7]).holding(&[C0, C0]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let ft_cards = objects(&engine, ZoneLocation::Hand(me()), C0);
    assert_eq!(ft_cards.len(), 2);
    assert!(
        offer(&engine).1.foretellable.contains(&ft_cards[0]),
        "Ranar makes the 1st foretell of the turn cost {{0}}"
    );
    engine
        .apply(me(), PlayerAction::Foretell { card: ft_cards[0] })
        .expect("1st foretell for {0}");
    assert_eq!(
        engine.state().object(ft_cards[0]).unwrap().zone,
        crate::zone::Zone::Exile
    );
    assert!(
        !offer(&engine).1.foretellable.contains(&ft_cards[1]),
        "Ranar discount applies only to the 1st foretell each turn"
    );

    // 2. Inquisitive Glimmer: Room doors ({2}{W} and {3}{U}) cost {1} less ({1}{W} and {2}{U}).
    let room_bf = the(&engine, ZoneLocation::Battlefield, C3);
    assert!(
        offer(&engine).1.unpaid_abilities.contains(&(
            room_bf,
            unlock_door(0),
            ManaCost::parse("{1}{W}"),
        )),
        "Glimmer reduces Left Door unlock cost from {{2}}{{W}} to {{1}}{{W}}"
    );
    add_pool(&mut engine, me(), ManaColor::White, 2);
    activate(&mut engine, me(), room_bf, unlock_door(0));
    assert!(
        engine.state().object(room_bf).unwrap().doors.is_unlocked(0),
        "Left Door unlocked for {{1}}{{W}}"
    );

    // 3. Fervent Champion: Equip {3} reverses under CR 734.1 when targeting Squire ({3}) on empty pool,
    //    and succeeds for {0} when targeting Fervent Champion!
    let sword_bf = the(&engine, ZoneLocation::Battlefield, C7);
    let champ_bf = the(&engine, ZoneLocation::Battlefield, C5);
    let squire_bf = the(&engine, ZoneLocation::Battlefield, C6);
    activate(&mut engine, me(), sword_bf, 0);
    let _ = engine.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![squire_bf],
        },
    );
    assert!(
        engine.state().zones.stack_is_empty()
            && engine
                .state()
                .object(sword_bf)
                .unwrap()
                .attached_to
                .is_none(),
        "targeting Squire ({{3}}) with empty mana pool must reverse under CR 734.1"
    );
    activate(&mut engine, me(), sword_bf, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![champ_bf],
            },
        )
        .expect("target Fervent Champion for {0}");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(sword_bf).unwrap().attached_to,
        Some(champ_bf)
    );

    // 4. Cosmos Charger: foretell on opponent's turn for {1} instead of {2}!
    let mut engine2 = bench(
        17_303,
        vec![foretell_card, charger],
        [Seat::with(&[C1]).holding(&[C0]), Seat::default()],
    );
    walk_until(&mut engine2, |e| {
        e.state().turn.active == them()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    let ft_opp = the(&engine2, ZoneLocation::Hand(me()), C0);
    assert!(
        !offer(&engine2).1.foretellable.contains(&ft_opp),
        "Charger reduces {{2}} to {{1}}, not {{0}}"
    );
    add_pool(&mut engine2, me(), ManaColor::Blue, 1);
    assert!(
        offer(&engine2).1.foretellable.contains(&ft_opp),
        "Cosmos Charger lets you foretell on opponent's turn for {{1}}"
    );
    engine2
        .apply(me(), PlayerAction::Foretell { card: ft_opp })
        .expect("foretell on opponent's turn for {1}");
    assert_eq!(
        engine2.state().object(ft_opp).unwrap().zone,
        crate::zone::Zone::Exile
    );
}

#[test]
fn wave17c_behold_exile_loyalty_and_target_or_ability_taxes() {
    static KITHKIN_SUBS: &[baylee_core::ids::SubtypeId] = &[creature::KITHKIN];
    static DRAGON_SUBS: &[baylee_core::ids::SubtypeId] = &[creature::DRAGON];
    static KITHKIN_FILTER: Filter = Filter::HasSubtype(creature::KITHKIN);
    static DRAGON_FILTER: Filter = Filter::HasSubtype(creature::DRAGON);
    static BEHOLD_EXILE_PARTS: &[CostPart] = &[CostPart::BeholdAndExile(&KITHKIN_FILTER)];
    static CLACHAN_AB: &[AbilityDef] = &[
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::ReturnLinkedExile { to: ZoneRef::Hand }]
        ),
        free(&[Effect::SacrificeSelf], None),
    ];
    static WALKER_AB: &[AbilityDef] = &[
        AbilityDef::Loyalty {
            cost: -3,
            effects: &[Effect::GainLife {
                amount: Amount::Fixed(4),
            }],
            targets: None,
            second_targets: None,
            third_targets: None,
        },
        AbilityDef::Loyalty {
            cost: 1,
            effects: &[Effect::GainLife {
                amount: Amount::Fixed(1),
            }],
            targets: None,
            second_targets: None,
            third_targets: None,
        },
    ];
    static CARTH_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::AdjustLoyaltyCost {
            who: PlayerRel::You,
            filter: &Filter::PLANESWALKER,
            delta: 1,
        }
    )];
    static EIDOLON_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::LoyaltyAbilitiesCostMore {
            who: PlayerRel::You,
            filter: &Filter::PLANESWALKER,
            amount: 1,
        }
    )];
    static SUPPRESSION_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::NonManaAbilitiesCostMore {
            who: PlayerRel::EachPlayer,
            filter: &Filter::Any,
            amount: 2,
        }
    )];
    static DORK_AB: &[AbilityDef] = &[
        mana_ability!(Cost::TAP, &[Effect::mana(ManaColor::Green, 1)]),
        free(
            &[Effect::GainLife {
                amount: Amount::Fixed(1),
            }],
            None,
        ),
    ];
    static DRAGONS_PREY_AB: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            filter: Filter::This,
            layer: Layer::Text,
            modifier: Modifier::RaiseCostIfTargets {
                who: PlayerRel::You,
                spell_filter: &Filter::This,
                target_filter: &DRAGON_FILTER,
                extra_mana: mana!("{2}"),
                extra_life: 0,
                abilities: false,
            },
            source_zone: ZoneRef::Stack,
            condition: None,
        }),
        spell!(
            &[Effect::Destroy {
                target: TargetSpec::Object(&Filter::CREATURE),
                no_regen: false,
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
    ];
    static TERROR_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::RaiseCostIfTargets {
            who: PlayerRel::EachOpponent,
            spell_filter: &Filter::Any,
            target_filter: &Filter::This,
            extra_mana: ManaCost::ZERO,
            extra_life: 3,
            abilities: false,
        }
    )];

    let clachan = card(
        C0,
        FaceDef {
            subtypes: KITHKIN_SUBS,
            mandatory_additional_costs: BEHOLD_EXILE_PARTS,
            ..creature_face("Champion of the Clachan Probe", "{1}{W}", 4, 5)
        },
        KeywordSet::EMPTY,
        CLACHAN_AB,
    );
    let kithkin = card(
        C1,
        FaceDef {
            subtypes: KITHKIN_SUBS,
            ..creature_face("Wizened Cenn", "{W}", 2, 2)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let walker = card(
        C2,
        FaceDef {
            loyalty: Some(2),
            ..face("Test Walker", "{2}{B}", TypeSet::PLANESWALKER)
        },
        KeywordSet::EMPTY,
        WALKER_AB,
    );
    let carth = card(
        C3,
        creature_face("Carth Probe", "{2}{B}{G}", 3, 5),
        KeywordSet::EMPTY,
        CARTH_AB,
    );
    let eidolon = card(
        C4,
        creature_face("Eidolon Probe", "{1}{W}", 2, 1),
        KeywordSet::EMPTY,
        EIDOLON_AB,
    );
    let suppression = card(
        C5,
        face("Suppression Field Probe", "{1}{W}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        SUPPRESSION_AB,
    );
    let dork = card(
        C6,
        face("Mana Relic", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        DORK_AB,
    );
    let dragons_prey = card(
        C7,
        FaceDef {
            subtypes: &[],
            ..face("Dragons Prey Probe", "{1}{B}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        DRAGONS_PREY_AB,
    );

    // 1. Champion of the Clachan (`BeholdAndExile` from hand, then from battlefield, plus LTB return).
    let mut engine = bench(
        17_304,
        vec![clachan, kithkin],
        [Seat::default().holding(&[C0, C0, C1]), Seat::default()],
    );
    to_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::White, 2);
    let clachans = objects(&engine, ZoneLocation::Hand(me()), C0);
    let kithkin_hand = the(&engine, ZoneLocation::Hand(me()), C1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: clachans[0] })
        .expect("cast Clachan #1");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![kithkin_hand],
            },
        )
        .expect("exile Kithkin from hand");
    settle(&mut engine, me());
    assert_eq!(objects(&engine, ZoneLocation::Exile(me()), C1).len(), 1);
    let clachan_bf = the(&engine, ZoneLocation::Battlefield, C0);

    // Clachan #2 in hand can behold-and-exile Clachan #1 from the battlefield!
    add_pool(&mut engine, me(), ManaColor::White, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: clachans[1] })
        .expect("cast Clachan #2");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![clachan_bf],
            },
        )
        .expect("exile Clachan #1 from battlefield");
    // Clachan #1's LTB trigger returns Kithkin from exile to hand, and Clachan #2 enters!
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Hand(me()), C1).len(),
        1,
        "Clachan #1 leaving battlefield returned its exiled Kithkin to hand"
    );
    let clachan2_bf = the(&engine, ZoneLocation::Battlefield, C0);
    activate(&mut engine, me(), clachan2_bf, 1);
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Hand(me()), C0).len(),
        1,
        "Clachan #2 leaving battlefield returned exiled Clachan #1 to hand"
    );

    // 2. Carth (`[+1]` loyalty cost -> `[-3]` becomes `[-2]`, usable at 2 loyalty) + Eidolon (`{1}` tax)
    //    + Suppression Field (spares mana abilities, taxes non-mana abilities by `{2}`).
    let mut engine2 = bench(
        17_305,
        vec![walker, carth, eidolon, suppression, dork],
        [
            Seat::with(&[C3, C4, C5, C6]).holding(&[C2]),
            Seat::default(),
        ],
    );
    to_main(&mut engine2, me());
    add_pool(&mut engine2, me(), ManaColor::Black, 3);
    let walker_hand = the(&engine2, ZoneLocation::Hand(me()), C2);
    engine2
        .apply(me(), PlayerAction::CastSpell { card: walker_hand })
        .expect("cast Test Walker so it enters with 2 loyalty counters");
    settle(&mut engine2, me());
    let walker_bf = the(&engine2, ZoneLocation::Battlefield, C2);
    let dork_bf = the(&engine2, ZoneLocation::Battlefield, C6);

    // Dork's mana ability (index 0) is FREE despite Suppression Field (`CR 605.1`), while index 1 costs `{2}`!
    let (_, legal2) = offer(&engine2);
    assert!(
        legal2.abilities.contains(&(dork_bf, 0)),
        "Suppression Field never taxes mana abilities"
    );
    assert!(
        !legal2.abilities.contains(&(dork_bf, 1)),
        "Suppression Field taxes non-mana ability {{0}} -> {{2}}"
    );
    // Loyalty ability 0 (`[-3]` adjusted by Carth to `[-2]`) is taxed by BOTH Eidolon (`{1}`) and
    // Suppression Field (`{2}`) = `{3}` total mana!
    assert!(
        legal2
            .unpaid_abilities
            .contains(&(walker_bf, 0, ManaCost::parse("{3}"))),
        "Carth unlocks [-3] at 2 loyalty (as [-2]), taxed to {{3}} by Eidolon + Suppression Field"
    );
    add_pool(&mut engine2, me(), ManaColor::Black, 3);
    activate(&mut engine2, me(), walker_bf, 0);
    settle(&mut engine2, me());
    assert_eq!(life(&engine2, me()), 24);

    // 3. Dragon's Prey (`{2}` extra mana targeting Dragon) & Terror of the Peaks (`3` extra life).
    let terror = card(
        C4,
        FaceDef {
            subtypes: DRAGON_SUBS,
            ..creature_face("Terror Probe", "{3}{R}{R}", 5, 4)
        },
        KeywordSet::EMPTY,
        TERROR_AB,
    );
    let mut engine3 = bench(
        17_306,
        vec![kithkin, terror, dragons_prey],
        [Seat::with(&[C1]).holding(&[C7]), Seat::with(&[C4]).at(20)],
    );
    to_main(&mut engine3, me());
    let kith_bf = the(&engine3, ZoneLocation::Battlefield, C1);
    let terror_bf = the(&engine3, ZoneLocation::Battlefield, C4);
    let prey_hand = the(&engine3, ZoneLocation::Hand(me()), C7);

    // With {1}{B} floating + 20 life, targeting Terror costs {3}{B} + 3 life -> reverses under CR 734.1!
    add_pool(&mut engine3, me(), ManaColor::Black, 2);
    engine3
        .apply(me(), PlayerAction::CastSpell { card: prey_hand })
        .expect("cast Dragon's Prey");
    let _ = engine3.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![terror_bf],
        },
    );
    assert_eq!(
        engine3.state().object(prey_hand).unwrap().zone,
        crate::zone::Zone::Hand,
        "targeting Terror Dragon on {{1}}{{B}} must reverse under CR 734.1"
    );
    assert_eq!(life(&engine3, me()), 20, "reversed cast pays no life");
    let _ = kith_bf;

    // Add {2} more mana -> now targeting Terror IS legal and charges {3}{B} + 3 life!
    add_pool(&mut engine3, me(), ManaColor::Black, 2);
    engine3
        .apply(me(), PlayerAction::CastSpell { card: prey_hand })
        .expect("cast Dragon's Prey with {3}{B}");
    engine3
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![terror_bf],
            },
        )
        .expect("target Terror Dragon paying {3}{B} + 3 life");
    settle(&mut engine3, me());
    assert_eq!(life(&engine3, me()), 17, "paid 3 life to Terror tax");
    assert!(objects(&engine3, ZoneLocation::Battlefield, C4).is_empty());
}

#[test]
fn wave17c_graveyard_grants_surveil_play_and_yixlid_jailer() {
    static YOUR_GY_CREATURE: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::OwnedByYou,
        Filter::InZone(ZoneRef::Graveyard),
    ]);
    static YOUR_GY_LAND: Filter = Filter::And(&[
        Filter::LAND,
        Filter::OwnedByYou,
        Filter::InZone(ZoneRef::Graveyard),
    ]);
    static SURVEILLED_GY: Filter = Filter::And(&[
        Filter::SurveilledByYouThisTurn,
        Filter::OwnedByYou,
        Filter::InZone(ZoneRef::Graveyard),
    ]);
    static GRANTOR_AB: &[AbilityDef] = &[
        static_ability!(YOUR_GY_CREATURE, Modifier::GrantsUnearth(mana!("{1}{B}"))),
        static_ability!(YOUR_GY_CREATURE, Modifier::GrantsScavengeManaCost),
        static_ability!(YOUR_GY_LAND, Modifier::GrantsDredge(2)),
        free(&[Effect::SacrificeSelf], None),
    ];
    static JAILER_AB: &[AbilityDef] = &[static_ability!(
        Filter::InZone(ZoneRef::Graveyard),
        Modifier::LoseAllAbilities
    )];
    static EYE_AB: &[AbilityDef] = &[
        static_ability!(
            SURVEILLED_GY,
            Modifier::MayCastWithAltCost {
                who: PlayerRel::You,
                zones: AltCastZones::Zones(&[ZoneRef::Graveyard]),
                grant_zone_permission: true,
                allow_lands: true,
                with_flash: false,
                max_mana_value: None,
                once_per_turn: false,
                price: AltCastPrice::PayLifeEqualToManaValue,
            }
        ),
        free(
            &[Effect::Surveil {
                amount: Amount::Fixed(1),
            }],
            None,
        ),
        free(
            &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }],
            None,
        ),
    ];
    static UNDERDOG_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::This,
        layer: Layer::Ability,
        modifier: Modifier::MayCastFromGraveyardUsingAlt(GraveyardAltKind::Blitz),
        source_zone: ZoneRef::Graveyard,
        condition: None,
    })];
    static CULLER_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::This,
        layer: Layer::Ability,
        modifier: Modifier::MayCastFromGraveyardUsingAlt(GraveyardAltKind::Warp),
        source_zone: ZoneRef::Graveyard,
        condition: None,
    })];

    let grantor = card(
        C0,
        creature_face("Sedris-Varolz-Bloom", "{3}{B}{G}", 3, 3),
        KeywordSet::EMPTY,
        GRANTOR_AB,
    );
    let dead_brute = card(
        C1,
        creature_face("Dead Brute", "{1}{G}", 3, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let dead_bear = card(
        C2,
        creature_face("Dead Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let jailer = card(
        C3,
        creature_face("Yixlid Jailer Probe", "{1}{B}", 2, 1),
        KeywordSet::EMPTY,
        JAILER_AB,
    );
    let eye = card(
        C4,
        creature_face("Eye of Duskmantle Probe", "{5}{U}{B}", 3, 8),
        KeywordSet::EMPTY,
        EYE_AB,
    );
    static WISDOM_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(5),
    }])];
    let wisdom = card(
        C5,
        face("Dark Wisdom", "{2}{U}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        WISDOM_AB,
    );
    let underdog = card(
        C6,
        FaceDef {
            blitz: Some(cost!("{1}{B}", PayLife(2))),
            ..creature_face("Tenacious Underdog Probe", "{1}{B}", 3, 2)
        },
        KeywordSet::EMPTY,
        UNDERDOG_AB,
    );
    let culler = card(
        C7,
        FaceDef {
            warp: Some(cost!("{B}")),
            ..creature_face("Timeline Culler Probe", "{B}{B}", 2, 2)
        },
        KeywordSet::EMPTY,
        CULLER_AB,
    );

    let mut engine = bench(
        17_307,
        vec![
            grantor, dead_brute, dead_bear, jailer, eye, wisdom, underdog, culler,
        ],
        [
            Seat::with(&[C0, C4]).holding(&[C1, C2, C3, C5, C6, C7]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let grantor_bf = the(&engine, ZoneLocation::Battlefield, C0);
    let eye_bf = the(&engine, ZoneLocation::Battlefield, C4);
    let brute_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let bear_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let wisdom_id = the(&engine, ZoneLocation::Hand(me()), C5);
    let underdog_id = the(&engine, ZoneLocation::Hand(me()), C6);
    let culler_id = the(&engine, ZoneLocation::Hand(me()), C7);
    move_to_graveyard(&mut engine, me(), brute_id);
    move_to_graveyard(&mut engine, me(), bear_id);
    move_to_graveyard(&mut engine, me(), underdog_id);
    move_to_graveyard(&mut engine, me(), culler_id);

    // 1. GrantsScavengeManaCost: Scavenge Dead Brute ({1}{G}, power 3) onto Eye of Duskmantle!
    add_pool(&mut engine, me(), ManaColor::Green, 2);
    let scavenge_slot = offer(&engine)
        .1
        .abilities
        .iter()
        .copied()
        .find(|&(src, _)| src == brute_id)
        .expect("Scavenge offered on Dead Brute for {1}{G}")
        .1;
    activate(&mut engine, me(), brute_id, scavenge_slot);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![eye_bf],
            },
        )
        .expect("target Eye for Scavenge");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(brute_id).unwrap().zone,
        crate::zone::Zone::Exile
    );
    assert_eq!(pt_of(&engine, eye_bf), (6, 11));

    // 1b. GrantsDredge(2): move a Forest from library to graveyard, activate Draw 1 (eye_bf index 2)
    //     -> Dredge 2 is offered for the graveyard land, returning it to hand and milling 2!
    let lib_forest = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .unwrap();
    move_to_graveyard(&mut engine, me(), lib_forest);
    activate(&mut engine, me(), eye_bf, 2); // Draw 1
    while !matches!(engine.pending(), Pending::ChooseCards { .. }) {
        let p = engine.pending().clone();
        assert!(walk_past(&mut engine, &p), "unexpected {p:?}");
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![lib_forest],
            },
        )
        .expect("choose granted Dredge 2 on graveyard Forest");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(lib_forest).unwrap().zone,
        crate::zone::Zone::Hand,
        "Necrobloom GrantsDredge(2) returned graveyard Forest to hand"
    );

    // 2. GrantsUnearth: activate Unearth ({1}{B}) on Dead Bear, then sacrifice Grantor while on stack
    //    -> Dead Bear STILL enters the battlefield with Haste!
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    let unearth_slot = offer(&engine)
        .1
        .abilities
        .iter()
        .copied()
        .find(|&(src, _)| src == bear_id)
        .expect("Unearth offered on Dead Bear for {1}{B}")
        .1;
    activate(&mut engine, me(), bear_id, unearth_slot);
    // Sacrifice Grantor in response (ability index 3)!
    activate(&mut engine, me(), grantor_bf, 3);
    settle(&mut engine, me());
    let bear_bf = the(&engine, ZoneLocation::Battlefield, C2);
    assert!(
        engine
            .state()
            .object(bear_bf)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::HASTE),
        "Unearthed Dead Bear enters with Haste even after Grantor left the battlefield"
    );

    // 3. Eye of Duskmantle: non-surveilled graveyard card is NOT castable; surveilled card casts for MV life!
    move_to_graveyard(&mut engine, me(), wisdom_id);
    assert!(
        !offer(&engine).1.castable.contains(&wisdom_id),
        "card put into graveyard without surveilling is NOT castable via Eye of Duskmantle"
    );
    move_to_top_of_library(&mut engine, me(), wisdom_id);
    activate(&mut engine, me(), eye_bf, 1); // Surveil 1
    while !matches!(engine.pending(), Pending::Arrange { .. }) {
        let p = engine.pending().clone();
        assert!(walk_past(&mut engine, &p));
    }
    let Pending::Arrange {
        piles: arrange_piles,
        cards,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    let piles: Vec<Vec<ObjectId>> = arrange_piles
        .iter()
        .map(|pile| {
            if pile.place == ArrangePlace::Graveyard {
                cards.clone()
            } else {
                Vec::new()
            }
        })
        .collect();
    engine
        .apply(me(), PlayerAction::Arrange { piles })
        .expect("surveil Wisdom into graveyard");
    settle(&mut engine, me());

    assert!(
        offer(&engine).1.castable.contains(&wisdom_id),
        "surveilled card in graveyard is castable via Eye of Duskmantle for MV life (3)"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: wisdom_id })
        .expect("cast surveilled Wisdom paying 3 life");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
            unreachable!();
        };
        let pos = options
            .iter()
            .position(|o| o.kind != CastModeKind::Normal)
            .unwrap_or(0);
        engine
            .apply(me(), PlayerAction::ChooseMode(pos))
            .expect("choose life alt cost");
    }
    assert_eq!(life(&engine, me()), 17, "paid 3 life (= MV) on cast");
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 22, "gained 5 life on resolution");

    // 4. Tenacious Underdog (`Blitz` from graveyard) & Timeline Culler (`Warp` from graveyard),
    //    then Yixlid Jailer shuts both off via `LoseAllAbilities`!
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    assert!(
        offer(&engine).1.castable.contains(&underdog_id),
        "Tenacious Underdog castable from graveyard via Blitz"
    );
    assert!(
        offer(&engine).1.castable.contains(&culler_id),
        "Timeline Culler castable from graveyard via Warp"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: culler_id })
        .expect("cast Timeline Culler from graveyard via Warp");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        choose_cast_mode_kind(&mut engine, me(), CastModeKind::Warp);
    }
    settle(&mut engine, me());
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, C7).len(), 1);

    // Cast Yixlid Jailer ({1}{B}) -> Underdog in graveyard loses `MayCastFromGraveyardUsingAlt(Blitz)`!
    let jailer_hand = the(&engine, ZoneLocation::Hand(me()), C3);
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: jailer_hand })
        .expect("cast Yixlid Jailer");
    settle(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Black, 4);
    assert!(
        !offer(&engine).1.castable.contains(&underdog_id),
        "Yixlid Jailer removes graveyard abilities, shutting off Tenacious Underdog's Blitz"
    );
}

#[test]
fn wave17c_one_or_more_cards_discarded_and_double_pt() {
    static NONLAND_FILTER: Filter = Filter::Not(&Filter::LAND);
    static RIELLE_AB: &[AbilityDef] = &[triggered!(
        Trigger::OneOrMoreCardsDiscarded {
            who: PlayerRel::You,
            filter: &Filter::Any,
            first_this_turn: true,
            per_player: true,
        },
        &[Effect::GainLife {
            amount: Amount::EventAmount,
        }]
    )];
    static VERONICA_AB: &[AbilityDef] = &[triggered!(
        Trigger::OneOrMoreCardsDiscarded {
            who: PlayerRel::You,
            filter: &NONLAND_FILTER,
            first_this_turn: true,
            per_player: true,
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(10),
        }]
    )];
    static PER_PLAYER_WATCHER_AB: &[AbilityDef] = &[triggered!(
        Trigger::OneOrMoreCardsDiscarded {
            who: PlayerRel::EachPlayer,
            filter: &Filter::Any,
            first_this_turn: false,
            per_player: true,
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(0),
        }]
    )];
    static BATCH_WATCHER_AB: &[AbilityDef] = &[triggered!(
        Trigger::OneOrMoreCardsDiscarded {
            who: PlayerRel::EachPlayer,
            filter: &Filter::Any,
            first_this_turn: false,
            per_player: false,
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(0),
        }]
    )];
    static TOLUZ_AB: &[AbilityDef] = &[
        triggered!(
            Trigger::OneOrMoreCardsDiscarded {
                who: PlayerRel::You,
                filter: &Filter::Any,
                first_this_turn: false,
                per_player: true,
            },
            &[Effect::ChooseCardEx {
                chooser: PlayerRel::You,
                zone: ChooseZone::Found,
                filter: &Filter::Any,
                min: 0,
                max: Amount::Fixed(0),
                reveal_chosen: false,
                movement: ChooseMove::None,
                not_chosen_movement: ChooseMove::ExileLinked,
                then: &[],
                for_unchosen: &[],
            }]
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnLinkedExile { to: ZoneRef::Hand }]
        ),
        free(&[Effect::SacrificeSelf], None),
    ];
    static DISCARD_DRIVER_AB: &[AbilityDef] = &[
        free(
            &[Effect::DiscardForPlayers {
                who: PlayerRel::You,
                count: 1,
            }],
            None,
        ),
        free(
            &[Effect::DiscardForPlayers {
                who: PlayerRel::You,
                count: 2,
            }],
            None,
        ),
        free(
            &[Effect::DiscardForPlayers {
                who: PlayerRel::EachPlayer,
                count: 1,
            }],
            None,
        ),
        free(
            &[
                Effect::continuous(
                    &Filter::YOUR_CREATURE,
                    Modifier::ModifyPT(-4, 0),
                    Duration::UntilEndOfTurn,
                ),
                Effect::DoublePT {
                    subject: DoublePtSubject::Filter(&Filter::YOUR_CREATURE),
                    double_power: true,
                    double_toughness: true,
                    keywords: KeywordSet::VIGILANCE,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
            None,
        ),
        free(
            &[Effect::DoublePT {
                subject: DoublePtSubject::Source,
                double_power: true,
                double_toughness: false,
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            None,
        ),
    ];

    let rielle = card(
        C0,
        creature_face("Rielle Probe", "{1}{U}{R}", 0, 3),
        KeywordSet::EMPTY,
        RIELLE_AB,
    );
    let veronica = card(
        C1,
        creature_face("Veronica Probe", "{2}{R}", 3, 3),
        KeywordSet::EMPTY,
        VERONICA_AB,
    );
    let per_player_w = card(
        C2,
        creature_face("PerPlayer Watcher", "{1}{B}", 1, 3),
        KeywordSet::EMPTY,
        PER_PLAYER_WATCHER_AB,
    );
    let batch_w = card(
        C3,
        creature_face("Batch Watcher", "{1}{B}", 6, 5),
        KeywordSet::EMPTY,
        BATCH_WATCHER_AB,
    );
    let toluz = card(
        C4,
        creature_face("Toluz Probe", "{W}{U}{B}", 3, 1),
        KeywordSet::EMPTY,
        TOLUZ_AB,
    );
    let driver = card(
        C5,
        creature_face("Discard-Double Driver", "{2}{G}", 7, 4),
        KeywordSet::EMPTY,
        DISCARD_DRIVER_AB,
    );
    let waste_land = card(C6, land_face("Barren Waste"), KeywordSet::EMPTY, &[]);
    let bear = card(
        C7,
        creature_face("Grizzly", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        17_308,
        vec![
            rielle,
            veronica,
            per_player_w,
            batch_w,
            toluz,
            driver,
            waste_land,
            bear,
        ],
        [
            Seat::with(&[C0, C1, C2, C3, C4, C5]).holding(&[C6, C7, C7, C7]),
            Seat::default().holding(&[C7]),
        ],
    );
    to_main(&mut engine, me());

    let driver_bf = the(&engine, ZoneLocation::Battlefield, C5);
    let per_p_bf = the(&engine, ZoneLocation::Battlefield, C2);
    let batch_bf = the(&engine, ZoneLocation::Battlefield, C3);
    let toluz_bf = the(&engine, ZoneLocation::Battlefield, C4);

    // 1. Discard 1 Land (`waste_land`) first:
    //    - Rielle (`first_this_turn: true`, `&Filter::Any`) triggers for +1 life (`EventAmount == 1`).
    //    - Veronica (`first_this_turn: true`, `&NONLAND_FILTER`) does NOT trigger AND keeps her
    //      first-time-this-turn slot!
    let land_hand = the(&engine, ZoneLocation::Hand(me()), C6);
    activate(&mut engine, me(), driver_bf, 0);
    while !matches!(
        engine.pending(),
        Pending::DiscardChoice { .. } | Pending::ChooseCards { .. }
    ) {
        let p = engine.pending().clone();
        if let Pending::Arrange { player, cards, .. } = p {
            engine
                .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                .unwrap();
        } else {
            assert!(walk_past(&mut engine, &p), "unexpected {p:?}");
        }
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![land_hand],
            },
        )
        .expect("discard land");
    while !engine.state().zones.stack_is_empty()
        || !matches!(engine.pending(), Pending::Priority { player, .. } if *player == me())
    {
        let p = engine.pending().clone();
        if let Pending::Arrange { player, cards, .. } = p {
            engine
                .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                .unwrap();
        } else {
            assert!(walk_past(&mut engine, &p), "unexpected {p:?}");
        }
    }
    assert_eq!(
        life(&engine, me()),
        21,
        "Rielle gained +1 life from land discard; Veronica did not trigger"
    );

    // 2. Discard 2 nonland Bears simultaneously (`count: 2`):
    //    - Rielle does NOT trigger again (`first_this_turn` already used).
    //    - Veronica triggers ONCE (batching both nonland Bears into a single trigger -> +10 life)!
    //    - Toluz exiles BOTH Bears into linked exile!
    let bears_hand = objects(&engine, ZoneLocation::Hand(me()), C7);
    activate(&mut engine, me(), driver_bf, 1);
    while !matches!(
        engine.pending(),
        Pending::DiscardChoice { .. } | Pending::ChooseCards { .. }
    ) {
        let p = engine.pending().clone();
        assert!(walk_past(&mut engine, &p), "unexpected {p:?}");
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bears_hand[0], bears_hand[1]],
            },
        )
        .expect("discard 2 Bears simultaneously");
    while !engine.state().zones.stack_is_empty()
        || !matches!(engine.pending(), Pending::Priority { player, .. } if *player == me())
    {
        let p = engine.pending().clone();
        if let Pending::Arrange { player, cards, .. } = p {
            engine
                .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                .unwrap();
        } else {
            assert!(walk_past(&mut engine, &p), "unexpected {p:?}");
        }
    }
    assert_eq!(
        life(&engine, me()),
        31,
        "Veronica triggered exactly once (+10 life) on simultaneous 2-card nonland discard"
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Exile(me()), C7).len(),
        2,
        "Toluz exiled both discarded Bears"
    );

    // 3. Simultaneous multi-player discard (`EachPlayer`, 1 card each):
    //    `per_player: true` triggers TWICE, `per_player: false` triggers ONCE!
    let p_before = triggered(&engine, per_p_bf);
    let b_before = triggered(&engine, batch_bf);
    activate(&mut engine, me(), driver_bf, 2);
    while !engine.state().zones.stack_is_empty()
        || !matches!(engine.pending(), Pending::Priority { player, .. } if *player == me())
    {
        match engine.pending().clone() {
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .unwrap();
            }
            Pending::DiscardChoice { player, count } => {
                let h = engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(player))
                    .clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: h[..usize::from(count)].to_vec(),
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: options[..usize::from(min)].to_vec(),
                        },
                    )
                    .unwrap();
            }
            p => assert!(walk_past(&mut engine, &p), "unexpected {p:?}"),
        }
    }
    assert_eq!(
        triggered(&engine, per_p_bf) - p_before,
        2,
        "per_player: true fires once per discarding player (2)"
    );
    assert_eq!(
        triggered(&engine, batch_bf) - b_before,
        1,
        "per_player: false batches simultaneous multi-player discard into 1 trigger"
    );

    // Sacrifice Toluz -> dies trigger returns all linked exiled cards (land + 3 Bears = 4 cards) to hand!
    activate(&mut engine, me(), toluz_bf, 2);
    settle(&mut engine, me());
    assert_eq!(objects(&engine, ZoneLocation::Hand(me()), C6).len(), 1);
    assert_eq!(objects(&engine, ZoneLocation::Hand(me()), C7).len(), 3);

    // 4. Effect::DoublePT:
    //    - `driver_bf` (7/4) gets `-4/-0` -> `3/4`, then `DoublePT` -> `6/8` with Vigilance!
    //    - `rielle_bf` (0/3) gets `-4/-0` -> `-4/3`, then `DoublePT` (`CR 701.10b` negative power!) -> `-8/6`!
    //    - Then `DoublePtSubject::Source` on `driver_bf` (`6/8`, power only) -> `12/8`!
    let rielle_bf = the(&engine, ZoneLocation::Battlefield, C0);
    activate(&mut engine, me(), driver_bf, 3);
    settle(&mut engine, me());
    assert_eq!(pt_of(&engine, driver_bf), (6, 8));
    assert_eq!(
        pt_of(&engine, rielle_bf),
        (-8, 6),
        "CR 701.10b: doubling negative power (-4) applies -4 for -8"
    );
    assert!(
        engine
            .state()
            .object(driver_bf)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::VIGILANCE)
    );
    activate(&mut engine, me(), driver_bf, 4);
    settle(&mut engine, me());
    assert_eq!(pt_of(&engine, driver_bf), (12, 8));
}
