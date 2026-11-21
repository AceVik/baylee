//! Wave 10b engine rule tests:
//! - `Splice` (`CR 702.47a–e`): `KeywordSet::SPLICE`, `AbilityDef::Splice`,
//!   `ChoicePrompt::Splice`, revealing 1+ splice cards in hand (each at most
//!   once per spell), paying mana + non-mana splice costs, appending effects &
//!   target clauses while keeping revealed cards in hand, copy preservation,
//!   and non-matching spell negative control.
//! - `Awaken` (`CR 702.113a`): `KeywordSet::AWAKEN`, `AbilityDef::Awaken`,
//!   `CastModeKind::Awaken`, `Effect::AwakenLand`, untargeted wrath ordering
//!   before awakening target land, haste Elemental `0/0` + `N` `+1/+1` counters
//!   still-a-land projection, and 0-land availability guard.
//! - `Conspire` (`CR 702.78a–b`): `KeywordSet::CONSPIRE`,
//!   `Filter::SharesColorWithSource`, `YesNoPrompt::Conspire`, tapping two
//!   untapped color-sharing creatures (including summoning-sick), retargetable
//!   spell copy on cast surviving counterspell, and multiple conspire instances.
//! - `Cipher` (`CR 702.99a–c`): `KeywordSet::CIPHER`, `Effect::Cipher`,
//!   `Effect::CipherCastEncoded`, `ChoicePrompt::CipherEncode` (non-targeting
//!   onto shroud creature), combat-damage free copy cast without re-encoding,
//!   and controller-change tracking.
//! - `Champion` (`CR 702.72a–b`): `KeywordSet::CHAMPION`,
//!   `AbilityDef::Champion`, `Effect::Champion`, `Effect::ReturnChampioned`,
//!   `ChoicePrompt::Champion`, LTB return, and sacrifice when absent/declined.
//! - `Tribute` (`CR 702.104a–b`): `KeywordSet::TRIBUTE`,
//!   `EnterModifier::Tribute`, `YesNoPrompt::Tribute`, `Rider::TributePaid`,
//!   and `Condition::TributeWasNotPaid`.

use super::*;
use crate::choice::{CastModeKind, ChoicePrompt, YesNoPrompt};
use baylee_cards_dsl::prelude::{spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, Condition, Cost, CostPart, Effect, EnterModifier, Filter, KeywordSet, Modifier,
    PlayerRel, TargetReq, TargetSpec, Trigger,
};
use baylee_core::generated::subtypes::creature::{ELEMENTAL, GOBLIN};
use baylee_core::generated::subtypes::spell::ARCANE;
use baylee_core::ids::Defender;
use baylee_core::mana::ManaColor;

const ARCANE_POKE: u32 = 9300;
const PLAIN_POKE: u32 = 9301;
const SPLICE_PING: u32 = 9302;
const SPLICE_SAC_GAIN: u32 = 9303;
const COPY_INSTANT: u32 = 9304;
const FODDER_CREATURE: u32 = 9305;
const AWAKEN_WRATH: u32 = 9306;
const CONSPIRE_BOLT: u32 = 9307;
const CONSPIRE_GRANTER: u32 = 9308;
const RED_ALLY_A: u32 = 9309;
const RED_ALLY_B: u32 = 9310;
const GREEN_ALLY: u32 = 9311;
const COUNTER_INSTANT: u32 = 9312;
const CIPHER_DRAIN: u32 = 9313;
const SHROUD_ROGUE: u32 = 9314;
const STEAL_SORCERY: u32 = 9315;
const CHAMPION_GOBLIN: u32 = 9317;
const ALLY_GOBLIN: u32 = 9318;
const KILL_INSTANT: u32 = 9319;
const TRIBUTE_HYDRA: u32 = 9320;

static ARCANE_FILTER: Filter = Filter::HasSubtype(ARCANE);
static GOBLIN_FILTER: Filter = Filter::HasSubtype(GOBLIN);

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

fn advance_to_attackers(engine: &mut Bench, seat: PlayerId) {
    for _ in 0..512 {
        if matches!(engine.pending(), Pending::ChooseAttackers { player, .. } if *player == seat) {
            return;
        }
        let pending = engine.pending().clone();
        assert!(
            walk_past(engine, &pending),
            "unexpected pending while advancing to attackers: {pending:?}"
        );
    }
    panic!("failed to reach ChooseAttackers for {seat:?}");
}

fn advance_to_step(engine: &mut Bench, seat: PlayerId, target_step: Step) {
    for _ in 0..512 {
        if holds_priority_in(engine, seat, target_step) {
            return;
        }
        match engine.pending().clone() {
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .expect("order simultaneous triggers");
            }
            Pending::DiscardChoice { player, count } => {
                let hand = engine.state.zones.list(ZoneLocation::Hand(player)).clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand[..usize::from(count)].to_vec(),
                        },
                    )
                    .expect("cleanup discard");
            }
            pending => {
                assert!(
                    walk_past(engine, &pending),
                    "unexpected pending while advancing to {target_step:?}: {pending:?}"
                );
            }
        }
    }
    panic!("failed to reach {target_step:?} for {seat:?}");
}

fn target_player(engine: &mut Bench, who: PlayerId, target: PlayerId) {
    engine
        .apply(
            who,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .unwrap();
}

fn target_object(engine: &mut Bench, who: PlayerId, obj: ObjectId) {
    engine
        .apply(
            who,
            PlayerAction::ChooseTargets {
                objects: vec![obj],
                players: vec![],
            },
        )
        .unwrap();
}

fn choose_objs(engine: &mut Bench, who: PlayerId, objs: &[ObjectId]) {
    engine
        .apply(
            who,
            PlayerAction::ChooseObjects {
                objects: objs.to_vec(),
            },
        )
        .unwrap();
}

fn splice_bench() -> Bench {
    static POKE_FX: [AbilityDef; 1] = [spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )];
    static SPLICE_PING_AB: [AbilityDef; 2] = [
        spell!(
            &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::AnyTarget,
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget))
        ),
        AbilityDef::splice_mana(&ARCANE_FILTER, ManaCost::parse("{1}{R}")),
    ];
    static SAC_PARTS: [CostPart; 1] = [CostPart::Sacrifice(&Filter::CREATURE)];
    static SPLICE_SAC_AB: [AbilityDef; 2] = [
        spell!(&[Effect::gain_life(3)]),
        AbilityDef::splice(
            &ARCANE_FILTER,
            Cost {
                mana: ManaCost::parse("{U}"),
                parts: &SAC_PARTS,
            },
        ),
    ];
    static COPY_AB: [AbilityDef; 1] = [spell!(
        &[Effect::CopyTargetSpell { mods: &[] }],
        targets = Some(TargetReq::one(TargetSpec::Spell(
            &Filter::INSTANT_OR_SORCERY
        )))
    )];
    let arcane_inst = |n| FaceDef {
        subtypes: &[ARCANE],
        ..face(n, "{R}", TypeSet::INSTANT)
    };
    bench(
        930,
        vec![
            card(
                ARCANE_POKE,
                arcane_inst("Spirit Poke"),
                KeywordSet::EMPTY,
                &POKE_FX,
            ),
            card(
                PLAIN_POKE,
                face("Plain Poke", "{R}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                &POKE_FX,
            ),
            card(
                SPLICE_PING,
                arcane_inst("Glacial Bolt"),
                KeywordSet::SPLICE,
                &SPLICE_PING_AB,
            ),
            card(
                SPLICE_SAC_GAIN,
                arcane_inst("Soul Sip"),
                KeywordSet::SPLICE,
                &SPLICE_SAC_AB,
            ),
            card(
                COPY_INSTANT,
                face("Twin Echo", "{U}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                &COPY_AB,
            ),
            card(
                FODDER_CREATURE,
                creature_face("Kami Wisp", "{1}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[FODDER_CREATURE]).holding(&[
                ARCANE_POKE,
                PLAIN_POKE,
                SPLICE_PING,
                SPLICE_SAC_GAIN,
                COPY_INSTANT,
            ]),
            Seat::default(),
        ],
    )
}

#[test]
fn splice_onto_arcane_appends_effects_targets_keeps_hand_and_copies() {
    let mut engine = splice_bench();
    to_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    let plain = the(&engine, ZoneLocation::Hand(me()), PLAIN_POKE);
    engine
        .apply(me(), PlayerAction::CastSpell { card: plain })
        .unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
    target_player(&mut engine, me(), them());
    settle_stack(&mut engine, me());
    assert_eq!(life(&engine, them()), 19);

    add_pool(&mut engine, me(), ManaColor::Red, 3);
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    let arcane = the(&engine, ZoneLocation::Hand(me()), ARCANE_POKE);
    let s_ping = the(&engine, ZoneLocation::Hand(me()), SPLICE_PING);
    let s_sac = the(&engine, ZoneLocation::Hand(me()), SPLICE_SAC_GAIN);
    let fodder = the(&engine, ZoneLocation::Battlefield, FODDER_CREATURE);
    engine
        .apply(me(), PlayerAction::CastSpell { card: arcane })
        .unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::Splice,
            min: 0,
            max: 1,
            ..
        }
    ));
    choose_objs(&mut engine, me(), &[s_ping]);
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::Splice,
            min: 0,
            max: 1,
            ..
        }
    ));
    choose_objs(&mut engine, me(), &[s_sac]);
    target_player(&mut engine, me(), them());
    target_player(&mut engine, me(), them());
    choose_objs(&mut engine, me(), &[fodder]);
    assert_eq!(the(&engine, ZoneLocation::Hand(me()), SPLICE_PING), s_ping);
    assert_eq!(
        the(&engine, ZoneLocation::Hand(me()), SPLICE_SAC_GAIN),
        s_sac
    );

    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    let stack_spell = engine.state.zones.list(ZoneLocation::Stack)[0];
    let copier = the(&engine, ZoneLocation::Hand(me()), COPY_INSTANT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: copier })
        .unwrap();
    target_object(&mut engine, me(), stack_spell);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    // CopyTargetSpell offers Retarget for both base and spliced target clauses (`of: 2`); keep both (`[]`).
    choose_objs(&mut engine, me(), &[]);
    choose_objs(&mut engine, me(), &[]);
    settle_stack(&mut engine, me());
    assert_eq!(life(&engine, them()), 13);
    assert_eq!(life(&engine, me()), 26);
}

#[test]
fn awaken_wrath_destroys_creatures_then_animates_haste_elemental_land() {
    static WRATH_AB: [AbilityDef; 2] = [
        spell!(&[Effect::DestroyAll {
            filter: &Filter::CREATURE,
            no_regen: false,
        }]),
        AbilityDef::awaken(3, ManaCost::parse("{3}{W}{W}")),
    ];
    let mut engine = bench(
        931,
        vec![
            card(
                AWAKEN_WRATH,
                face("Planar Wake", "{3}{W}{W}", TypeSet::SORCERY),
                KeywordSet::AWAKEN,
                &WRATH_AB,
            ),
            card(
                FODDER_CREATURE,
                creature_face("Doomed Ox", "{2}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[FODDER_CREATURE]).holding(&[AWAKEN_WRATH, forest()]),
            Seat::with(&[FODDER_CREATURE]),
        ],
    );
    to_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::White, 5);
    let land_in_hand = the(&engine, ZoneLocation::Hand(me()), forest());
    engine
        .apply(me(), PlayerAction::PlayLand { card: land_in_hand })
        .unwrap();
    let my_forest = the(&engine, ZoneLocation::Battlefield, forest());
    let wrath = the(&engine, ZoneLocation::Hand(me()), AWAKEN_WRATH);
    engine
        .apply(me(), PlayerAction::CastSpell { card: wrath })
        .unwrap();
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseCastMode for Awaken, got {:?}",
            engine.pending()
        );
    };
    let awaken_idx = options
        .iter()
        .position(|o| o.kind == CastModeKind::Awaken)
        .expect("Awaken mode offered when caster controls a land");
    engine
        .apply(me(), PlayerAction::ChooseMode(awaken_idx))
        .unwrap();
    target_object(&mut engine, me(), my_forest);
    settle_stack(&mut engine, me());
    assert_eq!(engine.state.zones.list(ZoneLocation::Battlefield).len(), 1);
    let proj = engine.state.object(my_forest).unwrap().characteristics();
    assert!(proj.types.contains(TypeSet::LAND) && proj.types.contains(TypeSet::CREATURE));
    assert!(proj.subtypes.contains(ELEMENTAL));
    assert!(proj.keywords.contains(KeywordSet::HASTE));
    assert_eq!((proj.power, proj.toughness), (Some(3), Some(3)));
}

fn conspire_bench() -> Bench {
    static BOLT_AB: [AbilityDef; 1] = [spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )];
    const STACK_INST_SORC: Filter = Filter::And(&[
        Filter::INSTANT_OR_SORCERY,
        Filter::InZone(baylee_cards_dsl::ZoneRef::Stack),
    ]);
    static GRANTER_AB: [AbilityDef; 1] = [static_ability!(
        STACK_INST_SORC,
        Modifier::AddKeyword(KeywordSet::CONSPIRE)
    )];
    static COUNTER_AB: [AbilityDef; 1] = [spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )];
    bench(
        932,
        vec![
            card(
                CONSPIRE_BOLT,
                face("Ember Pact", "{1}{R}", TypeSet::INSTANT),
                KeywordSet::CONSPIRE,
                &BOLT_AB,
            ),
            card(
                CONSPIRE_GRANTER,
                face("Wort Idol", "{2}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &GRANTER_AB,
            ),
            card(
                RED_ALLY_A,
                creature_face("Red Scout", "{R}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                RED_ALLY_B,
                creature_face("Red Duo", "{R}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                GREEN_ALLY,
                creature_face("Green Elf", "{G}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                COUNTER_INSTANT,
                face("Hard No", "{U}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                &COUNTER_AB,
            ),
        ],
        [
            Seat::with(&[
                CONSPIRE_GRANTER,
                RED_ALLY_A,
                RED_ALLY_A,
                RED_ALLY_A,
                GREEN_ALLY,
            ])
            .holding(&[CONSPIRE_BOLT, RED_ALLY_B, COUNTER_INSTANT]),
            Seat::default(),
        ],
    )
}

#[test]
fn conspire_requires_two_color_sharing_creatures_copies_and_survives_counter() {
    let mut engine = conspire_bench();
    to_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Red, 3);
    let ally_b_card = the(&engine, ZoneLocation::Hand(me()), RED_ALLY_B);
    engine
        .apply(me(), PlayerAction::CastSpell { card: ally_b_card })
        .unwrap();
    settle_stack(&mut engine, me());
    let ally_a = objects(&engine, ZoneLocation::Battlefield, RED_ALLY_A)[0];
    let ally_b = the(&engine, ZoneLocation::Battlefield, RED_ALLY_B);

    let bolt = the(&engine, ZoneLocation::Hand(me()), CONSPIRE_BOLT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: bolt })
        .unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::Conspire,
            ..
        }
    ));
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::Conspire,
            ..
        }
    ));
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    target_player(&mut engine, me(), me());
    choose_objs(&mut engine, me(), &[ally_a]);
    choose_objs(&mut engine, me(), &[ally_b]);
    assert!(
        engine
            .state
            .object(ally_a)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    assert!(
        engine
            .state
            .object(ally_b)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );

    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    target_player(&mut engine, me(), them());

    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    let counter = the(&engine, ZoneLocation::Hand(me()), COUNTER_INSTANT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: counter })
        .unwrap();
    target_object(&mut engine, me(), bolt);
    settle_stack(&mut engine, me());
    assert_eq!(life(&engine, me()), 20);
    assert_eq!(life(&engine, them()), 17);
}

#[test]
fn cipher_encodes_onto_shroud_creature_and_casts_copy_on_combat_damage() {
    static CIPHER_AB: [AbilityDef; 1] = [spell!(&[
        Effect::LoseLife {
            amount: Amount::Fixed(2),
            target: PlayerRel::Opponent,
        },
        Effect::Cipher,
    ])];
    let mut engine = bench(
        933,
        vec![
            card(
                CIPHER_DRAIN,
                face("Hidden Whisper", "{1}{B}", TypeSet::SORCERY),
                KeywordSet::CIPHER,
                &CIPHER_AB,
            ),
            card(
                SHROUD_ROGUE,
                creature_face("Veiled Agent", "{1}{U}", 1, 1),
                KeywordSet::SHROUD.union(KeywordSet::HASTE),
                &[],
            ),
        ],
        [
            Seat::with(&[SHROUD_ROGUE]).holding(&[CIPHER_DRAIN]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let rogue = the(&engine, ZoneLocation::Battlefield, SHROUD_ROGUE);
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    let cipher = the(&engine, ZoneLocation::Hand(me()), CIPHER_DRAIN);
    engine
        .apply(me(), PlayerAction::CastSpell { card: cipher })
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::CipherEncode,
            ..
        }
    ));
    choose_objs(&mut engine, me(), &[rogue]);
    assert_eq!(life(&engine, them()), 18);

    advance_to_attackers(&mut engine, me());
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(rogue, Defender::Player(them()))],
            },
        )
        .unwrap();
    for _ in 0..32 {
        if matches!(
            engine.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::PlayFromExile,
                ..
            }
        ) {
            break;
        }
        let p = engine.pending().clone();
        assert!(walk_past(&mut engine, &p));
    }
    choose_objs(&mut engine, me(), &[cipher]);
    settle_stack(&mut engine, me());
    assert_eq!(life(&engine, them()), 15);
    assert_eq!(engine.state.zones.list(ZoneLocation::Exile(me())).len(), 1);
    let old_ver = engine.state.object(rogue).unwrap().version;
    let encoded_exile = engine.state.zones.list(ZoneLocation::Exile(me()))[0];
    assert!(engine.state.object(encoded_exile).unwrap().riders.contains(
        &crate::object::Rider::CipherEncodedOn {
            host: rogue,
            version: old_ver,
        }
    ));
}

#[test]
fn cipher_controlled_by_new_controller_triggers_for_stealer() {
    static CIPHER_AB: [AbilityDef; 1] = [spell!(&[
        Effect::LoseLife {
            amount: Amount::Fixed(2),
            target: PlayerRel::Opponent,
        },
        Effect::Cipher,
    ])];
    static STEAL_AB: [AbilityDef; 1] = [spell!(
        &[Effect::ChangeController {
            new_controller: PlayerRel::You,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    let mut engine = bench(
        934,
        vec![
            card(
                CIPHER_DRAIN,
                face("Hidden Whisper", "{1}{B}", TypeSet::SORCERY),
                KeywordSet::CIPHER,
                &CIPHER_AB,
            ),
            card(
                SHROUD_ROGUE,
                creature_face("Open Agent", "{1}{U}", 1, 1),
                KeywordSet::HASTE,
                &[],
            ),
            card(
                STEAL_SORCERY,
                face("Turn Coat", "{U}", TypeSet::SORCERY),
                KeywordSet::EMPTY,
                &STEAL_AB,
            ),
        ],
        [
            Seat::with(&[SHROUD_ROGUE]).holding(&[CIPHER_DRAIN]),
            Seat::default().holding(&[STEAL_SORCERY]),
        ],
    );
    to_main(&mut engine, me());
    let rogue = the(&engine, ZoneLocation::Battlefield, SHROUD_ROGUE);
    add_pool(&mut engine, me(), ManaColor::Black, 2);
    let cipher = the(&engine, ZoneLocation::Hand(me()), CIPHER_DRAIN);
    engine
        .apply(me(), PlayerAction::CastSpell { card: cipher })
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    choose_objs(&mut engine, me(), &[rogue]);

    advance_to_step(&mut engine, them(), Step::Main);
    add_pool(&mut engine, them(), ManaColor::Blue, 1);
    let steal = the(&engine, ZoneLocation::Hand(them()), STEAL_SORCERY);
    engine
        .apply(them(), PlayerAction::CastSpell { card: steal })
        .unwrap();
    target_object(&mut engine, them(), rogue);
    settle_stack(&mut engine, them());
    advance_to_attackers(&mut engine, them());
    engine
        .apply(
            them(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(rogue, Defender::Player(me()))],
            },
        )
        .unwrap();
    for _ in 0..32 {
        if matches!(engine.pending(), Pending::ChooseCards { player, prompt: ChoicePrompt::PlayFromExile, .. } if *player == them())
        {
            break;
        }
        let p = engine.pending().clone();
        assert!(walk_past(&mut engine, &p));
    }
    choose_objs(&mut engine, them(), &[cipher]);
    settle_stack(&mut engine, them());
    assert_eq!(life(&engine, me()), 17);
}

#[test]
fn champion_exiles_matching_permanent_returns_on_ltb_and_sacrifices_when_absent() {
    static CHAMP_AB: [AbilityDef; 1] = [AbilityDef::champion(&GOBLIN_FILTER)];
    static KILL_AB: [AbilityDef; 1] = [spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    let goblin_face = |n, p, t| FaceDef {
        subtypes: &[GOBLIN],
        ..creature_face(n, "{1}{R}", p, t)
    };
    let mut engine = bench(
        935,
        vec![
            card(
                CHAMPION_GOBLIN,
                goblin_face("Boggart Boss", 3, 3),
                KeywordSet::CHAMPION,
                &CHAMP_AB,
            ),
            card(
                ALLY_GOBLIN,
                goblin_face("Boggart Grunt", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                KILL_INSTANT,
                face("Snuff", "{B}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                &KILL_AB,
            ),
        ],
        [
            Seat::default().holding(&[CHAMPION_GOBLIN, CHAMPION_GOBLIN, ALLY_GOBLIN, KILL_INSTANT]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Red, 6);
    let champ1 = objects(&engine, ZoneLocation::Hand(me()), CHAMPION_GOBLIN)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: champ1 })
        .unwrap();
    settle_stack(&mut engine, me());
    assert!(
        engine
            .state
            .zones
            .list(ZoneLocation::Battlefield)
            .is_empty()
    );

    let ally = the(&engine, ZoneLocation::Hand(me()), ALLY_GOBLIN);
    engine
        .apply(me(), PlayerAction::CastSpell { card: ally })
        .unwrap();
    settle_stack(&mut engine, me());
    let ally_bf = the(&engine, ZoneLocation::Battlefield, ALLY_GOBLIN);
    let champ2 = the(&engine, ZoneLocation::Hand(me()), CHAMPION_GOBLIN);
    engine
        .apply(me(), PlayerAction::CastSpell { card: champ2 })
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::Champion,
            ..
        }
    ));
    choose_objs(&mut engine, me(), &[ally_bf]);
    settle_stack(&mut engine, me());
    let champ_bf = the(&engine, ZoneLocation::Battlefield, CHAMPION_GOBLIN);
    assert_eq!(engine.state.object(ally_bf).unwrap().zone, Zone::Exile);

    add_pool(&mut engine, me(), ManaColor::Black, 1);
    let kill = the(&engine, ZoneLocation::Hand(me()), KILL_INSTANT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: kill })
        .unwrap();
    target_object(&mut engine, me(), champ_bf);
    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state.object(ally_bf).unwrap().zone,
        Zone::Battlefield
    );
}

#[test]
fn tribute_paid_places_counters_and_suppresses_not_paid_trigger_declined_fires_it() {
    static HYDRA_ENTER: [EnterModifier; 1] = [EnterModifier::tribute(2)];
    static HYDRA_AB: [AbilityDef; 1] = [triggered!(
        Trigger::ETB,
        &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::Player(PlayerRel::Opponent),
        }],
        condition = Some(Condition::TributeWasNotPaid)
    )];
    let hydra_face = FaceDef {
        enter_modifiers: &HYDRA_ENTER,
        ..creature_face("Nessian Raider", "{2}{G}", 3, 3)
    };
    let mut engine = bench(
        936,
        vec![card(
            TRIBUTE_HYDRA,
            hydra_face,
            KeywordSet::TRIBUTE,
            &HYDRA_AB,
        )],
        [
            Seat::default().holding(&[TRIBUTE_HYDRA, TRIBUTE_HYDRA]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Green, 6);
    let h1 = objects(&engine, ZoneLocation::Hand(me()), TRIBUTE_HYDRA)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: h1 })
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::YesNo { player, prompt: YesNoPrompt::Tribute { counters: 2, .. }, .. } if *player == them()
    ));
    engine.apply(them(), PlayerAction::YesNo(true)).unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state.object(h1).unwrap().characteristics().power,
        Some(5)
    );
    assert_eq!(life(&engine, them()), 20);

    let h2 = the(&engine, ZoneLocation::Hand(me()), TRIBUTE_HYDRA);
    engine
        .apply(me(), PlayerAction::CastSpell { card: h2 })
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::YesNo { player, prompt: YesNoPrompt::Tribute { counters: 2, .. }, .. } if *player == them()
    ));
    engine.apply(them(), PlayerAction::YesNo(false)).unwrap();
    settle_stack(&mut engine, me());
    assert_eq!(
        engine.state.object(h2).unwrap().characteristics().power,
        Some(3)
    );
    assert_eq!(life(&engine, them()), 17);
}
