//! Wave 18b deterministic gameplay tests:
//! 1. `KeywordSet::ASSIST` (`CR 702.132`): 2P & 3P assistant selection, generic
//!    mana contribution, and `snapshot_hash()` while suspended at Assist.
//! 2. `Waterbend` (`CR 701.67a`): `CostPart::Waterbend(n)` tapping artifacts/creatures
//!    + mana payment, `AbilityDef::WardWaterbend`, and `Effect::PlayerMayWaterbendOr`.
//! 3. `GenericChoice` runtime primitives:
//!    - `EnterModifier::ChooseMode` (`Condition::ChosenMode`)
//!    - `EnterModifier::EachPlayerChooseMode` (APNAP order & `ControlledByPlayerWhoChoseMode`)
//!    - `Effect::PlayerMayPayOneOfCostsOr` (pay cost vs decline)
//!    - `Effect::PlayersChooseMode` (APNAP order, `PrisonersDilemma` secret aggregate,
//!      and `snapshot_hash()` sensitivity).
//! 4. `Bookkeeping` runtime primitives: `Effect::CreepingDread` (APNAP order + same-type
//!    vs different-type life loss), `Effect::Flickerform` (delayed Aura reattach),
//!    `Effect::PhyrexianGrimoire`, `Effect::TaintedPact` (duplicate name stop),
//!    and `Effect::SearchLibraryAttachedToEvent`.

#![allow(clippy::too_many_lines, clippy::items_after_statements)]

use super::*;
use crate::choice::{ChoicePrompt, TargetPrompt, YesNoPrompt};
use crate::event::Cause;
use crate::object::Status;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::prelude::{spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, Condition, CostPart, EnterModifier, Filter, Modifier, MultiPlayerChoice,
    MultiPlayerChoiceAggregate, PlayerRel, TargetReq, TargetSpec, Trigger,
};
use baylee_core::generated::subtypes::enchantment;
use baylee_core::mana::ManaColor;

const W0: u32 = 99_880;
const W1: u32 = 99_881;
const W2: u32 = 99_882;
const W3: u32 = 99_883;
const W4: u32 = 99_884;
const W5: u32 = 99_885;
const W6: u32 = 99_886;
const W7: u32 = 99_887;
const W8: u32 = 99_888;
const W9: u32 = 99_889;
const W10: u32 = 99_890;

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

#[test]
fn wave18b_assist_2p_and_3p_mana_contribution() {
    static GAIN_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(4),
    }])];
    let assist_spell = card(
        W0,
        FaceDef {
            keywords: KeywordSet::ASSIST,
            ..face("Assist Boon", "{3}{W}", TypeSet::SORCERY)
        },
        KeywordSet::ASSIST,
        GAIN_AB,
    );

    // 2P: me() has {1}{W}, them() has {2}; Assist pays the {2} generic gap.
    let mut engine = bench(
        18_001,
        vec![assist_spell],
        [Seat::default().holding(&[W0]), Seat::default()],
    );
    to_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::White, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    add_pool(&mut engine, them(), ManaColor::Colorless, 2);

    let spell_id = the(&engine, ZoneLocation::Hand(me()), W0);
    engine
        .apply(me(), PlayerAction::CastSpell { card: spell_id })
        .expect("assist cast begins");
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::Assist,
            ..
        } if *player == me()
    ));
    let h_yesno = engine.snapshot_hash();
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("confirm use assist");
    assert!(matches!(
        engine.pending(),
        Pending::ChooseNumber {
            player,
            min: 0,
            max: 2,
            ..
        } if *player == them()
    ));
    let h_num = engine.snapshot_hash();
    assert_ne!(h_yesno, h_num);
    engine
        .apply(them(), PlayerAction::ChooseNumber(2))
        .expect("helper pays 2 generic");
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 24);
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);

    // 3P: seat 0 picks between seat 1 and seat 2 as assistant.
    let mut g3 = bench(
        18_002,
        vec![assist_spell],
        [
            Seat::default().holding(&[W0]),
            Seat::default(),
            Seat::default(),
        ],
    );
    to_main(&mut g3, me());
    add_pool(&mut g3, me(), ManaColor::White, 1);
    add_pool(&mut g3, me(), ManaColor::Colorless, 1);
    add_pool(&mut g3, PlayerId::new(2), ManaColor::Colorless, 2);
    let spell3 = the(&g3, ZoneLocation::Hand(me()), W0);
    g3.apply(me(), PlayerAction::CastSpell { card: spell3 })
        .unwrap();
    g3.apply(me(), PlayerAction::YesNo(true)).unwrap();
    assert!(matches!(
        g3.pending(),
        Pending::ChoosePlayer { player, options }
            if *player == me() && options.contains(&PlayerId::new(2))
    ));
    g3.apply(me(), PlayerAction::ChoosePlayer(PlayerId::new(2)))
        .unwrap();
    g3.apply(PlayerId::new(2), PlayerAction::ChooseNumber(2))
        .unwrap();
    settle(&mut g3, me());
    assert_eq!(life(&g3, me()), 24);
}

#[test]
fn wave18b_waterbend_cost_ward_and_unless() {
    static WB_SPELL_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(3),
    }])];
    static WB_WARD_AB: &[AbilityDef] = &[AbilityDef::WardWaterbend { mana: 2 }];
    static WB_PING_AB: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static WB_COUNTER_AB: &[AbilityDef] = &[spell!(
        &[Effect::PlayerMayWaterbendOr {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::Fixed(2),
            effect: &Effect::CounterTargetSpell,
        }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )];

    let wb_spell = card(
        W0,
        FaceDef {
            mandatory_additional_costs: &[CostPart::Waterbend(3)],
            ..face("Waterbend Surge", "{U}", TypeSet::SORCERY)
        },
        KeywordSet::EMPTY,
        WB_SPELL_AB,
    );
    let artifact_pawn = card(
        W1,
        face("Relic Pawn", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let ward_merfolk = card(
        W2,
        creature_face("Kyoshi Merfolk", "{1}{U}", 2, 2),
        KeywordSet::EMPTY,
        WB_WARD_AB,
    );
    let ping = card(
        W3,
        face("Shock Bolt", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        WB_PING_AB,
    );
    let wb_counter = card(
        W4,
        face("Waterbend Quash", "{1}{U}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        WB_COUNTER_AB,
    );
    let bear = card(
        W5,
        creature_face("Hill Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    // 1. Casting spell with Waterbend(3): tap 2 artifacts + pay {1}{U}.
    let mut engine = bench(
        18_010,
        vec![
            wb_spell,
            artifact_pawn,
            ward_merfolk,
            ping,
            wb_counter,
            bear,
        ],
        [
            Seat::with(&[W1, W1, W2]).holding(&[W0, W4]),
            Seat::with(&[W1, W1]).holding(&[W3, W5]),
        ],
    );
    to_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    let arts = objects(&engine, ZoneLocation::Battlefield, W1)
        .into_iter()
        .filter(|&id| engine.state().object(id).unwrap().controller == me())
        .collect::<Vec<_>>();
    assert_eq!(arts.len(), 2);

    let wb_hand = the(&engine, ZoneLocation::Hand(me()), W0);
    engine
        .apply(me(), PlayerAction::CastSpell { card: wb_hand })
        .unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseTargets {
            player,
            reason: TargetPrompt::Convoke,
            ..
        } if *player == me()
    ));
    let h_wb = engine.snapshot_hash();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: arts.clone(),
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 23);
    assert!(arts.iter().all(|&id| {
        engine
            .state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    }));

    // 2. Ward:Waterbend<2> counters opponent's targeted spell when declined.
    to_main(&mut engine, them());
    add_pool(&mut engine, them(), ManaColor::Red, 1);
    let merfolk_id = the(&engine, ZoneLocation::Battlefield, W2);
    let ping_hand = the(&engine, ZoneLocation::Hand(them()), W3);
    engine
        .apply(them(), PlayerAction::CastSpell { card: ping_hand })
        .unwrap();
    engine
        .apply(
            them(),
            PlayerAction::ChooseObjects {
                objects: vec![merfolk_id],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayWaterbend { mana: 2 },
            ..
        } if *player == them()
    ));
    let h_ward = engine.snapshot_hash();
    assert_ne!(h_wb, h_ward);
    engine.apply(them(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, them());
    assert_eq!(
        engine.state().object(merfolk_id).unwrap().zone,
        crate::zone::Zone::Battlefield
    );

    // 3. Effect::PlayerMayWaterbendOr paid by tapping 2 opponent artifacts -> spell NOT countered.
    add_pool(&mut engine, them(), ManaColor::Green, 2);
    let bear_hand = the(&engine, ZoneLocation::Hand(them()), W5);
    engine
        .apply(them(), PlayerAction::CastSpell { card: bear_hand })
        .unwrap();
    let bear_stack = the(&engine, ZoneLocation::Stack, W5);
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    add_pool(&mut engine, me(), ManaColor::Blue, 2);
    let counter_hand = the(&engine, ZoneLocation::Hand(me()), W4);
    engine
        .apply(me(), PlayerAction::CastSpell { card: counter_hand })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bear_stack],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(them(), PlayerAction::YesNo(true)).unwrap();
    let opp_arts = objects(&engine, ZoneLocation::Battlefield, W1)
        .into_iter()
        .filter(|&id| engine.state().object(id).unwrap().controller == them())
        .collect::<Vec<_>>();
    assert_eq!(opp_arts.len(), 2);
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards {
            player,
            prompt: ChoicePrompt::Waterbend,
            ..
        } if *player == them()
    ));
    engine
        .apply(them(), PlayerAction::ChooseObjects { objects: opp_arts })
        .unwrap();
    settle(&mut engine, them());
    assert_eq!(objects(&engine, ZoneLocation::Battlefield, W5).len(), 1);
}

#[test]
fn wave18b_generic_choice_etb_pay_unless_and_per_player_modal() {
    static SIEGE_MODS: &[EnterModifier] = &[EnterModifier::ChooseMode(&["Khans", "Dragons"])];
    static SIEGE_AB: &[AbilityDef] = &[triggered!(
        Trigger::StepBegin {
            step: baylee_cards_dsl::StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(2),
        }],
        condition = Some(Condition::ChosenMode(0))
    )];
    static ARCHANGEL_MODS: &[EnterModifier] =
        &[EnterModifier::EachPlayerChooseMode(&["War", "Peace"])];
    static WAR_FILTER: Filter =
        Filter::And(&[Filter::CREATURE, Filter::ControlledByPlayerWhoChoseMode(0)]);
    static PEACE_FILTER: Filter =
        Filter::And(&[Filter::CREATURE, Filter::ControlledByPlayerWhoChoseMode(1)]);
    static ARCHANGEL_AB: &[AbilityDef] = &[
        static_ability!(WAR_FILTER, Modifier::ModifyPT(3, 0)),
        static_ability!(PEACE_FILTER, Modifier::ModifyPT(0, 3)),
    ];
    static PAY_UNLESS_AB: &[AbilityDef] = &[spell!(
        &[Effect::PlayerMayPayOneOfCostsOr {
            player: PlayerRel::Chosen,
            costs: &[
                CostPart::Sacrifice(&Filter::ControlledByYou),
                CostPart::Discard(&Filter::Any),
            ],
            otherwise: &[Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::Chosen),
                amount: Amount::Fixed(3),
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
    )];
    static PD_AB: &[AbilityDef] = &[spell!(&[Effect::PlayersChooseMode(MultiPlayerChoice {
        players: PlayerRel::EachOpponent,
        labels: &["Silence", "Snitch"],
        branches: &[
            &[Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::EventPlayer),
                amount: Amount::Fixed(4),
            }],
            &[Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::EventPlayer),
                amount: Amount::Fixed(8),
            }],
            &[Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::EventPlayer),
                amount: Amount::Fixed(12),
            }],
        ],
        secret: true,
        aggregate: MultiPlayerChoiceAggregate::PrisonersDilemma,
    })])];

    let siege = card(
        W0,
        FaceDef {
            enter_modifiers: SIEGE_MODS,
            ..face("Siege Test", "{0}", TypeSet::ENCHANTMENT)
        },
        KeywordSet::EMPTY,
        SIEGE_AB,
    );
    let archangel = card(
        W1,
        FaceDef {
            enter_modifiers: ARCHANGEL_MODS,
            ..creature_face("Strife Angel", "{0}", 2, 2)
        },
        KeywordSet::EMPTY,
        ARCHANGEL_AB,
    );
    let bear = card(
        W2,
        creature_face("Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let torment = card(
        W3,
        face("Torment Test", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        PAY_UNLESS_AB,
    );
    let pd = card(
        W4,
        face("Dilemma Test", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        PD_AB,
    );

    let mut engine = bench(
        18_020,
        vec![siege, archangel, bear, torment, pd],
        [
            Seat::with(&[W2]).holding(&[W0, W1, W3]),
            Seat::with(&[W2]).holding(&[W2]),
        ],
    );
    to_main(&mut engine, me());

    let siege_hand = the(&engine, ZoneLocation::Hand(me()), W0);
    engine
        .apply(me(), PlayerAction::CastSpell { card: siege_hand })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    settle(&mut engine, me());

    // Archangel ETB: APNAP asks active player (me) first, then opponent (them).
    let angel_hand = the(&engine, ZoneLocation::Hand(me()), W1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: angel_hand })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCastMode { player, .. } if *player == me()
    ));
    engine.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCastMode { player, .. } if *player == them()
    ));
    engine.apply(them(), PlayerAction::ChooseMode(1)).unwrap();
    settle(&mut engine, me());

    let my_bear = objects(&engine, ZoneLocation::Battlefield, W2)
        .into_iter()
        .find(|&id| engine.state().object(id).unwrap().controller == me())
        .unwrap();
    let opp_bear = objects(&engine, ZoneLocation::Battlefield, W2)
        .into_iter()
        .find(|&id| engine.state().object(id).unwrap().controller == them())
        .unwrap();
    assert_eq!(
        engine
            .state()
            .object(my_bear)
            .unwrap()
            .characteristics()
            .power,
        Some(5)
    );
    assert_eq!(
        engine
            .state()
            .object(opp_bear)
            .unwrap()
            .characteristics()
            .toughness,
        Some(5)
    );

    // Upkeep trigger fires because mode 0 (Khans) was chosen.
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    assert_eq!(life(&engine, me()), 22);

    // PayUnlessOneOfCostsOr: opponent discards instead of taking 3 damage.
    let torment_hand = the(&engine, ZoneLocation::Hand(me()), W3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: torment_hand })
        .unwrap();
    engine
        .apply(me(), PlayerAction::ChoosePlayer(them()))
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let h_pay = engine.snapshot_hash();
    engine.apply(them(), PlayerAction::ChooseMode(1)).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards { player, .. } if *player == them()
    ));
    let opp_card = the(&engine, ZoneLocation::Hand(them()), W2);
    engine
        .apply(
            them(),
            PlayerAction::ChooseObjects {
                objects: vec![opp_card],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_ne!(h_pay, engine.snapshot_hash());
    assert_eq!(life(&engine, them()), 20);
    assert_eq!(objects(&engine, ZoneLocation::Hand(them()), W2).len(), 0);

    // 3P Prisoner's Dilemma secret choices -> mixed (Silence takes 12, Snitch takes 0).
    let mut g3 = bench(
        18_021,
        vec![pd],
        [
            Seat::default().holding(&[W4]),
            Seat::default(),
            Seat::default(),
        ],
    );
    to_main(&mut g3, me());
    let pd_hand = the(&g3, ZoneLocation::Hand(me()), W4);
    g3.apply(me(), PlayerAction::CastSpell { card: pd_hand })
        .unwrap();
    walk_until(&mut g3, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let h_pd1 = g3.snapshot_hash();
    g3.apply(PlayerId::new(1), PlayerAction::ChooseMode(0))
        .unwrap();
    let h_pd2 = g3.snapshot_hash();
    assert_ne!(h_pd1, h_pd2);
    g3.apply(PlayerId::new(2), PlayerAction::ChooseMode(1))
        .unwrap();
    settle(&mut g3, me());
    assert_eq!(life(&g3, PlayerId::new(1)), 8);
    assert_eq!(life(&g3, PlayerId::new(2)), 20);
}

#[test]
fn wave18b_bookkeeping_effects_work_end_to_end() {
    static CREEP_AB: &[AbilityDef] = &[free(&[Effect::CreepingDread { damage: 3 }], None)];
    static FLICKER_AB: &[AbilityDef] = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        free(&[Effect::Flickerform], None),
    ];
    static BUFF_AURA_AB: &[AbilityDef] = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 0)),
    ];
    static GRIMOIRE_AB: &[AbilityDef] = &[free(
        &[Effect::PhyrexianGrimoire {
            chooser: PlayerRel::Chosen,
        }],
        Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )];
    static PACT_AB: &[AbilityDef] = &[free(&[Effect::TaintedPact], None)];
    static AURA_FILTER: Filter = Filter::HasSubtype(enchantment::AURA);
    static SOV_AB: &[AbilityDef] = &[triggered!(
        Trigger::AttacksAlone(&Filter::YOUR_CREATURE),
        &[Effect::SearchLibraryAttachedToEvent {
            filter: &AURA_FILTER,
            optional: true,
        }]
    )];
    static AURA_SUBS: &[baylee_core::ids::SubtypeId] = &[enchantment::AURA];

    let creep = card(
        W0,
        face("Creep", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        CREEP_AB,
    );
    let flicker = card(
        W1,
        FaceDef {
            subtypes: AURA_SUBS,
            ..face("Flicker Aura", "{0}", TypeSet::ENCHANTMENT)
        },
        KeywordSet::EMPTY,
        FLICKER_AB,
    );
    let buff_aura = card(
        W2,
        FaceDef {
            subtypes: AURA_SUBS,
            ..face("Strength Aura", "{0}", TypeSet::ENCHANTMENT)
        },
        KeywordSet::EMPTY,
        BUFF_AURA_AB,
    );
    let bear = card(
        W3,
        creature_face("Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let grimoire = card(
        W4,
        face("Grimoire", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        GRIMOIRE_AB,
    );
    let pact = card(
        W5,
        face("Pact", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PACT_AB,
    );
    let sov = card(
        W6,
        creature_face("Sov", "{0}", 4, 5),
        KeywordSet::EMPTY,
        SOV_AB,
    );
    let relic = card(
        W7,
        face("Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let shock = card(
        W8,
        face("Shock", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        &[],
    );
    let knight = card(
        W9,
        creature_face("Knight", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let second_relic = card(
        W10,
        face("Relic Two", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );

    // Put all needed probe cards cleanly in starting zones so no raw object spawning is required.
    let mut engine = bench(
        18_030,
        vec![
            creep,
            flicker,
            buff_aura,
            bear,
            grimoire,
            pact,
            sov,
            relic,
            shock,
            knight,
            second_relic,
        ],
        [
            Seat::with(&[W0, W3, W4, W5, W9, W3]).holding(&[W1, W2, W7, W8, W8, W7]),
            Seat::default().holding(&[W8, W10]),
        ],
    );
    to_main(&mut engine, me());

    // 1a. CreepingDread negative: me() discards Relic (Artifact), them() discards Shock (Instant) -> no life loss.
    let creep_id = the(&engine, ZoneLocation::Battlefield, W0);
    let my_relics = objects(&engine, ZoneLocation::Hand(me()), W7);
    let opp_shock = the(&engine, ZoneLocation::Hand(them()), W8);
    activate(&mut engine, me(), creep_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards { player, prompt: ChoicePrompt::Discard, .. } if *player == me()
    ));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![my_relics[0]],
            },
        )
        .unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards { player, prompt: ChoicePrompt::Discard, .. } if *player == them()
    ));
    engine
        .apply(
            them(),
            PlayerAction::ChooseObjects {
                objects: vec![opp_shock],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 20);

    // 1b. CreepingDread positive: both discard Artifact (my_relics[1] vs second_relic) -> them() loses 3 life!
    let opp_relic = the(&engine, ZoneLocation::Hand(them()), W10);
    activate(&mut engine, me(), creep_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![my_relics[1]],
            },
        )
        .unwrap();
    engine
        .apply(
            them(),
            PlayerAction::ChooseObjects {
                objects: vec![opp_relic],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 17);

    // 2. Phyrexian Grimoire: move Bear + Knight into my graveyard, opp exiles Bear, Knight returns to hand.
    let bears = objects(&engine, ZoneLocation::Battlefield, W3);
    let bear_gy = bears[0];
    let host_bear = bears[1];
    let knight_id = the(&engine, ZoneLocation::Battlefield, W9);
    move_to_graveyard(&mut engine, me(), knight_id);
    move_to_graveyard(&mut engine, me(), bear_gy);
    let grimoire_id = the(&engine, ZoneLocation::Battlefield, W4);
    activate(&mut engine, me(), grimoire_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards { player, min: 1, max: 1, .. } if *player == them()
    ));
    engine
        .apply(
            them(),
            PlayerAction::ChooseObjects {
                objects: vec![bear_gy],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(bear_gy).unwrap().zone,
        crate::zone::Zone::Exile
    );
    assert_eq!(
        engine.state().object(knight_id).unwrap().zone,
        crate::zone::Zone::Hand
    );

    // 3. Tainted Pact: put [Shock (top-2), Knight (top-1), Shock (top)] -> auto-stops on duplicate Shock!
    let my_shocks = objects(&engine, ZoneLocation::Hand(me()), W8);
    let top_shock = my_shocks[0];
    let dup_shock = my_shocks[1];
    move_to_top_of_library(&mut engine, me(), dup_shock);
    move_to_top_of_library(&mut engine, me(), knight_id);
    move_to_top_of_library(&mut engine, me(), top_shock);

    let pact_id = the(&engine, ZoneLocation::Battlefield, W5);
    activate(&mut engine, me(), pact_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let h_pact = engine.snapshot_hash();
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseCards { player, .. } if *player == me()));
    assert_ne!(h_pact, engine.snapshot_hash());
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(dup_shock).unwrap().zone,
        crate::zone::Zone::Exile
    );

    // 4. Flickerform: attach Flicker + Strength Aura to host_bear, activate Flickerform, verify end-step return & attach!
    let flicker_hand = the(&engine, ZoneLocation::Hand(me()), W1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: flicker_hand })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![host_bear],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    let buff_hand = the(&engine, ZoneLocation::Hand(me()), W2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: buff_hand })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![host_bear],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    let flicker_bf = the(&engine, ZoneLocation::Battlefield, W1);
    let buff_bf = the(&engine, ZoneLocation::Battlefield, W2);
    assert_eq!(
        engine
            .state()
            .object(host_bear)
            .unwrap()
            .characteristics()
            .power,
        Some(4)
    );
    activate(&mut engine, me(), flicker_bf, 1);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(host_bear).unwrap().zone,
        crate::zone::Zone::Exile
    );
    assert_eq!(
        engine.state().object(flicker_bf).unwrap().zone,
        crate::zone::Zone::Exile
    );
    assert_eq!(
        engine.state().object(buff_bf).unwrap().zone,
        crate::zone::Zone::Exile
    );

    to_main(&mut engine, them());
    let returned_bear = the(&engine, ZoneLocation::Battlefield, W3);
    assert_eq!(
        engine
            .state()
            .object(returned_bear)
            .unwrap()
            .characteristics()
            .power,
        Some(4)
    );
    assert_eq!(
        engine
            .state()
            .object(the(&engine, ZoneLocation::Battlefield, W1))
            .unwrap()
            .attached_to,
        Some(returned_bear)
    );
    assert_eq!(
        engine
            .state()
            .object(the(&engine, ZoneLocation::Battlefield, W2))
            .unwrap()
            .attached_to,
        Some(returned_bear)
    );

    // 5. SearchLibraryAttachedToEvent (`Sovereigns of Lost Alara`): Sov attacks alone, fetches Buff Aura from library attached!
    let mut g5 = bench(
        18_031,
        vec![sov, buff_aura],
        [Seat::with(&[W6]).holding(&[W2]), Seat::default()],
    );
    to_main(&mut g5, me());
    let lib_aura = the(&g5, ZoneLocation::Hand(me()), W2);
    move_to_top_of_library(&mut g5, me(), lib_aura);
    let sov_bf = the(&g5, ZoneLocation::Battlefield, W6);
    walk_until(&mut g5, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    g5.apply(
        me(),
        PlayerAction::DeclareAttackers {
            attackers: vec![(sov_bf, baylee_core::ids::Defender::Player(them()))],
        },
    )
    .unwrap();
    walk_until(&mut g5, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    assert!(matches!(
        g5.pending(),
        Pending::ChooseCards {
            player,
            prompt: ChoicePrompt::SearchLibrary,
            ..
        } if *player == me()
    ));
    g5.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![lib_aura],
        },
    )
    .unwrap();
    walk_until(&mut g5, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert_eq!(
        g5.state().object(lib_aura).unwrap().attached_to,
        Some(sov_bf)
    );
    assert_eq!(
        g5.state().object(sov_bf).unwrap().characteristics().power,
        Some(6)
    );

    // 6. Faerie Artisans (`Filter::CreatedBySource` + `Rider::CreatedBy`):
    //    creating a new copy exiles the earlier token created by this source.
    static ARTISAN_EXILE: Filter = Filter::And(&[
        Filter::IsToken,
        Filter::CreatedBySource,
        Filter::NotRemembered,
    ]);
    static ARTISAN_AB: &[AbilityDef] = &[triggered!(
        Trigger::EntersBattlefield(&Filter::And(&[
            Filter::CREATURE,
            Filter::Not(&Filter::IsToken),
            Filter::ControlledByOpponent,
        ])),
        &[Effect::CaptureAffected {
            first: &[Effect::CreateTokenCopy {
                target: TargetSpec::EventObject,
                controller: PlayerRel::You,
                amount: Amount::Fixed(1),
                mods: &[baylee_cards_dsl::CopyMod::AddType(TypeSet::ARTIFACT)],
                tapped: false,
                attacking: false,
                haste_until_eot: false,
                cleanup: baylee_cards_dsl::CopyCleanup::None,
            }],
            then: &[Effect::ExileAll {
                filter: &ARTISAN_EXILE,
                linked: false,
            }],
        }]
    )];
    let artisan = card(
        W0,
        creature_face("Artisan Probe", "{3}{U}", 2, 2),
        KeywordSet::EMPTY,
        ARTISAN_AB,
    );
    let mut g6 = bench(
        18_032,
        vec![artisan, bear, knight],
        [Seat::with(&[W0]), Seat::default().holding(&[W3, W9])],
    );
    to_main(&mut g6, them());
    let my_artifact_tokens = |e: &Bench| {
        e.state()
            .battlefield_seen()
            .filter(|&id| {
                e.state().object(id).is_some_and(|o| {
                    o.controller == me()
                        && !o.is_card()
                        && o.characteristics().types.contains(TypeSet::ARTIFACT)
                })
            })
            .collect::<Vec<_>>()
    };
    let opp_bear = the(&g6, ZoneLocation::Hand(them()), W3);
    g6.apply(them(), PlayerAction::CastSpell { card: opp_bear })
        .unwrap();
    settle(&mut g6, them());
    let first_toks = my_artifact_tokens(&g6);
    assert_eq!(first_toks.len(), 1);
    let opp_knight = the(&g6, ZoneLocation::Hand(them()), W9);
    g6.apply(them(), PlayerAction::CastSpell { card: opp_knight })
        .unwrap();
    settle(&mut g6, them());
    let second_toks = my_artifact_tokens(&g6);
    assert_eq!(second_toks.len(), 1);
    assert_ne!(second_toks[0], first_toks[0]);
    assert!(
        g6.state()
            .object(first_toks[0])
            .is_none_or(|o| o.zone != crate::zone::Zone::Battlefield)
    );
}

#[test]
fn wave18b_assist_window_rewind_and_waterbend_activated() {
    static GAIN_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(4),
    }])];
    static WASTES_AB: &[AbilityDef] = &[baylee_cards_dsl::prelude::activated!(
        Cost::TAP,
        &[Effect::mana(ManaColor::Colorless, 1)],
        mana_ability = true
    )];
    static WB2_COST: Cost = Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: &[CostPart::Waterbend(2)],
    };
    static WBX_COST: Cost = Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: &[CostPart::WaterbendX],
    };
    static WB2_AB: &[AbilityDef] = &[
        baylee_cards_dsl::prelude::activated!(
            WB2_COST,
            &[Effect::GainLife {
                amount: Amount::Fixed(2),
            }]
        ),
        baylee_cards_dsl::prelude::activated!(WBX_COST, &[Effect::GainLife { amount: Amount::X }]),
    ];

    let setup_assist = || {
        let assist_spell = card(
            W0,
            FaceDef {
                keywords: KeywordSet::ASSIST,
                ..face("Assist Boon", "{1}{W}", TypeSet::SORCERY)
            },
            KeywordSet::ASSIST,
            GAIN_AB,
        );
        let wastes = card(
            W1,
            face("Wastes Probe", "{0}", TypeSet::LAND),
            KeywordSet::EMPTY,
            WASTES_AB,
        );
        let wb_device = card(
            W2,
            face("Waterbend Totem", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            WB2_AB,
        );
        let artifact_pawn = card(
            W3,
            face("Relic Pawn", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        );

        let mut engine = bench(
            18_040,
            vec![assist_spell, wastes, wb_device, artifact_pawn],
            [
                Seat::with(&[W2, W3]).holding(&[W0]),
                Seat::with(&[W1]).holding(&[]),
            ],
        );
        to_main(&mut engine, me());
        add_pool(&mut engine, me(), ManaColor::White, 1);
        let wastes_id = the(&engine, ZoneLocation::Battlefield, W1);
        let spell_id = the(&engine, ZoneLocation::Hand(me()), W0);
        engine
            .apply(me(), PlayerAction::CastSpell { card: spell_id })
            .unwrap();
        engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
        assert!(matches!(
            engine.pending(),
            Pending::Priority { player, .. } if *player == them()
        ));
        activate(&mut engine, them(), wastes_id, 0);
        assert!(
            engine
                .state()
                .object(wastes_id)
                .unwrap()
                .status
                .contains(Status::TAPPED)
        );
        engine.apply(them(), PlayerAction::PassPriority).unwrap();
        assert!(matches!(
            engine.pending(),
            Pending::ChooseNumber {
                player,
                min: 0,
                max: 1,
                reason: crate::choice::NumberPrompt::Assist,
            } if *player == them()
        ));
        (engine, spell_id, wastes_id)
    };

    // 1a. Helper chooses 0 -> caster is {1} short -> CR 734.1 rewinds cast & helper's tap.
    let (mut cancelled, spell_c, wastes_c) = setup_assist();
    cancelled
        .apply(them(), PlayerAction::ChooseNumber(0))
        .unwrap();
    assert_eq!(
        cancelled.state().object(spell_c).unwrap().zone,
        crate::zone::Zone::Hand
    );
    assert!(
        !cancelled
            .state()
            .object(wastes_c)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "helper's mana ability must rewind on CR 734.1 cast cancellation"
    );
    assert_eq!(cancelled.state().players[1].mana_pool.total(), 0);
    assert!(matches!(
        cancelled.pending(),
        Pending::Priority { player, .. } if *player == me()
    ));

    // 1b. Helper pays 1 -> spell resolves.
    let (mut engine, _, _) = setup_assist();
    engine.apply(them(), PlayerAction::ChooseNumber(1)).unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 24);
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);

    // 2. Activated CostPart::Waterbend(2): choosing 0 taps and paying purely {2} mana.
    let device_id = the(&engine, ZoneLocation::Battlefield, W2);
    let pawn_id = the(&engine, ZoneLocation::Battlefield, W3);
    add_pool(&mut engine, me(), ManaColor::Colorless, 4);
    activate(&mut engine, me(), device_id, 0);
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards {
            player,
            min: 0,
            prompt: ChoicePrompt::Waterbend,
            ..
        } if *player == me()
    ));
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 26);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(
        !engine
            .state()
            .object(pawn_id)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    // 3. Activated CostPart::WaterbendX: X = 3, tap 1 artifact + pay {2} mana -> +3 life.
    activate(&mut engine, me(), device_id, 1);
    assert!(matches!(
        engine.pending(),
        Pending::ChooseNumber { player, .. } if *player == me()
    ));
    engine.apply(me(), PlayerAction::ChooseNumber(3)).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards {
            player,
            min: 0,
            prompt: ChoicePrompt::Waterbend,
            ..
        } if *player == me()
    ));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![pawn_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 29);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(
        engine
            .state()
            .object(pawn_id)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}

#[test]
fn wave18b_snapshot_hash_and_fingerprint_sensitivity() {
    static WB_COUNTER_1_AB: &[AbilityDef] = &[spell!(
        &[Effect::PlayerMayWaterbendOr {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::Fixed(1),
            effect: &Effect::CounterTargetSpell,
        }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )];
    static WB_COUNTER_2_AB: &[AbilityDef] = &[spell!(
        &[Effect::PlayerMayWaterbendOr {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::Fixed(2),
            effect: &Effect::CounterTargetSpell,
        }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )];
    static FLICKER_AB: &[AbilityDef] = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        free(&[Effect::Flickerform], None),
    ];
    static BUFF_AB: &[AbilityDef] = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 0)),
    ];
    static ASSIST_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];
    static WASTES_AB: &[AbilityDef] = &[baylee_cards_dsl::prelude::activated!(
        Cost::TAP,
        &[Effect::mana(ManaColor::Colorless, 1)],
        mana_ability = true
    )];
    static AURA_SUBS: &[baylee_core::ids::SubtypeId] = &[enchantment::AURA];
    static SIEGE_MODS: &[EnterModifier] = &[EnterModifier::ChooseMode(&["Khans", "Dragons"])];
    static ARCHANGEL_MODS: &[EnterModifier] =
        &[EnterModifier::EachPlayerChooseMode(&["War", "Peace"])];

    let bear = card(
        W0,
        creature_face("Bear", "{0}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let wb1 = card(
        W1,
        face("Quash 1", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        WB_COUNTER_1_AB,
    );
    let wb2 = card(
        W2,
        face("Quash 2", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        WB_COUNTER_2_AB,
    );
    let pawn = card(
        W3,
        face("Pawn", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let flicker = card(
        W4,
        FaceDef {
            subtypes: AURA_SUBS,
            ..face("Flickerform", "{0}", TypeSet::ENCHANTMENT)
        },
        KeywordSet::EMPTY,
        FLICKER_AB,
    );
    let buff = card(
        W5,
        FaceDef {
            subtypes: AURA_SUBS,
            ..face("Buff Aura", "{0}", TypeSet::ENCHANTMENT)
        },
        KeywordSet::EMPTY,
        BUFF_AB,
    );
    let assist = card(
        W6,
        FaceDef {
            keywords: KeywordSet::ASSIST,
            ..face("Assist", "{1}", TypeSet::SORCERY)
        },
        KeywordSet::ASSIST,
        ASSIST_AB,
    );
    let wastes = card(
        W7,
        face("Wastes", "{0}", TypeSet::LAND),
        KeywordSet::EMPTY,
        WASTES_AB,
    );
    let siege = card(
        W8,
        FaceDef {
            enter_modifiers: SIEGE_MODS,
            ..face("Siege", "{0}", TypeSet::ENCHANTMENT)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let angel = card(
        W9,
        FaceDef {
            enter_modifiers: ARCHANGEL_MODS,
            ..creature_face("Angel", "{0}", 2, 2)
        },
        KeywordSet::EMPTY,
        &[],
    );

    // 1. AwaitingOp::PlayerMayWaterbendConfirm (mana=1 vs mana=2) & PlayerMayWaterbendTaps.
    let setup_wb = |counter_w: u32| {
        let mut e = bench(
            18_050,
            vec![bear, wb1, wb2, pawn],
            [
                Seat::with(&[W3, W3]).holding(&[W0]),
                Seat::default().holding(&[W1, W2]),
            ],
        );
        to_main(&mut e, me());
        let spell = the(&e, ZoneLocation::Hand(me()), W0);
        e.apply(me(), PlayerAction::CastSpell { card: spell })
            .unwrap();
        let stack_spell = the(&e, ZoneLocation::Stack, W0);
        e.apply(me(), PlayerAction::PassPriority).unwrap();
        let counter = the(&e, ZoneLocation::Hand(them()), counter_w);
        e.apply(them(), PlayerAction::CastSpell { card: counter })
            .unwrap();
        e.apply(
            them(),
            PlayerAction::ChooseObjects {
                objects: vec![stack_spell],
            },
        )
        .unwrap();
        walk_until(&mut e, |st| matches!(st.pending(), Pending::YesNo { .. }));
        e
    };
    let g_wb1 = setup_wb(W1);
    let mut g_wb2 = setup_wb(W2);
    assert_ne!(g_wb1.snapshot_hash(), g_wb2.snapshot_hash());
    assert_ne!(g_wb1.fingerprint(), g_wb2.fingerprint());
    let h_confirm = g_wb2.snapshot_hash();
    let fp_confirm = g_wb2.fingerprint();
    g_wb2.apply(me(), PlayerAction::YesNo(true)).unwrap();
    assert!(matches!(
        g_wb2.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::Waterbend,
            ..
        }
    ));
    assert_ne!(h_confirm, g_wb2.snapshot_hash());
    assert_ne!(fp_confirm, g_wb2.fingerprint());

    // 2. DelayedAction::FlickerformReturn: 0 extra auras vs 1 extra aura changes snapshot_hash/fingerprint.
    let setup_flicker = |with_buff: bool| {
        let mut e = bench(
            18_051,
            vec![bear, flicker, buff],
            [Seat::with(&[W0]).holding(&[W4, W5]), Seat::default()],
        );
        to_main(&mut e, me());
        let bear_id = the(&e, ZoneLocation::Battlefield, W0);
        let fl_hand = the(&e, ZoneLocation::Hand(me()), W4);
        e.apply(me(), PlayerAction::CastSpell { card: fl_hand })
            .unwrap();
        e.apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bear_id],
            },
        )
        .unwrap();
        settle(&mut e, me());
        if with_buff {
            let buff_hand = the(&e, ZoneLocation::Hand(me()), W5);
            e.apply(me(), PlayerAction::CastSpell { card: buff_hand })
                .unwrap();
            e.apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![bear_id],
                },
            )
            .unwrap();
            settle(&mut e, me());
        }
        let fl_id = the(&e, ZoneLocation::Battlefield, W4);
        activate(&mut e, me(), fl_id, 1);
        settle(&mut e, me());
        e
    };
    let g_flick_a = setup_flicker(false);
    let g_flick_b = setup_flicker(true);
    assert_ne!(g_flick_a.snapshot_hash(), g_flick_b.snapshot_hash());
    assert_ne!(g_flick_a.fingerprint(), g_flick_b.fingerprint());

    // 3. CastWizard.assist / PaymentContinuation::AssistMana + EnterModifier::{ChooseMode, EachPlayerChooseMode}.
    let mut g_assist = bench(
        18_052,
        vec![assist, wastes],
        [
            Seat::default().holding(&[W6]),
            Seat::with(&[W7]).holding(&[]),
        ],
    );
    to_main(&mut g_assist, me());
    let a_hand = the(&g_assist, ZoneLocation::Hand(me()), W6);
    g_assist
        .apply(me(), PlayerAction::CastSpell { card: a_hand })
        .unwrap();
    g_assist.apply(me(), PlayerAction::YesNo(true)).unwrap();
    let h_before_tap = g_assist.snapshot_hash();
    let fp_before_tap = g_assist.fingerprint();
    let w_id = the(&g_assist, ZoneLocation::Battlefield, W7);
    activate(&mut g_assist, them(), w_id, 0);
    assert_ne!(h_before_tap, g_assist.snapshot_hash());
    assert_ne!(fp_before_tap, g_assist.fingerprint());

    // Suspended EnterModifier::ChooseMode & EachPlayerChooseMode distinct across chosen modes.
    let mut g_siege = bench(
        18_053,
        vec![siege],
        [Seat::default().holding(&[W8]), Seat::default()],
    );
    to_main(&mut g_siege, me());
    let siege_hand = the(&g_siege, ZoneLocation::Hand(me()), W8);
    g_siege
        .apply(me(), PlayerAction::CastSpell { card: siege_hand })
        .unwrap();
    walk_until(&mut g_siege, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let h_choose_mode = g_siege.snapshot_hash();
    let fp_choose_mode = g_siege.fingerprint();
    assert_ne!(h_choose_mode, 0);
    g_siege.apply(me(), PlayerAction::ChooseMode(0)).unwrap();
    settle(&mut g_siege, me());
    assert_ne!(h_choose_mode, g_siege.snapshot_hash());
    assert_ne!(fp_choose_mode, g_siege.fingerprint());

    let setup_angel = |mode: usize| {
        let mut e = bench(
            18_054,
            vec![angel],
            [Seat::default().holding(&[W9]), Seat::default()],
        );
        to_main(&mut e, me());
        let angel_hand = the(&e, ZoneLocation::Hand(me()), W9);
        e.apply(me(), PlayerAction::CastSpell { card: angel_hand })
            .unwrap();
        walk_until(&mut e, |st| {
            matches!(st.pending(), Pending::ChooseCastMode { .. })
        });
        e.apply(me(), PlayerAction::ChooseMode(mode)).unwrap();
        e
    };
    let g_m0 = setup_angel(0);
    let g_m1 = setup_angel(1);
    assert!(matches!(
        g_m0.pending(),
        Pending::ChooseCastMode { player, .. } if *player == them()
    ));
    assert_ne!(g_m0.snapshot_hash(), g_m1.snapshot_hash());
    assert_ne!(g_m0.fingerprint(), g_m1.fingerprint());
}
