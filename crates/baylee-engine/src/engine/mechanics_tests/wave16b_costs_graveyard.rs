//! Wave 16b deterministic gameplay tests:
//! 1. Permanent spell mandatory additional costs (`SacrificeX` + `SpellCastMetric::XPaid`,
//!    `ExileFromGraveyard` / `ExilePermanent` `Rider::SpellCostExile` linking on resolution
//!    per `CR 607.2a` / `CR 400.7`).
//! 2. `OptionalCost` extensions: `Blight<N>` (`PutCounterOnOther` `-1/-1`), `Waterbend` on
//!    `FaceDef::optional_cost` (`CR 701.67a`), `SpellCastMetric::OptionalCostPaid` (*Bite Down
//!    on Crime* castability on reduced mana), and APNAP `Effect::PutCountersOnChosenControlled`
//!    (`DB$ Blight`, `CR 101.4`).
//! 3. `CostPart::RevealFromHand`, `CostPart::RevealFromHandX` (*Martyr* cycle),
//!    `CostPart::RevealSelfFromHand` (`Forecast`, `CR 702.57a–b`), Morph reveal turn-face-up,
//!    `Amount::BeheldOrRevealedPower` / `Amount::BeheldOrRevealedManaValue` (`CR 608.2h`), and
//!    `Condition::ControlledSubtypeOnCast` (`CR 601.2h` Tarkir Dragon presence).
//! 4. Graveyard target & static grants (`Modifier::GrantsFlashbackCost`,
//!    `Modifier::GrantsJumpStart`, `Modifier::GrantsEscape`, `Modifier::GrantsEmbalmManaCost`,
//!    `Modifier::GrantsEncoreManaCost`).

#![allow(clippy::too_many_lines, clippy::items_after_statements)]

use super::*;
use crate::choice::{CastModeKind, ChoicePrompt, TURN_FACE_UP};
use crate::event::Cause;
use crate::object::Status;
use crate::turn::Step;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::prelude::{activated, spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, Condition, CostPart, CounterKind, Duration, Filter, ModeCount, Modifier, PlayerRel,
    SpellCastMetric, SpellMode, StaticAbility, StepKind, TargetSpec, Trigger, ZoneRef, cost,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::creature;
use baylee_core::mana::ManaColor;

const C0: u32 = 99_810;
const C1: u32 = 99_811;
const C2: u32 = 99_812;
const C3: u32 = 99_813;
const C4: u32 = 99_814;
const C5: u32 = 99_815;
const C6: u32 = 99_816;
const C7: u32 = 99_817;

static DRAGON_SUBS: &[baylee_core::ids::SubtypeId] = &[creature::DRAGON];

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

fn counters_on(engine: &Bench, id: ObjectId, kind: CounterKind) -> u16 {
    engine.state().object(id).unwrap().counters.get(kind)
}

#[test]
fn wave16b_permanent_spell_mandatory_costs_and_spell_cost_exile_linking() {
    // 1. Dargo-style `SacrificeX` + `SpellCastMetric::XPaid` (`-2` per sacrificed creature):
    //    `{6}{R}` creature cast on just `{R}` by sacrificing 3 creatures!
    static SAC_X_PARTS: &[CostPart] = &[CostPart::SacrificeX(&Filter::YOUR_CREATURE)];
    static DARGO_AB: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::This,
        layer: baylee_cards_dsl::Layer::Text,
        modifier: Modifier::AdjustSpellCostByCast {
            who: PlayerRel::You,
            filter: &Filter::This,
            delta_per_unit: -2,
            metric: SpellCastMetric::XPaid,
        },
        source_zone: ZoneRef::Stack,
        condition: None,
    })];
    let dargo_face = FaceDef {
        mandatory_additional_costs: SAC_X_PARTS,
        ..creature_face("Dargo Probe", "{6}{R}", 7, 5)
    };
    let dargo = card(C0, dargo_face, KeywordSet::EMPTY, DARGO_AB);
    let fodder = card(
        C1,
        creature_face("Fodder", "{0}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        16_201,
        vec![dargo, fodder],
        [Seat::with(&[C1, C1, C1]).holding(&[C0]), Seat::default()],
    );
    to_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Red, 1);

    let dargo_hand = the(&engine, ZoneLocation::Hand(me()), C0);
    let fodders = objects(&engine, ZoneLocation::Battlefield, C1);
    assert_eq!(fodders.len(), 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: dargo_hand })
        .expect("Dargo offered with 3 creatures + {R}");
    engine
        .apply(me(), PlayerAction::ChooseNumber(3))
        .expect("announce X = 3");
    for f in fodders {
        engine
            .apply(me(), PlayerAction::ChooseObjects { objects: vec![f] })
            .expect("sacrifice fodder");
    }
    settle(&mut engine, me());
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, C0).len(), 1);
    assert!(objects(&engine, ZoneLocation::Battlefield, C1).is_empty());

    // 2. Fear of Abduction / Redemptor Dreadnought `SpellCostExile` linking:
    //    Exile a creature you control as a spell cost; when the permanent leaves the
    //    battlefield, `Card.ExiledWithSource` (`Effect::ReturnLinkedExile`) returns that card!
    static EXILE_CREATURE_PARTS: &[CostPart] = &[CostPart::ExilePermanent(&Filter::YOUR_CREATURE)];
    static ABDUCTOR_AB: &[AbilityDef] = &[
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::ReturnLinkedExile { to: ZoneRef::Hand }]
        ),
        free(&[Effect::SacrificeSelf], None),
    ];
    let abductor_face = FaceDef {
        mandatory_additional_costs: EXILE_CREATURE_PARTS,
        ..creature_face("Abductor Probe", "{0}", 5, 5)
    };
    let abductor = card(C2, abductor_face, KeywordSet::EMPTY, ABDUCTOR_AB);

    let mut engine2 = bench(
        16_202,
        vec![abductor, fodder],
        [Seat::with(&[C1]).holding(&[C2]), Seat::default()],
    );
    to_main(&mut engine2, me());
    let exiled_fodder = the(&engine2, ZoneLocation::Battlefield, C1);
    let abductor_hand = the(&engine2, ZoneLocation::Hand(me()), C2);
    engine2
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: abductor_hand,
            },
        )
        .expect("cast abductor");
    engine2
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![exiled_fodder],
            },
        )
        .expect("exile fodder as spell cost");
    settle(&mut engine2, me());

    let abductor_bf = the(&engine2, ZoneLocation::Battlefield, C2);
    assert_eq!(
        crate::eval::public_linked_exile_cards(engine2.state(), abductor_bf),
        vec![exiled_fodder],
        "permanent entering from stack must inherit linked cost exile"
    );

    // Sacrifice Abductor -> LTB trigger returns the cost-exiled Fodder to hand!
    activate(&mut engine2, me(), abductor_bf, 1);
    settle(&mut engine2, me());
    assert!(
        objects(&engine2, ZoneLocation::Hand(me()), C1).contains(&exiled_fodder),
        "LTB trigger must return the card exiled to pay the permanent spell's cost"
    );
}

#[test]
fn wave16b_blight_optional_cost_bite_down_waterbend_and_apnap_blight() {
    // 1. Cinder Strike (`optional_cost` Blight 1): 2 damage unpaid, 4 damage paid.
    static BLIGHT_1_PARTS: &[CostPart] = &[CostPart::PutCounterOnOther {
        filter: &Filter::YOUR_CREATURE,
        kind: CounterKind::M1M1,
        n: 1,
    }];
    static CINDER_AB: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            target: TargetSpec::Object(&Filter::CREATURE),
            amount: Amount::IfCondition {
                condition: &Condition::OptionalCostPaid,
                then: &Amount::Fixed(4),
                otherwise: &Amount::Fixed(2),
            },
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    let cinder = card(
        C0,
        FaceDef {
            optional_cost: Some(Cost {
                mana: ManaCost::ZERO,
                parts: BLIGHT_1_PARTS,
            }),
            ..face("Cinder Probe", "{0}", TypeSet::SORCERY)
        },
        KeywordSet::EMPTY,
        CINDER_AB,
    );
    let bear = card(
        C1,
        creature_face("Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let wall = card(
        C2,
        creature_face("Wall", "{2}{W}", 0, 5),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        16_203,
        vec![cinder, bear, wall],
        [Seat::with(&[C1]).holding(&[C0]), Seat::with(&[C2])],
    );
    to_main(&mut engine, me());
    let my_bear = the(&engine, ZoneLocation::Battlefield, C1);
    let opp_wall = the(&engine, ZoneLocation::Battlefield, C2);
    let cinder_hand = the(&engine, ZoneLocation::Hand(me()), C0);

    engine
        .apply(me(), PlayerAction::CastSpell { card: cinder_hand })
        .expect("cast cinder");
    choose_cast_mode_kind(&mut engine, me(), CastModeKind::OptionalCost);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_wall],
                players: vec![],
            },
        )
        .expect("target wall");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![my_bear],
            },
        )
        .expect("blight my bear");
    settle(&mut engine, me());
    assert_eq!(counters_on(&engine, my_bear, CounterKind::M1M1), 1);
    assert_eq!(engine.state().object(opp_wall).unwrap().damage, 4);

    // 2. Bite Down on Crime (`CollectEvidence(6)` + `SpellCastMetric::OptionalCostPaid` -2):
    //    `{3}{G}` spell castable on `{1}{G}` ONLY when graveyard has MV >= 6!
    static EVIDENCE_6_PARTS: &[CostPart] = &[CostPart::CollectEvidence(6)];
    static BITE_AB: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            filter: Filter::This,
            layer: baylee_cards_dsl::Layer::Text,
            modifier: Modifier::AdjustSpellCostByCast {
                who: PlayerRel::You,
                filter: &Filter::This,
                delta_per_unit: -2,
                metric: SpellCastMetric::OptionalCostPaid,
            },
            source_zone: ZoneRef::Stack,
            condition: None,
        }),
        spell!(&[Effect::GainLife {
            amount: Amount::Fixed(3),
        }]),
    ];
    let bite = card(
        C3,
        FaceDef {
            optional_cost: Some(Cost {
                mana: ManaCost::ZERO,
                parts: EVIDENCE_6_PARTS,
            }),
            ..face("Bite Probe", "{3}{G}", TypeSet::SORCERY)
        },
        KeywordSet::EMPTY,
        BITE_AB,
    );
    let wurm = card(
        C4,
        creature_face("Dead Wurm", "{4}{G}{G}", 6, 6),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine2 = bench(
        16_204,
        vec![bite, wurm],
        [Seat::default().holding(&[C3, C4]), Seat::default()],
    );
    to_main(&mut engine2, me());
    add_pool(&mut engine2, me(), ManaColor::Green, 2); // {1}{G} available, NOT {3}{G}
    let bite_id = the(&engine2, ZoneLocation::Hand(me()), C3);
    let wurm_id = the(&engine2, ZoneLocation::Hand(me()), C4);
    assert!(
        !offer(&engine2).1.castable.contains(&bite_id),
        "without 6 MV in graveyard, {{3}}{{G}} Bite Probe must not be castable on {{G}}{{G}}"
    );
    move_to_graveyard(&mut engine2, me(), wurm_id);
    assert!(
        offer(&engine2).1.castable.contains(&bite_id),
        "with 6 MV in graveyard, OptionalCost reduction (-2) makes Bite Probe castable on {{G}}{{G}}"
    );
    engine2
        .apply(me(), PlayerAction::CastSpell { card: bite_id })
        .expect("cast bite");
    if matches!(engine2.pending(), Pending::ChooseCastMode { .. }) {
        choose_cast_mode_kind(&mut engine2, me(), CastModeKind::OptionalCost);
    }
    engine2
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![wurm_id],
            },
        )
        .expect("collect evidence 6");
    settle(&mut engine2, me());
    assert_eq!(life(&engine2, me()), 23);
    assert_eq!(
        engine2.state().object(wurm_id).unwrap().zone,
        crate::zone::Zone::Exile
    );

    // 3. Waterbend on `optional_cost` (`Katara`): `{0}` + optional `{2}` waterbend paid by tapping 2 creatures!
    static KATARA_AB: &[AbilityDef] = &[spell!(&[Effect::IfCondition {
        condition: Condition::OptionalCostPaid,
        then: &[Effect::GainLife {
            amount: Amount::Fixed(5),
        }],
        otherwise: &[],
    }])];
    let katara = card(
        C5,
        FaceDef {
            optional_cost: Some(cost!("{2}")),
            waterbend: true,
            ..face("Katara Probe", "{0}", TypeSet::SORCERY)
        },
        KeywordSet::EMPTY,
        KATARA_AB,
    );
    let mut engine3 = bench(
        16_205,
        vec![katara, bear],
        [Seat::with(&[C1, C1]).holding(&[C5]), Seat::default()],
    );
    to_main(&mut engine3, me());
    let bears = objects(&engine3, ZoneLocation::Battlefield, C1);
    let katara_id = the(&engine3, ZoneLocation::Hand(me()), C5);
    engine3
        .apply(me(), PlayerAction::CastSpell { card: katara_id })
        .expect("cast katara");
    choose_cast_mode_kind(&mut engine3, me(), CastModeKind::OptionalCost);
    engine3
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: bears.clone(),
            },
        )
        .expect("tap 2 bears for waterbend {2}");
    settle(&mut engine3, me());
    assert_eq!(life(&engine3, me()), 25);
    assert!(bears.iter().all(|&b| {
        engine3
            .state()
            .object(b)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    }));

    // 4. APNAP `Effect::PutCountersOnChosenControlled` (`High Perfect Morcant` in 3-player):
    static MORCANT_AB: &[AbilityDef] = &[spell!(&[Effect::PutCountersOnChosenControlled {
        who: PlayerRel::EachOpponent,
        filter: &Filter::CREATURE,
        kind: CounterKind::M1M1,
        count: Amount::Fixed(1),
    }])];
    let morcant = card(
        C6,
        face("Morcant Probe", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        MORCANT_AB,
    );
    let mut engine4 = bench(
        16_206,
        vec![morcant, wall],
        [
            Seat::default().holding(&[C6]),
            Seat::with(&[C2]),
            Seat::with(&[C2]),
        ],
    );
    to_main(&mut engine4, me());
    let p1 = PlayerId::new(1);
    let p2 = PlayerId::new(2);
    let w1 = engine4
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| engine4.state().object(id).unwrap().controller == p1)
        .unwrap();
    let w2 = engine4
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| engine4.state().object(id).unwrap().controller == p2)
        .unwrap();
    let m_id = the(&engine4, ZoneLocation::Hand(me()), C6);
    engine4
        .apply(me(), PlayerAction::CastSpell { card: m_id })
        .expect("cast morcant");
    for seat in [me(), p1, p2] {
        engine4
            .apply(seat, PlayerAction::PassPriority)
            .expect("pass");
    }
    // Seat 1 chooses first (APNAP), then seat 2!
    let Pending::ChooseCards {
        player: first_chooser,
        ..
    } = engine4.pending().clone()
    else {
        panic!("expected ChooseCards for p1, got {:?}", engine4.pending());
    };
    assert_eq!(first_chooser, p1);
    engine4
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![w1] })
        .expect("p1 blights w1");
    let Pending::ChooseCards {
        player: second_chooser,
        ..
    } = engine4.pending().clone()
    else {
        panic!("expected ChooseCards for p2, got {:?}", engine4.pending());
    };
    assert_eq!(second_chooser, p2);
    engine4
        .apply(p2, PlayerAction::ChooseObjects { objects: vec![w2] })
        .expect("p2 blights w2");
    assert_eq!(counters_on(&engine4, w1, CounterKind::M1M1), 1);
    assert_eq!(counters_on(&engine4, w2, CounterKind::M1M1), 1);
}

#[test]
fn wave16b_reveal_morph_martyr_forecast_beheld_stats_and_dragon_presence() {
    // 1. Morph with `RevealFromHand` (*Dragon's Eye Savants*) + Martyr `RevealFromHandX` + Forecast!
    static BLUE_FILTER: Filter = Filter::HasColor(ColorSet::of(Color::Blue));
    static WHITE_FILTER: Filter = Filter::HasColor(ColorSet::of(Color::White));
    static REVEAL_BLUE_PARTS: &[CostPart] = &[CostPart::RevealFromHand(&BLUE_FILTER)];
    let savants = card(
        C0,
        FaceDef {
            morph: Some(ManaCost::ZERO),
            morph_parts: REVEAL_BLUE_PARTS,
            ..creature_face("Savants Probe", "{1}{U}", 0, 6)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let blue_card = card(
        C1,
        face("Blue Filler", "{U}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        &[],
    );
    static MARTYR_AB: &[AbilityDef] = &[activated!(
        cost!(RevealFromHandX(&WHITE_FILTER), SacrificeSelf),
        &[Effect::GainLife {
            amount: Amount::Times {
                base: &Amount::X,
                factor: 3,
            },
        }]
    )];
    static FORECAST_AB: &[AbilityDef] = &[activated!(
        cost!(RevealSelfFromHand),
        &[Effect::GainLife {
            amount: Amount::Fixed(2),
        }],
        zone = ActivationZone::Hand,
        limit = ActivationLimit::PerTurn(1),
        condition = Some(Condition::All(&[
            Condition::YourTurn,
            Condition::DuringStep(StepKind::Upkeep),
        ]))
    )];
    let martyr = card(
        C2,
        creature_face("Martyr Probe", "{W}", 1, 1),
        KeywordSet::EMPTY,
        MARTYR_AB,
    );
    let forecast_card = card(
        C3,
        creature_face("Forecast Probe", "{2}{W}", 2, 1),
        KeywordSet::EMPTY,
        FORECAST_AB,
    );

    let mut engine = bench(
        16_207,
        vec![savants, blue_card, martyr, forecast_card],
        [
            Seat::with(&[C2]).holding(&[C0, C1, C3, C3]),
            Seat::default(),
        ],
    );
    walk_until(&mut engine, |e| holds_priority_in(e, me(), Step::Upkeep));
    let forecasts = objects(&engine, ZoneLocation::Hand(me()), C3);
    assert_eq!(forecasts.len(), 2);
    activate(&mut engine, me(), forecasts[0], 0);
    assert!(
        engine.state().hand_card_revealed(forecasts[0]),
        "Forecast must reveal the source card in hand during upkeep"
    );
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 22);
    assert!(
        !offer(&engine).1.abilities.contains(&(forecasts[0], 0)),
        "Forecast is once per turn per card"
    );

    // Move to main phase: revealed-in-hand flag from upkeep is cleared!
    to_main(&mut engine, me());
    assert!(
        !engine.state().hand_card_revealed(forecasts[0]),
        "Forecast reveal clears once upkeep ends (CR 702.57b)"
    );

    // Activate Martyr of Sands revealing BOTH white Forecast cards in hand (X = 2 -> +6 life)!
    let martyr_bf = the(&engine, ZoneLocation::Battlefield, C2);
    activate(&mut engine, me(), martyr_bf, 0);
    engine
        .apply(me(), PlayerAction::ChooseNumber(2))
        .expect("announce X = 2");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![forecasts[0], forecasts[1]],
            },
        )
        .expect("reveal 2 white cards");
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 28);

    // Cast Savants face-down for {3}, then turn face up by revealing `blue_card`!
    add_pool(&mut engine, me(), ManaColor::Colorless, 3);
    let savants_hand = the(&engine, ZoneLocation::Hand(me()), C0);
    let blue_in_hand = the(&engine, ZoneLocation::Hand(me()), C1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: savants_hand })
        .expect("cast savants");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        choose_cast_mode_kind(&mut engine, me(), CastModeKind::Disguise);
    }
    settle(&mut engine, me());
    let savants_bf = the(&engine, ZoneLocation::Battlefield, C0);
    assert!(
        engine
            .state()
            .object(savants_bf)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    activate(&mut engine, me(), savants_bf, TURN_FACE_UP);
    let Pending::ChooseCards { prompt, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards for morph reveal");
    };
    assert_eq!(prompt, ChoicePrompt::Generic);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![blue_in_hand],
            },
        )
        .expect("reveal blue card for morph");
    assert!(
        !engine
            .state()
            .object(savants_bf)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(
        engine
            .state()
            .object(savants_bf)
            .unwrap()
            .characteristics()
            .toughness,
        Some(6)
    );

    // 2. Tarkir Dragon presence (`ControlledSubtypeOnCast`) + `BeheldOrRevealedPower` / `ManaValue`:
    static DRAGON_FILTER: Filter = Filter::HasSubtype(creature::DRAGON);
    static BEHOLD_DRAGON: &[CostPart] = &[CostPart::Behold(&DRAGON_FILTER)];
    static REVEAL_DRAGON: &[CostPart] = &[CostPart::RevealFromHand(&DRAGON_FILTER)];
    const PRESENCE_COND: Condition = Condition::Any(&[
        Condition::OptionalCostPaid,
        Condition::ControlledSubtypeOnCast(creature::DRAGON),
    ]);

    let dragon = card(
        C4,
        FaceDef {
            subtypes: DRAGON_SUBS,
            ..creature_face("Shivan Probe", "{4}{R}{R}", 5, 5)
        },
        KeywordSet::EMPTY,
        &[],
    );
    static DRAGONS_FIRE_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Sum {
            left: &Amount::BeheldOrRevealedPower,
            right: &Amount::BeheldOrRevealedManaValue,
        },
    }])];
    static PREROGATIVE_AB: &[AbilityDef] = &[
        static_ability!(
            Filter::This,
            Modifier::AddKeyword(KeywordSet::UNCOUNTERABLE),
            zone = ZoneRef::Stack,
            condition = Some(PRESENCE_COND)
        ),
        spell!(&[Effect::GainLife {
            amount: Amount::IfCondition {
                condition: &PRESENCE_COND,
                then: &Amount::Fixed(4),
                otherwise: &Amount::Fixed(1),
            },
        }]),
    ];
    static CANCEL_AB: &[AbilityDef] = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )];
    let dragons_fire = card(
        C5,
        FaceDef {
            optional_cost: Some(Cost {
                mana: ManaCost::ZERO,
                parts: BEHOLD_DRAGON,
            }),
            ..face("Dragon's Fire Probe", "{0}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        DRAGONS_FIRE_AB,
    );
    let prerogative = card(
        C6,
        FaceDef {
            optional_cost: Some(Cost {
                mana: ManaCost::ZERO,
                parts: REVEAL_DRAGON,
            }),
            ..face("Prerogative Probe", "{0}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        PREROGATIVE_AB,
    );
    let cancel = card(
        C7,
        face("Cancel Probe", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        CANCEL_AB,
    );

    let mut engine2 = bench(
        16_208,
        vec![dragon, dragons_fire, prerogative, cancel],
        [
            Seat::with(&[C4]).holding(&[C4, C5, C6]),
            Seat::default().holding(&[C7]),
        ],
    );
    to_main(&mut engine2, me());

    // Cast Dragon's Fire beholding the 5/5 MV-6 Dragon in hand -> gains 5 + 6 = 11 life!
    let hand_dragon = the(&engine2, ZoneLocation::Hand(me()), C4);
    let df_id = the(&engine2, ZoneLocation::Hand(me()), C5);
    engine2
        .apply(me(), PlayerAction::CastSpell { card: df_id })
        .expect("cast dragon's fire");
    choose_cast_mode_kind(&mut engine2, me(), CastModeKind::OptionalCost);
    engine2
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![hand_dragon],
            },
        )
        .expect("reveal dragon from hand");
    settle(&mut engine2, me());
    assert_eq!(life(&engine2, me()), 31);

    // Cast Prerogative WITHOUT revealing, while controlling battlefield Dragon:
    // even if the battlefield Dragon leaves before resolution, `ControlledSubtypeOnCast`
    // keeps Prerogative uncounterable and grants the 4-life branch!
    let bf_dragon = the(&engine2, ZoneLocation::Battlefield, C4);
    let prerog_id = the(&engine2, ZoneLocation::Hand(me()), C6);
    engine2
        .apply(me(), PlayerAction::CastSpell { card: prerog_id })
        .expect("cast prerogative");
    choose_cast_mode_kind(&mut engine2, me(), CastModeKind::Normal);
    // Move the Dragon off the battlefield while Prerogative is on the stack!
    move_to_graveyard(&mut engine2, me(), bf_dragon);
    engine2
        .apply(me(), PlayerAction::PassPriority)
        .expect("me passes");
    let cancel_id = the(&engine2, ZoneLocation::Hand(them()), C7);
    engine2
        .apply(them(), PlayerAction::CastSpell { card: cancel_id })
        .expect("them casts cancel");
    engine2
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![prerog_id],
                players: vec![],
            },
        )
        .expect("target prerogative");
    settle(&mut engine2, me());
    assert_eq!(
        life(&engine2, me()),
        35,
        "Prerogative must remain uncounterable and remember ControlledSubtypeOnCast (+4 life)"
    );
}

#[test]
fn wave16b_graveyard_grants_flashback_jump_start_embalm_and_encore() {
    // 1. `GrantsFlashbackCost({0})` & `GrantsJumpStart`:
    static BOLT_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(3),
    }])];
    static GRANTER_AB: &[AbilityDef] = &[
        free(
            &[Effect::ModifyTargets {
                modifier: Modifier::GrantsFlashbackCost(ManaCost::ZERO),
                duration: Duration::UntilEndOfTurn,
            }],
            Some(TargetReq::one(TargetSpec::CardInGraveyard(
                &Filter::INSTANT_OR_SORCERY,
                PlayerRel::You,
            ))),
        ),
        free(
            &[Effect::ModifyTargets {
                modifier: Modifier::GrantsJumpStart,
                duration: Duration::UntilEndOfTurn,
            }],
            Some(TargetReq::one(TargetSpec::CardInGraveyard(
                &Filter::INSTANT_OR_SORCERY,
                PlayerRel::You,
            ))),
        ),
        free(
            &[Effect::ModifyTargets {
                modifier: Modifier::GrantsEmbalmManaCost,
                duration: Duration::UntilEndOfTurn,
            }],
            Some(TargetReq::one(TargetSpec::CardInGraveyard(
                &Filter::CREATURE,
                PlayerRel::You,
            ))),
        ),
        free(
            &[Effect::ModifyTargets {
                modifier: Modifier::GrantsEncoreManaCost,
                duration: Duration::UntilEndOfTurn,
            }],
            Some(TargetReq::one(TargetSpec::CardInGraveyard(
                &Filter::CREATURE,
                PlayerRel::You,
            ))),
        ),
    ];
    let bolt = card(
        C0,
        face("Grave Bolt", "{2}{R}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        BOLT_AB,
    );
    let granter = card(
        C1,
        face("Grave Granter", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        GRANTER_AB,
    );
    let small_cat = card(
        C2,
        creature_face("Small Cat", "{W}", 2, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let big_angel = card(
        C3,
        creature_face("Big Angel", "{4}{W}", 4, 4),
        KeywordSet::FLYING,
        &[],
    );
    let filler = card(
        C4,
        face("Discard Fodder", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        16_209,
        vec![bolt, granter, small_cat, big_angel, filler],
        [
            Seat::with(&[C1]).holding(&[C0, C0, C2, C3, C4]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let granter_bf = the(&engine, ZoneLocation::Battlefield, C1);
    let bolts = objects(&engine, ZoneLocation::Hand(me()), C0);
    let cat_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let angel_id = the(&engine, ZoneLocation::Hand(me()), C3);
    let fodder_id = the(&engine, ZoneLocation::Hand(me()), C4);
    for &id in &[bolts[0], bolts[1], cat_id, angel_id] {
        move_to_graveyard(&mut engine, me(), id);
    }

    // Grant Flashback {0} to bolts[0] -> cast for {0}, exiles on resolution!
    activate(&mut engine, me(), granter_bf, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bolts[0]],
                players: vec![],
            },
        )
        .expect("target bolt 0");
    settle(&mut engine, me());
    engine
        .apply(me(), PlayerAction::CastSpell { card: bolts[0] })
        .expect("cast bolt 0 via granted Flashback {0}");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        choose_cast_mode_kind(&mut engine, me(), CastModeKind::Normal);
    }
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 23);
    assert_eq!(
        engine.state().object(bolts[0]).unwrap().zone,
        crate::zone::Zone::Exile,
        "granted FlashbackCost must exile the spell on resolution (CR 702.34a)"
    );

    // Grant Jump-start to bolts[1] -> cast for {2}{R} + discard `fodder_id`, exiles on resolution!
    activate(&mut engine, me(), granter_bf, 1);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bolts[1]],
                players: vec![],
            },
        )
        .expect("target bolt 1");
    settle(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Red, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: bolts[1] })
        .expect("cast bolt 1 via granted Jump-start");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        choose_cast_mode_kind(&mut engine, me(), CastModeKind::JumpStart);
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![fodder_id],
            },
        )
        .expect("discard fodder for jump-start");
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 26);
    assert_eq!(
        engine.state().object(bolts[1]).unwrap().zone,
        crate::zone::Zone::Exile,
        "granted Jump-start must exile the spell on resolution (CR 702.133a)"
    );
    assert_eq!(
        engine.state().object(fodder_id).unwrap().zone,
        crate::zone::Zone::Graveyard
    );

    // Grant Embalm to BOTH Small Cat ({W}) and Big Angel ({4}{W}):
    // With only {W} in pool, Small Cat's granted Embalm IS offered while Big Angel's is NOT!
    for &cid in &[cat_id, angel_id] {
        activate(&mut engine, me(), granter_bf, 2);
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![cid],
                    players: vec![],
                },
            )
            .expect("target creature in graveyard");
        settle(&mut engine, me());
    }
    add_pool(&mut engine, me(), ManaColor::White, 1);
    let (_, legal) = offer(&engine);
    let cat_grant = legal
        .abilities
        .iter()
        .copied()
        .find(|&(src, _)| src == cat_id);
    let angel_grant = legal
        .abilities
        .iter()
        .copied()
        .find(|&(src, _)| src == angel_id);
    assert!(
        cat_grant.is_some() && angel_grant.is_none(),
        "each graveyard card must bind its own mana cost for GrantsEmbalmManaCost: {legal:?}"
    );
    let (_, slot) = cat_grant.unwrap();
    activate(&mut engine, me(), cat_id, slot);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(cat_id).unwrap().zone,
        crate::zone::Zone::Exile
    );
    let zombie_tokens: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let o = engine.state().object(id).unwrap();
            o.card.is_none()
                && o.characteristics().subtypes.contains(creature::ZOMBIE)
                && o.characteristics().colors == ColorSet::of(Color::White)
        })
        .collect();
    assert_eq!(
        zombie_tokens.len(),
        1,
        "Embalm creates a white Zombie token copy"
    );

    // Grant Encore to Small Cat ({W}) in a 3-player game -> creates 2 hasty token copies,
    // one per opponent, that must attack that opponent and are sacrificed at the end step!
    let mut engine2 = bench(
        16_210,
        vec![granter, small_cat],
        [
            Seat::with(&[C1]).holding(&[C2]),
            Seat::default(),
            Seat::default(),
        ],
    );
    to_main(&mut engine2, me());
    let g2 = the(&engine2, ZoneLocation::Battlefield, C1);
    let c2 = the(&engine2, ZoneLocation::Hand(me()), C2);
    move_to_graveyard(&mut engine2, me(), c2);
    activate(&mut engine2, me(), g2, 3);
    engine2
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![c2],
                players: vec![],
            },
        )
        .expect("target small cat for Encore");
    settle(&mut engine2, me());

    add_pool(&mut engine2, me(), ManaColor::White, 1);
    let (_, legal2) = offer(&engine2);
    let (_, encore_slot) = legal2
        .abilities
        .iter()
        .copied()
        .find(|&(src, _)| src == c2)
        .expect("Encore ability offered on Small Cat");
    activate(&mut engine2, me(), c2, encore_slot);
    for seat in [me(), PlayerId::new(1), PlayerId::new(2)] {
        engine2
            .apply(seat, PlayerAction::PassPriority)
            .expect("pass");
    }
    assert_eq!(
        engine2.state().object(c2).unwrap().zone,
        crate::zone::Zone::Exile
    );
    let encore_tokens: Vec<_> = engine2
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let o = engine2.state().object(id).unwrap();
            o.card.is_none() && o.characteristics().keywords.contains(KeywordSet::HASTE)
        })
        .collect();
    assert_eq!(
        encore_tokens.len(),
        2,
        "Encore in 3-player game creates one hasty token copy per opponent"
    );
    let required: Vec<_> = encore_tokens
        .iter()
        .filter_map(|&id| {
            engine2
                .state()
                .object(id)
                .unwrap()
                .riders
                .iter()
                .find_map(|r| match *r {
                    crate::object::Rider::MustAttackPlayerThisTurn(p) => Some(p),
                    _ => None,
                })
        })
        .collect();
    assert_eq!(required, vec![PlayerId::new(1), PlayerId::new(2)]);
}

#[test]
fn wave16b_silvergill_either_reveal_and_pyrrhic_entwine_blight() {
    static MERFOLK_SUBS: &[baylee_core::ids::SubtypeId] = &[creature::MERFOLK];
    static MERFOLK_FILTER: Filter = Filter::HasSubtype(creature::MERFOLK);
    static REVEAL_MERFOLK_COST: Cost = cost!(RevealFromHand(&MERFOLK_FILTER));
    static PAY_3_COST: Cost = cost!("{3}");
    static SILVERGILL_PARTS: &[CostPart] = &[CostPart::Either(&REVEAL_MERFOLK_COST, &PAY_3_COST)];
    static BLIGHT_2_ENTWINE: Cost = cost!(PutCounterOnOther {
        filter: &Filter::YOUR_CREATURE,
        kind: CounterKind::M1M1,
        n: 2,
    });
    static PYRRHIC_MODES: &[SpellMode] = &[
        SpellMode {
            effects: &[Effect::GainLife {
                amount: Amount::Fixed(2),
            }],
            targets: None,
            second_targets: None,
            cost_override: None,
            additional_cost: None,
        },
        SpellMode {
            effects: &[Effect::GainLife {
                amount: Amount::Fixed(3),
            }],
            targets: None,
            second_targets: None,
            cost_override: None,
            additional_cost: None,
        },
    ];

    let silvergill = card(
        C0,
        FaceDef {
            subtypes: MERFOLK_SUBS,
            mandatory_additional_costs: SILVERGILL_PARTS,
            ..creature_face("Silvergill Probe", "{1}{U}", 2, 1)
        },
        KeywordSet::EMPTY,
        &[],
    );
    static PYRRHIC_AB: &[AbilityDef] = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: PYRRHIC_MODES,
    }];
    let pyrrhic = card(
        C1,
        FaceDef {
            entwine: Some(BLIGHT_2_ENTWINE),
            ..face("Pyrrhic Probe", "{0}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        PYRRHIC_AB,
    );
    let ox = card(
        C2,
        creature_face("Ox", "{2}{G}", 2, 4),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        16_211,
        vec![silvergill, pyrrhic, ox],
        [Seat::with(&[C2]).holding(&[C0, C0, C1]), Seat::default()],
    );
    to_main(&mut engine, me());

    // 1. Two Silvergills in hand + {1}{U}: cast the first revealing the second for {1}{U} (NOT {4}{U})!
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    let adepts = objects(&engine, ZoneLocation::Hand(me()), C0);
    assert_eq!(adepts.len(), 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: adepts[0] })
        .expect("Silvergill castable on {1}{U} with another Merfolk in hand");
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine
            .apply(me(), PlayerAction::YesNo(true))
            .expect("choose Reveal branch of CostPart::Either");
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![adepts[1]],
            },
        )
        .expect("reveal second Merfolk");
    settle(&mut engine, me());
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, C0).len(), 1);

    // Second Silvergill now has NO other Merfolk in hand -> requires {4}{U}, NOT castable on {1}{U}!
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    assert!(
        !offer(&engine).1.castable.contains(&adepts[1]),
        "without another Merfolk in hand, Silvergill requires {{4}}{{U}}"
    );

    // 2. Pyrrhic Strike entwine with Blight 2 -> chooses both modes (+5 life) and puts 2 -1/-1 counters on Ox!
    let ox_bf = the(&engine, ZoneLocation::Battlefield, C2);
    let pyrrhic_hand = the(&engine, ZoneLocation::Hand(me()), C1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: pyrrhic_hand })
        .expect("cast pyrrhic");
    choose_cast_mode_kind(&mut engine, me(), CastModeKind::Modes(0b11));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![ox_bf],
            },
        )
        .expect("blight 2 on ox");
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 25);
    assert_eq!(counters_on(&engine, ox_bf, CounterKind::M1M1), 2);
}
