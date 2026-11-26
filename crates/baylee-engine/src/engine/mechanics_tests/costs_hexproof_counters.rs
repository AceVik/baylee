//! Played rule tests for cost adjustment effects/statics, conditional hexproof
//! (`CR 702.11d–f`), and generic counter manipulation (`CR 122`, `CR 701.9e`).

#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::{CastModeKind, ChoicePrompt, NumberPrompt, PLOT_CARD, TURN_FACE_UP};
use crate::zone::ZoneLocation;
use baylee_cards_dsl::{
    ActivatedAbilityKind, ActivationLimit, ActivationTiming, ActivationX, ActivationZone, Amount,
    CommanderRule, Condition, Cost, CostPart, CostReduction, CounterKind, CounterReplaceOp,
    CounterSubject, Coverage, Effect, EquipTargetMetric, Filter, KeywordSet, Layer, Modifier,
    MoveCounterCount, MoveCounterDest, MoveCounterSource, PartnerKind, PlayerRel,
    RemoveCounterCount, RemoveCounterSubject, ReplacementRule, StaticAbility, TargetCostCheck,
    TargetReq, TargetSpec, ThisWay, ZoneRef, ZoneSel,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{artifact, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
    SeatSpec,
};
use baylee_core::types::{SupertypeSet, TypeSet};

// Card indices strictly reserved for this module (`96_100..=96_199`).
const IDX_HEX_BLACK: u32 = 96_100;
const IDX_HEX_OWN_COLORS: u32 = 96_101;
const IDX_HEX_ABILITIES: u32 = 96_102;
const IDX_PLAIN_HEX: u32 = 96_103;
const IDX_SHROUD: u32 = 96_104;
const IDX_PLAYER_HEX_UB: u32 = 96_105;
const IDX_IGNORE_HEX_CREATURES: u32 = 96_106;
const IDX_IGNORE_HEX_ALL: u32 = 96_107;
const IDX_LOSE_HEXPROOF: u32 = 96_108;
const IDX_MAKE_BLUE: u32 = 96_109;
const IDX_BLACK_SPELL: u32 = 96_110;
const IDX_GREEN_SPELL: u32 = 96_111;
const IDX_RED_SPELL: u32 = 96_112;
const IDX_BLUE_SPELL: u32 = 96_113;
const IDX_BLACK_PINGER: u32 = 96_114;
const IDX_GREEN_PINGER: u32 = 96_115;

const IDX_BEAR: u32 = 96_120;
const IDX_BEAR_2: u32 = 96_121;
const IDX_BEAR_3: u32 = 96_122;
const IDX_MULT_P1P1: u32 = 96_123;
const IDX_MULT_ALL: u32 = 96_124;
const IDX_MULT_PLAYER: u32 = 96_125;
const IDX_DOUBLING: u32 = 96_126;
const IDX_HARDENED: u32 = 96_127;
const IDX_MOVE_SINGLE_SLOT: u32 = 96_128;
const IDX_MOVE_TWO_SLOT_ONE: u32 = 96_129;
const IDX_MOVE_TWO_SLOT_THREE: u32 = 96_130;
const IDX_MOVE_GOLDBERRY: u32 = 96_131;
const IDX_MOVE_FILTER_ALL: u32 = 96_132;
const IDX_MOVE_DISTRIBUTE: u32 = 96_133;
const IDX_REMOVE_EXACT_2: u32 = 96_134;
const IDX_REMOVE_PLAYER_UPTO: u32 = 96_135;
const IDX_REMOVE_CHOICE_1: u32 = 96_136;
const IDX_REMOVE_FILTER_ALL: u32 = 96_137;

const IDX_FLASHBACK_RED: u32 = 96_140;
const IDX_FLASHBACK_SPELL: u32 = 96_141;
const IDX_DASH_RED: u32 = 96_142;
const IDX_DASH_CREATURE: u32 = 96_143;
const IDX_PLOT_RED: u32 = 96_144;
const IDX_PLOT_SPELL: u32 = 96_145;
const IDX_MORPH_TAXER: u32 = 96_146;
const IDX_MORPH_CREATURE: u32 = 96_147;
const IDX_MANIFEST_CREATURE: u32 = 96_148;
const IDX_PROWLER: u32 = 96_149;
const IDX_ARTIFACT_CREATURE: u32 = 96_150;
const IDX_FIRST_SPELL_RED: u32 = 96_151;
const IDX_LIESA_CMD: u32 = 96_152;
const IDX_SIEGE_TAXER: u32 = 96_153;
const IDX_MULTI_TARGET_SPELL: u32 = 96_155;
const IDX_GY_CHECK_SPELL: u32 = 96_156;
const IDX_STACK_CHECK_COUNTER: u32 = 96_157;
const IDX_IF_MANA_SPELL: u32 = 96_158;
const IDX_IF_TARGETS_SPELL: u32 = 96_159;

const IDX_ACT_TAXER: u32 = 96_160;
const IDX_ACT_PINGER: u32 = 96_161;
const IDX_FIRST_ACT_RED: u32 = 96_162;
const IDX_BOAST_RED: u32 = 96_163;
const IDX_BOAST_CREATURE: u32 = 96_164;
const IDX_NINJUTSU_RED: u32 = 96_165;
const IDX_NINJA_CARD: u32 = 96_166;
const IDX_EQUIP_RED_POWER: u32 = 96_167;
const IDX_EQUIP_RED_COLOR: u32 = 96_168;
const IDX_EQUIP_RED_COUNTER: u32 = 96_169;
const IDX_EQUIPMENT: u32 = 96_170;
const IDX_BIG_CREATURE: u32 = 96_171;
const IDX_MULTI_COLOR_CREATURE: u32 = 96_172;
const IDX_SMALL_CREATURE: u32 = 96_173;

const IDX_ELF_1: u32 = 96_181;
const IDX_ELF_2: u32 = 96_182;
const IDX_ELF_3: u32 = 96_183;
const IDX_AURA_POWER: u32 = 96_184;
const IDX_SAC_OUTLET: u32 = 96_185;
const IDX_ENERGY_SPENDER: u32 = 96_186;
const IDX_ENERGY_READER: u32 = 96_187;
const IDX_PLAIN_ARTIFACT: u32 = 96_188;

static BLACK_FILTER: Filter = Filter::HasColor(ColorSet::of(Color::Black));
static BLUE_OR_BLACK_FILTER: Filter = Filter::Or(&[
    Filter::HasColor(ColorSet::of(Color::Blue)),
    Filter::HasColor(ColorSet::of(Color::Black)),
]);
static OTHER_YOUR_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::ControlledByYou,
    Filter::Not(&Filter::This),
]);
static FLASHBACK_SPELL_FILTER: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::CastForFlashback]);
static DASH_SPELL_FILTER: Filter = Filter::And(&[Filter::ControlledByYou, Filter::CastForDash]);
static TARGET_PING: &[Effect] = &[Effect::DealDamage {
    amount: Amount::Fixed(1),
    target: TargetSpec::AnyTarget,
}];
static TARGET_PLAYER_OR_CREATURE: TargetReq = TargetReq::one(TargetSpec::AnyTarget);

fn add_mana(engine: &mut Bench, seat: PlayerId, color: ManaColor, n: u32) {
    let pool = &mut engine.dev_state_mut(seat).unwrap().players[seat.get() as usize].mana_pool;
    for _ in 0..n {
        pool.add(color, 1);
    }
    engine.refresh_offer();
}

fn seed_counter(st: &mut crate::state::GameState, id: ObjectId, kind: CounterKind, n: u16) {
    st.object_mut(id).unwrap().counters.add(kind, n);
}

fn player_counter(engine: &Bench, seat: PlayerId, kind: CounterKind) -> u16 {
    engine.state().players[usize::from(seat.get())]
        .counters()
        .into_iter()
        .find_map(|(k, c)| (k == kind).then_some(c))
        .unwrap_or(0)
}

const fn static_ab(filter: Filter, modifier: Modifier) -> AbilityDef {
    AbilityDef::Static(StaticAbility {
        filter,
        layer: modifier.layer(),
        modifier,
        source_zone: ZoneRef::Battlefield,
        condition: None,
    })
}

const fn spell_ab(effects: &'static [Effect], targets: Option<TargetReq>) -> AbilityDef {
    AbilityDef::Spell {
        effects,
        targets,
        second_targets: None,
        third_targets: None,
        condition: None,
    }
}

const fn spell_ab2(
    effects: &'static [Effect],
    targets: Option<TargetReq>,
    second_targets: Option<TargetReq>,
) -> AbilityDef {
    AbilityDef::Spell {
        effects,
        targets,
        second_targets,
        third_targets: None,
        condition: None,
    }
}

const fn act_ab(
    mana: &'static str,
    effects: &'static [Effect],
    targets: Option<TargetReq>,
) -> AbilityDef {
    AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::parse(mana),
            parts: &[],
        },
        effects,
        targets,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        x_value: ActivationX::Any,
        cost_reduction: None,
    }
}

fn target_options_for_spell(engine: &Bench, seat: PlayerId, card_id: ObjectId) -> Vec<ObjectId> {
    crate::eval::spell_target_options_for(
        &TargetSpec::Object(&Filter::CREATURE),
        engine.state(),
        seat,
        card_id,
        None,
        None,
        false,
    )
}

fn target_options_for_ability(
    engine: &Bench,
    seat: PlayerId,
    source_id: ObjectId,
) -> Vec<ObjectId> {
    crate::eval::target_options(
        &TargetSpec::Object(&Filter::CREATURE),
        engine.state(),
        seat,
        source_id,
    )
}

// =========================================================================
// 1. Conditional Hexproof (CR 702.11d-f)
// =========================================================================

#[test]
fn hexproof_from_filter_blocks_matching_opponent_spells_and_abilities_only() {
    static HEX_BLACK_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::HexproofFrom(&BLACK_FILTER),
    )];
    static SPELL_PING: &[AbilityDef] = &[spell_ab(
        TARGET_PING,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static PINGER_ABILS: &[AbilityDef] = &[act_ab(
        "{0}",
        TARGET_PING,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let cards = vec![
        card(
            IDX_HEX_BLACK,
            creature_face("Knight of Malice", "{1}{W}", 2, 2),
            KeywordSet::EMPTY,
            HEX_BLACK_ABILS,
        ),
        card(
            IDX_BLACK_SPELL,
            face("Dark Ping", "{B}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SPELL_PING,
        ),
        card(
            IDX_GREEN_SPELL,
            face("Green Ping", "{G}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SPELL_PING,
        ),
        card(
            IDX_BLACK_PINGER,
            creature_face("Black Pinger", "{B}", 1, 1),
            KeywordSet::EMPTY,
            PINGER_ABILS,
        ),
        card(
            IDX_GREEN_PINGER,
            creature_face("Green Pinger", "{G}", 1, 1),
            KeywordSet::EMPTY,
            PINGER_ABILS,
        ),
    ];

    let mut engine = bench(
        96_100,
        cards,
        [
            Seat::with(&[IDX_HEX_BLACK]).holding(&[IDX_BLACK_SPELL]),
            Seat::with(&[IDX_BLACK_PINGER, IDX_GREEN_PINGER])
                .holding(&[IDX_BLACK_SPELL, IDX_GREEN_SPELL]),
        ],
    );
    to_main(&mut engine, me());

    let knight = the(&engine, ZoneLocation::Battlefield, IDX_HEX_BLACK);
    let own_black = the(&engine, ZoneLocation::Hand(me()), IDX_BLACK_SPELL);
    let opp_black = the(&engine, ZoneLocation::Hand(them()), IDX_BLACK_SPELL);
    let opp_green = the(&engine, ZoneLocation::Hand(them()), IDX_GREEN_SPELL);
    let opp_b_pinger = the(&engine, ZoneLocation::Battlefield, IDX_BLACK_PINGER);
    let opp_g_pinger = the(&engine, ZoneLocation::Battlefield, IDX_GREEN_PINGER);

    assert!(
        !target_options_for_spell(&engine, them(), opp_black).contains(&knight),
        "opponent black spell must be blocked by HexproofFrom(black)"
    );
    assert!(
        !target_options_for_ability(&engine, them(), opp_b_pinger).contains(&knight),
        "opponent black source ability must be blocked by HexproofFrom(black)"
    );
    assert!(
        target_options_for_spell(&engine, them(), opp_green).contains(&knight),
        "opponent green spell must be allowed past HexproofFrom(black)"
    );
    assert!(
        target_options_for_ability(&engine, them(), opp_g_pinger).contains(&knight),
        "opponent green source ability must be allowed past HexproofFrom(black)"
    );
    assert!(
        target_options_for_spell(&engine, me(), own_black).contains(&knight),
        "controller may still target own creature with a black spell"
    );
}

#[test]
fn hexproof_from_own_colors_tracks_dynamic_colors_and_cr_702_11f_strips_conditional_hexproof() {
    static OWN_COLORS_ABILS: &[AbilityDef] =
        &[static_ab(Filter::This, Modifier::HexproofFromOwnColors)];
    static MAKE_BLUE_ABILS: &[AbilityDef] = &[static_ab(
        Filter::CREATURE,
        Modifier::SetColor(ColorSet::of(Color::Blue)),
    )];
    static LOSE_HEX_ABILS: &[AbilityDef] = &[static_ab(
        Filter::CREATURE,
        Modifier::CantHaveKeywords(KeywordSet::HEXPROOF),
    )];
    static SPELL_PING: &[AbilityDef] = &[spell_ab(
        TARGET_PING,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let cards = vec![
        card(
            IDX_HEX_OWN_COLORS,
            creature_face("Chameleon Guard", "{R}{G}", 3, 3),
            KeywordSet::EMPTY,
            OWN_COLORS_ABILS,
        ),
        card(
            IDX_MAKE_BLUE,
            face("Blue Wash", "{U}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            MAKE_BLUE_ABILS,
        ),
        card(
            IDX_LOSE_HEXPROOF,
            face("Bonds of Mortality", "{1}{G}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            LOSE_HEX_ABILS,
        ),
        card(
            IDX_RED_SPELL,
            face("Red Ping", "{R}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SPELL_PING,
        ),
        card(
            IDX_GREEN_SPELL,
            face("Green Ping", "{G}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SPELL_PING,
        ),
        card(
            IDX_BLUE_SPELL,
            face("Blue Ping", "{U}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SPELL_PING,
        ),
    ];

    let mut engine = bench(
        96_101,
        cards,
        [
            Seat::with(&[IDX_HEX_OWN_COLORS]).holding(&[IDX_MAKE_BLUE, IDX_LOSE_HEXPROOF]),
            Seat::with(&[]).holding(&[IDX_RED_SPELL, IDX_GREEN_SPELL, IDX_BLUE_SPELL]),
        ],
    );
    to_main(&mut engine, me());

    let guard = the(&engine, ZoneLocation::Battlefield, IDX_HEX_OWN_COLORS);
    let opp_red = the(&engine, ZoneLocation::Hand(them()), IDX_RED_SPELL);
    let opp_green = the(&engine, ZoneLocation::Hand(them()), IDX_GREEN_SPELL);
    let opp_blue = the(&engine, ZoneLocation::Hand(them()), IDX_BLUE_SPELL);

    // Initially red+green: blocks red & green, allows blue.
    assert!(!target_options_for_spell(&engine, them(), opp_red).contains(&guard));
    assert!(!target_options_for_spell(&engine, them(), opp_green).contains(&guard));
    assert!(target_options_for_spell(&engine, them(), opp_blue).contains(&guard));

    // Cast Blue Wash to set creature's color to blue: now blocks blue, allows red!
    let blue_wash = the(&engine, ZoneLocation::Hand(me()), IDX_MAKE_BLUE);
    add_mana(&mut engine, me(), ManaColor::Blue, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: blue_wash })
        .expect("cast blue wash");
    settle(&mut engine, me());

    assert!(!target_options_for_spell(&engine, them(), opp_blue).contains(&guard));
    assert!(target_options_for_spell(&engine, them(), opp_red).contains(&guard));

    // CR 702.11f: CantHaveKeywords(HEXPROOF) removes conditional hexproof too!
    let lose_hex = the(&engine, ZoneLocation::Hand(me()), IDX_LOSE_HEXPROOF);
    add_mana(&mut engine, me(), ManaColor::Green, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: lose_hex })
        .expect("cast bonds");
    settle(&mut engine, me());

    assert!(
        target_options_for_spell(&engine, them(), opp_blue).contains(&guard),
        "CR 702.11f: CantHaveKeywords(HEXPROOF) must strip HexproofFromOwnColors"
    );
}

#[test]
fn hexproof_from_abilities_blocks_opponent_abilities_but_allows_spells_and_own_abilities() {
    static HEX_ABILS: &[AbilityDef] = &[static_ab(Filter::This, Modifier::HexproofFromAbilities)];
    static SPELL_PING: &[AbilityDef] = &[spell_ab(
        TARGET_PING,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static PINGER_ABILS: &[AbilityDef] = &[act_ab(
        "{0}",
        TARGET_PING,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let cards = vec![
        card(
            IDX_HEX_ABILITIES,
            creature_face("Ability Shielder", "{2}", 2, 2),
            KeywordSet::EMPTY,
            HEX_ABILS,
        ),
        card(
            IDX_RED_SPELL,
            face("Red Ping", "{R}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SPELL_PING,
        ),
        card(
            IDX_GREEN_PINGER,
            creature_face("Pinger", "{G}", 1, 1),
            KeywordSet::EMPTY,
            PINGER_ABILS,
        ),
    ];

    let mut engine = bench(
        96_102,
        cards,
        [
            Seat::with(&[IDX_HEX_ABILITIES, IDX_GREEN_PINGER]),
            Seat::with(&[IDX_GREEN_PINGER]).holding(&[IDX_RED_SPELL]),
        ],
    );
    to_main(&mut engine, me());

    let shielder = the(&engine, ZoneLocation::Battlefield, IDX_HEX_ABILITIES);
    let opp_spell = the(&engine, ZoneLocation::Hand(them()), IDX_RED_SPELL);
    let pingers = objects(&engine, ZoneLocation::Battlefield, IDX_GREEN_PINGER);
    let my_pinger = pingers
        .iter()
        .copied()
        .find(|&id| engine.state().object(id).unwrap().controller == me())
        .unwrap();
    let opp_pinger = pingers
        .iter()
        .copied()
        .find(|&id| engine.state().object(id).unwrap().controller == them())
        .unwrap();

    assert!(
        !target_options_for_ability(&engine, them(), opp_pinger).contains(&shielder),
        "opponent activated ability must be blocked by HexproofFromAbilities"
    );
    assert!(
        target_options_for_spell(&engine, them(), opp_spell).contains(&shielder),
        "opponent spell must be allowed past HexproofFromAbilities"
    );
    assert!(
        target_options_for_ability(&engine, me(), my_pinger).contains(&shielder),
        "controller activated ability must be allowed on own creature"
    );
}

#[test]
fn player_hexproof_from_and_ignore_opponents_hexproof_matching() {
    static PLAYER_HEX_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::PlayerHexproofFrom {
            who: PlayerRel::You,
            from: &BLUE_OR_BLACK_FILTER,
        },
    )];
    static HEX_BLACK_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::HexproofFrom(&BLACK_FILTER),
    )];
    static IGNORE_CREATURES_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::IgnoreOpponentsHexproofMatching {
            permanent_filter: &Filter::CREATURE,
            players: false,
        },
    )];
    static IGNORE_ALL_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::IgnoreOpponentsHexproofMatching {
            permanent_filter: &Filter::Any,
            players: true,
        },
    )];
    static SPELL_ANY: &[AbilityDef] = &[spell_ab(TARGET_PING, Some(TARGET_PLAYER_OR_CREATURE))];

    let cards = vec![
        card(
            IDX_PLAYER_HEX_UB,
            face("Veil Emblem", "{G}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            PLAYER_HEX_ABILS,
        ),
        card(
            IDX_PLAIN_HEX,
            creature_face("Hex Bear", "{1}{G}", 2, 2),
            KeywordSet::HEXPROOF,
            &[],
        ),
        card(
            IDX_HEX_BLACK,
            creature_face("Black Hex Bear", "{1}{W}", 2, 2),
            KeywordSet::EMPTY,
            HEX_BLACK_ABILS,
        ),
        card(
            IDX_SHROUD,
            creature_face("Shroud Bear", "{1}{U}", 2, 2),
            KeywordSet::SHROUD,
            &[],
        ),
        card(
            IDX_IGNORE_HEX_CREATURES,
            face("Glaring Spotlight", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            IGNORE_CREATURES_ABILS,
        ),
        card(
            IDX_IGNORE_HEX_ALL,
            face("Detection Tower Plus", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            IGNORE_ALL_ABILS,
        ),
        card(
            IDX_BLACK_SPELL,
            face("Black Bolt", "{B}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SPELL_ANY,
        ),
        card(
            IDX_RED_SPELL,
            face("Red Bolt", "{R}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SPELL_ANY,
        ),
    ];

    let mut engine = bench(
        96_103,
        cards,
        [
            Seat::with(&[IDX_PLAYER_HEX_UB, IDX_PLAIN_HEX, IDX_HEX_BLACK, IDX_SHROUD])
                .holding(&[IDX_BLACK_SPELL]),
            Seat::with(&[]).holding(&[
                IDX_BLACK_SPELL,
                IDX_RED_SPELL,
                IDX_IGNORE_HEX_CREATURES,
                IDX_IGNORE_HEX_ALL,
            ]),
        ],
    );
    to_main(&mut engine, them());

    let plain_hex = the(&engine, ZoneLocation::Battlefield, IDX_PLAIN_HEX);
    let black_hex = the(&engine, ZoneLocation::Battlefield, IDX_HEX_BLACK);
    let shroud = the(&engine, ZoneLocation::Battlefield, IDX_SHROUD);
    let opp_black = the(&engine, ZoneLocation::Hand(them()), IDX_BLACK_SPELL);
    let opp_red = the(&engine, ZoneLocation::Hand(them()), IDX_RED_SPELL);
    let own_black = the(&engine, ZoneLocation::Hand(me()), IDX_BLACK_SPELL);

    let players_for = |e: &Bench, seat: PlayerId, src: ObjectId| {
        crate::eval::target_player_options_from(
            e.state(),
            &TargetSpec::AnyTarget,
            seat,
            crate::eval::live_context(e.state(), src),
        )
    };

    // Veil blocks opponent's black bolt targeting `me()`, allows red bolt and own black bolt.
    assert!(!players_for(&engine, them(), opp_black).contains(&me()));
    assert!(players_for(&engine, them(), opp_red).contains(&me()));
    assert!(players_for(&engine, me(), own_black).contains(&me()));

    // Cast IgnoreOpponentsHexproofMatching { permanent_filter: CREATURE, players: false }.
    let spotlight = the(
        &engine,
        ZoneLocation::Hand(them()),
        IDX_IGNORE_HEX_CREATURES,
    );
    add_mana(&mut engine, them(), ManaColor::Colorless, 1);
    engine
        .apply(them(), PlayerAction::CastSpell { card: spotlight })
        .expect("cast spotlight");
    settle(&mut engine, them());

    let opts = target_options_for_spell(&engine, them(), opp_black);
    assert!(
        opts.contains(&plain_hex) && opts.contains(&black_hex),
        "IgnoreOpponentsHexproofMatching must bypass both plain and conditional hexproof"
    );
    assert!(
        !opts.contains(&shroud),
        "IgnoreOpponentsHexproofMatching must NOT bypass Shroud"
    );
    assert!(
        !players_for(&engine, them(), opp_black).contains(&me()),
        "players: false must NOT bypass PlayerHexproofFrom"
    );

    // Cast IgnoreOpponentsHexproofMatching { players: true }.
    let tower = the(&engine, ZoneLocation::Hand(them()), IDX_IGNORE_HEX_ALL);
    add_mana(&mut engine, them(), ManaColor::Colorless, 1);
    engine
        .apply(them(), PlayerAction::CastSpell { card: tower })
        .expect("cast tower");
    settle(&mut engine, them());

    assert!(
        players_for(&engine, them(), opp_black).contains(&me()),
        "players: true must bypass PlayerHexproofFrom"
    );
}

// =========================================================================
// 2. Generic Counter Manipulation (CR 122 / CR 701.9e)
// =========================================================================

#[test]
fn multiply_counters_specific_all_kinds_player_and_doubling_season() {
    static MULT_P1P1_ABILS: &[AbilityDef] = &[spell_ab(
        &[Effect::MultiplyCounters {
            subject: CounterSubject::Targets,
            kind: Some(CounterKind::P1P1),
            factor: 2,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static MULT_ALL_ABILS: &[AbilityDef] = &[spell_ab(
        &[Effect::MultiplyCounters {
            subject: CounterSubject::Targets,
            kind: None,
            factor: 2,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static MULT_PLAYER_ABILS: &[AbilityDef] = &[spell_ab(
        &[Effect::MultiplyCounters {
            subject: CounterSubject::Player(PlayerRel::You),
            kind: Some(CounterKind::Energy),
            factor: 2,
        }],
        None,
    )];
    static DOUBLING_ABILS: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ModifyCounterPlacement {
            object_filter: &Filter::ControlledByYou,
            kind: None,
            placer: None,
            effect_only: false,
            op: CounterReplaceOp::Multiply(2),
        },
    )];

    let cards = vec![
        card(
            IDX_BEAR,
            creature_face("Counter Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_MULT_P1P1,
            face("Double P1P1", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            MULT_P1P1_ABILS,
        ),
        card(
            IDX_MULT_ALL,
            face("Double All", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            MULT_ALL_ABILS,
        ),
        card(
            IDX_MULT_PLAYER,
            face("Double Energy", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            MULT_PLAYER_ABILS,
        ),
        card(
            IDX_DOUBLING,
            face("Doubling Season Lite", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DOUBLING_ABILS,
        ),
    ];

    let mut engine = bench(
        96_120,
        cards,
        [
            Seat::with(&[IDX_BEAR]).holding(&[
                IDX_MULT_P1P1,
                IDX_MULT_ALL,
                IDX_MULT_PLAYER,
                IDX_DOUBLING,
                IDX_MULT_P1P1,
            ]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let bear = the(&engine, ZoneLocation::Battlefield, IDX_BEAR);
    {
        let st = engine.dev_state_mut(me()).unwrap();
        seed_counter(st, bear, CounterKind::P1P1, 2);
        seed_counter(st, bear, CounterKind::Charge, 3);
        st.put_player_counters_by(me(), me(), CounterKind::Energy, 3);
    }

    // 1. Specific kind: doubles P1P1 (2 -> 4), leaves Charge at 3.
    let sp1 = objects(&engine, ZoneLocation::Hand(me()), IDX_MULT_P1P1)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: sp1 })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        4
    );
    assert_eq!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        3
    );

    // 2. All kinds (kind: None): doubles both P1P1 (4 -> 8) and Charge (3 -> 6).
    let sp2 = the(&engine, ZoneLocation::Hand(me()), IDX_MULT_ALL);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sp2 })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        8
    );
    assert_eq!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        6
    );

    // 3. Player counters: doubles Energy (3 -> 6).
    let sp3 = the(&engine, ZoneLocation::Hand(me()), IDX_MULT_PLAYER);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sp3 })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(player_counter(&engine, me(), CounterKind::Energy), 6);

    // 4. CR 701.9e + Doubling Season: 8 P1P1 -> attempts to place 8 more -> doubled to 16 -> total 24!
    let doubling = the(&engine, ZoneLocation::Hand(me()), IDX_DOUBLING);
    engine
        .apply(me(), PlayerAction::CastSpell { card: doubling })
        .unwrap();
    settle(&mut engine, me());

    let sp4 = objects(&engine, ZoneLocation::Hand(me()), IDX_MULT_P1P1)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: sp4 })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        24,
        "CR 701.9e: doubling 8 under Doubling Season puts 16 more -> 24 total"
    );
}

#[test]
fn move_counters_single_slot_two_targets_and_exact_multi_kind_and_hardened_scales() {
    static SINGLE_SLOT_ABILS: &[AbilityDef] = &[spell_ab(
        &[
            Effect::MoveCounters {
                from: MoveCounterSource::FirstTarget,
                to: MoveCounterDest::SecondTarget,
                kind: Some(CounterKind::P1P1),
                count: MoveCounterCount::Exact(2),
            },
            Effect::GainLife {
                amount: Amount::ThisWay(ThisWay::CountersMoved),
            },
        ],
        Some(TargetReq {
            spec: TargetSpec::Object(&Filter::CREATURE),
            min: 2,
            max: 2,
            count_is_x: false,
            chooser: baylee_cards_dsl::TargetChooser::Controller,
            distinct_mana_values: false,
        }),
    )];
    static TWO_SLOT_ONE_ABILS: &[AbilityDef] = &[spell_ab2(
        &[
            Effect::MoveCounters {
                from: MoveCounterSource::FirstTarget,
                to: MoveCounterDest::SecondTarget,
                kind: None,
                count: MoveCounterCount::Exact(1),
            },
            Effect::GainLife {
                amount: Amount::ThisWay(ThisWay::CountersMoved),
            },
        ],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static TWO_SLOT_THREE_ABILS: &[AbilityDef] = &[spell_ab2(
        &[Effect::MoveCounters {
            from: MoveCounterSource::FirstTarget,
            to: MoveCounterDest::SecondTarget,
            kind: None,
            count: MoveCounterCount::Exact(3),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static HARDENED_ABILS: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ModifyCounterPlacement {
            object_filter: &Filter::YOUR_CREATURE,
            kind: Some(CounterKind::P1P1),
            placer: None,
            effect_only: false,
            op: CounterReplaceOp::Add(1),
        },
    )];

    let cards = vec![
        card(
            IDX_BEAR,
            creature_face("Donor Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_BEAR_2,
            creature_face("Dest Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_MOVE_SINGLE_SLOT,
            face("Bioshift Pair", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            SINGLE_SLOT_ABILS,
        ),
        card(
            IDX_MOVE_TWO_SLOT_ONE,
            face("Move One Any", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            TWO_SLOT_ONE_ABILS,
        ),
        card(
            IDX_MOVE_TWO_SLOT_THREE,
            face("Move Three Any", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            TWO_SLOT_THREE_ABILS,
        ),
        card(
            IDX_HARDENED,
            face("Hardened Scales", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            HARDENED_ABILS,
        ),
    ];

    let mut engine = bench(
        96_121,
        cards,
        [
            Seat::with(&[IDX_BEAR, IDX_BEAR_2, IDX_HARDENED]).holding(&[
                IDX_MOVE_SINGLE_SLOT,
                IDX_MOVE_TWO_SLOT_ONE,
                IDX_MOVE_TWO_SLOT_THREE,
            ]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let donor = the(&engine, ZoneLocation::Battlefield, IDX_BEAR);
    let dest = the(&engine, ZoneLocation::Battlefield, IDX_BEAR_2);
    {
        let st = engine.dev_state_mut(me()).unwrap();
        seed_counter(st, donor, CounterKind::P1P1, 4);
        seed_counter(st, donor, CounterKind::Charge, 3);
    }

    // 1. Single-slot 2 targets + CR 122.5 Hardened Scales: moves 2 P1P1 in one batch ->
    //    donor loses 2 (4 -> 2), dest gains 2 + 1 = 3! And ThisWay::CountersMoved == 2 (+2 life).
    let sp1 = the(&engine, ZoneLocation::Hand(me()), IDX_MOVE_SINGLE_SLOT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sp1 })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![donor, dest],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(donor)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
    assert_eq!(
        engine
            .state()
            .object(dest)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        3
    );
    assert_eq!(life(&engine, me()), 22, "ThisWay::CountersMoved == 2");

    // 2. Two-slot Exact(1) with kind: None prompts ChooseCastMode(Counter(k)).
    let sp2 = the(&engine, ZoneLocation::Hand(me()), IDX_MOVE_TWO_SLOT_ONE);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sp2 })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![donor],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![dest],
                players: vec![],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    assert_ne!(
        engine
            .resolution
            .as_ref()
            .and_then(|r| r.awaiting.as_ref())
            .map_or(0, crate::resolve::AwaitingOp::search_fingerprint),
        0,
        "tag 599 AwaitingOp::MoveCounters must produce non-zero search_fingerprint"
    );
    let charge_idx = match engine.pending() {
        Pending::ChooseCastMode { options, .. } => options
            .iter()
            .position(|o| o.kind == CastModeKind::Counter(CounterKind::Charge))
            .expect("Charge counter option offered"),
        other => panic!("expected ChooseCastMode, got {other:?}"),
    };
    engine
        .apply(me(), PlayerAction::ChooseMode(charge_idx))
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(donor)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        2
    );
    assert_eq!(
        engine
            .state()
            .object(dest)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        1
    );
    assert_eq!(life(&engine, me()), 23, "ThisWay::CountersMoved == 1");

    // 3. Two-slot Exact(3) across P1P1 (2) & Charge (2): prompts ChooseNumber(1..=2) for first kind.
    let sp3 = the(&engine, ZoneLocation::Hand(me()), IDX_MOVE_TWO_SLOT_THREE);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sp3 })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![donor],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![dest],
                players: vec![],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    let (min_n, max_n, first_kind) = match engine.pending() {
        Pending::ChooseNumber {
            min,
            max,
            reason: NumberPrompt::Counters { kind, .. },
            ..
        } => (*min, *max, *kind),
        other => panic!("expected ChooseNumber(Counters), got {other:?}"),
    };
    assert_eq!((min_n, max_n), (1, 2));
    // Choose 2 of first kind -> automatically moves 1 of the other kind.
    engine.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
    settle(&mut engine, me());
    if first_kind == CounterKind::P1P1 {
        assert_eq!(
            engine
                .state()
                .object(donor)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            0
        );
        assert_eq!(
            engine
                .state()
                .object(donor)
                .unwrap()
                .counters
                .get(CounterKind::Charge),
            1
        );
    } else {
        assert_eq!(
            engine
                .state()
                .object(donor)
                .unwrap()
                .counters
                .get(CounterKind::Charge),
            0
        );
        assert_eq!(
            engine
                .state()
                .object(donor)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            1
        );
    }
}

#[test]
fn move_counters_goldberry_filter_source_and_distribute_among() {
    static GOLDBERRY_ABILS: &[AbilityDef] = &[act_ab(
        "{0}",
        &[
            Effect::MoveCounters {
                from: MoveCounterSource::FirstTarget,
                to: MoveCounterDest::Source,
                kind: None,
                count: MoveCounterCount::OneOfEachKindNotOnDest,
            },
            Effect::GainLife {
                amount: Amount::ThisWay(ThisWay::CountersMoved),
            },
        ],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static FILTER_ALL_ABILS: &[AbilityDef] = &[act_ab(
        "{0}",
        &[Effect::MoveCounters {
            from: MoveCounterSource::Filter(&OTHER_YOUR_CREATURE),
            to: MoveCounterDest::Source,
            kind: Some(CounterKind::P1P1),
            count: MoveCounterCount::All,
        }],
        None,
    )];
    static DISTRIBUTE_ABILS: &[AbilityDef] = &[act_ab(
        "{0}",
        &[Effect::MoveCounters {
            from: MoveCounterSource::Source,
            to: MoveCounterDest::DistributeAmong(&OTHER_YOUR_CREATURE),
            kind: Some(CounterKind::P1P1),
            count: MoveCounterCount::AnyNumber { min_one: false },
        }],
        None,
    )];

    let cards = vec![
        card(
            IDX_BEAR,
            creature_face("Donor", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_BEAR_2,
            creature_face("Ally", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_MOVE_GOLDBERRY,
            creature_face("Goldberry", "{1}{U}", 1, 3),
            KeywordSet::EMPTY,
            GOLDBERRY_ABILS,
        ),
        card(
            IDX_MOVE_FILTER_ALL,
            creature_face("Marauder", "{3}{B}", 2, 2),
            KeywordSet::EMPTY,
            FILTER_ALL_ABILS,
        ),
        card(
            IDX_MOVE_DISTRIBUTE,
            creature_face("Forgotten Ancient", "{3}{G}", 0, 3),
            KeywordSet::EMPTY,
            DISTRIBUTE_ABILS,
        ),
    ];

    let mut engine = bench(
        96_122,
        cards,
        [
            Seat::with(&[
                IDX_BEAR,
                IDX_BEAR_2,
                IDX_MOVE_GOLDBERRY,
                IDX_MOVE_FILTER_ALL,
                IDX_MOVE_DISTRIBUTE,
            ]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let donor = the(&engine, ZoneLocation::Battlefield, IDX_BEAR);
    let ally = the(&engine, ZoneLocation::Battlefield, IDX_BEAR_2);
    let goldberry = the(&engine, ZoneLocation::Battlefield, IDX_MOVE_GOLDBERRY);
    let marauder = the(&engine, ZoneLocation::Battlefield, IDX_MOVE_FILTER_ALL);
    let ancient = the(&engine, ZoneLocation::Battlefield, IDX_MOVE_DISTRIBUTE);

    {
        let st = engine.dev_state_mut(me()).unwrap();
        seed_counter(st, donor, CounterKind::P1P1, 2);
        seed_counter(st, donor, CounterKind::Charge, 1);
        seed_counter(st, donor, CounterKind::Shield, 1);
        seed_counter(st, goldberry, CounterKind::P1P1, 1);
        seed_counter(st, ally, CounterKind::P1P1, 2);
    }

    // 1. Goldberry: moves 1 Charge and 1 Shield (skips P1P1 already on Goldberry), +2 life!
    activate(&mut engine, me(), goldberry, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![donor],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(goldberry)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert_eq!(
        engine
            .state()
            .object(goldberry)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        1
    );
    assert_eq!(
        engine
            .state()
            .object(goldberry)
            .unwrap()
            .counters
            .get(CounterKind::Shield),
        1
    );
    assert_eq!(
        engine
            .state()
            .object(donor)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
    assert_eq!(
        engine
            .state()
            .object(donor)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        0
    );
    assert_eq!(
        life(&engine, me()),
        22,
        "Goldberry moved 2 distinct absent kinds"
    );

    // 2. Marauder (MoveCounterSource::Filter All): pulls all P1P1 from donor(2)+ally(2)+goldberry(1) = 5!
    activate(&mut engine, me(), marauder, 0);
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(marauder)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        5
    );
    assert_eq!(
        engine
            .state()
            .object(donor)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
    assert_eq!(
        engine
            .state()
            .object(ally)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );

    // 3. Forgotten Ancient (DistributeAmong AnyNumber): give ancient 3 P1P1, distribute 2 onto first dest, 0 on rest.
    seed_counter(
        engine.dev_state_mut(me()).unwrap(),
        ancient,
        CounterKind::P1P1,
        3,
    );
    activate(&mut engine, me(), ancient, 0);
    let mut gave_two = false;
    loop {
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
                || (e.state().zones.stack_is_empty()
                    && matches!(e.pending(), Pending::Priority { player, .. } if *player == me()))
        });
        if let Pending::ChooseNumber { max, .. } = engine.pending() {
            let val = if !gave_two && *max >= 2 {
                gave_two = true;
                2
            } else {
                0
            };
            engine.apply(me(), PlayerAction::ChooseNumber(val)).unwrap();
        } else {
            break;
        }
    }
    assert!(gave_two);
    assert_eq!(
        engine
            .state()
            .object(ancient)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "2 of 3 counters distributed away from Forgotten Ancient"
    );
}

#[test]
fn remove_counters_exact_player_choice_filter_and_this_way() {
    static REM_EXACT_2: &[AbilityDef] = &[spell_ab(
        &[
            Effect::RemoveCounters {
                subject: RemoveCounterSubject::Targets,
                kind: None,
                count: RemoveCounterCount::Exact(Amount::Fixed(2)),
            },
            Effect::GainLife {
                amount: Amount::ThisWay(ThisWay::CountersRemovedAny),
            },
        ],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static REM_PLAYER_UPTO: &[AbilityDef] = &[spell_ab(
        &[
            Effect::RemoveCounters {
                subject: RemoveCounterSubject::Player(PlayerRel::Opponent),
                kind: Some(CounterKind::Energy),
                count: RemoveCounterCount::UpTo(Amount::Fixed(3)),
            },
            Effect::GainLife {
                amount: Amount::ThisWay(ThisWay::CountersRemovedAny),
            },
        ],
        None,
    )];
    static REM_CHOICE_1: &[AbilityDef] = &[spell_ab(
        &[Effect::RemoveCounters {
            subject: RemoveCounterSubject::Choice(&Filter::CREATURE),
            kind: None,
            count: RemoveCounterCount::Exact(Amount::Fixed(1)),
        }],
        None,
    )];
    static REM_FILTER_ALL: &[AbilityDef] = &[spell_ab(
        &[Effect::RemoveCounters {
            subject: RemoveCounterSubject::Filter(&Filter::CREATURE),
            kind: Some(CounterKind::P1P1),
            count: RemoveCounterCount::All,
        }],
        None,
    )];

    let cards = vec![
        card(
            IDX_BEAR,
            creature_face("Bear A", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_PLAIN_HEX,
            creature_face("Hexproof Bear", "{1}{G}", 2, 2),
            KeywordSet::HEXPROOF,
            &[],
        ),
        card(
            IDX_REMOVE_EXACT_2,
            face("Remove Exact 2", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            REM_EXACT_2,
        ),
        card(
            IDX_REMOVE_PLAYER_UPTO,
            face("Drain Energy", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            REM_PLAYER_UPTO,
        ),
        card(
            IDX_REMOVE_CHOICE_1,
            face("Ferropede Choice", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            REM_CHOICE_1,
        ),
        card(
            IDX_REMOVE_FILTER_ALL,
            face("Sweep All P1P1", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            REM_FILTER_ALL,
        ),
    ];

    let mut engine = bench(
        96_123,
        cards,
        [
            Seat::with(&[IDX_BEAR]).holding(&[
                IDX_REMOVE_EXACT_2,
                IDX_REMOVE_PLAYER_UPTO,
                IDX_REMOVE_CHOICE_1,
                IDX_REMOVE_FILTER_ALL,
            ]),
            Seat::with(&[IDX_PLAIN_HEX]),
        ],
    );
    to_main(&mut engine, me());

    let bear = the(&engine, ZoneLocation::Battlefield, IDX_BEAR);
    let opp_hex = the(&engine, ZoneLocation::Battlefield, IDX_PLAIN_HEX);
    {
        let st = engine.dev_state_mut(me()).unwrap();
        seed_counter(st, bear, CounterKind::P1P1, 2);
        seed_counter(st, bear, CounterKind::Charge, 2);
        seed_counter(st, opp_hex, CounterKind::P1P1, 2);
        st.put_player_counters_by(them(), them(), CounterKind::Energy, 5);
    }

    // 1. Exact(2) across P1P1 + Charge: choose 1 of first kind -> 1 of second kind auto-removed, +2 life.
    let sp1 = the(&engine, ZoneLocation::Hand(me()), IDX_REMOVE_EXACT_2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sp1 })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(me(), PlayerAction::ChooseNumber(1)).unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert_eq!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        1
    );
    assert_eq!(life(&engine, me()), 22);

    // 2. Player(Opponent) UpTo(3) Energy: choose 2 -> opp energy 5 -> 3, +2 life.
    let sp2 = the(&engine, ZoneLocation::Hand(me()), IDX_REMOVE_PLAYER_UPTO);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sp2 })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
    settle(&mut engine, me());
    assert_eq!(player_counter(&engine, them(), CounterKind::Energy), 3);
    assert_eq!(life(&engine, me()), 24);

    // 3. Choice(&Filter::CREATURE) bypasses opponent's Hexproof!
    let sp3 = the(&engine, ZoneLocation::Hand(me()), IDX_REMOVE_CHOICE_1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sp3 })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Generic,
                ..
            }
        )
    });
    assert_ne!(
        engine
            .resolution
            .as_ref()
            .and_then(|r| r.awaiting.as_ref())
            .map_or(0, crate::resolve::AwaitingOp::search_fingerprint),
        0,
        "tag 600 AwaitingOp::RemoveCountersChoice must produce non-zero search_fingerprint"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![opp_hex],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(opp_hex)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "non-targeted RemoveCounterSubject::Choice must work on opponent's Hexproof creature"
    );

    // 4. Filter(&Filter::CREATURE) All P1P1 sweeps both creatures while keeping Charge on bear!
    let sp4 = the(&engine, ZoneLocation::Hand(me()), IDX_REMOVE_FILTER_ALL);
    engine
        .apply(me(), PlayerAction::CastSpell { card: sp4 })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
    assert_eq!(
        engine
            .state()
            .object(opp_hex)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
    assert_eq!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        1
    );
}

// =========================================================================
// 3. Cost Adjustments (Flashback, Dash, Plot, Morph, Prowler, FirstSpell, Liesa)
// =========================================================================

#[test]
fn flashback_dash_plot_and_morph_cost_adjustments() {
    static FB_RED_ABILS: &[AbilityDef] = &[static_ab(
        FLASHBACK_SPELL_FILTER,
        Modifier::SpellsCostLess(2),
    )];
    static DASH_RED_ABILS: &[AbilityDef] =
        &[static_ab(DASH_SPELL_FILTER, Modifier::SpellsCostLess(1))];
    static PLOT_RED_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::PlotActionCostsLess {
            who: PlayerRel::You,
            amount: 2,
        },
    )];
    static MORPH_TAX_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::MorphTurnFaceUpCostsMore { amount: 2 },
    )];
    static GAIN_1: &[AbilityDef] = &[spell_ab(
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        None,
    )];

    let cards = vec![
        card(
            IDX_FLASHBACK_RED,
            face("Catalyst Stone", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            FB_RED_ABILS,
        ),
        card(
            IDX_FLASHBACK_SPELL,
            FaceDef {
                flashback: Some(ManaCost::parse("{3}{R}")),
                ..face("Flashback Bolt", "{3}{R}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            GAIN_1,
        ),
        card(
            IDX_DASH_RED,
            creature_face("Warbringer", "{0}", 2, 2),
            KeywordSet::EMPTY,
            DASH_RED_ABILS,
        ),
        card(
            IDX_DASH_CREATURE,
            FaceDef {
                dash: Some(ManaCost::parse("{2}{R}")),
                ..creature_face("Dash Raider", "{2}{R}", 2, 2)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_PLOT_RED,
            creature_face("Doc Aurlock", "{0}", 2, 3),
            KeywordSet::EMPTY,
            PLOT_RED_ABILS,
        ),
        card(
            IDX_PLOT_SPELL,
            FaceDef {
                plot: Some(ManaCost::parse("{2}{U}")),
                ..face("Plot Insight", "{3}{U}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            GAIN_1,
        ),
        card(
            IDX_MORPH_TAXER,
            creature_face("Exiled Doomsayer", "{1}{W}", 1, 2),
            KeywordSet::EMPTY,
            MORPH_TAX_ABILS,
        ),
        card(
            IDX_MORPH_CREATURE,
            FaceDef {
                morph: Some(ManaCost::parse("{1}{G}")),
                ..creature_face("Morph Bear", "{2}{G}", 2, 2)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_MANIFEST_CREATURE,
            creature_face("Manifest Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        96_140,
        cards,
        [
            Seat::with(&[
                IDX_FLASHBACK_RED,
                IDX_DASH_RED,
                IDX_PLOT_RED,
                IDX_MORPH_TAXER,
            ])
            .holding(&[
                IDX_FLASHBACK_SPELL,
                IDX_DASH_CREATURE,
                IDX_PLOT_SPELL,
                IDX_MORPH_CREATURE,
                IDX_MANIFEST_CREATURE,
            ]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    // 1. Filter::CastForFlashback: hand cast costs full {3}{R} (cannot cast with {1}{R});
    //    moving to graveyard lets flashback cast with {1}{R}!
    let fb_card = the(&engine, ZoneLocation::Hand(me()), IDX_FLASHBACK_SPELL);
    add_mana(&mut engine, me(), ManaColor::Red, 2);
    let (_, legal) = offer(&engine);
    assert!(
        !legal.castable.contains(&fb_card),
        "CastForFlashback discount must NOT apply to normal hand cast"
    );
    let _ = engine.dev_state_mut(me()).unwrap().move_object(
        fb_card,
        ZoneLocation::Graveyard(me()),
        crate::zone::ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    engine.refresh_offer();
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&fb_card),
        "CastForFlashback discount must allow casting 3R flashback with 1R"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: fb_card })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 21);

    // 2. Filter::CastForDash: with {1}{R}, can cast ONLY via Dash (not Normal {2}{R}).
    let dash_card = the(&engine, ZoneLocation::Hand(me()), IDX_DASH_CREATURE);
    add_mana(&mut engine, me(), ManaColor::Red, 2);
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&dash_card),
        "CastForDash discount must allow casting 2R dash with 1R"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: dash_card })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(dash_card).unwrap().zone,
        Zone::Battlefield
    );

    // 3. PlotActionCostsLess: {2}{U} plot reduced by {2} -> payable with {U}!
    let plot_card = the(&engine, ZoneLocation::Hand(me()), IDX_PLOT_SPELL);
    add_mana(&mut engine, me(), ManaColor::Blue, 1);
    let (_, legal) = offer(&engine);
    assert!(
        legal.abilities.contains(&(plot_card, PLOT_CARD)),
        "PlotActionCostsLess must allow plotting 2U for U"
    );
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: plot_card,
                ability_index: PLOT_CARD,
            },
        )
        .unwrap();
    assert_eq!(engine.state().object(plot_card).unwrap().zone, Zone::Exile);

    // 4. MorphTurnFaceUpCostsMore (+{2}): Morph {1}{G} now costs {3}{G} (unaffordable with {1}{G}),
    //    while manifested non-morph creature {1}{G} is NOT taxed and turns face up for {1}{G}!
    let morph_id = the(&engine, ZoneLocation::Hand(me()), IDX_MORPH_CREATURE);
    let manifest_id = the(&engine, ZoneLocation::Hand(me()), IDX_MANIFEST_CREATURE);
    {
        let st = engine.dev_state_mut(me()).unwrap();
        let _ = st.move_object(
            morph_id,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        );
        st.object_mut(morph_id)
            .unwrap()
            .status
            .insert(crate::object::Status::FACE_DOWN);
        let _ = st.move_object(
            manifest_id,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        );
        st.object_mut(manifest_id)
            .unwrap()
            .status
            .insert(crate::object::Status::FACE_DOWN);
        st.object_mut(manifest_id)
            .unwrap()
            .riders
            .push(crate::object::Rider::Manifested);
    }
    add_mana(&mut engine, me(), ManaColor::Green, 2);
    let (_, legal) = offer(&engine);
    assert!(
        !legal.abilities.contains(&(morph_id, TURN_FACE_UP)),
        "MorphTurnFaceUpCostsMore must tax Morph 1G to 3G"
    );
    assert!(
        legal.abilities.contains(&(manifest_id, TURN_FACE_UP)),
        "MorphTurnFaceUpCostsMore must NOT tax manifested creature mana cost 1G"
    );
    // Add 2 more mana -> now Morph {3}{G} is offered and succeeds!
    add_mana(&mut engine, me(), ManaColor::Colorless, 2);
    let (_, legal) = offer(&engine);
    assert!(legal.abilities.contains(&(morph_id, TURN_FACE_UP)));
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: morph_id,
                ability_index: TURN_FACE_UP,
            },
        )
        .unwrap();
    assert!(
        !engine
            .state()
            .object(morph_id)
            .unwrap()
            .status
            .contains(crate::object::Status::FACE_DOWN)
    );
}

#[test]
fn cemetery_prowler_first_spell_and_commander_tax_replaced_with_life() {
    static PROWLER_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::SpellsCostLessPerSharedCardTypeWithExiled {
            who: PlayerRel::You,
        },
    )];
    static FIRST_SPELL_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::FirstSpellEachTurnCostAdjustByAmount {
            who: PlayerRel::You,
            filter: &Filter::CREATURE,
            amount: Amount::Fixed(2),
            increase: false,
        },
    )];
    static LIESA_ABILS: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        filter: Filter::This,
        layer: Layer::Text,
        modifier: Modifier::CommanderTaxReplacedWithLife { life_per_tax: 2 },
        source_zone: ZoneRef::Command,
        condition: None,
    })];

    let cards = vec![
        card(
            IDX_PROWLER,
            creature_face("Cemetery Prowler", "{1}{G}{G}", 3, 4),
            KeywordSet::EMPTY,
            PROWLER_ABILS,
        ),
        card(
            IDX_BEAR,
            creature_face("Bear One", "{3}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_BEAR_2,
            creature_face("Bear Two", "{3}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_PLAIN_ARTIFACT,
            face("Relic", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_ARTIFACT_CREATURE,
            FaceDef {
                types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
                ..creature_face("Golem", "{4}", 3, 3)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_FIRST_SPELL_RED,
            face("First Creature Discount", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            FIRST_SPELL_ABILS,
        ),
    ];

    // Part A: Cemetery Prowler counts each shared card type once, even with 2 exiled creatures.
    let mut engine = bench(
        96_141,
        cards.clone(),
        [
            Seat::with(&[IDX_PROWLER]).holding(&[
                IDX_BEAR,
                IDX_BEAR_2,
                IDX_PLAIN_ARTIFACT,
                IDX_ARTIFACT_CREATURE,
            ]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());
    let prowler = the(&engine, ZoneLocation::Battlefield, IDX_PROWLER);
    let exiled_c1 = the(&engine, ZoneLocation::Hand(me()), IDX_BEAR);
    let exiled_art = the(&engine, ZoneLocation::Hand(me()), IDX_PLAIN_ARTIFACT);
    let bear2 = the(&engine, ZoneLocation::Hand(me()), IDX_BEAR_2);
    let golem = the(&engine, ZoneLocation::Hand(me()), IDX_ARTIFACT_CREATURE);
    {
        let st = engine.dev_state_mut(me()).unwrap();
        let prowler_ver = st.object(prowler).unwrap().version;
        let _ = st.move_object(
            exiled_c1,
            ZoneLocation::Exile(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        );
        st.object_mut(exiled_c1)
            .unwrap()
            .riders
            .push(crate::object::Rider::ExiledWith {
                host: prowler,
                version: u16::try_from(prowler_ver).unwrap(),
            });
        let _ = st.move_object(
            exiled_art,
            ZoneLocation::Exile(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        );
        st.object_mut(exiled_art)
            .unwrap()
            .riders
            .push(crate::object::Rider::ExiledWith {
                host: prowler,
                version: u16::try_from(prowler_ver).unwrap(),
            });
    }
    // Bear Two ({3}{G}) shares Creature (1 type) -> {2}{G}, NOT affordable with {1}{G}.
    add_mana(&mut engine, me(), ManaColor::Green, 2);
    let (_, legal) = offer(&engine);
    assert!(!legal.castable.contains(&bear2));
    // Golem ({4}, Artifact Creature) shares Artifact + Creature (2 types) -> {2}, affordable with {G}{G}!
    assert!(
        legal.castable.contains(&golem),
        "Artifact Creature shares 2 card types with exiled cards -> costs 2"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: golem })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(golem).unwrap().zone,
        Zone::Battlefield
    );

    // Part B: FirstSpellEachTurnCostAdjustByAmount discounts 1st creature ({3}{G} -> {1}{G}) only.
    let mut engine = bench(
        96_142,
        cards,
        [
            Seat::with(&[IDX_FIRST_SPELL_RED]).holding(&[IDX_BEAR, IDX_BEAR_2]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());
    let b1 = the(&engine, ZoneLocation::Hand(me()), IDX_BEAR);
    let b2 = the(&engine, ZoneLocation::Hand(me()), IDX_BEAR_2);
    add_mana(&mut engine, me(), ManaColor::Green, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: b1 })
        .expect("1st creature discounted {3}{G} -> {1}{G}");
    settle(&mut engine, me());
    add_mana(&mut engine, me(), ManaColor::Green, 2);
    let (_, legal) = offer(&engine);
    assert!(
        !legal.castable.contains(&b2),
        "2nd creature spell this turn must NOT get FirstSpellEachTurnCostAdjustByAmount"
    );

    // Part C: CommanderTaxReplacedWithLife (Liesa) replaces {2} mana tax with 2 life/cast.
    let liesa_def: &'static CardDef = Box::leak(Box::new(CardDef {
        index: CardIndex::new(IDX_LIESA_CMD),
        oracle_id: "test-liesa",
        scryfall_id: "test-liesa",
        faces: Box::leak(Box::new([FaceDef {
            supertypes: SupertypeSet::LEGENDARY,
            ..creature_face("Liesa Test", "{1}{W}", 5, 5)
        }])),
        color_identity: ColorSet::of(Color::White),
        keywords: KeywordSet::FLYING,
        commander: CommanderRule::Legendary,
        partner: PartnerKind::None,
        fuse: false,
        coverage: Coverage::Implemented,
        abilities: LIESA_ABILS,
    }));
    let entry = |idx: u32| DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: CardIndex::new(idx),
        print: baylee_core::ids::PrintRef::new(0),
    };
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(forest())).collect();
    let seat_spec = |cmds: Vec<u32>, life: i32| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities {
            dev_commands: true,
            see_hidden: false,
        },
        deck: deck.clone(),
        sideboard: vec![],
        commanders: cmds.into_iter().map(entry).collect(),
        starting_life: Some(life),
        starting_hand: Some(vec![]),
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    let preset = GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format: FormatId::Freeform,
        seed: 96_143,
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
        seats: vec![seat_spec(vec![IDX_LIESA_CMD], 1), seat_spec(vec![], 20)],
    };
    let mut cmd_engine =
        Engine::new(&preset, SyntheticLookup::new(vec![liesa_def])).expect("commander bench");
    while let Pending::Mulligan { player, .. } = cmd_engine.pending().clone() {
        cmd_engine
            .apply(player, PlayerAction::MulliganKeep)
            .unwrap();
    }
    to_main(&mut cmd_engine, me());
    let liesa = the(&cmd_engine, ZoneLocation::Command(me()), IDX_LIESA_CMD);
    // Simulate 1 previous cast from command zone -> tax is 2 life (NOT {2} mana).
    cmd_engine.dev_state_mut(me()).unwrap().commanders[0][0].casts = 1;
    // With only 1 life, player cannot pay 2 life tax!
    add_mana(&mut cmd_engine, me(), ManaColor::White, 2);
    let (_, legal) = offer(&cmd_engine);
    assert!(
        !legal.castable.contains(&liesa),
        "Liesa must not be castable when life (1) < life commander tax (2)"
    );
    // Raise life to 10; with exact base cost {1}{W} (no {2} mana tax!), Liesa casts and deducts 2 life!
    cmd_engine.dev_state_mut(me()).unwrap().players[0].life = 10;
    cmd_engine.refresh_offer();
    let (_, legal) = offer(&cmd_engine);
    assert!(
        legal.castable.contains(&liesa),
        "Liesa must cast for base 1W + 2 life without paying 2 mana tax"
    );
    cmd_engine
        .apply(me(), PlayerAction::CastSpell { card: liesa })
        .unwrap();
    settle(&mut cmd_engine, me());
    assert_eq!(
        life(&cmd_engine, me()),
        8,
        "casting Liesa deducted 2 life tax"
    );
    assert_eq!(
        cmd_engine.state().object(liesa).unwrap().zone,
        Zone::Battlefield
    );
}

#[test]
fn target_based_spell_cost_modifiers_and_cost_reduction_variants() {
    static SIEGE_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::RaiseCostIfTargetsYouOrPermanent {
            who: PlayerRel::Opponent,
            spell_filter: &Filter::Any,
            permanent_filter: &Filter::YOUR_CREATURE,
            extra_mana: ManaCost::parse("{2}"),
            extra_life: 2,
        },
    )];
    static FIREBALL_SELF_ABILS: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            filter: Filter::This,
            layer: Layer::Text,
            modifier: Modifier::AdjustSpellCostBeyondFirstTarget {
                who: PlayerRel::You,
                spell_filter: &Filter::This,
                mana_per_extra_target: ManaCost::parse("{1}"),
            },
            source_zone: ZoneRef::Stack,
            condition: None,
        }),
        spell_ab(
            TARGET_PING,
            Some(TargetReq {
                spec: TargetSpec::Object(&Filter::CREATURE),
                min: 1,
                max: 3,
                count_is_x: false,
                chooser: baylee_cards_dsl::TargetChooser::Controller,
                distinct_mana_values: false,
            }),
        ),
    ];
    static GY_CHECK_ABILS: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            filter: Filter::This,
            layer: Layer::Text,
            modifier: Modifier::ReduceCostIfTargetsCheck {
                who: PlayerRel::You,
                spell_filter: &Filter::This,
                reduction: ManaCost::parse("{2}"),
                check: TargetCostCheck::TargetControllerGraveyardAtLeast(3),
            },
            source_zone: ZoneRef::Stack,
            condition: None,
        }),
        spell_ab(
            TARGET_PING,
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
    ];
    static STACK_CHECK_ABILS: &[AbilityDef] = &[
        AbilityDef::Static(StaticAbility {
            filter: Filter::This,
            layer: Layer::Text,
            modifier: Modifier::ReduceCostIfTargetsCheck {
                who: PlayerRel::You,
                spell_filter: &Filter::This,
                reduction: ManaCost::parse("{2}"),
                check: TargetCostCheck::TargetedStackTargetsYouControl(&Filter::CREATURE),
            },
            source_zone: ZoneRef::Stack,
            condition: None,
        }),
        spell_ab(
            &[Effect::CounterTargetSpell],
            Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
        ),
    ];
    static SPELL_PING: &[AbilityDef] = &[spell_ab(
        TARGET_PING,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static IF_MANA_ABILS: &[AbilityDef] = &[spell_ab(
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        None,
    )];

    let cards = vec![
        card(
            IDX_SIEGE_TAXER,
            face("Monastery Siege", "{2}{U}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            SIEGE_ABILS,
        ),
        card(
            IDX_BEAR,
            creature_face("Bear A", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_BEAR_2,
            creature_face("Bear B", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_BEAR_3,
            creature_face("Bear C", "{1}{G}", 3, 3),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_RED_SPELL,
            face("Ping", "{R}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SPELL_PING,
        ),
        card(
            IDX_MULTI_TARGET_SPELL,
            face("Fireball Lite", "{R}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            FIREBALL_SELF_ABILS,
        ),
        card(
            IDX_GY_CHECK_SPELL,
            face("Grave Discount", "{2}{B}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            GY_CHECK_ABILS,
        ),
        card(
            IDX_STACK_CHECK_COUNTER,
            face("Protect Counter", "{2}{U}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            STACK_CHECK_ABILS,
        ),
        card(
            IDX_IF_MANA_SPELL,
            FaceDef {
                cost_reduction: Some(CostReduction::IfMana {
                    condition: Condition::OpponentHasNoCardsInHand,
                    mana: ManaCost::parse("{1}{B}"),
                }),
                ..face("Empty Hand Drain", "{2}{B}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            IF_MANA_ABILS,
        ),
        card(
            IDX_IF_TARGETS_SPELL,
            FaceDef {
                cost_reduction: Some(CostReduction::IfTargetsMatch {
                    filter: &Filter::CREATURE,
                    min_count: 1,
                    amount: 2,
                }),
                ..face("Target Discount", "{2}{G}", TypeSet::INSTANT)
            },
            KeywordSet::EMPTY,
            SPELL_PING,
        ),
    ];

    let mut engine = bench(
        96_144,
        cards,
        [
            Seat::with(&[IDX_SIEGE_TAXER, IDX_BEAR, IDX_BEAR_2]).holding(&[
                IDX_MULTI_TARGET_SPELL,
                IDX_GY_CHECK_SPELL,
                IDX_STACK_CHECK_COUNTER,
                IDX_IF_MANA_SPELL,
                IDX_IF_TARGETS_SPELL,
            ]),
            Seat::with(&[IDX_BEAR_3]).holding(&[IDX_RED_SPELL, IDX_BEAR, IDX_BEAR, IDX_BEAR]),
        ],
    );
    to_main(&mut engine, me());

    let bear_a = the(&engine, ZoneLocation::Battlefield, IDX_BEAR);
    let bear_b = the(&engine, ZoneLocation::Battlefield, IDX_BEAR_2);
    let opp_bear = the(&engine, ZoneLocation::Battlefield, IDX_BEAR_3);

    // 1. AdjustSpellCostBeyondFirstTarget: targeting 3 creatures costs {R} + {2} = {2}{R}.
    let fb = the(&engine, ZoneLocation::Hand(me()), IDX_MULTI_TARGET_SPELL);
    add_mana(&mut engine, me(), ManaColor::Red, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: fb })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_a, bear_b, opp_bear],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "paid 2R for 3 targets"
    );

    // 2. ReduceCostIfTargetsCheck(TargetControllerGraveyardAtLeast(3)):
    //    opp has 0 cards in GY -> {2}{B} not castable with {B}; put 3 cards in opp GY -> castable for {B}!
    let gy_sp = the(&engine, ZoneLocation::Hand(me()), IDX_GY_CHECK_SPELL);
    add_mana(&mut engine, me(), ManaColor::Black, 1);
    let (_, legal) = offer(&engine);
    assert!(!legal.castable.contains(&gy_sp));
    {
        let opp_bears = objects(&engine, ZoneLocation::Hand(them()), IDX_BEAR);
        let st = engine.dev_state_mut(me()).unwrap();
        for b in opp_bears {
            let _ = st.move_object(
                b,
                ZoneLocation::Graveyard(them()),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            );
        }
    }
    engine.refresh_offer();
    let (_, legal) = offer(&engine);
    assert!(legal.castable.contains(&gy_sp));
    engine
        .apply(me(), PlayerAction::CastSpell { card: gy_sp })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_bear],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    // 3. CostReduction::IfTargetsMatch: {2}{G} costs {G} when targeting a creature.
    let tgt_sp = the(&engine, ZoneLocation::Hand(me()), IDX_IF_TARGETS_SPELL);
    add_mana(&mut engine, me(), ManaColor::Green, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: tgt_sp })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_bear],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // 4. CostReduction::IfMana + Condition::OpponentHasNoCardsInHand:
    //    opp still holds Red Ping -> {2}{B} not affordable with {1}; wait until opp casts Red Ping!
    let empty_sp = the(&engine, ZoneLocation::Hand(me()), IDX_IF_MANA_SPELL);
    add_mana(&mut engine, me(), ManaColor::Colorless, 1);
    let (_, legal) = offer(&engine);
    assert!(
        !legal.castable.contains(&empty_sp),
        "OpponentHasNoCardsInHand is false while opponent holds a card"
    );

    // Pass priority to opponent so opponent casts Red Ping targeting our Bear A under Monastery Siege (+{2} & 2 life),
    // and we counter it with ReduceCostIfTargetsCheck(TargetedStackTargetsYouControl) for just {U}!
    let opp_ping = the(&engine, ZoneLocation::Hand(them()), IDX_RED_SPELL);
    let my_counter = the(&engine, ZoneLocation::Hand(me()), IDX_STACK_CHECK_COUNTER);
    add_mana(&mut engine, them(), ManaColor::Red, 3);
    add_mana(&mut engine, me(), ManaColor::Blue, 1);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(them(), PlayerAction::CastSpell { card: opp_ping })
        .unwrap();
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_a],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(
        life(&engine, them()),
        18,
        "RaiseCostIfTargetsYouOrPermanent taxed opponent 2 life + 2 mana"
    );
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    // Opponent passes -> we cast Protect Counter ({2}{U} discounted by {2} to {U}) targeting opp_ping!
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(me(), PlayerAction::CastSpell { card: my_counter })
        .expect("TargetedStackTargetsYouControl discounts {2}{U} to {U}");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![opp_ping],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    // Now opponent's hand is empty! Empty Hand Drain ({2}{B} minus {1}{B} = {1}) is now castable with {1}!
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&empty_sp),
        "CostReduction::IfMana reduces 2B by 1B to 1 once opponent hand is empty"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: empty_sp })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 21);
}

// =========================================================================
// 4. Activated Ability Cost Adjustments
// =========================================================================

#[test]
fn activated_ability_additional_cost_first_each_turn_boast_ninjutsu_and_equip() {
    static ACT_TAX_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::ActivatedAbilitiesCostAdditional {
            who: PlayerRel::You,
            filter: &Filter::CREATURE,
            cost: CostPart::PayLife(2),
        },
    )];
    static PINGER_ABILS: &[AbilityDef] = &[act_ab(
        "{2}",
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        None,
    )];
    static FIRST_ACT_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::FirstActivatedAbilityEachTurnCostLess {
            who: PlayerRel::You,
            source_filter: &Filter::CREATURE,
            target_filter: &Filter::Any,
            amount: 2,
            your_turn_only: true,
        },
    )];
    static BOAST_RED_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::ActivatedAbilitiesWithKeywordCostLess {
            who: PlayerRel::You,
            keyword: ActivatedAbilityKind::Boast,
            amount: Amount::Fixed(2),
        },
    )];
    static BOAST_CREATURE_ABILS: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::parse("{2}{R}"),
            parts: &[],
        },
        effects: &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Boast,
        x_value: ActivationX::Any,
        cost_reduction: None,
    }];
    static NINJUTSU_RED_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::ActivatedAbilitiesWithKeywordCostLess {
            who: PlayerRel::You,
            keyword: ActivatedAbilityKind::Ninjutsu,
            amount: Amount::Fixed(1),
        },
    )];
    static NINJA_ABILS: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::parse("{1}{U}"),
            parts: &[CostPart::ReturnUnblockedAttacker],
        },
        effects: &[Effect::PutNinjutsuOntoBattlefield],
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Hand,
        limit: ActivationLimit::Unlimited,
        x_value: ActivationX::Any,
        cost_reduction: None,
    }];
    static EQUIP_POWER_RED: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::EquipCostsLessByTargetAmount {
            who: PlayerRel::You,
            metric: EquipTargetMetric::Power,
        },
    )];
    static EQUIP_COLOR_RED: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::EquipCostsLessByTargetAmount {
            who: PlayerRel::You,
            metric: EquipTargetMetric::ColorCount,
        },
    )];
    static EQUIP_COUNTER_RED: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::EquipCostsLessByTargetAmount {
            who: PlayerRel::You,
            metric: EquipTargetMetric::Counters(CounterKind::P1P1),
        },
    )];
    static EQUIP_ABILS: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::parse("{4}"),
            parts: &[],
        },
        effects: &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::YOUR_CREATURE),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::SorcerySpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        x_value: ActivationX::Any,
        cost_reduction: None,
    }];

    let cards = vec![
        card(
            IDX_ACT_TAXER,
            face("Activation Tax", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            ACT_TAX_ABILS,
        ),
        card(
            IDX_ACT_PINGER,
            creature_face("Pinger", "{1}", 1, 1),
            KeywordSet::EMPTY,
            PINGER_ABILS,
        ),
        card(
            IDX_FIRST_ACT_RED,
            face("First Activation Discount", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            FIRST_ACT_ABILS,
        ),
        card(
            IDX_BOAST_RED,
            face("Birgi Boast Discount", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            BOAST_RED_ABILS,
        ),
        card(
            IDX_BOAST_CREATURE,
            creature_face("Boaster", "{1}{R}", 2, 2),
            KeywordSet::HASTE,
            BOAST_CREATURE_ABILS,
        ),
        card(
            IDX_NINJUTSU_RED,
            face("Ninja Discount", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            NINJUTSU_RED_ABILS,
        ),
        card(
            IDX_NINJA_CARD,
            creature_face("Silver Ninja", "{3}{U}", 2, 2),
            KeywordSet::EMPTY,
            NINJA_ABILS,
        ),
        card(
            IDX_EQUIP_RED_POWER,
            face("Power Equip Discount", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            EQUIP_POWER_RED,
        ),
        card(
            IDX_EQUIP_RED_COLOR,
            face("Color Equip Discount", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            EQUIP_COLOR_RED,
        ),
        card(
            IDX_EQUIP_RED_COUNTER,
            face("Counter Equip Discount", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            EQUIP_COUNTER_RED,
        ),
        card(
            IDX_EQUIPMENT,
            FaceDef {
                types: TypeSet::ARTIFACT,
                subtypes: &[artifact::EQUIPMENT],
                ..face("Heavy Sword", "{1}", TypeSet::ARTIFACT)
            },
            KeywordSet::EMPTY,
            EQUIP_ABILS,
        ),
        card(
            IDX_BIG_CREATURE,
            creature_face("Giant", "{3}{G}", 3, 3),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_MULTI_COLOR_CREATURE,
            creature_face("Tri-Color", "{W}{U}{B}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_SMALL_CREATURE,
            creature_face("Mite", "{1}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    // Part A: ActivatedAbilitiesCostAdditional + FirstActivatedAbilityEachTurnCostLess
    let mut engine = bench(
        96_160,
        cards.clone(),
        [
            Seat::with(&[IDX_ACT_TAXER, IDX_FIRST_ACT_RED, IDX_ACT_PINGER]).at(20),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());
    let pinger = the(&engine, ZoneLocation::Battlefield, IDX_ACT_PINGER);
    // 1st activation: {2} discounted by {2} -> {0} mana, plus PayLife(2) additional cost!
    activate(&mut engine, me(), pinger, 0);
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        19,
        "paid 2 life additional cost, gained 1 life"
    );
    // 2nd activation same turn: no longer discounted -> requires {2} mana, not offered with 0 mana!
    let (_, legal) = offer(&engine);
    assert!(
        !legal.abilities.contains(&(pinger, 0)),
        "2nd activation this turn must NOT receive FirstActivatedAbilityEachTurnCostLess"
    );

    // Part B: Boast & Ninjutsu discounts in combat!
    let mut engine = bench(
        96_161,
        cards.clone(),
        [
            Seat::with(&[IDX_BOAST_RED, IDX_NINJUTSU_RED, IDX_BOAST_CREATURE])
                .holding(&[IDX_NINJA_CARD]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());
    let boaster = the(&engine, ZoneLocation::Battlefield, IDX_BOAST_CREATURE);
    let ninja = the(&engine, ZoneLocation::Hand(me()), IDX_NINJA_CARD);
    // Advance to DeclareAttackers and attack with boaster.
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(boaster, baylee_core::ids::Defender::Player(them()))],
            },
        )
        .unwrap();
    // Advance to post-blockers priority (no blockers).
    walk_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareBlockers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    // Boast {2}{R} discounted by {2} -> payable with {R}!
    add_mana(&mut engine, me(), ManaColor::Red, 1);
    activate(&mut engine, me(), boaster, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 23, "Boast activated at discounted R");

    // Ninjutsu {1}{U} discounted by {1} -> payable with {U}!
    add_mana(&mut engine, me(), ManaColor::Blue, 1);
    let (_, legal) = offer(&engine);
    assert!(
        legal.abilities.contains(&(ninja, 0)),
        "Ninjutsu 1U must be discounted to U"
    );
    activate(&mut engine, me(), ninja, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![boaster],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(ninja).unwrap().zone,
        Zone::Battlefield
    );

    // Part C: EquipCostsLessByTargetAmount (Power, ColorCount, Counters)
    for (idx_red, target_idx, add_p1p1, expected_cost) in [
        (IDX_EQUIP_RED_POWER, IDX_BIG_CREATURE, 0_u32, 1_u32), // {4} - power 3 = {1}
        (IDX_EQUIP_RED_COLOR, IDX_MULTI_COLOR_CREATURE, 0, 1), // {4} - 3 colors = {1}
        (IDX_EQUIP_RED_COUNTER, IDX_SMALL_CREATURE, 2, 2),     // {4} - 2 P1P1 = {2}
    ] {
        let mut eq_eng = bench(
            96_162 + u64::from(expected_cost),
            cards.clone(),
            [
                Seat::with(&[idx_red, IDX_EQUIPMENT, target_idx]),
                Seat::with(&[]),
            ],
        );
        to_main(&mut eq_eng, me());
        let eq = the(&eq_eng, ZoneLocation::Battlefield, IDX_EQUIPMENT);
        let tgt = the(&eq_eng, ZoneLocation::Battlefield, target_idx);
        if add_p1p1 > 0 {
            seed_counter(
                eq_eng.dev_state_mut(me()).unwrap(),
                tgt,
                CounterKind::P1P1,
                add_p1p1 as u16,
            );
        }
        add_mana(&mut eq_eng, me(), ManaColor::Colorless, expected_cost);
        activate(&mut eq_eng, me(), eq, 0);
        eq_eng
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![tgt],
                    players: vec![],
                },
            )
            .unwrap();
        settle(&mut eq_eng, me());
        assert_eq!(eq_eng.state().object(eq).unwrap().attached_to, Some(tgt));
        assert_eq!(eq_eng.state().players[0].mana_pool.total(), 0);
    }
}

// =========================================================================
// 5. New Conditions, Amounts, and Per-Turn Hash Invariance
// =========================================================================

#[test]
fn new_conditions_amounts_and_per_turn_hash_invariance() {
    static SAC_ABILS: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::Sacrifice(&Filter::ControlledByYou)],
        },
        effects: &[],
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        x_value: ActivationX::Any,
        cost_reduction: None,
    }];
    static ENERGY_SPEND_ABILS: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::PayEnergy(2)],
        },
        effects: &[],
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        x_value: ActivationX::Any,
        cost_reduction: None,
    }];
    static ENERGY_READER_ABILS: &[AbilityDef] = &[
        act_ab(
            "{0}",
            &[
                Effect::GainLife {
                    amount: Amount::EnergyRemovedThisTurn(PlayerRel::You),
                },
                Effect::GainLife {
                    amount: Amount::OpponentsAttackedThisTurn,
                },
                Effect::IfCondition {
                    condition: Condition::PlayerAttackedYouLastTurn,
                    then: &[Effect::GainLife {
                        amount: Amount::Fixed(1),
                    }],
                    otherwise: &[],
                },
            ],
            None,
        ),
        act_ab(
            "{0}",
            &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::Player(PlayerRel::Opponent),
            }],
            None,
        ),
    ];

    let cards = vec![
        card(
            IDX_ELF_1,
            FaceDef {
                subtypes: &[creature::ELF],
                ..creature_face("Elf 1", "{G}", 2, 2)
            },
            KeywordSet::HASTE,
            &[],
        ),
        card(
            IDX_ELF_2,
            FaceDef {
                subtypes: &[creature::ELF],
                ..creature_face("Elf 2", "{G}", 2, 2)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_ELF_3,
            FaceDef {
                subtypes: &[creature::ELF],
                ..creature_face("Elf 3", "{G}", 2, 2)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_AURA_POWER,
            FaceDef {
                types: TypeSet::ARTIFACT,
                subtypes: &[artifact::EQUIPMENT],
                ..face("Power Lens", "{1}", TypeSet::ARTIFACT)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_PLAIN_ARTIFACT,
            face("Bauble", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_SAC_OUTLET,
            face("Altar", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            SAC_ABILS,
        ),
        card(
            IDX_ENERGY_SPENDER,
            face("Energy Sink", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            ENERGY_SPEND_ABILS,
        ),
        card(
            IDX_ENERGY_READER,
            face("Energy Reader", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            ENERGY_READER_ABILS,
        ),
    ];

    let mut engine = bench(
        96_180,
        cards.clone(),
        [
            Seat::with(&[
                IDX_ELF_1,
                IDX_ELF_2,
                IDX_AURA_POWER,
                IDX_PLAIN_ARTIFACT,
                IDX_SAC_OUTLET,
                IDX_ENERGY_SPENDER,
                IDX_ENERGY_READER,
            ]),
            Seat::with(&[IDX_ELF_3]),
        ],
    );
    to_main(&mut engine, me());

    let elf1 = the(&engine, ZoneLocation::Battlefield, IDX_ELF_1);
    let elf2 = the(&engine, ZoneLocation::Battlefield, IDX_ELF_2);
    let aura = the(&engine, ZoneLocation::Battlefield, IDX_AURA_POWER);
    let bauble = the(&engine, ZoneLocation::Battlefield, IDX_PLAIN_ARTIFACT);
    let altar = the(&engine, ZoneLocation::Battlefield, IDX_SAC_OUTLET);
    let spender = the(&engine, ZoneLocation::Battlefield, IDX_ENERGY_SPENDER);

    // Attach aura to elf1 (power 2) -> Amount::AttachedPower == 2.
    engine
        .dev_state_mut(me())
        .unwrap()
        .object_mut(aura)
        .unwrap()
        .attached_to = Some(elf1);
    assert_eq!(
        crate::eval::amount(&Amount::AttachedPower, engine.state(), me(), aura, None),
        2
    );

    // YouControlSharingCreatureType: 2 Elves -> >=2 true, >=3 false!
    assert!(crate::eval::condition_holds(
        engine.state(),
        me(),
        elf1,
        Condition::YouControlSharingCreatureType(2)
    ));
    assert!(!crate::eval::condition_holds(
        engine.state(),
        me(),
        elf1,
        Condition::YouControlSharingCreatureType(3)
    ));

    // AnyPlayerLifeAtMostHalfStarting (starting 20 -> <= 10): false at 20, true at 10.
    assert!(!crate::eval::condition_holds(
        engine.state(),
        me(),
        elf1,
        Condition::AnyPlayerLifeAtMostHalfStarting
    ));
    engine.dev_state_mut(me()).unwrap().players[1].life = 10;
    assert!(crate::eval::condition_holds(
        engine.state(),
        me(),
        elf1,
        Condition::AnyPlayerLifeAtMostHalfStarting
    ));

    // IsDay / IsNight: both false when day_night is None; toggle Day / Night.
    assert!(!crate::eval::condition_holds(
        engine.state(),
        me(),
        elf1,
        Condition::IsDay
    ));
    assert!(!crate::eval::condition_holds(
        engine.state(),
        me(),
        elf1,
        Condition::IsNight
    ));
    engine.dev_state_mut(me()).unwrap().day_night = Some(crate::turn::DayNight::Night);
    assert!(crate::eval::condition_holds(
        engine.state(),
        me(),
        elf1,
        Condition::IsNight
    ));
    assert!(!crate::eval::condition_holds(
        engine.state(),
        me(),
        elf1,
        Condition::IsDay
    ));
    engine.dev_state_mut(me()).unwrap().day_night = Some(crate::turn::DayNight::Day);
    assert!(crate::eval::condition_holds(
        engine.state(),
        me(),
        elf1,
        Condition::IsDay
    ));

    // DistinctCardTypesInZone: Creature + Enchantment + Artifact = 3 distinct types on our battlefield!
    assert_eq!(
        crate::eval::amount(
            &Amount::DistinctCardTypesInZone {
                zone: ZoneSel::Battlefield,
                filter: &Filter::ControlledByYou,
            },
            engine.state(),
            me(),
            elf1,
            None,
        ),
        3
    );

    // DistinctSacrificedCardTypesThisTurn: sacrifice elf2 (Creature) + bauble (Artifact) -> 2!
    activate(&mut engine, me(), altar, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![elf2],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    activate(&mut engine, me(), altar, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bauble],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        crate::eval::amount(
            &Amount::DistinctSacrificedCardTypesThisTurn(PlayerRel::You),
            engine.state(),
            me(),
            elf1,
            None,
        ),
        2
    );

    // EnergyRemovedThisTurn: pay 2 energy -> amount == 2, reader gains 2 life!
    engine
        .dev_state_mut(me())
        .unwrap()
        .put_player_counters_by(me(), me(), CounterKind::Energy, 4);
    engine.refresh_offer();
    let hash_before = engine.state().snapshot_hash();
    activate(&mut engine, me(), spender, 0);
    settle(&mut engine, me());
    assert_ne!(
        engine.state().snapshot_hash(),
        hash_before,
        "tracked EnergyRemovedThisTurn must affect snapshot_hash when reader is present"
    );
    assert_eq!(
        crate::eval::amount(
            &Amount::EnergyRemovedThisTurn(PlayerRel::You),
            engine.state(),
            me(),
            elf1,
            None,
        ),
        2
    );

    // OpponentsAttackedThisTurn + PlayerAttackedYouLastTurn + OpponentsDealtDamageThisTurn:
    // Attack them() with elf1 -> OpponentsAttackedThisTurn == 1, combat damage deals 2 ->
    // OpponentsDealtDamageThisTurn == 1, NonCombatDamageDealtToOpponentsThisTurn == 0!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf1, baylee_core::ids::Defender::Player(them()))],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    assert_eq!(
        crate::eval::amount(
            &Amount::OpponentsAttackedThisTurn,
            engine.state(),
            me(),
            elf1,
            None
        ),
        1
    );
    assert_eq!(
        crate::eval::amount(
            &Amount::OpponentsDealtDamageThisTurn,
            engine.state(),
            me(),
            elf1,
            None
        ),
        1
    );
    assert_eq!(
        crate::eval::amount(
            &Amount::NonCombatDamageDealtToOpponentsThisTurn,
            engine.state(),
            me(),
            elf1,
            None,
        ),
        0
    );
    // Deal 2 noncombat damage to opponent -> NonCombatDamageDealtToOpponentsThisTurn == 2!
    let reader = the(&engine, ZoneLocation::Battlefield, IDX_ENERGY_READER);
    activate(&mut engine, me(), reader, 1);
    settle(&mut engine, me());
    assert_eq!(
        crate::eval::amount(
            &Amount::NonCombatDamageDealtToOpponentsThisTurn,
            engine.state(),
            me(),
            elf1,
            None,
        ),
        2
    );

    // Advance to them()'s main phase: PlayerAttackedYouLastTurn is true for them() (me() attacked them() last turn)!
    to_main(&mut engine, them());
    let opp_elf = the(&engine, ZoneLocation::Battlefield, IDX_ELF_3);
    assert!(
        crate::eval::condition_holds(
            engine.state(),
            them(),
            opp_elf,
            Condition::PlayerAttackedYouLastTurn
        ),
        "PlayerAttackedYouLastTurn must be true on them()'s turn after me() attacked them()"
    );

    // Hash invariance when NO card in the game reads energy_removed / opponents_attacked:
    let mut untracked = bench(
        96_181,
        cards,
        [Seat::with(&[IDX_ELF_1]), Seat::with(&[IDX_ELF_3])],
    );
    to_main(&mut untracked, me());
    assert!(!untracked.state().per_turn.energy_removed.tracked);
    assert!(!untracked.state().per_turn.opponents_attacked.tracked);
    assert!(!untracked.state().last_turn_attacked_players.tracked);
    let base_hash = untracked.state().snapshot_hash();
    let base_loop = untracked.state().loop_signature();
    untracked.state.per_turn.energy_removed.items.push(
        crate::state::per_turn::EnergyRemovedRecord {
            player: me(),
            amount: 5,
        },
    );
    untracked.state.per_turn.opponents_attacked.items.push(
        crate::state::per_turn::OpponentAttackedRecord {
            attacker: me(),
            defender: them(),
        },
    );
    assert_eq!(
        untracked.state().snapshot_hash(),
        base_hash,
        "untracked per_turn records must not alter snapshot_hash when no card reads them"
    );
    assert_eq!(
        untracked.state().loop_signature(),
        base_loop,
        "untracked per_turn records must not alter loop_signature when no card reads them"
    );
}

#[test]
fn hash_sensitivity_for_filters_subject_awaiting_and_tracked_turn_logs() {
    static FB_CREATED_ABILS: &[AbilityDef] = &[act_ab(
        "{0}",
        &[Effect::CreateContinuousEffect {
            filter: &Filter::CastForFlashback,
            layer: Layer::Text,
            modifier: Modifier::SpellsCostLess(1),
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        }],
        None,
    )];
    static DASH_CREATED_ABILS: &[AbilityDef] = &[act_ab(
        "{0}",
        &[Effect::CreateContinuousEffect {
            filter: &Filter::CastForDash,
            layer: Layer::Text,
            modifier: Modifier::SpellsCostLess(1),
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        }],
        None,
    )];

    // Tags 595 & 596: Filter::CastForFlashback vs Filter::CastForDash produce distinct created-effect hashes.
    let mut e_fb = bench(
        96_190,
        vec![card(
            IDX_BEAR,
            creature_face("Hash Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            FB_CREATED_ABILS,
        )],
        [Seat::with(&[IDX_BEAR]), Seat::with(&[])],
    );
    let mut e_dash = bench(
        96_190,
        vec![card(
            IDX_BEAR,
            creature_face("Hash Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            DASH_CREATED_ABILS,
        )],
        [Seat::with(&[IDX_BEAR]), Seat::with(&[])],
    );
    to_main(&mut e_fb, me());
    to_main(&mut e_dash, me());
    let bear_fb = the(&e_fb, ZoneLocation::Battlefield, IDX_BEAR);
    let bear_dash = the(&e_dash, ZoneLocation::Battlefield, IDX_BEAR);
    activate(&mut e_fb, me(), bear_fb, 0);
    settle(&mut e_fb, me());
    activate(&mut e_dash, me(), bear_dash, 0);
    settle(&mut e_dash, me());
    assert_ne!(
        e_fb.state().snapshot_hash(),
        e_dash.state().snapshot_hash(),
        "tags 595 vs 596 must produce distinct snapshot_hash"
    );

    let bear = bear_fb;
    let bear_obj = e_fb.state().object(bear).unwrap().clone();
    let bear_ref = baylee_core::ids::DamageSourceRef {
        object: bear,
        version: bear_obj.version,
    };

    // Tags 601, 602, 603, 604: tracked PerTurnTracked records move both snapshot_hash and loop_signature.
    let mut st = e_fb.state().clone();
    st.per_turn.energy_removed.tracked = true;
    st.per_turn.opponents_attacked.tracked = true;
    st.per_turn.activated_abilities_this_turn.tracked = true;
    st.last_turn_attacked_players.tracked = true;

    let h0 = st.snapshot_hash();
    let l0 = st.loop_signature();
    st.per_turn
        .energy_removed
        .items
        .push(crate::state::per_turn::EnergyRemovedRecord {
            player: me(),
            amount: 2,
        });
    let h1 = st.snapshot_hash();
    let l1 = st.loop_signature();
    assert_ne!(h1, h0, "tag 601 must change snapshot_hash");
    assert_ne!(l1, l0, "tag 601 must change loop_signature");

    st.per_turn
        .opponents_attacked
        .items
        .push(crate::state::per_turn::OpponentAttackedRecord {
            attacker: me(),
            defender: them(),
        });
    let h2 = st.snapshot_hash();
    let l2 = st.loop_signature();
    assert_ne!(h2, h1, "tag 602 must change snapshot_hash");
    assert_ne!(l2, l1, "tag 602 must change loop_signature");

    let bear_snap = std::sync::Arc::new(bear_obj);
    st.per_turn.activated_abilities_this_turn.items.push(
        crate::state::per_turn::ActivatedAbilityRecord {
            activator: me(),
            source: bear_ref,
            snapshot: std::sync::Arc::clone(&bear_snap),
            target_snapshots: vec![bear_snap],
        },
    );
    let h3 = st.snapshot_hash();
    let l3 = st.loop_signature();
    assert_ne!(h3, h2, "tag 603 must change snapshot_hash");
    assert_ne!(l3, l2, "tag 603 must change loop_signature");

    st.last_turn_attacked_players
        .items
        .push(crate::state::per_turn::LastTurnAttackedRecord {
            attacker: me(),
            defender: them(),
        });
    let h4 = st.snapshot_hash();
    let l4 = st.loop_signature();
    assert_ne!(h4, h3, "tag 604 must change snapshot_hash");
    assert_ne!(l4, l3, "tag 604 must change loop_signature");

    // Tag 598: SubjectContext::counters_moved changes fingerprint when > 0.
    let mut ctx0 = crate::resolve::SubjectContext::default();
    let f0 = ctx0.fingerprint();
    ctx0.counters_moved = 3;
    assert_ne!(
        ctx0.fingerprint(),
        f0,
        "tag 598 must change SubjectContext fingerprint"
    );
}

// =========================================================================
// 6. Review Regression Coverage (Layer 6 HexproofFrom*, Disguise untaxed,
//    mid-turn PlayerAttackedYouLastTurn hash, RemoveCounters/MoveCounters
//    reads, activation target LKI snapshots, AttachedPower LKI, and
//    RaiseCostIfTargetsYouOrPermanent guaranteed player preview)
// =========================================================================

#[test]
fn hexproof_from_layer_ability_obeys_lose_all_abilities_timestamps() {
    static GRANT_HEX_BLACK: &[AbilityDef] = &[spell_ab(
        &[Effect::AddTempModifier {
            target: TargetSpec::Object(&Filter::CREATURE),
            modifier: Modifier::HexproofFrom(&BLACK_FILTER),
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static DRESS_DOWN_ABILS: &[AbilityDef] =
        &[static_ab(Filter::CREATURE, Modifier::LoseAllAbilities)];
    static BLACK_PING_ABILS: &[AbilityDef] = &[spell_ab(
        TARGET_PING,
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let cards = vec![
        card(
            IDX_BEAR,
            creature_face("Plain Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_BLUE_SPELL,
            face("Grant Hex Black", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            GRANT_HEX_BLACK,
        ),
        card(
            IDX_LOSE_HEXPROOF,
            face("Dress Down", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DRESS_DOWN_ABILS,
        ),
        card(
            IDX_BLACK_SPELL,
            face("Dark Ping", "{B}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            BLACK_PING_ABILS,
        ),
    ];

    let mut engine = bench(
        96_191,
        cards,
        [
            Seat::with(&[IDX_BEAR]).holding(&[IDX_BLUE_SPELL, IDX_LOSE_HEXPROOF, IDX_BLUE_SPELL]),
            Seat::with(&[]).holding(&[IDX_BLACK_SPELL]),
        ],
    );
    to_main(&mut engine, me());

    let bear = the(&engine, ZoneLocation::Battlefield, IDX_BEAR);
    let opp_black = the(&engine, ZoneLocation::Hand(them()), IDX_BLACK_SPELL);
    let grants = objects(&engine, ZoneLocation::Hand(me()), IDX_BLUE_SPELL);

    // T1: grant HexproofFrom(Black) -> opponent's black spell is blocked.
    engine
        .apply(me(), PlayerAction::CastSpell { card: grants[0] })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert!(
        !target_options_for_spell(&engine, them(), opp_black).contains(&bear),
        "T1 granted HexproofFrom(Black) must block opponent black spell"
    );

    // T2: Dress Down (LoseAllAbilities) enters -> strips T1 HexproofFrom(Black) in Layer 6!
    let dress = the(&engine, ZoneLocation::Hand(me()), IDX_LOSE_HEXPROOF);
    engine
        .apply(me(), PlayerAction::CastSpell { card: dress })
        .unwrap();
    settle(&mut engine, me());
    assert!(
        target_options_for_spell(&engine, them(), opp_black).contains(&bear),
        "CR 613.1f/613.7: T2 LoseAllAbilities must strip T1 granted HexproofFrom(Black)"
    );

    // T3: grant HexproofFrom(Black) AFTER T2 Dress Down -> survives in Layer 6 timestamp order!
    engine
        .apply(me(), PlayerAction::CastSpell { card: grants[1] })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert!(
        !target_options_for_spell(&engine, them(), opp_black).contains(&bear),
        "CR 613.7: T3 granted HexproofFrom(Black) must override T2 LoseAllAbilities"
    );
}

#[test]
fn morph_turn_face_up_tax_applies_to_megamorph_but_not_disguise() {
    static MORPH_TAX_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::MorphTurnFaceUpCostsMore { amount: 2 },
    )];
    let cards = vec![
        card(
            IDX_MORPH_TAXER,
            creature_face("Exiled Doomsayer", "{1}{W}", 1, 2),
            KeywordSet::EMPTY,
            MORPH_TAX_ABILS,
        ),
        card(
            IDX_MORPH_CREATURE,
            FaceDef {
                megamorph: Some(ManaCost::parse("{1}{G}")),
                ..creature_face("Megamorph Bear", "{2}{G}", 2, 2)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_MANIFEST_CREATURE,
            FaceDef {
                disguise: Some(ManaCost::parse("{1}{G}")),
                ..creature_face("Disguise Bear", "{2}{G}", 2, 2)
            },
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        96_192,
        cards,
        [
            Seat::with(&[IDX_MORPH_TAXER]).holding(&[IDX_MORPH_CREATURE, IDX_MANIFEST_CREATURE]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let mega_id = the(&engine, ZoneLocation::Hand(me()), IDX_MORPH_CREATURE);
    let disg_id = the(&engine, ZoneLocation::Hand(me()), IDX_MANIFEST_CREATURE);
    {
        let st = engine.dev_state_mut(me()).unwrap();
        let _ = st.move_object(
            mega_id,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        );
        st.object_mut(mega_id)
            .unwrap()
            .status
            .insert(crate::object::Status::FACE_DOWN);
        let _ = st.move_object(
            disg_id,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        );
        st.object_mut(disg_id)
            .unwrap()
            .status
            .insert(crate::object::Status::FACE_DOWN);
    }
    // With {1}{G}: Megamorph is taxed to {3}{G} (NOT offered), Disguise is untaxed {1}{G} (OFFERED)!
    add_mana(&mut engine, me(), ManaColor::Green, 2);
    let (_, legal) = offer(&engine);
    assert!(
        !legal.abilities.contains(&(mega_id, TURN_FACE_UP)),
        "CR 702.37b: MorphTurnFaceUpCostsMore must tax Megamorph"
    );
    assert!(
        legal.abilities.contains(&(disg_id, TURN_FACE_UP)),
        "CR 702.168: MorphTurnFaceUpCostsMore must NOT tax Disguise"
    );
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: disg_id,
                ability_index: TURN_FACE_UP,
            },
        )
        .unwrap();
    assert!(
        !engine
            .state()
            .object(disg_id)
            .unwrap()
            .status
            .contains(crate::object::Status::FACE_DOWN)
    );
}

#[test]
fn player_attacked_last_turn_only_deck_hashes_mid_turn_and_counter_effects_enable_reads() {
    static LAST_TURN_ONLY_ABILS: &[AbilityDef] = &[act_ab(
        "{0}",
        &[Effect::IfCondition {
            condition: Condition::PlayerAttackedYouLastTurn,
            then: &[Effect::GainLife {
                amount: Amount::Fixed(1),
            }],
            otherwise: &[],
        }],
        None,
    )];
    static REMOVE_TRACKED_AMT_ABILS: &[AbilityDef] = &[spell_ab(
        &[Effect::RemoveCounters {
            subject: RemoveCounterSubject::Source,
            kind: None,
            count: RemoveCounterCount::Exact(Amount::EnergyRemovedThisTurn(PlayerRel::You)),
        }],
        None,
    )];
    static MOVE_TRACKED_FILTER_ABILS: &[AbilityDef] = &[spell_ab(
        &[Effect::MoveCounters {
            from: MoveCounterSource::Filter(&Filter::CmcAtMostAmount(
                &Amount::OpponentsAttackedThisTurn,
            )),
            to: MoveCounterDest::Source,
            kind: None,
            count: MoveCounterCount::All,
        }],
        None,
    )];

    // A: PlayerAttackedYouLastTurn-only deck must track per_turn.opponents_attacked mid-turn!
    let mut engine = bench(
        96_193,
        vec![
            card(
                IDX_ELF_1,
                creature_face("Haste Elf", "{G}", 2, 2),
                KeywordSet::HASTE,
                &[],
            ),
            card(
                IDX_ENERGY_READER,
                face("LastTurn Watcher", "{0}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                LAST_TURN_ONLY_ABILS,
            ),
        ],
        [Seat::with(&[IDX_ELF_1, IDX_ENERGY_READER]), Seat::with(&[])],
    );
    to_main(&mut engine, me());
    assert!(
        engine.state().per_turn.opponents_attacked.tracked,
        "PlayerAttackedYouLastTurn-only deck must enable per_turn.opponents_attacked.tracked"
    );
    let h_before = engine.state().snapshot_hash();
    let l_before = engine.state().loop_signature();
    engine.state.per_turn.opponents_attacked.items.push(
        crate::state::per_turn::OpponentAttackedRecord {
            attacker: me(),
            defender: them(),
        },
    );
    assert_ne!(
        engine.state().snapshot_hash(),
        h_before,
        "mid-turn attack must change snapshot_hash"
    );
    assert_ne!(
        engine.state().loop_signature(),
        l_before,
        "mid-turn attack must change loop_signature"
    );

    // B: state/reads.rs traverses Effect::RemoveCounters (amount) and Effect::MoveCounters (filter).
    let rem_eng = bench(
        96_194,
        vec![card(
            IDX_REMOVE_EXACT_2,
            face("Energy Remove", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            REMOVE_TRACKED_AMT_ABILS,
        )],
        [
            Seat::with(&[]).holding(&[IDX_REMOVE_EXACT_2]),
            Seat::with(&[]),
        ],
    );
    assert!(
        rem_eng.state().per_turn.energy_removed.tracked,
        "Effect::RemoveCounters embedding EnergyRemovedThisTurn must enable energy_removed.tracked"
    );

    let mov_card = card(
        IDX_MOVE_FILTER_ALL,
        face("Move Cast", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        MOVE_TRACKED_FILTER_ABILS,
    );
    let mov_eng = bench(
        96_195,
        vec![mov_card],
        [
            Seat::with(&[]).holding(&[IDX_MOVE_FILTER_ALL]),
            Seat::with(&[]),
        ],
    );
    assert!(
        mov_eng.state().per_turn.opponents_attacked.tracked,
        "Effect::MoveCounters filter embedding OpponentsAttackedThisTurn must enable opponents_attacked.tracked"
    );
}

#[test]
fn first_activated_ability_uses_target_snapshot_lki_after_first_target_dies() {
    static HOJO_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::FirstActivatedAbilityEachTurnCostLess {
            who: PlayerRel::You,
            source_filter: &Filter::Any,
            target_filter: &Filter::YOUR_CREATURE,
            amount: 2,
            your_turn_only: false,
        },
    )];
    static TARGETED_PUMP_ABILS: &[AbilityDef] = &[act_ab(
        "{2}",
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
    )];
    static SAC_ABILS: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::Sacrifice(&Filter::YOUR_CREATURE)],
        },
        effects: &[],
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        x_value: ActivationX::Any,
        cost_reduction: None,
    }];

    let cards = vec![
        card(
            IDX_FIRST_ACT_RED,
            creature_face("Hojo", "{1}{U}", 1, 3),
            KeywordSet::EMPTY,
            HOJO_ABILS,
        ),
        card(
            IDX_ACT_PINGER,
            face("Targeted Device", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            TARGETED_PUMP_ABILS,
        ),
        card(
            IDX_SAC_OUTLET,
            face("Altar", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            SAC_ABILS,
        ),
        card(
            IDX_BEAR,
            creature_face("Target A", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_BEAR_2,
            creature_face("Target B", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    let mut engine = bench(
        96_196,
        cards,
        [
            Seat::with(&[
                IDX_FIRST_ACT_RED,
                IDX_ACT_PINGER,
                IDX_SAC_OUTLET,
                IDX_BEAR,
                IDX_BEAR_2,
            ]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());

    let device = the(&engine, ZoneLocation::Battlefield, IDX_ACT_PINGER);
    let altar = the(&engine, ZoneLocation::Battlefield, IDX_SAC_OUTLET);
    let bear_a = the(&engine, ZoneLocation::Battlefield, IDX_BEAR);

    // 1st activation targeting bear_a: {2} discounted to {0}!
    activate(&mut engine, me(), device, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![bear_a],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 21);

    // Sacrifice bear_a so it is now in the graveyard (no longer YOUR_CREATURE in live state).
    activate(&mut engine, me(), altar, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bear_a],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().object(bear_a).unwrap().zone, Zone::Graveyard);

    // 2nd activation targeting bear_b MUST NOT get the discount again!
    let (_, legal) = offer(&engine);
    assert!(
        !legal.abilities.contains(&(device, 0)),
        "FirstActivatedAbilityEachTurnCostLess must remember activation-time target snapshot after target dies"
    );
}

#[test]
fn attached_power_uses_lki_after_equipment_leaves_and_siege_previews_guaranteed_player_tax() {
    static SAC_EQUIP_LKI_ABILS: &[AbilityDef] = &[AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::SacrificeSelf],
        },
        effects: &[Effect::GainLife {
            amount: Amount::AttachedPower,
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        x_value: ActivationX::Any,
        cost_reduction: None,
    }];
    static SIEGE_ABILS: &[AbilityDef] = &[static_ab(
        Filter::This,
        Modifier::RaiseCostIfTargetsYouOrPermanent {
            who: PlayerRel::Opponent,
            spell_filter: &Filter::Any,
            permanent_filter: &Filter::YOUR_CREATURE,
            extra_mana: ManaCost::parse("{2}"),
            extra_life: 0,
        },
    )];
    static FACE_BURN_ABILS: &[AbilityDef] = &[spell_ab(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::AnyOpponent,
        }],
        Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )];

    let cards = vec![
        card(
            IDX_BIG_CREATURE,
            creature_face("Big 4/4", "{3}{G}", 4, 4),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            IDX_EQUIPMENT,
            FaceDef {
                types: TypeSet::ARTIFACT,
                subtypes: &[artifact::EQUIPMENT],
                ..face("LKI Sword", "{1}", TypeSet::ARTIFACT)
            },
            KeywordSet::EMPTY,
            SAC_EQUIP_LKI_ABILS,
        ),
        card(
            IDX_SIEGE_TAXER,
            face("Monastery Siege", "{2}{U}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            SIEGE_ABILS,
        ),
        card(
            IDX_RED_SPELL,
            face("Opponent Burn", "{R}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            FACE_BURN_ABILS,
        ),
    ];

    let mut engine = bench(
        96_197,
        cards,
        [
            Seat::with(&[IDX_BIG_CREATURE, IDX_EQUIPMENT, IDX_SIEGE_TAXER]),
            Seat::with(&[]).holding(&[IDX_RED_SPELL]),
        ],
    );
    to_main(&mut engine, me());

    let big = the(&engine, ZoneLocation::Battlefield, IDX_BIG_CREATURE);
    let sword = the(&engine, ZoneLocation::Battlefield, IDX_EQUIPMENT);
    engine
        .dev_state_mut(me())
        .unwrap()
        .object_mut(sword)
        .unwrap()
        .attached_to = Some(big);
    engine.refresh_offer();

    // Activate SacrificeSelf ability on attached Equipment -> resolves after sword is in graveyard -> gains 4 life via LKI!
    activate(&mut engine, me(), sword, 0);
    settle(&mut engine, me());
    assert_eq!(engine.state().object(sword).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        life(&engine, me()),
        24,
        "CR 608.2h: Amount::AttachedPower must read LKI of host (4)"
    );

    // Pass to opponent: Opponent Burn ({R}, TargetSpec::AnyOpponent) MUST target me() (Siege controller) ->
    // preview min_mana_increase includes {2}, so with only {R} it is NOT offered in legal.castable!
    let opp_burn = the(&engine, ZoneLocation::Hand(them()), IDX_RED_SPELL);
    to_main(&mut engine, them());
    add_mana(&mut engine, them(), ManaColor::Red, 1);
    let (_, legal) = offer(&engine);
    assert!(
        !legal.castable.contains(&opp_burn),
        "RaiseCostIfTargetsYouOrPermanent preview must include guaranteed player targeting tax"
    );
    add_mana(&mut engine, them(), ManaColor::Red, 2);
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&opp_burn),
        "Opponent Burn becomes castable once guaranteed player tax is affordable"
    );
}
