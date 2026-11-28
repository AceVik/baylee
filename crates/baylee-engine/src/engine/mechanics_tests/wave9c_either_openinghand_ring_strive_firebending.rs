//! Wave 9c engine rule tests:
//! 1. `CostPart::Either(&'static Cost, &'static Cost)` (`CR 118.8`): spell
//!    additional costs and activated costs with two branches, prompting via
//!    `YesNoPrompt::ChooseFirstCostBranch` when both branches are payable and
//!    auto-selecting the sole payable branch when only one can be paid.
//! 2. Pre-game opening-hand actions (`CR 103.6a–b`): `OpeningHandOnBattlefield`
//!    (*Leylines*, available to starting and non-starting players) and
//!    `OpeningHandRevealAtFirstUpkeep` (*Chancellors* / *Sphinx of Foresight*,
//!    revealing until turn 1 begins and firing at the first upkeep).
//! 3. `The Ring Tempts You` (`CR 701.54a–e`): per-player `ring_temptations` and
//!    `ring_bearer`, 0-creature temptation trigger, mandatory
//!    `ChoicePrompt::ChooseRingBearer`, `Filter::RingBearer`,
//!    `Condition::HasRingBearer`, all 4 cumulative Ring emblem tiers, and
//!    clearing on LTB or control change.
//! 4. `Strive` (`CR 207.2c`, `CR 601.2f`): `AbilityDef::PerExtraTargetCost`
//!    adding `(targets.saturating_sub(1)) * cost` before generic reductions.
//! 5. `Firebending N` (`CR 702.189a–b`): `KeywordSet::FIREBENDING`,
//!    `Effect::Firebend(Amount)`, and `Trigger::Firebends(PlayerRel)` adding
//!    combat-sustained `{R}` mana that survives combat steps and drains at end
//!    of combat.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::{ArrangePrompt, ChoicePrompt, YesNoPrompt};
use crate::turn::Step;
use baylee_cards_dsl::prelude::{activated, cost, mana, spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, Condition, Cost, CostPart, Duration, Filter, Modifier, PlayerRel, TargetChooser,
    TargetReq, TargetSpec, Trigger,
};
use baylee_core::ids::Defender;
use baylee_core::mana::ManaColor;
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
    SeatController, SeatSpec,
};
use baylee_core::types::SupertypeSet;

const SPARK_HARVEST_LIKE: u32 = 8901;
const EITHER_ALTAR: u32 = 8902;
const VANILLA_FODDER: u32 = 8903;
const LEYLINE_AURA: u32 = 8904;
const CHANCELLOR_SPIRE: u32 = 8905;
const RING_SORCERY: u32 = 8906;
const RING_WATCHER: u32 = 8907;
const HALFLING_SCOUT: u32 = 8908;
const KNIGHT_ALLY: u32 = 8909;
const BIG_OGRE: u32 = 8910;
const TOUGH_WALL_GOBLIN: u32 = 8911;
const STEAL_CREATURE_SORCERY: u32 = 8912;
const STRIVE_FLAME: u32 = 8913;
const RUBY_GOBLIN_REDUCER: u32 = 8914;
const FIREBENDER_MONK: u32 = 8915;
const FIREBEND_ASCENSION: u32 = 8916;
const COMBAT_INSTANT_BOLT: u32 = 8917;
const LIFE_OR_TWO_SPELL: u32 = 8918;

fn leak(id: u32, face: FaceDef) -> &'static CardDef {
    let keywords = face.keywords;
    let abilities = face.abilities;
    card(id, face, keywords, abilities)
}

fn refresh_offer(engine: &mut Bench) {
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

fn drain_stack(engine: &mut Bench) {
    while !engine.state.zones.stack_is_empty()
        || matches!(engine.pending(), Pending::Arrange { .. })
    {
        match engine.pending().clone() {
            Pending::Arrange {
                player,
                cards,
                prompt: ArrangePrompt::TriggerOrder,
                ..
            } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("order simultaneous triggers");
            }
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("pass priority");
            }
            other => panic!("unexpected pending while draining stack: {other:?}"),
        }
    }
}

fn spark_harvest_like() -> &'static CardDef {
    static BRANCH_A: Cost = cost!("", Sacrifice(&Filter::CREATURE));
    static BRANCH_B: Cost = cost!("{3}{B}");
    static MANDATORY: [CostPart; 1] = [CostPart::Either(&BRANCH_A, &BRANCH_B)];
    static ABILITIES: [AbilityDef; 1] = [spell!(&[Effect::GainLife {
        amount: Amount::Fixed(4),
    }])];
    leak(
        SPARK_HARVEST_LIKE,
        FaceDef {
            mandatory_additional_costs: &MANDATORY,
            abilities: &ABILITIES,
            ..face("Spark Harvest Probe", "{B}", TypeSet::SORCERY)
        },
    )
}

fn life_or_two_spell() -> &'static CardDef {
    static BRANCH_A: Cost = cost!("", PayLife(5));
    static BRANCH_B: Cost = cost!("{2}");
    static MANDATORY: [CostPart; 1] = [CostPart::Either(&BRANCH_A, &BRANCH_B)];
    static ABILITIES: [AbilityDef; 1] = [spell!(&[Effect::DrawCards {
        amount: Amount::Fixed(1),
    }])];
    leak(
        LIFE_OR_TWO_SPELL,
        FaceDef {
            mandatory_additional_costs: &MANDATORY,
            abilities: &ABILITIES,
            ..face("Life or Two Probe", "{G}", TypeSet::INSTANT)
        },
    )
}

fn either_altar() -> &'static CardDef {
    static ACT_A: Cost = cost!("", PayLife(2));
    static ACT_B: Cost = cost!("", Sacrifice(&Filter::CREATURE));
    static PARTS: [CostPart; 2] = [CostPart::TapSelf, CostPart::Either(&ACT_A, &ACT_B)];
    static ABILITIES: [AbilityDef; 1] = [activated!(
        Cost {
            mana: ManaCost::ZERO,
            parts: &PARTS,
        },
        &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }]
    )];
    leak(
        EITHER_ALTAR,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Either Altar", "{2}", TypeSet::ARTIFACT)
        },
    )
}

fn vanilla_fodder() -> &'static CardDef {
    leak(VANILLA_FODDER, creature_face("Vanilla Fodder", "{1}", 1, 1))
}

fn leyline_aura() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 2] = [
        AbilityDef::OPENING_HAND_ON_BATTLEFIELD,
        static_ability!(Filter::YOUR_CREATURE, Modifier::ModifyPT(1, 1)),
    ];
    leak(
        LEYLINE_AURA,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Leyline of Might", "{2}{G}{G}", TypeSet::ENCHANTMENT)
        },
    )
}

fn chancellor_spire() -> &'static CardDef {
    static REVEAL_FX: [Effect; 1] = [Effect::GainLife {
        amount: Amount::Fixed(3),
    }];
    static ABILITIES: [AbilityDef; 1] =
        [AbilityDef::opening_hand_reveal_at_first_upkeep(&REVEAL_FX)];
    leak(
        CHANCELLOR_SPIRE,
        FaceDef {
            abilities: &ABILITIES,
            ..creature_face("Chancellor Probe", "{4}{W}{W}{W}", 5, 7)
        },
    )
}

fn ring_sorcery() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [spell!(&[Effect::RingTemptsYou])];
    leak(
        RING_SORCERY,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Call of the Ring Probe", "{W}", TypeSet::SORCERY)
        },
    )
}

fn ring_watcher() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 3] = [
        triggered!(
            Trigger::RingTemptsYou {
                who: PlayerRel::You,
            },
            &[Effect::GainLife {
                amount: Amount::EventAmount,
            }]
        ),
        static_ability!(Filter::RingBearer, Modifier::ModifyPT(1, 1)),
        activated!(
            cost!("{0}"),
            &[Effect::GainLife {
                amount: Amount::Fixed(2),
            }],
            condition = Some(Condition::HasRingBearer),
            limit = ActivationLimit::PerTurn(1)
        ),
    ];
    leak(
        RING_WATCHER,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Ring Watcher", "{1}{W}", TypeSet::ENCHANTMENT)
        },
    )
}

fn halfling_scout() -> &'static CardDef {
    leak(
        HALFLING_SCOUT,
        FaceDef {
            keywords: KeywordSet::TRAMPLE.union(KeywordSet::HASTE),
            ..creature_face("Halfling Scout", "{G}", 5, 5)
        },
    )
}

fn knight_ally() -> &'static CardDef {
    leak(KNIGHT_ALLY, creature_face("Knight Ally", "{1}{W}", 2, 2))
}

fn big_ogre() -> &'static CardDef {
    leak(BIG_OGRE, creature_face("Big Ogre", "{3}{R}", 7, 7))
}

fn tough_wall_goblin() -> &'static CardDef {
    leak(
        TOUGH_WALL_GOBLIN,
        FaceDef {
            keywords: KeywordSet::INDESTRUCTIBLE,
            ..creature_face("Tough Wall Goblin", "{1}{R}", 1, 4)
        },
    )
}

fn steal_creature_sorcery() -> &'static CardDef {
    static TAKE: [Effect; 1] = [Effect::continuous(
        &Filter::This,
        Modifier::GainControl,
        Duration::Indefinitely,
    )];
    static ABILITIES: [AbilityDef; 1] = [spell!(
        &TAKE,
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    leak(
        STEAL_CREATURE_SORCERY,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Steal Creature", "{U}", TypeSet::SORCERY)
        },
    )
}

fn strive_flame() -> &'static CardDef {
    static PUMP: [Effect; 1] = [Effect::PumpTarget {
        power: Amount::Fixed(2),
        toughness: Amount::Fixed(0),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }];
    static ABILITIES: [AbilityDef; 2] = [
        AbilityDef::strive(mana!("{2}{R}")),
        spell!(
            &PUMP,
            targets = Some(TargetReq {
                spec: TargetSpec::Object(&Filter::CREATURE),
                min: 1,
                max: 99,
                count_is_x: false,
                distinct_mana_values: false,
                chooser: TargetChooser::Controller,
            })
        ),
    ];
    leak(
        STRIVE_FLAME,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Strive Flame", "{1}{R}", TypeSet::INSTANT)
        },
    )
}

fn ruby_goblin_reducer() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [static_ability!(
        Filter::INSTANT,
        Modifier::SpellsCostLess(2)
    )];
    leak(
        RUBY_GOBLIN_REDUCER,
        FaceDef {
            abilities: &ABILITIES,
            ..creature_face("Ruby Goblin", "{1}{R}", 2, 2)
        },
    )
}

fn firebender_monk() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::Firebend(Amount::Fixed(3))]
    )];
    leak(
        FIREBENDER_MONK,
        FaceDef {
            keywords: KeywordSet::FIREBENDING.union(KeywordSet::HASTE),
            abilities: &ABILITIES,
            ..creature_face("Firebender Monk", "{2}{R}", 2, 3)
        },
    )
}

fn firebend_ascension() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [triggered!(
        Trigger::Firebends(PlayerRel::You),
        &[Effect::GainLife {
            amount: Amount::EventAmount,
        }]
    )];
    leak(
        FIREBEND_ASCENSION,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Firebend Ascension", "{1}{R}", TypeSet::ENCHANTMENT)
        },
    )
}

fn combat_instant_bolt() -> &'static CardDef {
    static ABILITIES: [AbilityDef; 1] = [spell!(&[Effect::DealDamage {
        amount: Amount::Fixed(2),
        target: TargetSpec::Player(PlayerRel::EachOpponent),
    }])];
    leak(
        COMBAT_INSTANT_BOLT,
        FaceDef {
            abilities: &ABILITIES,
            ..face("Combat Instant Bolt", "{2}{R}", TypeSet::INSTANT)
        },
    )
}

fn wave9c_cards() -> Vec<&'static CardDef> {
    vec![
        spark_harvest_like(),
        either_altar(),
        vanilla_fodder(),
        leyline_aura(),
        chancellor_spire(),
        ring_sorcery(),
        ring_watcher(),
        halfling_scout(),
        knight_ally(),
        big_ogre(),
        tough_wall_goblin(),
        steal_creature_sorcery(),
        strive_flame(),
        ruby_goblin_reducer(),
        firebender_monk(),
        firebend_ascension(),
        combat_instant_bolt(),
        life_or_two_spell(),
    ]
}

#[test]
fn cost_part_either_prompts_when_both_payable_auto_selects_sole_branch_and_works_on_activations() {
    let mut engine = bench(
        2,
        wave9c_cards(),
        [
            Seat::with(&[EITHER_ALTAR]).holding(&[
                SPARK_HARVEST_LIKE,
                SPARK_HARVEST_LIKE,
                VANILLA_FODDER,
                VANILLA_FODDER,
            ]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let altar = the(&engine, ZoneLocation::Battlefield, EITHER_ALTAR);
    let harvests = objects(&engine, ZoneLocation::Hand(me()), SPARK_HARVEST_LIKE);
    let (s1, s2) = (harvests[0], harvests[1]);
    let fodders = objects(&engine, ZoneLocation::Hand(me()), VANILLA_FODDER);
    let (f1, f2) = (fodders[0], fodders[1]);

    // 1) Only {B} and 0 creatures -> neither branch A (sacrifice creature) nor branch B ({3}{B}) is payable.
    add_pool(&mut engine, me(), ManaColor::Black, 1);
    assert!(
        !engine.compute_legal(me()).castable.contains(&s1),
        "Spark Harvest must not be castable with {{B}} and 0 creatures"
    );

    // 2) Add {3}{B} more (total 5 black = {3}{B}{B}) with 0 creatures -> auto-selects branch B without prompting!
    add_pool(&mut engine, me(), ManaColor::Black, 4);
    engine
        .apply(me(), PlayerAction::CastSpell { card: s1 })
        .expect("castable via sole payable mana branch");
    assert_eq!(
        engine.state.players[0].mana_pool.total(),
        0,
        "sole payable {{3}}{{B}} branch spent all 5 mana without YesNo prompt"
    );
    drain_stack(&mut engine);
    assert_eq!(life(&engine, me()), 24);

    // 3) Cast both fodder creatures onto battlefield, then cast s2 with BOTH {3}{B}{B} AND creature available -> prompts!
    add_pool(&mut engine, me(), ManaColor::Colorless, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: f1 })
        .expect("cast f1");
    drain_stack(&mut engine);
    engine
        .apply(me(), PlayerAction::CastSpell { card: f2 })
        .expect("cast f2");
    drain_stack(&mut engine);

    add_pool(&mut engine, me(), ManaColor::Black, 5);
    engine
        .apply(me(), PlayerAction::CastSpell { card: s2 })
        .expect("start cast with both branches payable");
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::ChooseFirstCostBranch { index: 0 },
                ..
            } if *player == me()
        ),
        "expected ChooseFirstCostBranch prompt when both branches are payable, got {:?}",
        engine.pending()
    );
    // Choose Yes (first branch = sacrifice creature): spends only {B}, leaving 4 mana in pool!
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("choose first branch (sacrifice)");
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![f1] })
        .expect("choose fodder creature");
    assert_eq!(engine.state.object(f1).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state.players[0].mana_pool.total(),
        4,
        "first branch spent only {{B}}, leaving 4 black mana floating"
    );
    drain_stack(&mut engine);
    assert_eq!(life(&engine, me()), 28);

    // 4) Activated ability with CostPart::Either(PayLife(2), Sacrifice(CREATURE)):
    refresh_offer(&mut engine);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: altar,
                ability_index: 0,
            },
        )
        .expect("activate Either Altar with both branches payable");
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::ChooseFirstCostBranch { index: 0 },
            ..
        }
    ));
    // Choose No (second branch = sacrifice creature): life stays 28, f2 sacrificed!
    engine
        .apply(me(), PlayerAction::YesNo(false))
        .expect("choose second branch on activation");
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![f2] })
        .expect("sacrifice f2");
    drain_stack(&mut engine);
    assert_eq!(life(&engine, me()), 28);
    assert_eq!(engine.state.object(f2).unwrap().zone, Zone::Graveyard);
}

/// CR 601.2b, 118.8, 601.2f, 601.2g: casting a `CostPart::Either(PayLife(5), {2})`
/// spell off the pool (`Paying::FromThePool`) with only the base `{G}` floating
/// must still prompt `ChooseFirstCostBranch` when untapped mana sources can
/// produce `{2}` in the CR 601.2g payment window, and must settle silently to
/// the life branch once no untapped mana sources remain.
#[test]
fn either_cost_prompts_when_mana_branch_is_payable_with_untapped_mana_sources() {
    let mut engine = bench(
        2,
        wave9c_cards(),
        [
            Seat::with(&[forest(), forest(), forest()])
                .holding(&[LIFE_OR_TWO_SPELL, LIFE_OR_TWO_SPELL]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let forests = objects(&engine, ZoneLocation::Battlefield, forest());
    assert_eq!(forests.len(), 3);
    let spells = objects(&engine, ZoneLocation::Hand(me()), LIFE_OR_TWO_SPELL);
    let (s1, s2) = (spells[0], spells[1]);

    // Tap one Forest for {G} (pool route), leaving two Forests untapped.
    engine
        .apply(
            me(),
            PlayerAction::ActivateManaAbility { source: forests[0] },
        )
        .expect("tap first Forest");
    engine
        .apply(me(), PlayerAction::CastSpell { card: s1 })
        .expect("start casting off pool with {G} floating");

    // Both branches are payable (5 life or {2} via the two untapped Forests).
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::ChooseFirstCostBranch { index: 0 },
                ..
            } if *player == me()
        ),
        "expected ChooseFirstCostBranch when mana branch can be paid in CR 601.2g window, got {:?}",
        engine.pending()
    );

    // Choose No ({2}) -> payment window opens -> tap the remaining two Forests.
    engine
        .apply(me(), PlayerAction::YesNo(false))
        .expect("choose {2} branch");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == me()),
        "expected CR 601.2g payment window for {{2}}, got {:?}",
        engine.pending()
    );
    engine
        .apply(
            me(),
            PlayerAction::ActivateManaAbility { source: forests[1] },
        )
        .expect("tap second Forest in window");
    engine
        .apply(
            me(),
            PlayerAction::ActivateManaAbility { source: forests[2] },
        )
        .expect("tap third Forest in window");
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("close payment window and finish cast");
    drain_stack(&mut engine);
    assert_eq!(life(&engine, me()), 20, "choosing {{2}} paid no life");

    // Now untap only ONE Forest and float {G}: total reach is {G}{G} (< {2}{G}),
    // so the {2} branch is NOT payable and the second spell settles silently on
    // PayLife(5) without double-counting the Forest across mana_abilities + abilities.
    engine
        .state
        .object_mut(forests[0])
        .unwrap()
        .status
        .remove(crate::object::Status::TAPPED);
    add_pool(&mut engine, me(), ManaColor::Green, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: s2 })
        .expect("cast second spell with {G} floating and only one untapped Forest");
    assert!(
        !matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::ChooseFirstCostBranch { .. },
                ..
            }
        ),
        "must not offer {{2}} when floating {{G}} plus one untapped Forest cannot pay {{2}}{{G}}, got {:?}",
        engine.pending()
    );
    drain_stack(&mut engine);
    assert_eq!(life(&engine, me()), 15, "only PayLife(5) was payable");
}

/// CR 119.4: `face_additional_mana_floor` includes the `{2}` branch of
/// `CostPart::Either(PayLife(5), {2})` whenever the caster cannot pay 5 life
/// (below 5 life, or under a cannot-lose-life lock), and returns `{0}` while
/// 5 life is payable.
#[test]
fn face_additional_mana_floor_includes_mana_branch_when_life_branch_unpayable() {
    let mut engine = bench(
        2,
        wave9c_cards(),
        [
            Seat::with(&[forest(), forest(), forest()])
                .holding(&[LIFE_OR_TWO_SPELL])
                .at(3),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let spell = the(&engine, ZoneLocation::Hand(me()), LIFE_OR_TWO_SPELL);
    let face = &life_or_two_spell().faces[0];

    // At 3 life, PayLife(5) is unpayable (CR 119.4) -> floor is {2}, base is {2}{G}.
    assert_eq!(
        crate::casting::face_additional_mana_floor(&engine.state, me(), spell, face),
        mana!("{2}"),
        "at 3 life the 5-life branch is unpayable, so the floor must include {{2}}"
    );
    assert_eq!(
        crate::casting::face_base_mana_cost(&engine.state, me(), spell, face),
        mana!("{2}{G}"),
        "at 3 life the minimum base mana cost is {{2}}{{G}}"
    );

    // At 5 life, PayLife(5) is legally payable (CR 119.4) -> floor is {0}.
    engine.state.players[0].life = 5;
    assert_eq!(
        crate::casting::face_additional_mana_floor(&engine.state, me(), spell, face),
        ManaCost::ZERO,
        "at 5 life PayLife(5) is payable (CR 119.4), so floor is zero"
    );

    // At 1 life, PayLife(5) is unpayable -> floor is {2}.
    engine.state.players[0].life = 1;
    assert_eq!(
        crate::casting::face_additional_mana_floor(&engine.state, me(), spell, face),
        mana!("{2}"),
        "at 1 life the floor must include {{2}}"
    );
}

#[test]
fn opening_hand_leyline_and_chancellor_reveal_actions_obey_cr_103_6() {
    let mk_entry = |card: u32| DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: CardIndex::new(card),
        print: PrintRef::new(0),
    };
    let deck: Vec<DeckEntry> = (0..60).map(|_| mk_entry(forest())).collect();
    let mk_seat = |hand: Vec<u32>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: SeatCapabilities {
            dev_commands: true,
            see_hidden: false,
        },
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: Some(hand.into_iter().map(mk_entry).collect()),
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    let preset = GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format: FormatId::Freeform,
        seed: 7,
        house_rules: HouseRules {
            starting_player: Some(0),
            ..HouseRules::default()
        },
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![
            mk_seat(vec![
                LEYLINE_AURA,
                CHANCELLOR_SPIRE,
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ]),
            mk_seat(vec![
                LEYLINE_AURA,
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ]),
        ],
    };
    let mut engine =
        Engine::new(&preset, SyntheticLookup::new(wave9c_cards())).expect("new engine");

    let p0_leyline = the(&engine, ZoneLocation::Hand(me()), LEYLINE_AURA);
    let p0_chancellor = the(&engine, ZoneLocation::Hand(me()), CHANCELLOR_SPIRE);
    let p1_leyline = the(&engine, ZoneLocation::Hand(them()), LEYLINE_AURA);

    engine
        .apply(me(), PlayerAction::MulliganKeep)
        .expect("p0 keeps");
    engine
        .apply(them(), PlayerAction::MulliganKeep)
        .expect("p1 keeps");

    // Starting player me() is asked OpeningHandBattlefield for Leyline and OpeningHandReveal for Chancellor!
    let mut saw_p0_leyline = false;
    let mut saw_p0_chancellor = false;
    while matches!(engine.pending(), Pending::YesNo { player, .. } if *player == me()) {
        match engine.pending().clone() {
            Pending::YesNo {
                prompt: YesNoPrompt::OpeningHandBattlefield,
                ..
            } => {
                saw_p0_leyline = true;
                engine
                    .apply(me(), PlayerAction::YesNo(true))
                    .expect("p0 puts Leyline onto battlefield");
            }
            Pending::YesNo {
                prompt: YesNoPrompt::OpeningHandReveal,
                ..
            } => {
                saw_p0_chancellor = true;
                engine
                    .apply(me(), PlayerAction::YesNo(true))
                    .expect("p0 reveals Chancellor");
                assert!(
                    engine.state.hand_card_revealed(p0_chancellor),
                    "CR 103.6b: Chancellor remains revealed until the first turn begins"
                );
            }
            other => panic!("unexpected p0 opening hand prompt: {other:?}"),
        }
    }
    assert!(saw_p0_leyline, "starting player p0 must be offered Leyline");
    assert!(
        saw_p0_chancellor,
        "starting player p0 must be offered Chancellor reveal"
    );
    assert_eq!(
        engine.state.object(p0_leyline).unwrap().zone,
        Zone::Battlefield,
        "p0's Leyline began the game on the battlefield"
    );

    // Non-starting player them() declines Leyline -> stays in hand, turn 1 begins!
    while matches!(engine.pending(), Pending::YesNo { player, .. } if *player == them()) {
        engine
            .apply(them(), PlayerAction::YesNo(false))
            .expect("p1 declines Leyline");
    }
    assert_eq!(engine.state.object(p1_leyline).unwrap().zone, Zone::Hand);
    assert!(
        !engine.state.hand_card_revealed(p0_chancellor),
        "CR 103.6b: opening-hand reveal ends as the first turn begins"
    );

    // At the beginning of the first upkeep, Chancellor's delayed trigger is on the stack!
    drain_stack(&mut engine);
    assert_eq!(
        life(&engine, me()),
        23,
        "Chancellor's first-upkeep delayed trigger gained 3 life"
    );
}

#[test]
fn the_ring_tempts_you_tracks_zero_creature_trigger_all_four_tiers_and_ltb_or_control_clearing() {
    let mut engine = bench(
        2,
        wave9c_cards(),
        [
            Seat::with(&[RING_WATCHER]).holding(&[
                RING_SORCERY,
                RING_SORCERY,
                RING_SORCERY,
                RING_SORCERY,
                HALFLING_SCOUT,
                KNIGHT_ALLY,
            ]),
            Seat::with(&[BIG_OGRE, TOUGH_WALL_GOBLIN]).holding(&[STEAL_CREATURE_SORCERY]),
        ],
    );
    to_main(&mut engine, me());

    let watcher = the(&engine, ZoneLocation::Battlefield, RING_WATCHER);
    let sorceries = objects(&engine, ZoneLocation::Hand(me()), RING_SORCERY);
    let halfling = the(&engine, ZoneLocation::Hand(me()), HALFLING_SCOUT);
    let knight = the(&engine, ZoneLocation::Hand(me()), KNIGHT_ALLY);
    let ogre = the(&engine, ZoneLocation::Battlefield, BIG_OGRE);
    let goblin = the(&engine, ZoneLocation::Battlefield, TOUGH_WALL_GOBLIN);
    let steal = the(&engine, ZoneLocation::Hand(them()), STEAL_CREATURE_SORCERY);

    // 1) Temptation #1 with 0 creatures controlled (`CR 701.54d`):
    add_pool(&mut engine, me(), ManaColor::White, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sorceries[0] })
        .expect("cast Ring Sorcery #1");
    drain_stack(&mut engine);
    assert_eq!(
        life(&engine, me()),
        21,
        "Trigger::RingTemptsYou fired with EventAmount = 1 even with 0 creatures"
    );
    assert!(
        !offer(&engine).1.abilities.contains(&(watcher, 2)),
        "Condition::HasRingBearer is false with no Ring-bearer"
    );

    // Cast Halfling Scout (5/5 trample haste) & Knight Ally (2/2):
    add_pool(&mut engine, me(), ManaColor::Green, 1);
    add_pool(&mut engine, me(), ManaColor::White, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: halfling })
        .expect("cast Halfling");
    drain_stack(&mut engine);
    engine
        .apply(me(), PlayerAction::CastSpell { card: knight })
        .expect("cast Knight");
    drain_stack(&mut engine);

    // 2) Temptations #2, #3, #4 choosing Halfling each time (`ChoicePrompt::ChooseRingBearer`):
    for (idx, expected_tempts) in (1..=3).zip(2..=4_i32) {
        add_pool(&mut engine, me(), ManaColor::White, 1);
        engine
            .apply(
                me(),
                PlayerAction::CastSpell {
                    card: sorceries[idx],
                },
            )
            .expect("cast Ring Sorcery");
        engine
            .apply(me(), PlayerAction::PassPriority)
            .expect("me pass");
        engine
            .apply(them(), PlayerAction::PassPriority)
            .expect("them pass");
        assert!(
            matches!(
                engine.pending(),
                Pending::ChooseCards {
                    player,
                    min: 1,
                    max: 1,
                    prompt: ChoicePrompt::ChooseRingBearer,
                    ..
                } if *player == me()
            ),
            "expected mandatory ChooseRingBearer prompt on temptation #{expected_tempts}"
        );
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![halfling],
                },
            )
            .expect("choose Halfling as Ring-bearer");
        drain_stack(&mut engine);
    }
    // Life = 21 + 2 + 3 + 4 = 30!
    assert_eq!(life(&engine, me()), 30);

    // Tier 1 + Filter::RingBearer + Condition::HasRingBearer:
    let h_chars = engine.state.object(halfling).unwrap().characteristics();
    assert!(
        h_chars.supertypes.contains(SupertypeSet::LEGENDARY),
        "Tier 1 makes Ring-bearer legendary"
    );
    assert_eq!(
        h_chars.power,
        Some(6),
        "Filter::RingBearer static +1/+1 pumped Halfling from 5/5 to 6/6"
    );
    assert_eq!(
        engine.state.object(knight).unwrap().characteristics().power,
        Some(2),
        "non-Ring-bearer Knight is not pumped"
    );
    refresh_offer(&mut engine);
    assert!(
        offer(&engine).1.abilities.contains(&(watcher, 2)),
        "Condition::HasRingBearer is true while Halfling is Ring-bearer"
    );

    // Combat: test Tier 1 (7/7 Ogre can't block 6/6 Ring-bearer; 1/4 Indestructible Goblin can),
    // Tier 2 (attack loot), Tier 3 (blocker sacrificed at CombatEnd), Tier 4 (combat damage -> opp loses 3 life)!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(halfling, Defender::Player(them()))],
            },
        )
        .expect("declare Halfling attacker");
    while !engine.state.zones.stack_is_empty() {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .expect("discard 1 card for Ring Tier 2 loot");
            }
            other => panic!("unexpected pending during Tier 2 loot: {other:?}"),
        }
    }
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Graveyard(me())).len(),
        5,
        "4 Ring Sorceries + 1 discarded card from Tier 2 loot"
    );

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                them(),
                PlayerAction::DeclareBlockers {
                    blockers: vec![(ogre, halfling)],
                },
            )
            .is_err(),
        "Tier 1: 7/7 Ogre with greater power must NOT be able to block 6/6 Ring-bearer"
    );
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(goblin, halfling)],
            },
        )
        .expect("1/4 Goblin with lesser power legally blocks 6/6 Ring-bearer");
    drain_stack(&mut engine);

    while !(engine.state.turn.phase == Phase::SecondMain
        && holds_priority_in(&engine, me(), Step::Main))
    {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseNumber { player, min, .. } => {
                engine
                    .apply(player, PlayerAction::ChooseNumber(min))
                    .unwrap();
            }
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .unwrap();
            }
            other => panic!("unexpected combat pending: {other:?}"),
        }
    }
    assert_eq!(
        engine.state.object(goblin).unwrap().zone,
        Zone::Graveyard,
        "Tier 3 sacrificed the indestructible blocking Goblin at end of combat"
    );
    assert_eq!(
        life(&engine, them()),
        15,
        "20 - 2 trample combat damage - 3 life from Tier 4 = 15"
    );

    // Control change clears Ring-bearer (`CR 701.54a`): them() steals Halfling!
    to_main(&mut engine, them());
    add_pool(&mut engine, them(), ManaColor::Blue, 1);
    engine
        .apply(them(), PlayerAction::CastSpell { card: steal })
        .expect("them casts Steal Creature");
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![halfling],
                players: vec![],
            },
        )
        .expect("target Halfling");
    drain_stack(&mut engine);
    assert!(
        !engine
            .state
            .object(halfling)
            .unwrap()
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY),
        "CR 701.54a: losing control of Ring-bearer removes Ring-bearer designation"
    );
    assert_eq!(
        engine
            .state
            .object(halfling)
            .unwrap()
            .characteristics()
            .power,
        Some(5)
    );
}

#[test]
fn strive_scales_mana_cost_per_target_beyond_first_and_interacts_with_reductions() {
    let mut engine = bench(
        2,
        wave9c_cards(),
        [
            Seat::with(&[VANILLA_FODDER, VANILLA_FODDER, VANILLA_FODDER]).holding(&[
                STRIVE_FLAME,
                STRIVE_FLAME,
                STRIVE_FLAME,
                RUBY_GOBLIN_REDUCER,
            ]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let fodders = objects(&engine, ZoneLocation::Battlefield, VANILLA_FODDER);
    let (c1, c2, c3) = (fodders[0], fodders[1], fodders[2]);
    let flames = objects(&engine, ZoneLocation::Hand(me()), STRIVE_FLAME);
    let (sf1, sf2, sf3) = (flames[0], flames[1], flames[2]);
    let reducer = the(&engine, ZoneLocation::Hand(me()), RUBY_GOBLIN_REDUCER);

    // 1) 1 target costs base {1}{R}:
    add_pool(&mut engine, me(), ManaColor::Red, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sf1 })
        .expect("cast Strive Flame for 1 target");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![c1],
                players: vec![],
            },
        )
        .expect("target 1 creature");
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);
    drain_stack(&mut engine);
    assert_eq!(
        engine.state.object(c1).unwrap().characteristics().power,
        Some(3)
    );

    // 2) 3 targets costs {1}{R} + 2 * {2}{R} = {5}{R}{R}{R} (8 total, >= 3 red):
    // With only 5 red mana and 0 lands, choosing 3 targets cannot be paid and cancels the cast (CR 734.2)!
    add_pool(&mut engine, me(), ManaColor::Red, 5);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sf2 })
        .expect("start Strive Flame");
    let _ = engine.apply(
        me(),
        PlayerAction::ChooseTargets {
            objects: vec![c1, c2, c3],
            players: vec![],
        },
    );
    assert_eq!(
        engine.state.object(sf2).unwrap().zone,
        Zone::Hand,
        "3 targets ({{5}}{{R}}{{R}}{{R}}) cannot be paid with 5 mana, rewinding sf2 to hand"
    );
    assert_eq!(engine.state.players[0].mana_pool.total(), 5);

    // Add 3 more colorless mana (total 5R + 3C = 8 mana -> pays {5}{R}{R}{R}):
    add_pool(&mut engine, me(), ManaColor::Colorless, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sf2 })
        .expect("cast Strive Flame with 8 mana");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![c1, c2, c3],
                players: vec![],
            },
        )
        .expect("3 targets paid cleanly with {5}{R}{R}{R}");
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);
    drain_stack(&mut engine);
    assert_eq!(
        engine.state.object(c2).unwrap().characteristics().power,
        Some(3)
    );
    assert_eq!(
        engine.state.object(c3).unwrap().characteristics().power,
        Some(3)
    );

    // 3) Cast Ruby Goblin Reducer (SpellsCostLess(2)): 2 targets costs {1}{R} + {2}{R} - {2} = {1}{R}{R} (3 mana)!
    add_pool(&mut engine, me(), ManaColor::Red, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: reducer })
        .expect("cast Ruby Goblin Reducer");
    drain_stack(&mut engine);

    add_pool(&mut engine, me(), ManaColor::Red, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sf3 })
        .expect("cast Strive Flame under SpellsCostLess(2)");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![c1, c2],
                players: vec![],
            },
        )
        .expect("2 targets reduced from {3}{R}{R} to {1}{R}{R}");
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);
    drain_stack(&mut engine);
}

#[test]
fn firebending_adds_combat_sustained_red_mana_fires_firebends_trigger_and_drains_at_combat_end() {
    let mut engine = bench(
        2,
        wave9c_cards(),
        [
            Seat::with(&[FIREBENDER_MONK, FIREBENDER_MONK, FIREBEND_ASCENSION])
                .holding(&[COMBAT_INSTANT_BOLT]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let monks = objects(&engine, ZoneLocation::Battlefield, FIREBENDER_MONK);
    let (monk1, monk2) = (monks[0], monks[1]);
    let bolt = the(&engine, ZoneLocation::Hand(me()), COMBAT_INSTANT_BOLT);

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (monk1, Defender::Player(them())),
                    (monk2, Defender::Player(them())),
                ],
            },
        )
        .expect("declare both Firebender Monks attackers");
    // Two Firebending 3 triggers resolve -> adds 6 {R} AND triggers Firebend Ascension twice (+6 life -> 26)!
    drain_stack(&mut engine);
    assert_eq!(
        engine.state.players[0].mana_pool.total(),
        6,
        "two Firebending 3 triggers added 6 {{R}}"
    );
    assert_eq!(
        life(&engine, me()),
        26,
        "Trigger::Firebends fired twice with EventAmount = 3 each"
    );

    // Advance into DeclareBlockers: 6 {R} survives step transition!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    assert_eq!(
        engine.state.players[0].mana_pool.total(),
        6,
        "Firebending mana does not empty between DeclareAttackers and DeclareBlockers"
    );

    // Advance into CombatDamage: 6 {R} survives into CombatDamage step!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.state.turn.step, Step::CombatDamage);
    assert_eq!(
        engine.state.players[0].mana_pool.total(),
        6,
        "Firebending mana survives into CombatDamage step"
    );

    // Spend 3 {R} on Combat Instant Bolt during CombatDamage, leaving 3 {R} unspent:
    engine
        .apply(me(), PlayerAction::CastSpell { card: bolt })
        .expect("cast {2}{R} instant using Firebending mana in CombatDamage step");
    drain_stack(&mut engine);
    assert_eq!(engine.state.players[0].mana_pool.total(), 3);
    assert_eq!(
        life(&engine, them()),
        14,
        "20 - 4 combat damage - 2 bolt damage = 14"
    );

    // Walk to SecondMain: unspent 3 {R} empties when CombatEnd finishes!
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && holds_priority_in(e, me(), Step::Main)
    });
    assert_eq!(
        engine.state.players[0].mana_pool.total(),
        0,
        "unspent Firebending mana empties at end of combat before SecondMain"
    );
}
