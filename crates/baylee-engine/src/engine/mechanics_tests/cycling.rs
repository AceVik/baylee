//! Cycling triggers: battlefield listeners (`Trigger::Cycled`), self-triggers
//! resolving before the cycling draw (CR 702.29c, CR 603.3), and triggers
//! firing when discard replacement sends the cycled card to exile (CR 702.29c).

use super::*;
use baylee_cards_dsl::{
    Cost, CostPart, Duration, Filter, Find, Modifier, PlayerRel, Trigger, triggered,
};
use baylee_core::mana::ManaCost;

const CYCLER_VANILLA: u32 = 7901;
const CYCLER_LIFTER: u32 = 7902;
const CYCLER_TYPE: u32 = 7903;
const TRANSMUTE_SPELL: u32 = 7904;
const WATCHER_ALL: u32 = 7905;
const WATCHER_OPP: u32 = 7906;
const WATCHER_CREATURE: u32 = 7907;
const EXILE_REPLACER: u32 = 7908;

const CYCLE_FREE: Cost = Cost {
    mana: ManaCost::ZERO,
    parts: &[CostPart::DiscardSelf],
};

static BASIC_LAND: Filter = Filter::BASIC_LAND;
static CREATURE_FILTER: Filter = Filter::CREATURE;

static CYCLER_VANILLA_ABILITIES: &[AbilityDef] = &[AbilityDef::Activated {
    cost: CYCLE_FREE,
    effects: &[Effect::draw(1)],
    targets: None,
    second_targets: None,
    third_targets: None,
    timing: ActivationTiming::InstantSpeed,
    mana_ability: false,
    zone: ActivationZone::Hand,
    limit: ActivationLimit::Unlimited,
    x_value: baylee_cards_dsl::ActivationX::Any,
    cost_reduction: None,
}];

static CYCLER_LIFTER_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Activated {
        cost: CYCLE_FREE,
        effects: &[Effect::draw(1)],
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Hand,
        limit: ActivationLimit::Unlimited,
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    },
    triggered!(
        Trigger::CycledThis,
        &[Effect::SearchLibrary {
            filter: &BASIC_LAND,
            finds: &[Find::HAND],
            optional: true,
        }]
    ),
];

static CYCLER_TYPE_ABILITIES: &[AbilityDef] = &[AbilityDef::Activated {
    cost: CYCLE_FREE,
    effects: &[Effect::SearchLibrary {
        filter: &BASIC_LAND,
        finds: &[Find::HAND],
        optional: false,
    }],
    targets: None,
    second_targets: None,
    third_targets: None,
    timing: ActivationTiming::InstantSpeed,
    mana_ability: false,
    zone: ActivationZone::Hand,
    limit: ActivationLimit::Unlimited,
    x_value: baylee_cards_dsl::ActivationX::Any,
    cost_reduction: None,
}];

static TRANSMUTE_ABILITIES: &[AbilityDef] = &[AbilityDef::Activated {
    cost: CYCLE_FREE,
    effects: &[Effect::SearchLibrary {
        filter: &Filter::Any,
        finds: &[Find::HAND],
        optional: false,
    }],
    targets: None,
    second_targets: None,
    third_targets: None,
    timing: ActivationTiming::SorcerySpeed,
    mana_ability: false,
    zone: ActivationZone::Hand,
    limit: ActivationLimit::Unlimited,
    x_value: baylee_cards_dsl::ActivationX::Any,
    cost_reduction: None,
}];

static WATCHER_ALL_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::Cycled {
        by: PlayerRel::EachPlayer,
        filter: &Filter::Any,
    },
    &[Effect::gain_life(1)]
)];

static WATCHER_OPP_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::Cycled {
        by: PlayerRel::Opponent,
        filter: &Filter::Any,
    },
    &[Effect::gain_life(2)]
)];

static WATCHER_CREATURE_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::Cycled {
        by: PlayerRel::EachPlayer,
        filter: &CREATURE_FILTER,
    },
    &[Effect::gain_life(3)]
)];

static EXILE_REPLACER_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::continuous(
        &Filter::Any,
        Modifier::ExileInsteadOfYourGraveyard,
        Duration::UntilEndOfTurn,
    )],
    None,
)];

fn cards() -> Vec<&'static CardDef> {
    vec![
        card(
            CYCLER_VANILLA,
            face("Plain Cycle", "{2}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            CYCLER_VANILLA_ABILITIES,
        ),
        card(
            CYCLER_LIFTER,
            creature_face("Lifter", "{4}{G}", 4, 4),
            KeywordSet::EMPTY,
            CYCLER_LIFTER_ABILITIES,
        ),
        card(
            CYCLER_TYPE,
            creature_face("Type Cycler", "{4}{W}", 3, 3),
            KeywordSet::EMPTY,
            CYCLER_TYPE_ABILITIES,
        ),
        card(
            TRANSMUTE_SPELL,
            face("Sorcery Transmuter", "{2}{U}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            TRANSMUTE_ABILITIES,
        ),
        card(
            WATCHER_ALL,
            face("All Watcher", "{1}{W}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            WATCHER_ALL_ABILITIES,
        ),
        card(
            WATCHER_OPP,
            face("Opp Watcher", "{1}{U}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            WATCHER_OPP_ABILITIES,
        ),
        card(
            WATCHER_CREATURE,
            face("Creature Watcher", "{1}{R}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            WATCHER_CREATURE_ABILITIES,
        ),
        card(
            EXILE_REPLACER,
            face("Exile Cage", "{1}{B}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            EXILE_REPLACER_ABILITIES,
        ),
    ]
}

fn settle_with_search(engine: &mut Bench, seat: PlayerId, pick_first: bool) -> bool {
    let mut saw_search = false;
    loop {
        match engine.pending().clone() {
            Pending::ChooseCards {
                player, options, ..
            } => {
                saw_search = true;
                let chosen = if pick_first && !options.is_empty() {
                    vec![options[0]]
                } else {
                    vec![]
                };
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: chosen })
                    .unwrap();
            }
            Pending::Priority { player, .. }
                if engine.state().zones.stack_is_empty() && player == seat =>
            {
                return saw_search;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending state during search resolution: {other:?}"),
        }
    }
}

/// Battlefield `Trigger::Cycled` listeners respect `PlayerRel` (yours vs
/// opponent's vs each player) and `filter` (creature vs noncreature), react to
/// instant-speed typecycling (CR 702.29f),
/// and ignore sorcery-speed transmute (CR 702.53a).
#[test]
fn cycling_battlefield_triggers_filter_player_card_type_and_typecycling() {
    let mut engine = bench(
        7901,
        cards(),
        [
            Seat::with(&[WATCHER_ALL, WATCHER_OPP, WATCHER_CREATURE]).holding(&[
                CYCLER_VANILLA,
                CYCLER_TYPE,
                TRANSMUTE_SPELL,
            ]),
            Seat::with(&[]).holding(&[CYCLER_VANILLA, CYCLER_LIFTER]),
        ],
    );
    to_main(&mut engine, me());

    let w_all = the(&engine, ZoneLocation::Battlefield, WATCHER_ALL);
    let w_opp = the(&engine, ZoneLocation::Battlefield, WATCHER_OPP);
    let w_creature = the(&engine, ZoneLocation::Battlefield, WATCHER_CREATURE);

    // 1. P0 cycles noncreature CYCLER_VANILLA -> WATCHER_ALL (+1), not WATCHER_OPP, not WATCHER_CREATURE.
    let c_spell = the(&engine, ZoneLocation::Hand(me()), CYCLER_VANILLA);
    activate(&mut engine, me(), c_spell, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 21);
    assert_eq!(triggered(&engine, w_all), 1);
    assert_eq!(triggered(&engine, w_opp), 0);
    assert_eq!(triggered(&engine, w_creature), 0);

    // 2. P0 activates sorcery-speed Transmute -> NOT cycling, triggers nothing.
    let transmute = the(&engine, ZoneLocation::Hand(me()), TRANSMUTE_SPELL);
    activate(&mut engine, me(), transmute, 0);
    settle_with_search(&mut engine, me(), true);
    assert_eq!(triggered(&engine, w_all), 1);

    // 3. P0 activates creature Typecycling (CR 702.29f) -> WATCHER_ALL (+1) AND WATCHER_CREATURE (+3).
    let type_c = the(&engine, ZoneLocation::Hand(me()), CYCLER_TYPE);
    activate(&mut engine, me(), type_c, 0);
    settle_with_search(&mut engine, me(), true);
    assert_eq!(life(&engine, me()), 25);
    assert_eq!(triggered(&engine, w_all), 2);
    assert_eq!(triggered(&engine, w_creature), 1);

    // 4. Opponent cycles noncreature -> WATCHER_ALL (+1) and WATCHER_OPP (+2).
    to_main(&mut engine, them());
    let opp_spell = the(&engine, ZoneLocation::Hand(them()), CYCLER_VANILLA);
    activate(&mut engine, them(), opp_spell, 0);
    settle(&mut engine, them());
    assert_eq!(life(&engine, me()), 28);
    assert_eq!(triggered(&engine, w_all), 3);
    assert_eq!(triggered(&engine, w_opp), 1);
    assert_eq!(triggered(&engine, w_creature), 1);

    // 5. Opponent cycles creature -> all three watchers trigger (+1 +2 +3 = 34).
    let opp_cre = the(&engine, ZoneLocation::Hand(them()), CYCLER_LIFTER);
    activate(&mut engine, them(), opp_cre, 0);
    settle_with_search(&mut engine, them(), false);
    assert_eq!(life(&engine, me()), 34);
    assert_eq!(triggered(&engine, w_all), 4);
    assert_eq!(triggered(&engine, w_opp), 2);
    assert_eq!(triggered(&engine, w_creature), 2);
}

/// CR 702.29c / CR 603.3: `Trigger::CycledThis` goes onto the stack ABOVE the
/// cycling draw ability, so its search resolves before the card draw.
#[test]
fn cycled_this_trigger_stacks_above_and_resolves_before_cycling_draw() {
    let mut engine = bench(
        7902,
        cards(),
        [Seat::with(&[]).holding(&[CYCLER_LIFTER]), Seat::default()],
    );
    to_main(&mut engine, me());
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    let card_id = the(&engine, ZoneLocation::Hand(me()), CYCLER_LIFTER);

    activate(&mut engine, me(), card_id, 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        2,
        "cycling draw and CycledThis trigger should both sit on the stack"
    );

    loop {
        match engine.pending().clone() {
            Pending::ChooseCards {
                player, options, ..
            } => {
                assert_eq!(
                    engine.state().zones.list(ZoneLocation::Hand(me())).len(),
                    hand_before - 1,
                    "CycledThis resolves before the cycling draw adds a card to hand"
                );
                assert_eq!(
                    engine.state().zones.list(ZoneLocation::Stack).len(),
                    2,
                    "both CycledThis (resolving on top) and cycling draw (waiting below) sit on stack"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
                assert_eq!(
                    engine.state().zones.list(ZoneLocation::Stack).len(),
                    1,
                    "CycledThis finished resolving first, leaving cycling draw on the stack"
                );
                assert_eq!(
                    engine.state().zones.list(ZoneLocation::Hand(me())).len(),
                    hand_before,
                    "SearchLibrary added 1 land before the cycling draw resolved"
                );
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected before search: {other:?}"),
        }
    }
    settle(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before + 1
    );
}

/// CR 702.29c: "These abilities trigger from whatever zone the card winds up in
/// after it's cycled." An exile replacement effect sending the discarded card
/// to exile must still fire both `Trigger::CycledThis` and `Trigger::Cycled`.
#[test]
fn cycling_triggers_when_discard_is_replaced_into_exile() {
    let mut engine = bench(
        7903,
        cards(),
        [
            Seat::with(&[EXILE_REPLACER, WATCHER_ALL, WATCHER_CREATURE]).holding(&[CYCLER_LIFTER]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let replacer = the(&engine, ZoneLocation::Battlefield, EXILE_REPLACER);
    activate(&mut engine, me(), replacer, 0);
    settle(&mut engine, me());

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    let card_id = the(&engine, ZoneLocation::Hand(me()), CYCLER_LIFTER);
    let w_all = the(&engine, ZoneLocation::Battlefield, WATCHER_ALL);
    let w_creature = the(&engine, ZoneLocation::Battlefield, WATCHER_CREATURE);

    activate(&mut engine, me(), card_id, 0);
    assert_eq!(
        engine.state().object(card_id).unwrap().zone,
        Zone::Exile,
        "ExileInsteadOfYourGraveyard sent cycled card to Exile"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(me()))
            .is_empty(),
        "nothing entered the graveyard"
    );

    let saw_search = settle_with_search(&mut engine, me(), false);
    assert!(saw_search, "Trigger::CycledThis fired even from Exile");
    assert_eq!(
        life(&engine, me()),
        24,
        "both Cycled watchers (+1 and +3) fired"
    );
    assert_eq!(triggered(&engine, w_all), 1);
    assert_eq!(triggered(&engine, w_creature), 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before
    );
}
