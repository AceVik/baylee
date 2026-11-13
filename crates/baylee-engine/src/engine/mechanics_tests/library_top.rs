//! Playing the current library top for its mana value in life.
//!
//! The grant is resolved through an activated ability. Cards are moved to the
//! library only as test setup; every permission, offer and payment below is
//! then exercised through the ordinary engine action path.

use super::*;
use crate::choice::CastModeKind;
use baylee_cards_dsl::{
    AltCondition, AlternativeCost, CostPart, Duration, Filter, ModeCount, Modifier, SpellMode,
};

const GRANTOR: u32 = 7210;
const SORCERY: u32 = 7211;
const INSTANT: u32 = 7212;
const X_SPELL: u32 = 7213;
const TEST_LAND: u32 = 7214;
const COSTLESS: u32 = 7215;
const ALTERNATIVE: u32 = 7216;
const FIXED_LIFE: u32 = 7217;
const TAXER: u32 = 7218;
const DISCOUNT: u32 = 7219;
const FODDER: u32 = 7220;
const SACRIFICE_SPELL: u32 = 7221;
const PAY_X_AND_FIXED: u32 = 7222;
const KICKED: u32 = 7223;
const MODAL: u32 = 7224;
const MDFC: u32 = 7225;
const OTHER_LAND: u32 = 7226;
const LIFE_KICKER: u32 = 7227;
const BACK_LIFE_KICKER: u32 = 7228;

static GRANT_EFFECTS: &[Effect] = &[
    Effect::continuous(
        &Filter::Any,
        Modifier::LookAtLibraryTop,
        Duration::UntilEndOfTurn,
    ),
    Effect::continuous(
        &Filter::Any,
        Modifier::PlayFromLibraryTopForManaValueLife,
        Duration::UntilEndOfTurn,
    ),
];
static GRANT: &[AbilityDef] = &[free(GRANT_EFFECTS, None)];
static NOTHING: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[],
    targets: None,
    second_targets: None,
    condition: None,
}];
static ALTERNATIVE_COST: &[AlternativeCost] = &[AlternativeCost {
    cost: Cost {
        mana: ManaCost::parse("{1}"),
        parts: &[],
    },
    condition: AltCondition::Always,
}];
static FIXED_LIFE_COST: &[CostPart] = &[CostPart::PayLife(2)];
static TAX: &[AbilityDef] = &[baylee_cards_dsl::static_ability!(
    Filter::Any,
    Modifier::SpellsCostMore(2)
)];
static PRICE_REDUCTION: &[AbilityDef] = &[baylee_cards_dsl::static_ability!(
    Filter::Any,
    Modifier::SpellsCostLess(1)
)];
static SACRIFICE_COST: &[CostPart] = &[CostPart::Sacrifice(&Filter::CREATURE)];
static PAY_X_AND_FIXED_COST: &[CostPart] = &[CostPart::PayLife(2), CostPart::PayLifeX];
static KICKER_COST: &[Cost] = &[Cost {
    mana: ManaCost::parse("{2}"),
    parts: &[],
}];
static LIFE_KICKER_COST: &[Cost] = &[Cost {
    mana: ManaCost::ZERO,
    parts: &[CostPart::PayLife(2)],
}];
static BACK_LIFE_KICKER_COST: &[Cost] = &[Cost {
    mana: ManaCost::ZERO,
    parts: &[CostPart::PayLife(4)],
}];
static MODAL_SPELL: &[AbilityDef] = &[AbilityDef::ModalSpell {
    modes: &[
        SpellMode {
            effects: &[],
            targets: None,
            second_targets: None,
            cost_override: None,
            additional_cost: None,
        },
        SpellMode {
            effects: &[],
            targets: None,
            second_targets: None,
            cost_override: None,
            additional_cost: Some(ManaCost::parse("{1}")),
        },
    ],
    choose: ModeCount::ONE,
}];

fn plain_spell(
    index: u32,
    name: &'static str,
    cost: &'static str,
    kind: TypeSet,
) -> &'static CardDef {
    card(index, face(name, cost, kind), KeywordSet::EMPTY, NOTHING)
}

fn definitions() -> Vec<&'static CardDef> {
    vec![
        card(
            GRANTOR,
            face("Library grantor", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            GRANT,
        ),
        plain_spell(SORCERY, "Three-value sorcery", "{2}{B}", TypeSet::SORCERY),
        plain_spell(INSTANT, "One-value instant", "{U}", TypeSet::INSTANT),
        plain_spell(X_SPELL, "X and green", "{X}{G}", TypeSet::SORCERY),
        card(TEST_LAND, land_face("Library land"), KeywordSet::EMPTY, &[]),
        card(
            COSTLESS,
            FaceDef {
                mana_cost: ManaCost::ZERO,
                ..face("No printed mana cost", "{0}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            NOTHING,
        ),
        card(
            ALTERNATIVE,
            FaceDef {
                alternative_costs: ALTERNATIVE_COST,
                ..face("Printed alternative cost", "{4}{B}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            NOTHING,
        ),
        card(
            FIXED_LIFE,
            FaceDef {
                mandatory_additional_costs: FIXED_LIFE_COST,
                ..face("Additional fixed life", "{B}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            NOTHING,
        ),
        card(
            TAXER,
            face("Spell tax", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            TAX,
        ),
        card(
            DISCOUNT,
            face("Spell discount", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            PRICE_REDUCTION,
        ),
        card(
            FODDER,
            creature_face("Cost fodder", "{1}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            SACRIFICE_SPELL,
            FaceDef {
                mandatory_additional_costs: SACRIFICE_COST,
                ..face("Sacrifice addition", "{B}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            NOTHING,
        ),
        card(
            PAY_X_AND_FIXED,
            FaceDef {
                mandatory_additional_costs: PAY_X_AND_FIXED_COST,
                ..face("Fixed and X life", "{B}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            NOTHING,
        ),
        card(
            KICKED,
            FaceDef {
                additional_costs: KICKER_COST,
                ..face("Kicked library spell", "{B}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            NOTHING,
        ),
        card(
            MODAL,
            face("Modal library spell", "{B}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            MODAL_SPELL,
        ),
        library_mdfc(),
        card(
            OTHER_LAND,
            land_face("Second library land"),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            LIFE_KICKER,
            FaceDef {
                mandatory_additional_costs: FIXED_LIFE_COST,
                additional_costs: LIFE_KICKER_COST,
                ..face("Life kicker", "{B}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            NOTHING,
        ),
        Box::leak(Box::new(CardDef {
            index: CardIndex::new(BACK_LIFE_KICKER),
            oracle_id: "test",
            scryfall_id: "test",
            faces: Box::leak(Box::new([
                land_face("Life kicker front"),
                FaceDef {
                    abilities: NOTHING,
                    additional_costs: BACK_LIFE_KICKER_COST,
                    ..face("Life kicker back", "{2}{U}", TypeSet::SORCERY)
                },
            ])),
            abilities: NOTHING,
            ..CardDef::DEFAULT
        })),
    ]
}

fn library_mdfc() -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(MDFC),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([
            land_face("Library land front"),
            FaceDef {
                abilities: NOTHING,
                ..face("Library spell back", "{2}{U}", TypeSet::SORCERY)
            },
        ])),
        abilities: NOTHING,
        ..CardDef::DEFAULT
    }))
}

/// Moves setup cards to the library in slice order, making the last one top.
fn setup(seed: u64, hand: &[u32], starting_life: i32, granted: bool) -> Bench {
    setup_with(seed, hand, starting_life, granted, &[])
}

/// The same board with additional permanents needed by a particular cost.
fn setup_with(
    seed: u64,
    hand: &[u32],
    starting_life: i32,
    granted: bool,
    extra_battlefield: &[u32],
) -> Bench {
    let mut battlefield = vec![GRANTOR, forest()];
    battlefield.extend_from_slice(extra_battlefield);
    let mut engine = bench(
        seed,
        definitions(),
        [
            Seat::with(&battlefield).holding(hand).at(starting_life),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    for &index in hand {
        let card = the(&engine, ZoneLocation::Hand(me()), index);
        engine
            .dev_state_mut(me())
            .unwrap()
            .move_object(
                card,
                ZoneLocation::Library(me()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    engine.refresh_offer();
    if granted {
        let source = the(&engine, ZoneLocation::Battlefield, GRANTOR);
        activate(&mut engine, me(), source, 0);
        settle(&mut engine, me());
    }
    engine
}

#[track_caller]
fn choose_library_route(engine: &mut Bench) {
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        return;
    };
    assert_eq!(player, me());
    assert!(
        options
            .iter()
            .all(|option| { matches!(option.kind, CastModeKind::LibraryTopLife { .. }) })
    );
    let route = options
        .iter()
        .position(|option| matches!(option.kind, CastModeKind::LibraryTopLife { .. }))
        .expect("the library route is offered");
    engine
        .apply(me(), PlayerAction::ChooseMode(route))
        .expect("the offered library route is accepted");
}

#[track_caller]
fn cast_top(engine: &mut Bench, card: ObjectId) {
    let (player, legal) = offer(engine);
    assert_eq!(player, me());
    assert!(
        legal.castable.contains(&card),
        "the current top is offered: {legal:?}"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card })
        .expect("the offered top spell begins casting");
    choose_library_route(engine);
    assert_eq!(
        engine.state().object(card).expect("the spell exists").zone,
        Zone::Stack,
        "the paid spell reached the stack: {:?}",
        engine.pending()
    );
}

#[test]
fn no_grant_opens_neither_the_spell_nor_private_top_information() {
    let mut engine = setup(7210, &[SORCERY], 20, false);
    let card = the(&engine, ZoneLocation::Library(me()), SORCERY);
    let (_, legal) = offer(&engine);
    assert!(!legal.castable.contains(&card));
    assert!(!legal.payable.contains(&card));
    assert!(!crate::casting::library_top_visible_to(
        engine.state(),
        me(),
        me()
    ));
    assert!(
        engine
            .apply(me(), PlayerAction::CastSpell { card })
            .is_err()
    );
}

#[test]
fn a_top_spell_pays_exact_mana_value_in_life_without_spending_mana() {
    let mut engine = setup(7211, &[SORCERY], 20, true);
    let card = the(&engine, ZoneLocation::Library(me()), SORCERY);
    float_all(&mut engine, me());
    let pool = engine.state().players[0].mana_pool.clone();
    cast_top(&mut engine, card);
    assert_eq!(life(&engine, me()), 17);
    assert_eq!(engine.state().players[0].mana_pool, pool);
    let spell = engine.state().object(card).unwrap();
    assert!(spell.alt_cast);
    assert_eq!(spell.characteristics().mana_value(), 3);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 17, "life is paid exactly once");
}

#[test]
fn only_the_exact_current_top_of_the_controller_library_is_open() {
    let mut engine = setup(7212, &[SORCERY, INSTANT], 20, true);
    let below = the(&engine, ZoneLocation::Library(me()), SORCERY);
    let top = the(&engine, ZoneLocation::Library(me()), INSTANT);
    let (_, legal) = offer(&engine);
    assert!(legal.castable.contains(&top));
    assert!(!legal.castable.contains(&below));
    assert!(!crate::casting::library_top_life_open(
        engine.state(),
        me(),
        below
    ));
    engine
        .dev_state_mut(me())
        .unwrap()
        .move_object(
            top,
            ZoneLocation::Library(them()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();
    assert!(!crate::casting::library_top_life_open(
        engine.state(),
        me(),
        top
    ));
    assert!(!crate::casting::library_top_life_open(
        engine.state(),
        them(),
        top
    ));
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&below),
        "the newly exposed top opens"
    );
}

#[test]
fn a_top_land_uses_the_ordinary_land_drop_and_costs_no_life() {
    let mut engine = setup(7213, &[OTHER_LAND, TEST_LAND], 20, true);
    let card = the(&engine, ZoneLocation::Library(me()), TEST_LAND);
    let second = the(&engine, ZoneLocation::Library(me()), OTHER_LAND);
    let (_, legal) = offer(&engine);
    assert!(legal.lands.contains(&card));
    assert!(!legal.castable.contains(&card));
    engine.apply(me(), PlayerAction::PlayLand { card }).unwrap();
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Battlefield);
    assert_eq!(life(&engine, me()), 20);
    assert!(!offer(&engine).1.lands.contains(&card));
    assert!(
        !offer(&engine).1.lands.contains(&second),
        "the newly exposed land does not grant a second land drop"
    );
    assert!(
        engine
            .apply(me(), PlayerAction::PlayLand { card: second })
            .is_err()
    );
    assert_eq!(engine.state().object(second).unwrap().zone, Zone::Library);
}

#[test]
fn ordinary_spell_timing_still_applies_with_a_grant_and_nonempty_stack() {
    for (index, castable_with_stack) in [(SORCERY, false), (INSTANT, true)] {
        let mut engine = setup(7214 + u64::from(index), &[index], 20, true);
        let card = the(&engine, ZoneLocation::Library(me()), index);
        assert!(offer(&engine).1.castable.contains(&card));
        let source = the(&engine, ZoneLocation::Battlefield, GRANTOR);
        activate(&mut engine, me(), source, 0);
        assert!(!engine.state().zones.stack_is_empty());
        assert_eq!(
            offer(&engine).1.castable.contains(&card),
            castable_with_stack,
            "timing for card {index}"
        );
        if castable_with_stack {
            cast_top(&mut engine, card);
            assert_eq!(life(&engine, me()), 19);
        }
    }
}

#[test]
fn replacement_mana_cost_forces_x_to_zero_but_preserves_the_spell_mana_value() {
    let mut engine = setup(7215, &[X_SPELL], 20, true);
    let card = the(&engine, ZoneLocation::Library(me()), X_SPELL);
    let options = engine.cast_options(me(), card, false).unwrap();
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].cost, ManaCost::ZERO);
    assert!(matches!(
        options[0].kind,
        CastModeKind::LibraryTopLife {
            life: 1,
            face: 0,
            ..
        }
    ));
    cast_top(&mut engine, card);
    assert_eq!(life(&engine, me()), 19);
    let spell = engine.state().object(card).unwrap();
    assert_eq!(spell.x_value, 0);
    assert_eq!(spell.characteristics().mana_value(), 1);
}

#[test]
fn unpayable_life_neither_offers_nor_partially_pays_for_the_spell() {
    let mut engine = setup(7216, &[SORCERY], 2, true);
    let card = the(&engine, ZoneLocation::Library(me()), SORCERY);
    let (_, legal) = offer(&engine);
    assert!(!legal.castable.contains(&card));
    assert!(!legal.payable.contains(&card));
    let before = engine.state().snapshot_hash();
    assert!(
        engine
            .apply(me(), PlayerAction::CastSpell { card })
            .is_err()
    );
    assert_eq!(engine.state().snapshot_hash(), before);
    assert_eq!(life(&engine, me()), 2);
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Library);
}

#[test]
fn fixed_additional_life_is_paid_in_addition_to_mana_value() {
    let mut engine = setup(7217, &[FIXED_LIFE], 20, true);
    let card = the(&engine, ZoneLocation::Library(me()), FIXED_LIFE);
    cast_top(&mut engine, card);
    assert_eq!(life(&engine, me()), 17, "one for mana value and two extra");
}

#[test]
fn a_card_without_a_printed_mana_cost_has_zero_value_for_this_route() {
    let mut engine = setup(7218, &[COSTLESS], 20, true);
    let card = the(&engine, ZoneLocation::Library(me()), COSTLESS);
    let options = engine.cast_options(me(), card, false).unwrap();
    assert!(matches!(
        options.as_slice(),
        [option] if matches!(option.kind, CastModeKind::LibraryTopLife { life: 0, .. })
    ));
    cast_top(&mut engine, card);
    assert_eq!(life(&engine, me()), 20);
}

#[test]
fn the_life_route_does_not_combine_with_a_printed_alternative_cost() {
    let mut engine = setup(7219, &[ALTERNATIVE], 20, true);
    let card = the(&engine, ZoneLocation::Library(me()), ALTERNATIVE);
    let options = engine.cast_options(me(), card, false).unwrap();
    assert_eq!(options.len(), 1);
    assert!(matches!(
        options[0].kind,
        CastModeKind::LibraryTopLife { life: 5, .. }
    ));
    cast_top(&mut engine, card);
    assert_eq!(life(&engine, me()), 15);
}

#[test]
fn private_look_is_granted_only_to_the_effect_controller() {
    let engine = setup(7220, &[SORCERY], 20, true);
    assert!(crate::casting::library_top_visible_to(
        engine.state(),
        me(),
        me()
    ));
    assert!(!crate::casting::library_top_visible_to(
        engine.state(),
        them(),
        me()
    ));
    assert!(!crate::casting::library_top_visible_to(
        engine.state(),
        me(),
        them()
    ));
}

#[test]
fn the_temporary_permission_survives_its_source_but_ends_in_cleanup() {
    let mut engine = setup(7221, &[SORCERY], 20, true);
    let source = the(&engine, ZoneLocation::Battlefield, GRANTOR);
    engine
        .dev_state_mut(me())
        .unwrap()
        .move_object(
            source,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();
    let card = the(&engine, ZoneLocation::Library(me()), SORCERY);
    cast_top(&mut engine, card);
    assert_eq!(life(&engine, me()), 17, "the whole cast remains available");
    settle(&mut engine, me());
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    assert!(!offer(&engine).1.castable.contains(&card));
    assert!(!crate::casting::library_top_visible_to(
        engine.state(),
        me(),
        me()
    ));
}

#[track_caller]
fn choose_announced_route(engine: &mut Bench, predicate: impl Fn(&CastModeKind) -> bool) {
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected announced library routes: {:?}", engine.pending())
    };
    let route = options
        .iter()
        .position(|option| predicate(&option.kind))
        .expect("the requested library route is offered");
    engine.apply(me(), PlayerAction::ChooseMode(route)).unwrap();
}

#[test]
fn taxes_and_reductions_change_only_the_remaining_mana_cost() {
    let mut engine = setup_with(7300, &[SORCERY], 20, true, &[TAXER, DISCOUNT]);
    let card = the(&engine, ZoneLocation::Library(me()), SORCERY);
    float_all(&mut engine, me());
    let options = engine.cast_options(me(), card, false).unwrap();
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].cost, ManaCost::parse("{1}"));
    assert!(matches!(
        options[0].kind,
        CastModeKind::LibraryTopLife { life: 3, .. }
    ));
    cast_top(&mut engine, card);
    assert_eq!(life(&engine, me()), 17);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

#[test]
fn a_mandatory_sacrifice_is_chosen_and_paid_alongside_the_life() {
    let mut engine = setup_with(7301, &[SACRIFICE_SPELL], 20, true, &[FODDER]);
    let card = the(&engine, ZoneLocation::Library(me()), SACRIFICE_SPELL);
    let fodder = the(&engine, ZoneLocation::Battlefield, FODDER);
    engine
        .apply(me(), PlayerAction::CastSpell { card })
        .unwrap();
    choose_library_route(&mut engine);
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected the sacrifice choice: {:?}", engine.pending())
    };
    assert_eq!((min, max), (1, 1));
    assert_eq!(options, vec![fodder]);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Stack);
    assert_eq!(engine.state().object(fodder).unwrap().zone, Zone::Graveyard);
    assert_eq!(life(&engine, me()), 19);
}

#[test]
fn pay_life_x_reserves_mana_value_and_fixed_life_before_setting_x() {
    let mut engine = setup(7302, &[PAY_X_AND_FIXED], 10, true);
    let card = the(&engine, ZoneLocation::Library(me()), PAY_X_AND_FIXED);
    engine
        .apply(me(), PlayerAction::CastSpell { card })
        .unwrap();
    choose_library_route(&mut engine);
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("expected the life-X choice: {:?}", engine.pending())
    };
    assert_eq!((min, max), (0, 7));
    engine.apply(me(), PlayerAction::ChooseNumber(3)).unwrap();
    let spell = engine.state().object(card).unwrap();
    assert_eq!(spell.zone, Zone::Stack);
    assert_eq!(spell.x_value, 3);
    assert_eq!(life(&engine, me()), 4, "one mana value, two fixed, three X");
}

#[test]
fn a_kicked_route_keeps_its_kicker_and_charges_only_its_additional_mana() {
    let mut engine = setup_with(7303, &[KICKED], 20, true, &[forest()]);
    let card = the(&engine, ZoneLocation::Library(me()), KICKED);
    float_all(&mut engine, me());
    engine
        .apply(me(), PlayerAction::CastSpell { card })
        .unwrap();
    choose_announced_route(&mut engine, |kind| {
        matches!(
            kind,
            CastModeKind::LibraryTopLife {
                life: 1,
                kicked: true,
                ..
            }
        )
    });
    let spell = engine.state().object(card).unwrap();
    assert_eq!(spell.zone, Zone::Stack, "{:?}", engine.pending());
    assert_eq!(spell.times_kicked, 1);
    assert_eq!(life(&engine, me()), 19);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

#[test]
fn library_life_kicker_requires_all_life_costs_together() {
    let engine = setup(7310, &[LIFE_KICKER], 4, true);
    let card = the(&engine, ZoneLocation::Library(me()), LIFE_KICKER);
    let options = engine.cast_options(me(), card, false).unwrap();
    assert!(options.iter().any(|option| matches!(
        option.kind,
        CastModeKind::LibraryTopLife { kicked: false, .. }
    )));
    assert!(
        !options.iter().any(|option| matches!(
            option.kind,
            CastModeKind::LibraryTopLife { kicked: true, .. }
        )),
        "one life for mana value, two mandatory and two for kicker exceed four life"
    );
}

#[test]
fn library_life_kicker_pays_the_exact_combined_cost() {
    use crate::event::{Cause, GameEvent, LossReason};

    let mut engine = setup(7311, &[LIFE_KICKER], 5, true);
    let card = the(&engine, ZoneLocation::Library(me()), LIFE_KICKER);
    engine
        .apply(me(), PlayerAction::CastSpell { card })
        .unwrap();
    choose_announced_route(&mut engine, |kind| {
        matches!(kind, CastModeKind::LibraryTopLife { kicked: true, .. })
    });

    assert_eq!(life(&engine, me()), 0);
    assert!(matches!(engine.pending(), Pending::GameOver(_)));
    let journal = engine.state().journal.entries();
    assert!(journal.iter().any(|entry| matches!(
        entry.event,
        GameEvent::SpellCast { object, player, .. } if object == card && player == me()
    )));
    assert!(journal.iter().any(|entry| matches!(
        entry.event,
        GameEvent::LifeChanged { player, new: 0, cause: Cause::Cost, .. } if player == me()
    )));
    assert_eq!(engine.state().players[0].loss, Some(LossReason::Life));
}

#[test]
fn library_back_face_life_kicker_must_be_affordable() {
    let engine = setup(7312, &[BACK_LIFE_KICKER], 3, true);
    let card = the(&engine, ZoneLocation::Library(me()), BACK_LIFE_KICKER);
    let options = engine.cast_options(me(), card, false).unwrap();
    assert!(options.iter().any(|option| matches!(
        option.kind,
        CastModeKind::LibraryTopLife {
            face: 1,
            kicked: false,
            ..
        }
    )));
    assert!(
        !options.iter().any(|option| matches!(
            option.kind,
            CastModeKind::LibraryTopLife {
                face: 1,
                kicked: true,
                ..
            }
        )),
        "the back face's four-life kicker cannot be paid with three life"
    );
}

#[test]
fn a_modal_route_pays_the_selected_modes_additional_mana() {
    let mut engine = setup(7304, &[MODAL], 20, true);
    let card = the(&engine, ZoneLocation::Library(me()), MODAL);
    float_all(&mut engine, me());
    engine
        .apply(me(), PlayerAction::CastSpell { card })
        .unwrap();
    choose_announced_route(&mut engine, |kind| {
        matches!(
            kind,
            CastModeKind::LibraryTopLife {
                life: 1,
                modes: 2,
                ..
            }
        )
    });
    let spell = engine.state().object(card).unwrap();
    assert_eq!(spell.zone, Zone::Stack, "{:?}", engine.pending());
    assert_eq!(spell.mode_index, Some(1));
    assert_eq!(spell.modes, 0, "one chosen mode uses mode_index");
    assert_eq!(life(&engine, me()), 19);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

#[test]
fn an_mdfc_back_face_uses_its_own_mana_value_and_remains_on_that_face() {
    let mut engine = setup(7305, &[MDFC], 20, true);
    let card = the(&engine, ZoneLocation::Library(me()), MDFC);
    let options = engine.cast_options(me(), card, false).unwrap();
    assert!(matches!(
        options.as_slice(),
        [option] if matches!(
            option.kind,
            CastModeKind::LibraryTopLife {
                life: 3,
                face: 1,
                ..
            }
        )
    ));
    engine
        .apply(me(), PlayerAction::CastSpell { card })
        .unwrap();
    choose_library_route(&mut engine);
    let spell = engine.state().object(card).unwrap();
    assert_eq!(spell.zone, Zone::Stack, "{:?}", engine.pending());
    assert_eq!(spell.face_index, 1);
    assert_eq!(spell.characteristics().mana_value(), 3);
    assert_eq!(life(&engine, me()), 17);
}
