//! Wave 17a deterministic engine rule tests:
//! 1. Generalized trigger-doubling (`ReplacementRule::TriggerMultiplierMatching`, `CR 603.2d`):
//!    - `TriggerCause::Dies(&Filter::CREATURE)` (*Drivnod, Carnage Dominus*, *Teysa Karlov*):
//!      doubles `Dies` AND `LeavesBattlefield` triggers when a creature dies, does NOT double
//!      non-creature artifact destruction or creature exile.
//!    - `TriggerCause::LeavesBattlefield(&Filter::ARTIFACT)` (*Gandalf the White*): doubles LTB
//!      on artifact exile, not non-artifact creature exile.
//!    - `TriggerCause::EntersBattlefield(&Filter::ARTIFACT)` (*Panharmonicon*) +
//!      `TriggerCause::LifeGained(PlayerRel::You)` (*Dr. Beverly Crusher*) +
//!      conditional `TriggerMultiplierMatching` (*Sanctum of All*): artifact land ETB doubles
//!      Landfall while plain land ETB triggers once; your lifegain doubles while opponent lifegain
//!      triggers once; conditional doubler activates only while condition holds.
//!    - `TriggerCause::SpellCastOrCopy` (*Veyran*), `TriggerCause::Attack(&Filter::YOUR_CREATURE)`
//!      (*Wulfgar*), `TriggerCause::Sacrificed`, `TriggerCause::DealtDamage` (*Wayta*), and
//!      `TriggerCause::DealsCombatDamageToPlayer` (*Felix Five-Boots*).
//! 2. Cast-provenance queries (`Filter::{WasCast, WasCastByYou, CastFromZone}`,
//!    `Condition::{SourceWasCast, CastFromOtherThanHand, SourceCastFromZone}`):
//!    - `Condition::SourceWasCast` (*Anti-Venom*): fires when cast, suppressed on uncast reanimation,
//!      and preserves LKI (`CR 603.4`, `CR 608.2h`) when source is destroyed in response.
//!    - `Condition::CastFromOtherThanHand` (*Antiquities on the Loose*): false on hand cast, true on
//!      graveyard Flashback cast, and false on an uncast spell copy (`CR 707.10`).
//!    - `Filter::CastFromZone(ZoneRef::Exile)` (*Doc Aurlock*, *Prosper*): reduces cost on
//!      `projected_spell` during pricing when casting from exile, fires exile `SpellCast` trigger,
//!      and leaves `track_cast_origin_zone` inactive (zero-hash) on boards that do not read it.
//! 3. `Effect::Learn` (`CR 701.46a–b`, `CR 903.11`):
//!    - Wishes only `Lesson` cards from `ZoneLocation::OutsideGame(you)` (`ChoicePrompt::Wish`),
//!      revealing the chosen Lesson into hand.
//!    - Declining the wish (or having no Lessons outside the game) transitions to optional rummage
//!      (`ChoicePrompt::Discard`, `min: 0, max: 1` -> discard 1 to draw 1).
//!    - In `FormatId::Commander` (`CR 903.11`), outside-the-game Lesson cards are ignored and
//!      `Effect::Learn` goes straight to optional rummage.

#![allow(clippy::too_many_lines, clippy::items_after_statements)]

use super::*;
use crate::choice::{CastModeKind, ChoicePrompt};
use crate::event::{Cause, GameEvent};
use crate::turn::Step;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::prelude::{spell, static_ability, triggered};
use baylee_cards_dsl::{
    AltCastPrice, AltCastZones, Amount, Condition, Cost, CostPart, Duration, Effect, Filter,
    KeywordSet, Modifier, PlayerRel, ReplacementRule, TargetReq, TargetSpec, Trigger, TriggerCause,
    ZoneRef,
};
use baylee_core::generated::subtypes;
use baylee_core::ids::Defender;
use baylee_core::mana::ManaColor;
use baylee_core::types::TypeSet;

const W17A: u32 = 17_100;

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

fn move_to_zone(engine: &mut Bench, id: ObjectId, loc: ZoneLocation) {
    engine
        .state
        .move_object(id, loc, ZonePosition::Top, Cause::Effect)
        .expect("move test object");
    refresh_offer(engine);
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

fn settle_stack(engine: &mut Bench, seat: PlayerId) {
    for _ in 0..256 {
        if engine.state().zones.stack_is_empty()
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return;
        }
        match engine.pending().clone() {
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("order simultaneous triggers");
            }
            Pending::ChooseTargets { player, min: 0, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![],
                        },
                    )
                    .expect("keep original target on spell copy");
            }
            pending => {
                assert!(
                    walk_past(engine, &pending),
                    "unexpected pending while settling stack: {pending:?}"
                );
            }
        }
    }
    panic!("failed to settle stack");
}

fn bench_with_sideboard(
    seed: u64,
    format: FormatId,
    cards: Vec<&'static CardDef>,
    seats: [(Seat, Vec<u32>, Vec<u32>); 2],
) -> Bench {
    let deck: Vec<DeckEntry> = (0..99).map(|_| entry(forest())).collect();
    let make_seat = |(s, sb, cmds): (Seat, Vec<u32>, Vec<u32>)| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities {
            dev_commands: true,
            see_hidden: false,
        },
        deck: deck.clone(),
        sideboard: sb.into_iter().map(entry).collect(),
        commanders: cmds.into_iter().map(entry).collect(),
        starting_life: s.life,
        starting_hand: Some(s.hand.into_iter().map(entry).collect()),
        starting_battlefield: s.battlefield.into_iter().map(entry).collect(),
        emblems: vec![],
        team: None,
    };
    let preset = GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format,
        seed,
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
        seats: seats.into_iter().map(make_seat).collect(),
    };
    let mut engine =
        Engine::new(&preset, SyntheticLookup::new(cards)).expect("a game over the bench");
    while let Pending::Mulligan { player, .. } = engine.pending().clone() {
        engine
            .apply(player, PlayerAction::MulliganKeep)
            .expect("a seat keeps its hand");
    }
    engine
}

// =========================================================================
// 1. Generalized Panharmonicon (`ReplacementRule::TriggerMultiplierMatching`)
// =========================================================================

#[test]
fn wave17a_panharmonicon_creature_dies_and_ltb_doubling() {
    const C_DRIVNOD: u32 = W17A;
    const C_WATCHER: u32 = W17A + 1;
    const C_BEAR_A: u32 = W17A + 2;
    const C_BEAR_B: u32 = W17A + 3;
    const C_RELIC_A: u32 = W17A + 4;
    const C_RELIC_B: u32 = W17A + 5;
    const C_ZAPPER: u32 = W17A + 6;
    const C_GANDALF: u32 = W17A + 7;

    static DRIVNOD_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::TriggerMultiplierMatching {
            source_filter: &Filter::ControlledByYou,
            cause: TriggerCause::Dies(&Filter::CREATURE),
            condition: None,
        },
    )];
    static GANDALF_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::TriggerMultiplierMatching {
            source_filter: &Filter::ControlledByYou,
            cause: TriggerCause::LeavesBattlefield(&Filter::ARTIFACT),
            condition: None,
        },
    )];
    static WATCHER_AB: &[AbilityDef] = &[
        triggered!(
            Trigger::Dies(&Filter::Any),
            &[Effect::GainLife {
                amount: Amount::Fixed(2),
            }]
        ),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::Any),
            &[Effect::GainLife {
                amount: Amount::Fixed(1),
            }]
        ),
    ];
    static ZAPPER_AB: &[AbilityDef] = &[
        free(
            &[Effect::destroy(TargetSpec::Object(&Filter::Any))],
            Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
        ),
        free(
            &[Effect::Exile {
                target: TargetSpec::Object(&Filter::Any),
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
        ),
    ];

    let drivnod = card(
        C_DRIVNOD,
        creature_face("Drivnod", "{3}{B}{B}", 8, 3),
        KeywordSet::EMPTY,
        DRIVNOD_AB,
    );
    let gandalf = card(
        C_GANDALF,
        creature_face("Gandalf", "{3}{W}{W}", 4, 5),
        KeywordSet::EMPTY,
        GANDALF_AB,
    );
    let watcher = card(
        C_WATCHER,
        face("Death Watcher", "{1}{B}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        WATCHER_AB,
    );
    let bear_a = card(
        C_BEAR_A,
        creature_face("Bear A", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_b = card(
        C_BEAR_B,
        creature_face("Bear B", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let relic_a = card(
        C_RELIC_A,
        face("Relic A", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let relic_b = card(
        C_RELIC_B,
        face("Relic B", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let zapper = card(
        C_ZAPPER,
        face("Zapper", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        ZAPPER_AB,
    );

    let mut engine = bench(
        17_101,
        vec![
            drivnod, gandalf, watcher, bear_a, bear_b, relic_a, relic_b, zapper,
        ],
        [
            Seat::with(&[
                C_DRIVNOD, C_WATCHER, C_BEAR_A, C_BEAR_B, C_RELIC_A, C_RELIC_B, C_ZAPPER,
            ])
            .holding(&[C_GANDALF]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let zapper_id = the(&engine, ZoneLocation::Battlefield, C_ZAPPER);
    let bear_a_id = the(&engine, ZoneLocation::Battlefield, C_BEAR_A);
    let bear_b_id = the(&engine, ZoneLocation::Battlefield, C_BEAR_B);
    let relic_a_id = the(&engine, ZoneLocation::Battlefield, C_RELIC_A);
    let relic_b_id = the(&engine, ZoneLocation::Battlefield, C_RELIC_B);

    // 1) Creature dies: BOTH `Dies` (+2) and `LeavesBattlefield` (+1) double -> +6 life!
    activate(&mut engine, me(), zapper_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_a_id],
                players: vec![],
            },
        )
        .expect("target bear_a");
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        26,
        "Drivnod must double both Dies and LTB triggers when a creature dies"
    );

    // 2) Non-creature artifact destroyed: BOTH `Dies(&Filter::Any)` (+2) and `LeavesBattlefield` (+1)
    //    fire only ONCE (+3 -> 29, NOT +6 -> 32)!
    activate(&mut engine, me(), zapper_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![relic_a_id],
                players: vec![],
            },
        )
        .expect("target relic_a");
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        29,
        "Drivnod must NOT double non-creature artifact destruction"
    );

    // 3) Creature exiled: `LeavesBattlefield` fires only ONCE (+1 -> 30)!
    activate(&mut engine, me(), zapper_id, 1);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_b_id],
                players: vec![],
            },
        )
        .expect("target bear_b for exile");
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        30,
        "Drivnod must NOT double creature exile"
    );

    // 4) Put Gandalf onto battlefield and exile an artifact -> `LeavesBattlefield` doubles (+2 -> 32)!
    let gandalf_hand = the(&engine, ZoneLocation::Hand(me()), C_GANDALF);
    move_to_zone(&mut engine, gandalf_hand, ZoneLocation::Battlefield);
    activate(&mut engine, me(), zapper_id, 1);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![relic_b_id],
                players: vec![],
            },
        )
        .expect("target relic_b for exile");
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        32,
        "Gandalf must double LTB when an artifact leaves via exile"
    );
}

#[test]
fn wave17a_panharmonicon_filtered_etb_lifegain_and_conditional() {
    const C_PAN: u32 = W17A + 10;
    const C_CRUSHER: u32 = W17A + 11;
    const C_SANCTUM: u32 = W17A + 12;
    const C_LANDFALL: u32 = W17A + 13;
    const C_LIFE_WATCH: u32 = W17A + 14;
    const C_ART_LAND: u32 = W17A + 15;
    const C_PLAIN_LAND: u32 = W17A + 16;
    const C_HEAL: u32 = W17A + 17;
    const C_BAUBLE: u32 = W17A + 18;

    static PAN_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::TriggerMultiplierMatching {
            source_filter: &Filter::ControlledByYou,
            cause: TriggerCause::EntersBattlefield(&Filter::ARTIFACT),
            condition: None,
        },
    )];
    static CRUSHER_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::TriggerMultiplierMatching {
            source_filter: &Filter::ControlledByYou,
            cause: TriggerCause::LifeGained(PlayerRel::You),
            condition: None,
        },
    )];
    static SANCTUM_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::TriggerMultiplierMatching {
            source_filter: &Filter::ControlledByYou,
            cause: TriggerCause::Any,
            condition: Some(Condition::ControlCount(&Filter::ARTIFACT, 3)),
        },
    )];
    static LANDFALL_AB: &[AbilityDef] = &[triggered!(
        Trigger::EntersBattlefield(&Filter::LAND),
        &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }]
    )];
    static LIFE_WATCH_AB: &[AbilityDef] = &[triggered!(
        Trigger::GainsLife(PlayerRel::EachPlayer),
        &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }]
    )];
    static HEAL_AB: &[AbilityDef] = &[
        free(
            &[Effect::GainLife {
                amount: Amount::Fixed(1),
            }],
            None,
        ),
        free(
            &[Effect::GainLifeFor {
                amount: Amount::Fixed(1),
                who: PlayerRel::Opponent,
            }],
            None,
        ),
    ];

    let pan = card(
        C_PAN,
        face("Panharmonicon", "{4}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PAN_AB,
    );
    let crusher = card(
        C_CRUSHER,
        creature_face("Dr. Crusher", "{2}{W}", 2, 2),
        KeywordSet::EMPTY,
        CRUSHER_AB,
    );
    let sanctum = card(
        C_SANCTUM,
        face("Sanctum", "{W}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        SANCTUM_AB,
    );
    let landfall = card(
        C_LANDFALL,
        creature_face("Tracker", "{2}{G}", 2, 2),
        KeywordSet::EMPTY,
        LANDFALL_AB,
    );
    let life_watch = card(
        C_LIFE_WATCH,
        face("Life Watch", "{1}{W}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        LIFE_WATCH_AB,
    );
    let art_land = card(
        C_ART_LAND,
        FaceDef {
            types: TypeSet::ARTIFACT.union(TypeSet::LAND),
            ..land_face("Seat of the Synod")
        },
        KeywordSet::EMPTY,
        &[],
    );
    let plain_land = card(
        C_PLAIN_LAND,
        land_face("Plain Land"),
        KeywordSet::EMPTY,
        &[],
    );
    let heal = card(
        C_HEAL,
        face("Healer", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        HEAL_AB,
    );
    let bauble = card(
        C_BAUBLE,
        face("Bauble", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        17_102,
        vec![
            pan, crusher, sanctum, landfall, life_watch, art_land, plain_land, heal, bauble,
        ],
        [
            Seat::with(&[
                C_PAN,
                C_CRUSHER,
                C_SANCTUM,
                C_LANDFALL,
                C_LIFE_WATCH,
                C_HEAL,
            ])
            .holding(&[C_PLAIN_LAND, C_ART_LAND, C_BAUBLE]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let landfall_id = the(&engine, ZoneLocation::Battlefield, C_LANDFALL);
    let life_watch_id = the(&engine, ZoneLocation::Battlefield, C_LIFE_WATCH);
    let heal_id = the(&engine, ZoneLocation::Battlefield, C_HEAL);

    // Currently control only 1 artifact (Panharmonicon), so Sanctum condition (3+ artifacts) is FALSE.
    // Playing Plain Land triggers Landfall ONCE.
    let plain_id = the(&engine, ZoneLocation::Hand(me()), C_PLAIN_LAND);
    engine
        .apply(me(), PlayerAction::PlayLand { card: plain_id })
        .expect("play plain land");
    settle_stack(&mut engine, me());
    assert_eq!(
        triggered(&engine, landfall_id),
        1,
        "non-artifact land must trigger Landfall once"
    );

    // Reset land drop for the turn and play Artifact Land -> Panharmonicon doubles Landfall (1 + 2 = 3)!
    let art_land_id = the(&engine, ZoneLocation::Hand(me()), C_ART_LAND);
    engine.state.players[0].lands_played_this_turn = 0;
    refresh_offer(&mut engine);
    engine
        .apply(me(), PlayerAction::PlayLand { card: art_land_id })
        .expect("play artifact land");
    settle_stack(&mut engine, me());
    assert_eq!(
        triggered(&engine, landfall_id),
        3,
        "Artifact Land entering must double Landfall via Panharmonicon"
    );

    // Gain life yourself: Crusher doubles `GainsLife` (0 + 2 = 2); opponent gaining life triggers ONCE (2 + 1 = 3)!
    activate(&mut engine, me(), heal_id, 0);
    settle_stack(&mut engine, me());
    assert_eq!(
        triggered(&engine, life_watch_id),
        2,
        "Dr. Crusher must double your lifegain triggers"
    );

    activate(&mut engine, me(), heal_id, 1);
    settle_stack(&mut engine, me());
    assert_eq!(
        triggered(&engine, life_watch_id),
        3,
        "Dr. Crusher must NOT double opponent lifegain"
    );

    // Put 3rd artifact (Bauble) onto battlefield to turn on Sanctum (`ControlCount(&Filter::ARTIFACT, 3)`):
    // now opponent gaining life IS doubled by Sanctum (3 + 2 = 5)!
    let bauble_id = the(&engine, ZoneLocation::Hand(me()), C_BAUBLE);
    move_to_zone(&mut engine, bauble_id, ZoneLocation::Battlefield);
    activate(&mut engine, me(), heal_id, 1);
    settle_stack(&mut engine, me());
    assert_eq!(
        triggered(&engine, life_watch_id),
        5,
        "Conditional Sanctum must double once 3rd artifact is controlled"
    );
}

#[test]
fn wave17a_panharmonicon_spell_cast_copy_attack_sacrifice_and_damage() {
    const C_VEYRAN: u32 = W17A + 20;
    const C_WULFGAR: u32 = W17A + 21;
    const C_SAC_MULT: u32 = W17A + 22;
    const C_WAYTA: u32 = W17A + 23;
    const C_FELIX: u32 = W17A + 24;
    const C_MAGE: u32 = W17A + 25;
    const C_PING_INSTANT: u32 = W17A + 26;
    const C_COPIER: u32 = W17A + 27;
    const C_SAC_OUTLET: u32 = W17A + 28;
    const C_BAUBLE: u32 = W17A + 29;
    const C_ATTACK_WATCH: u32 = W17A + 30;
    const C_OPP_BEAR: u32 = W17A + 31;

    static VEYRAN_AB: &[AbilityDef] = &[
        AbilityDef::Replacement(ReplacementRule::TriggerMultiplierMatching {
            source_filter: &Filter::ControlledByYou,
            cause: TriggerCause::SpellCastOrCopy {
                filter: &Filter::INSTANT_OR_SORCERY,
                who: PlayerRel::You,
                allow_copy: true,
            },
            condition: None,
        }),
        triggered!(
            Trigger::SpellCastOrCopied(&Filter::INSTANT_OR_SORCERY),
            &[Effect::GainLife {
                amount: Amount::Fixed(1),
            }]
        ),
    ];
    static WULFGAR_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::TriggerMultiplierMatching {
            source_filter: &Filter::ControlledByYou,
            cause: TriggerCause::Attack(&Filter::YOUR_CREATURE),
            condition: None,
        },
    )];
    static SAC_MULT_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::TriggerMultiplierMatching {
            source_filter: &Filter::ControlledByYou,
            cause: TriggerCause::Sacrificed {
                filter: &Filter::ARTIFACT,
                who: PlayerRel::You,
            },
            condition: None,
        },
    )];
    static WAYTA_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::TriggerMultiplierMatching {
            source_filter: &Filter::ControlledByYou,
            cause: TriggerCause::DealtDamage(&Filter::YOUR_CREATURE),
            condition: None,
        },
    )];
    static FELIX_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::TriggerMultiplierMatching {
            source_filter: &Filter::ControlledByYou,
            cause: TriggerCause::DealsCombatDamageToPlayer(&Filter::YOUR_CREATURE),
            condition: None,
        },
    )];
    static MAGE_AB: &[AbilityDef] = &[
        triggered!(
            Trigger::Sacrificed {
                filter: &Filter::Any,
                who: PlayerRel::You,
            },
            &[Effect::GainLife {
                amount: Amount::Fixed(10),
            }]
        ),
        triggered!(
            Trigger::DealtDamage(&Filter::This),
            &[Effect::GainLife {
                amount: Amount::Fixed(100),
            }]
        ),
        triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::This),
            &[Effect::GainLife {
                amount: Amount::Fixed(1000),
            }]
        ),
    ];
    static ATTACK_WATCH_AB: &[AbilityDef] = &[triggered!(
        Trigger::Attacks(&Filter::Any),
        &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }]
    )];
    static PING_AB: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static COPIER_AB: &[AbilityDef] = &[free(
        &[Effect::CopyTargetSpell { mods: &[] }],
        Some(TargetReq::one(TargetSpec::Spell(
            &Filter::INSTANT_OR_SORCERY,
        ))),
    )];
    static SAC_PARTS: &[CostPart] = &[CostPart::Sacrifice(&Filter::ARTIFACT)];
    static SAC_OUTLET_AB: &[AbilityDef] = &[paid(
        Cost {
            mana: ManaCost::ZERO,
            parts: SAC_PARTS,
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(0),
        }],
        None,
    )];

    let veyran = card(
        C_VEYRAN,
        creature_face("Veyran", "{1}{U}{R}", 2, 2),
        KeywordSet::EMPTY,
        VEYRAN_AB,
    );
    let wulfgar = card(
        C_WULFGAR,
        creature_face("Wulfgar", "{3}{R}{G}", 4, 4),
        KeywordSet::EMPTY,
        WULFGAR_AB,
    );
    let sac_mult = card(
        C_SAC_MULT,
        face("Sac Doubler", "{2}{B}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        SAC_MULT_AB,
    );
    let wayta = card(
        C_WAYTA,
        creature_face("Wayta", "{R}{G}{W}", 1, 5),
        KeywordSet::EMPTY,
        WAYTA_AB,
    );
    let felix = card(
        C_FELIX,
        creature_face("Felix", "{2}{B}{G}{U}", 5, 4),
        KeywordSet::EMPTY,
        FELIX_AB,
    );
    let mage = card(
        C_MAGE,
        creature_face("Prowess Mage", "{1}{R}", 2, 4),
        KeywordSet::PROWESS,
        MAGE_AB,
    );
    let ping = card(
        C_PING_INSTANT,
        face("Ping", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        PING_AB,
    );
    let copier = card(
        C_COPIER,
        face("Copier", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        COPIER_AB,
    );
    let sac_outlet = card(
        C_SAC_OUTLET,
        face("Altar", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        SAC_OUTLET_AB,
    );
    let bauble = card(
        C_BAUBLE,
        face("Bauble", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let attack_watch = card(
        C_ATTACK_WATCH,
        face("War Drums", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        ATTACK_WATCH_AB,
    );
    let opp_bear = card(
        C_OPP_BEAR,
        creature_face("Opp Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        17_103,
        vec![
            veyran,
            wulfgar,
            sac_mult,
            wayta,
            felix,
            mage,
            ping,
            copier,
            sac_outlet,
            bauble,
            attack_watch,
            opp_bear,
        ],
        [
            Seat::with(&[
                C_VEYRAN,
                C_WULFGAR,
                C_SAC_MULT,
                C_WAYTA,
                C_FELIX,
                C_MAGE,
                C_COPIER,
                C_SAC_OUTLET,
                C_BAUBLE,
                C_ATTACK_WATCH,
            ])
            .holding(&[C_PING_INSTANT]),
            Seat::with(&[C_OPP_BEAR]),
        ],
    );
    to_main(&mut engine, me());
    let mage_id = the(&engine, ZoneLocation::Battlefield, C_MAGE);
    let copier_id = the(&engine, ZoneLocation::Battlefield, C_COPIER);
    let sac_outlet_id = the(&engine, ZoneLocation::Battlefield, C_SAC_OUTLET);
    let bauble_id = the(&engine, ZoneLocation::Battlefield, C_BAUBLE);
    let attack_watch_id = the(&engine, ZoneLocation::Battlefield, C_ATTACK_WATCH);
    let ping_id = the(&engine, ZoneLocation::Hand(me()), C_PING_INSTANT);

    // 1) Cast Ping targeting Prowess Mage, then copy Ping on the stack!
    // Cast triggers Veyran (+1 * 2 = +2) AND Prowess (+1/+1 * 2 = +2/+2 -> 4/6).
    // Copying Ping triggers Veyran (+1 * 2 = +2).
    // Two 1-damage Ping resolutions hit Prowess Mage -> Wayta doubles Enrage TWICE (2 * 200 = +400)!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: ping_id })
        .expect("cast Ping");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![mage_id],
                players: vec![],
            },
        )
        .expect("target mage_id");
    while let Pending::Arrange { player, cards, .. } = engine.pending().clone() {
        engine
            .apply(player, PlayerAction::Arrange { piles: vec![cards] })
            .expect("arrange cast triggers");
    }
    activate(&mut engine, me(), copier_id, 0);
    let stack_ping = engine.state().zones.list(ZoneLocation::Stack)[0];
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![stack_ping],
                players: vec![],
            },
        )
        .expect("target ping on stack");
    settle_stack(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(mage_id)
            .unwrap()
            .characteristics()
            .power,
        Some(4),
        "Veyran must double Prowess on instant cast (2 + 2 = 4)"
    );
    assert_eq!(
        life(&engine, me()),
        20 + 2 + 2 + 400,
        "Veyran (cast +2, copy +2) and Wayta (two 1-dmg pings each doubled +200 = +400)"
    );

    // 2) Sacrifice Bauble: Sac Doubler doubles `Trigger::Sacrificed` (+20 life)!
    activate(&mut engine, me(), sac_outlet_id, 0);
    if let Pending::ChooseCards { player, .. } = engine.pending().clone() {
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![bauble_id],
                },
            )
            .expect("sacrifice bauble");
    }
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        424 + 20,
        "Sacrificed artifact must double sacrifice trigger"
    );

    // 3) Combat: attack with Prowess Mage -> Wulfgar doubles `Attack` (2 draws) AND Felix doubles
    //    `DealsCombatDamageToPlayer` (+2000 life)!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(mage_id, Defender::Player(them()))],
            },
        )
        .expect("attack with mage");
    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), Step::Main) && e.state().turn.phase == Phase::SecondMain
    });
    assert_eq!(
        triggered(&engine, attack_watch_id),
        2,
        "Wulfgar must double your creature's attack trigger"
    );
    assert_eq!(
        life(&engine, me()),
        444 + 2000,
        "Felix Five-Boots must double combat damage to player trigger"
    );

    // 4) Opponent's turn: opponent attacks with Opp Bear -> Wulfgar does NOT double opponent's creature (2 + 1 = 3)!
    let opp_bear_id = the(&engine, ZoneLocation::Battlefield, C_OPP_BEAR);
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == them()),
    );
    engine
        .apply(
            them(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(opp_bear_id, Defender::Player(me()))],
            },
        )
        .expect("opponent attacks");
    walk_until(&mut engine, |e| {
        holds_priority_in(e, them(), Step::Main) && e.state().turn.phase == Phase::SecondMain
    });
    assert_eq!(
        triggered(&engine, attack_watch_id),
        3,
        "Wulfgar must NOT double opponent creature's attack trigger"
    );
}

// =========================================================================
// 2. Cast Provenance (`SourceWasCast`, `CastFromOtherThanHand`, `CastFromZone`)
// =========================================================================

#[test]
fn wave17a_cast_provenance_conditions_and_lki() {
    const C_AV: u32 = W17A + 40;
    const C_KILL: u32 = W17A + 41;
    const C_REANIM: u32 = W17A + 42;
    const C_ANTIQ: u32 = W17A + 43;
    const C_FB_GRANT: u32 = W17A + 44;
    const C_COPIER: u32 = W17A + 45;

    static AV_AB: &[AbilityDef] = &[triggered!(
        Trigger::ETB,
        &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }],
        condition = Some(Condition::SourceWasCast)
    )];
    static KILL_AB: &[AbilityDef] = &[free(
        &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static REANIM_AB: &[AbilityDef] = &[free(
        &[Effect::GraveyardToBattlefield {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            owner_control: false,
            counters: None,
        }],
        Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        ))),
    )];
    static ANTIQ_AB: &[AbilityDef] = &[spell!(&[
        Effect::GainLife {
            amount: Amount::Fixed(1),
        },
        Effect::IfCondition {
            condition: Condition::CastFromOtherThanHand,
            then: &[Effect::GainLife {
                amount: Amount::Fixed(5),
            }],
            otherwise: &[],
        },
    ])];
    static FB_GRANT_AB: &[AbilityDef] = &[free(
        &[Effect::ModifyTargets {
            modifier: Modifier::GrantsFlashbackCost(ManaCost::ZERO),
            duration: Duration::UntilEndOfTurn,
        }],
        Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::INSTANT_OR_SORCERY,
            PlayerRel::You,
        ))),
    )];
    static COPIER_AB: &[AbilityDef] = &[free(
        &[Effect::CopyTargetSpell { mods: &[] }],
        Some(TargetReq::one(TargetSpec::Spell(
            &Filter::INSTANT_OR_SORCERY,
        ))),
    )];

    let av = card(
        C_AV,
        creature_face("Anti-Venom", "{W}", 5, 5),
        KeywordSet::EMPTY,
        AV_AB,
    );
    let kill = card(
        C_KILL,
        face("Murderer", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        KILL_AB,
    );
    let reanim = card(
        C_REANIM,
        face("Reanimator", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        REANIM_AB,
    );
    let antiq = card(
        C_ANTIQ,
        face("Antiquities", "{W}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        ANTIQ_AB,
    );
    let fb_grant = card(
        C_FB_GRANT,
        face("Snapcaster", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        FB_GRANT_AB,
    );
    let copier = card(
        C_COPIER,
        face("Copier", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        COPIER_AB,
    );

    let mut engine = bench(
        17_104,
        vec![av, kill, reanim, antiq, fb_grant, copier],
        [
            Seat::with(&[C_KILL, C_REANIM, C_FB_GRANT, C_COPIER]).holding(&[C_AV, C_AV, C_ANTIQ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    // Zero-hash check: no card on this board reads Exile/Library/Command CastFromZone!
    assert!(
        !engine.state().per_turn.track_cast_origin_zone.0,
        "track_cast_origin_zone must stay inactive (zero-hash) when unused"
    );

    let kill_id = the(&engine, ZoneLocation::Battlefield, C_KILL);
    let reanim_id = the(&engine, ZoneLocation::Battlefield, C_REANIM);
    let fb_grant_id = the(&engine, ZoneLocation::Battlefield, C_FB_GRANT);
    let copier_id = the(&engine, ZoneLocation::Battlefield, C_COPIER);
    let avs = objects(&engine, ZoneLocation::Hand(me()), C_AV);

    // 1) Cast avs[0] AND destroy it in response while its ETB trigger is on the stack (CR 603.4 / CR 608.2h)!
    add_pool(&mut engine, me(), ManaColor::White, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: avs[0] })
        .expect("cast avs[0]");
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine.state().object(avs[0]).unwrap().zone,
        Zone::Battlefield
    );
    assert!(
        !engine.state().zones.stack_is_empty(),
        "ETB trigger must be on stack"
    );
    activate(&mut engine, me(), kill_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![avs[0]],
                players: vec![],
            },
        )
        .expect("destroy avs[0] in response");
    settle_stack(&mut engine, me());
    assert_eq!(engine.state().object(avs[0]).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        life(&engine, me()),
        23,
        "SourceWasCast ETB trigger must still resolve via incarnation LKI after source dies"
    );

    // 2) Reanimate avs[0] from graveyard -> was NOT cast -> ETB does NOT trigger (life stays 23)!
    activate(&mut engine, me(), reanim_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![avs[0]],
                players: vec![],
            },
        )
        .expect("reanimate avs[0]");
    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state().object(avs[0]).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        life(&engine, me()),
        23,
        "Reanimated creature must NOT satisfy Condition::SourceWasCast"
    );

    // 3) Cast Antiquities from hand AND copy it on the stack:
    //    - Hand cast: +1 life only (CastFromOtherThanHand == false).
    //    - Copy on stack (CR 707.10): +1 life only (copy was NOT cast -> CastFromOtherThanHand == false)!
    let antiq_id = the(&engine, ZoneLocation::Hand(me()), C_ANTIQ);
    add_pool(&mut engine, me(), ManaColor::White, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: antiq_id })
        .expect("cast Antiquities from hand");
    activate(&mut engine, me(), copier_id, 0);
    let stack_antiq = engine.state().zones.list(ZoneLocation::Stack)[0];
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![stack_antiq],
                players: vec![],
            },
        )
        .expect("copy Antiquities");
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        25,
        "Hand cast (+1) and uncast spell copy (+1) must both evaluate CastFromOtherThanHand as false"
    );

    // 4) Grant Flashback {0} to Antiquities in graveyard and cast from graveyard -> +1 + +5 = +6 life (31)!
    activate(&mut engine, me(), fb_grant_id, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![antiq_id],
                players: vec![],
            },
        )
        .expect("grant flashback");
    settle_stack(&mut engine, me());
    engine
        .apply(me(), PlayerAction::CastSpell { card: antiq_id })
        .expect("cast Antiquities from graveyard");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        choose_cast_mode_kind(&mut engine, me(), CastModeKind::Normal);
    }
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        31,
        "Flashback cast from graveyard must satisfy CastFromOtherThanHand"
    );
}

#[test]
fn wave17a_cast_from_exile_projected_discount_and_prosper_trigger() {
    const C_AURLOCK: u32 = W17A + 50;
    const C_PROSPER: u32 = W17A + 51;
    const C_PRESTON: u32 = W17A + 52;
    const C_EXILE_SPELL: u32 = W17A + 53;
    const C_HAND_SPELL: u32 = W17A + 54;

    static EXILE_CAST_FILTER: Filter = Filter::CastFromZone(ZoneRef::Exile);
    static UNCAST_CREATURE_FILTER: Filter =
        Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::WasCast)]);

    static AURLOCK_AB: &[AbilityDef] = &[
        static_ability!(
            Filter::CastFromZone(ZoneRef::Exile),
            Modifier::SpellsCostLess(2)
        ),
        static_ability!(
            Filter::Any,
            Modifier::MayCastWithAltCost {
                who: PlayerRel::You,
                zones: AltCastZones::Zones(&[ZoneRef::Exile]),
                grant_zone_permission: true,
                allow_lands: false,
                with_flash: false,
                max_mana_value: None,
                once_per_turn: false,
                price: AltCastPrice::Normal,
            }
        ),
    ];
    static PROSPER_AB: &[AbilityDef] = &[triggered!(
        Trigger::SpellCast(&EXILE_CAST_FILTER),
        &[Effect::GainLife {
            amount: Amount::Fixed(4),
        }]
    )];
    static PRESTON_AB: &[AbilityDef] = &[triggered!(
        Trigger::EntersBattlefield(&UNCAST_CREATURE_FILTER),
        &[Effect::GainLife {
            amount: Amount::Fixed(10),
        }]
    )];
    static SPELL_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];

    let aurlock = card(
        C_AURLOCK,
        creature_face("Doc Aurlock", "{G}{U}", 2, 3),
        KeywordSet::EMPTY,
        AURLOCK_AB,
    );
    let prosper = card(
        C_PROSPER,
        creature_face("Prosper", "{2}{B}{R}", 1, 4),
        KeywordSet::EMPTY,
        PROSPER_AB,
    );
    let preston = card(
        C_PRESTON,
        creature_face("Preston", "{3}{W}", 2, 5),
        KeywordSet::EMPTY,
        PRESTON_AB,
    );
    // Costs {2} generic: with ZERO floating mana, can ONLY be cast if Aurlock's exile discount applies via `projected_spell`!
    let exile_spell = card(
        C_EXILE_SPELL,
        face("Exile Bolt", "{2}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        SPELL_AB,
    );
    let hand_spell = card(
        C_HAND_SPELL,
        face("Hand Bolt", "{1}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        SPELL_AB,
    );

    let mut engine = bench(
        17_105,
        vec![aurlock, prosper, preston, exile_spell, hand_spell],
        [
            Seat::with(&[C_AURLOCK, C_PROSPER, C_PRESTON]).holding(&[C_EXILE_SPELL, C_HAND_SPELL]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    assert!(
        engine.state().per_turn.track_cast_origin_zone.0,
        "track_cast_origin_zone must activate when CastFromZone(ZoneRef::Exile) is on a card"
    );

    let exile_spell_id = the(&engine, ZoneLocation::Hand(me()), C_EXILE_SPELL);
    let hand_spell_id = the(&engine, ZoneLocation::Hand(me()), C_HAND_SPELL);
    move_to_zone(&mut engine, exile_spell_id, ZoneLocation::Exile(me()));

    // 1) Hand spell does NOT get exile discount from Doc Aurlock -> cannot be cast with 0 mana!
    let (_, legal) = offer(&engine);
    assert!(
        !legal.castable.contains(&hand_spell_id),
        "Hand spell must NOT receive exile-scoped cost reduction"
    );
    assert!(
        legal.castable.contains(&exile_spell_id),
        "Exiled two-generic spell MUST be castable with 0 mana via projected exile discount!"
    );

    // 2) Cast `exile_spell_id` from exile for {0} -> Prosper triggers (+4 life)!
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: exile_spell_id,
            },
        )
        .expect("cast exile_spell_id");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        choose_cast_mode_kind(&mut engine, me(), CastModeKind::Normal);
    }
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        25,
        "Exile spell (+1) and Prosper trigger (+4) must both resolve"
    );

    // 3) Cast `hand_spell_id` normally from hand -> Prosper does NOT trigger (+1 only -> 26)!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: hand_spell_id,
            },
        )
        .expect("cast hand_spell_id");
    settle_stack(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        26,
        "Prosper must NOT trigger on hand cast"
    );
}

// =========================================================================
// 3. `Effect::Learn` (`CR 701.46a–b`, `CR 903.11`)
// =========================================================================

#[test]
fn wave17a_learn_lesson_wish_rummage_and_commander() {
    const C_LEARNER: u32 = W17A + 60;
    const C_LESSON: u32 = W17A + 61;
    const C_NON_LESSON: u32 = W17A + 62;
    const C_FODDER: u32 = W17A + 63;
    const C_CMD: u32 = W17A + 64;

    static LEARNER_AB: &[AbilityDef] = &[free(&[Effect::Learn], None)];
    static LESSON_AB: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(2),
    }])];

    let learner = card(
        C_LEARNER,
        face("Study Hall", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        LEARNER_AB,
    );
    let lesson = card(
        C_LESSON,
        FaceDef {
            subtypes: &[subtypes::spell::LESSON],
            ..face("Environmental Sciences", "{2}", TypeSet::SORCERY)
        },
        KeywordSet::EMPTY,
        LESSON_AB,
    );
    let non_lesson = card(
        C_NON_LESSON,
        face("Ordinary Sorcery", "{2}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        LESSON_AB,
    );
    let fodder = card(
        C_FODDER,
        face("Extra Notes", "{1}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let cmd: &'static CardDef = Box::leak(Box::new(CardDef {
        index: CardIndex::new(C_CMD),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([FaceDef {
            supertypes: baylee_core::types::SupertypeSet::LEGENDARY,
            ..creature_face("Dean", "{2}", 2, 2)
        }])),
        color_identity: baylee_core::color::ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::Legendary,
        partner: PartnerKind::None,
        fuse: false,
        coverage: Coverage::Implemented,
        abilities: &[],
    }));

    // --- Part A: Non-Commander game with [Lesson, Non-Lesson] in sideboard ---
    let mut engine = bench_with_sideboard(
        17_106,
        FormatId::Freeform,
        vec![learner, lesson, non_lesson, fodder, cmd],
        [
            (
                Seat::with(&[C_LEARNER]).holding(&[C_FODDER, C_FODDER]),
                vec![C_LESSON, C_NON_LESSON],
                vec![],
            ),
            (Seat::default(), vec![], vec![]),
        ],
    );
    to_main(&mut engine, me());
    let learner_id = the(&engine, ZoneLocation::Battlefield, C_LEARNER);
    let lesson_sb = the(&engine, ZoneLocation::OutsideGame(me()), C_LESSON);
    let _non_lesson_sb = the(&engine, ZoneLocation::OutsideGame(me()), C_NON_LESSON);

    // 1) Activate Learn #1 -> offers Wish with ONLY `lesson_sb` (excludes `non_lesson_sb`);
    //    decline Wish (`[]`) -> transitions to optional Discard; decline Discard (`[]`) -> no-op!
    activate(&mut engine, me(), learner_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        prompt,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(prompt, ChoicePrompt::Wish);
    assert_eq!((min, max), (0, 1));
    assert_eq!(
        options,
        vec![lesson_sb],
        "Learn wish must offer ONLY Lesson cards from OutsideGame"
    );
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .expect("decline Lesson wish");

    let Pending::ChooseCards {
        prompt: rummage_prompt,
        min: r_min,
        max: r_max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "declining Learn wish must offer optional rummage discard, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(rummage_prompt, ChoicePrompt::Discard);
    assert_eq!((r_min, r_max), (0, 1));
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .expect("decline rummage discard");
    settle_stack(&mut engine, me());
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(me())).len(), 2);

    // 2) Activate Learn #2 -> choose `lesson_sb` -> Revealed event emitted, Lesson moves to Hand, no rummage asked!
    activate(&mut engine, me(), learner_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![lesson_sb],
            },
        )
        .expect("take Lesson from sideboard");
    settle_stack(&mut engine, me());
    assert_eq!(engine.state().object(lesson_sb).unwrap().zone, Zone::Hand);
    assert!(
        engine.journal().entries().iter().any(|e| matches!(
            &e.event,
            GameEvent::Revealed { cards, .. } if cards.contains(&lesson_sb)
        )),
        "CR 701.46a: taking a Lesson from outside the game must reveal it"
    );

    // 3) Activate Learn #3 -> no Lessons remain in OutsideGame (only non_lesson_sb) -> skips Wish,
    //    offers rummage directly -> discard 1 fodder to draw 1 Forest!
    let fodders = objects(&engine, ZoneLocation::Hand(me()), C_FODDER);
    activate(&mut engine, me(), learner_id, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        prompt: direct_prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(
        direct_prompt,
        ChoicePrompt::Discard,
        "when no Lesson remains outside the game, Learn must skip Wish and offer Discard directly"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![fodders[0]],
            },
        )
        .expect("discard fodder to rummage");
    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state().object(fodders[0]).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Hand(me()), forest()).len(),
        1,
        "discarding a card to Learn must draw a card"
    );

    // --- Part B: Commander format (`CR 903.11`) ignores Lesson in OutsideGame ---
    let mut cmd_engine = bench_with_sideboard(
        17_107,
        FormatId::Commander,
        vec![learner, lesson, non_lesson, fodder, cmd],
        [
            (
                Seat::with(&[C_LEARNER]).holding(&[C_FODDER]),
                vec![C_LESSON],
                vec![C_CMD],
            ),
            (Seat::default(), vec![], vec![C_CMD]),
        ],
    );
    to_main(&mut cmd_engine, me());
    let cmd_learner = the(&cmd_engine, ZoneLocation::Battlefield, C_LEARNER);
    activate(&mut cmd_engine, me(), cmd_learner, 0);
    walk_until(&mut cmd_engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        prompt: cmd_prompt, ..
    } = cmd_engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(
        cmd_prompt,
        ChoicePrompt::Discard,
        "CR 903.11: in Commander, Learn must skip outside-the-game Wish and offer rummage directly"
    );
}
