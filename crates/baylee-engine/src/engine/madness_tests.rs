//! Madness (CR 702.35): a discard into exile, a real triggered ability,
//! then an ordinary cast using the alternative madness cost.

use super::*;
use crate::engine::synthetic;
use crate::state::CardLookup;
use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, Cost, CostPart, Coverage, Effect, FaceDef, Filter, KeywordSet,
    ModeCount, SpellMode, TargetReq, TargetSpec,
};
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::mana::{ManaCost, ManaPayment};
use baylee_core::preset::DeckEntry;
use baylee_core::types::TypeSet;

const PAID: CardIndex = CardIndex::new(63_010);
const X_SPELL: CardIndex = CardIndex::new(63_011);
const LIFE: CardIndex = CardIndex::new(63_012);
const TARGETED: CardIndex = CardIndex::new(63_013);
const MODAL: CardIndex = CardIndex::new(63_014);
const STIFLE: CardIndex = CardIndex::new(63_015);
const ALL_ALTERNATIVES: CardIndex = CardIndex::new(63_016);
const SELF_DISCARD: CardIndex = CardIndex::new(63_017);
const SPLIT_SECOND_DISCARD: CardIndex = CardIndex::new(63_018);
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

static CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);
static GAIN_X: &[Effect] = &[Effect::GainLife { amount: Amount::X }];
static SELF_DISCARD_ABILITIES: &[AbilityDef] = &[AbilityDef::Activated {
    cost: Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::DiscardSelf],
    },
    effects: &[Effect::GainLife {
        amount: Amount::Fixed(1),
    }],
    targets: None,
    second_targets: None,
    timing: baylee_cards_dsl::ActivationTiming::InstantSpeed,
    mana_ability: false,
    zone: baylee_cards_dsl::ActivationZone::Hand,
    limit: baylee_cards_dsl::ActivationLimit::Unlimited,
    x_value: baylee_cards_dsl::ActivationX::Any,
    cost_reduction: None,
}];
static X_SPELL_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
    effects: GAIN_X,
    targets: None,
    second_targets: None,
    condition: None,
}];
static LIFE_COST: &[CostPart] = &[CostPart::PayLife(8)];
static TARGETED_SPELL: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[Effect::GainLife {
        amount: Amount::Fixed(1),
    }],
    targets: Some(TargetReq::one(TargetSpec::Object(&CREATURE))),
    second_targets: None,
    condition: None,
}];
static MODAL_SPELL: &[AbilityDef] = &[AbilityDef::ModalSpell {
    modes: &[
        SpellMode {
            effects: &[Effect::GainLife {
                amount: Amount::Fixed(3),
            }],
            targets: None,
            second_targets: None,
            cost_override: None,
            additional_cost: None,
        },
        SpellMode {
            effects: &[Effect::GainLife {
                amount: Amount::Fixed(5),
            }],
            targets: None,
            second_targets: None,
            cost_override: Some(ManaCost::parse("{3}{G}{G}")),
            additional_cost: None,
        },
    ],
    choose: ModeCount::ONE,
}];
static ALL_ALTERNATIVE_SPELL: &[AbilityDef] = &[AbilityDef::ModalSpell {
    modes: &[
        SpellMode {
            effects: &[Effect::GainLife {
                amount: Amount::Fixed(3),
            }],
            targets: None,
            second_targets: None,
            cost_override: Some(ManaCost::parse("{2}{G}")),
            additional_cost: None,
        },
        SpellMode {
            effects: &[Effect::GainLife {
                amount: Amount::Fixed(5),
            }],
            targets: None,
            second_targets: None,
            cost_override: Some(ManaCost::parse("{3}{G}{G}")),
            additional_cost: None,
        },
    ],
    choose: ModeCount::ONE,
}];
static COUNTER_ABILITY: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[Effect::CounterTargetAbility],
    targets: Some(TargetReq::one(TargetSpec::AbilityOnStack(&Filter::Any))),
    second_targets: None,
    condition: None,
}];

fn fixture(
    index: CardIndex,
    name: &'static str,
    types: TypeSet,
    printed: &'static str,
    madness: Cost,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    let face = FaceDef {
        name,
        mana_cost: ManaCost::parse(printed),
        types,
        madness: Some(madness),
        abilities,
        castable_from_hand: true,
        power: types.contains(TypeSet::CREATURE).then_some(2),
        toughness: types.contains(TypeSet::CREATURE).then_some(2),
        ..synthetic::land_face(name)
    };
    Box::leak(Box::new(CardDef {
        index,
        oracle_id: "test-madness",
        scryfall_id: "test-madness",
        faces: Box::leak(Box::new([face])),
        coverage: Coverage::Implemented,
        abilities,
        ..CardDef::DEFAULT
    }))
}

/// `{U}`, split second, and a card discarded as an additional cost.
fn split_second_discard() -> &'static CardDef {
    let name = "Split second discard fixture";
    let face = FaceDef {
        name,
        mana_cost: ManaCost::parse("{U}"),
        types: TypeSet::INSTANT,
        keywords: KeywordSet::SPLIT_SECOND,
        mandatory_additional_costs: &[CostPart::Discard(&Filter::Any)],
        abilities: X_SPELL_ABILITIES,
        castable_from_hand: true,
        ..synthetic::land_face(name)
    };
    Box::leak(Box::new(CardDef {
        index: SPLIT_SECOND_DISCARD,
        oracle_id: "test-madness",
        scryfall_id: "test-madness",
        faces: Box::leak(Box::new([face])),
        coverage: Coverage::Implemented,
        abilities: X_SPELL_ABILITIES,
        ..CardDef::DEFAULT
    }))
}

fn fixtures() -> Vec<&'static CardDef> {
    vec![
        split_second_discard(),
        fixture(
            PAID,
            "Paid madness fixture",
            TypeSet::CREATURE,
            "{5}{G}",
            Cost {
                mana: ManaCost::parse("{1}{G}"),
                parts: &[],
            },
            &[],
        ),
        fixture(
            X_SPELL,
            "X madness fixture",
            TypeSet::INSTANT,
            "{5}{G}",
            Cost {
                mana: ManaCost::parse("{X}{G}"),
                parts: &[],
            },
            X_SPELL_ABILITIES,
        ),
        fixture(
            LIFE,
            "Life-cost madness fixture",
            TypeSet::CREATURE,
            "{6}",
            Cost {
                mana: ManaCost::parse("{1}"),
                parts: LIFE_COST,
            },
            &[],
        ),
        fixture(
            TARGETED,
            "Targeted madness fixture",
            TypeSet::INSTANT,
            "{1}",
            Cost::FREE,
            TARGETED_SPELL,
        ),
        fixture(
            MODAL,
            "Modal madness fixture",
            TypeSet::INSTANT,
            "{4}{G}",
            Cost {
                mana: ManaCost::parse("{G}"),
                parts: &[],
            },
            MODAL_SPELL,
        ),
        fixture(
            ALL_ALTERNATIVES,
            "Only alternative modal costs fixture",
            TypeSet::INSTANT,
            "{4}{G}",
            Cost {
                mana: ManaCost::parse("{G}"),
                parts: &[],
            },
            ALL_ALTERNATIVE_SPELL,
        ),
        fixture(
            SELF_DISCARD,
            "Self-discard madness fixture",
            TypeSet::CREATURE,
            "{5}{G}",
            Cost::FREE,
            SELF_DISCARD_ABILITIES,
        ),
        fixture(
            STIFLE,
            "Ability counter fixture",
            TypeSet::INSTANT,
            "{U}",
            Cost::FREE,
            COUNTER_ABILITY,
        ),
    ]
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card,
        print: PrintRef::new(0),
    }
}

fn real(oracle_id: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle_id)
        .expect("test card exists")
        .index
}

fn basking_rootwalla() -> CardIndex {
    real("b2ec080f-ad5d-4274-91e3-bc6ac89fc3e4")
}

fn counterspell() -> CardIndex {
    real("cc187110-1148-4090-bbb8-e205694a39f5")
}

fn game(card: CardIndex, life: i32) -> Engine<synthetic::SyntheticLookup> {
    game_with_seats(card, life, 2)
}

fn game_with_seats(card: CardIndex, life: i32, seats: usize) -> Engine<synthetic::SyntheticLookup> {
    game_holding(
        &[card, real("06b60766-a1fb-4ed0-93f6-8423008f75b4")],
        life,
        seats,
    )
}

fn game_holding(hand: &[CardIndex], life: i32, seats: usize) -> Engine<synthetic::SyntheticLookup> {
    assert!(seats >= 2);
    let forest = CardIndex::new(synthetic::forest());
    let island = real("b2c6aa39-2d2a-459c-a555-fb48ba993373");
    let mut battlefield = vec![island.get(); 6];
    battlefield.extend([forest.get(); 3]);
    let mut preset = synthetic::preset(731, &battlefield);
    preset.seats[0].starting_life = Some(life);
    preset.seats[0].starting_hand = Some(hand.iter().copied().map(entry).collect());
    preset.seats[1].starting_hand = Some(vec![entry(STIFLE), entry(counterspell())]);
    preset.seats[1].starting_battlefield = vec![entry(island), entry(island)];
    let opponent = preset.seats[1].clone();
    preset.seats.resize(seats, opponent);
    let mut engine = Engine::new(&preset, synthetic::SyntheticLookup::new(fixtures())).unwrap();
    for _ in 0..seats {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("expected every seat's mulligan")
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    for _ in 0..20 {
        if matches!(engine.state().turn.phase, Phase::FirstMain) {
            return engine;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    panic!("never reached the first main phase");
}

fn hand_card<L: CardLookup>(engine: &Engine<L>, player: PlayerId, card: CardIndex) -> ObjectId {
    engine
        .state()
        .zones
        .list(ZoneLocation::Hand(player))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|object| object.card)
                .is_some_and(|printed| printed.index == card)
        })
        .expect("the fixture card is in hand")
}

fn tap_sources<L: CardLookup>(
    engine: &mut Engine<L>,
    player: PlayerId,
    count: usize,
    card: Option<CardIndex>,
) {
    for _ in 0..count {
        let Pending::Priority {
            player: priority,
            legal,
            ..
        } = engine.pending().clone()
        else {
            panic!("expected a mana opportunity, got {:?}", engine.pending());
        };
        assert_eq!(priority, player);
        let source = legal
            .mana_abilities
            .iter()
            .copied()
            .find(|source| {
                card.is_none_or(|card| {
                    engine
                        .state()
                        .object(*source)
                        .and_then(|object| object.card)
                        .is_some_and(|printed| printed.index == card)
                })
            })
            .expect("an untapped source of the requested kind remains");
        engine
            .apply(player, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
}

fn tap_lands<L: CardLookup>(engine: &mut Engine<L>, player: PlayerId, count: usize) {
    tap_sources(engine, player, count, None);
}

/// Casts Prying Eyes, discards the fixture and one other card, and stops
/// with the madness trigger on the stack before either player passes.
fn discard(engine: &mut Engine<synthetic::SyntheticLookup>, card: CardIndex) -> ObjectId {
    let prying_eyes = real("06b60766-a1fb-4ed0-93f6-8423008f75b4");
    let island = real("b2c6aa39-2d2a-459c-a555-fb48ba993373");
    let discarded = hand_card(engine, P0, card);
    let eyes = hand_card(engine, P0, prying_eyes);
    tap_sources(engine, P0, 6, Some(island));
    engine
        .apply(P0, PlayerAction::CastSpell { card: eyes })
        .unwrap();
    for _ in 0..100 {
        if matches!(
            engine.pending(),
            Pending::ChooseCards {
                player,
                min: 2,
                max: 2,
                ..
            } if *player == P0
        ) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(
            synthetic::walk_past(engine, &pending),
            "unexpected {pending:?}"
        );
    }
    let other = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(P0))
        .iter()
        .copied()
        .find(|id| *id != discarded)
        .expect("Prying Eyes supplied the second discard");
    let choice = engine.pending().clone();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![discarded, other],
            },
        )
        .unwrap_or_else(|error| {
            panic!(
                "discard selection failed in {choice:?}, then {:?}: {error:?}",
                engine.pending()
            )
        });
    assert_eq!(engine.state().object(discarded).unwrap().zone, Zone::Exile);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .any(|id| {
                engine.state().object(*id).is_some_and(|object| {
                    object.kind == ObjectKind::AbilityOnStack
                        && object
                            .ability
                            .is_some_and(|ability| ability.source == discarded)
                })
            })
    );
    discarded
}

/// Passes the trigger through. `false` means the rules correctly declined
/// the cast without asking, because no legal cast could be begun.
fn offered<L: CardLookup>(engine: &mut Engine<L>, card: ObjectId) -> bool {
    for _ in 0..100 {
        if matches!(
            engine.pending(),
            Pending::YesNo {
                player: P0,
                prompt: crate::choice::YesNoPrompt::Madness { card: asked },
                ..
            } if *asked == card
        ) {
            return true;
        }
        if engine.state().object(card).unwrap().zone == Zone::Graveyard {
            return false;
        }
        let pending = engine.pending().clone();
        assert!(
            synthetic::walk_past(engine, &pending),
            "unexpected {pending:?}"
        );
    }
    panic!("madness trigger did not finish resolving");
}

fn resolve_stack<L: CardLookup>(engine: &mut Engine<L>) {
    for _ in 0..100 {
        if engine.state().zones.stack_is_empty()
            && matches!(engine.pending(), Pending::Priority { .. })
        {
            return;
        }
        let pending = engine.pending().clone();
        assert!(
            synthetic::walk_past(engine, &pending),
            "unexpected {pending:?}"
        );
    }
    panic!("the stack did not empty");
}

fn pay_and_resolve<L: CardLookup>(engine: &mut Engine<L>, cost: &'static str, lands: usize) {
    assert_eq!(
        engine.payment_window(),
        Some((P0, ManaPayment::Fixed(ManaCost::parse(cost))))
    );
    let card = match &engine.mana_window {
        Some(PaymentWindow {
            suspended: PaymentContinuation::Miracle { wizard, .. },
            ..
        }) => wizard.card,
        other => panic!("expected a madness payment window, got {other:?}"),
    };
    tap_lands(engine, P0, lands);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    let events: Vec<_> = engine
        .state()
        .journal
        .entries()
        .iter()
        .filter(|entry| {
            matches!(&entry.event, GameEvent::SpellCast { object, .. } if *object == card)
                || matches!(&entry.event, GameEvent::CastCancelled { card: cancelled, .. } if *cancelled == card)
        })
        .collect();
    assert_eq!(
        engine.state().object(card).unwrap().zone,
        Zone::Stack,
        "the paid madness cast reached the stack: {events:#?}"
    );
    resolve_stack(engine);
}

fn event_count<L: CardLookup>(engine: &Engine<L>, predicate: impl Fn(&GameEvent) -> bool) -> usize {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter(|entry| predicate(&entry.event))
        .count()
}

#[test]
fn declining_madness_moves_only_the_exiled_incarnation_to_the_graveyard() {
    let mut engine = game(PAID, 20);
    let card = discard(&mut engine, PAID);
    assert!(offered(&mut engine, card));

    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();

    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        event_count(&engine, |event| {
            matches!(event, GameEvent::MadnessExiled { object, .. } if *object == card)
        }),
        1
    );
    assert_eq!(
        event_count(&engine, |event| {
            matches!(event, GameEvent::Discarded { object, .. } if *object == card)
        }),
        1
    );
    assert_eq!(
        event_count(&engine, |event| {
            matches!(event, GameEvent::CastCancelled { card: cancelled, .. } if *cancelled == card)
        }),
        0,
        "declining the optional cast never begins a cast to cancel"
    );
    assert_eq!(
        event_count(&engine, |event| {
            matches!(event, GameEvent::SpellCast { object, .. } if *object == card)
        }),
        0
    );
}

#[test]
fn a_short_madness_payment_reverses_mana_and_falls_back_to_the_graveyard() {
    for make_mana in [false, true] {
        let mut engine = game(PAID, 20);
        let card = discard(&mut engine, PAID);
        assert!(offered(&mut engine, card));
        engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("madness payment window")
        };
        let land = legal.mana_abilities[0];
        if make_mana {
            engine
                .apply(P0, PlayerAction::ActivateManaAbility { source: land })
                .unwrap();
            assert_eq!(engine.state().players[0].mana_pool.total(), 1);
        }

        engine.apply(P0, PlayerAction::PassPriority).unwrap();

        assert_eq!(engine.payment_window(), None);
        assert_eq!(engine.state().object(card).unwrap().zone, Zone::Graveyard);
        assert_eq!(engine.state().players[0].mana_pool.total(), 0);
        assert!(!synthetic::tapped(&engine, land));
        assert_eq!(
            event_count(&engine, |event| {
                matches!(event, GameEvent::CastCancelled {
                    card: cancelled,
                    reason: crate::event::CastCancelReason::Unpaid,
                    ..
                } if *cancelled == card)
            }),
            1,
            "the begun cast is recorded as reversed once (CR 734.1)"
        );
        assert_eq!(
            event_count(&engine, |event| {
                matches!(event, GameEvent::SpellCast { object, .. } if *object == card)
            }),
            0
        );
        assert_eq!(
            event_count(&engine, |event| {
                matches!(event, GameEvent::MadnessExiled { object, .. } if *object == card)
            }),
            1
        );
    }
}

#[test]
fn a_life_payment_equal_to_the_players_life_is_legal() {
    let mut engine = game(LIFE, 8);
    let card = discard(&mut engine, LIFE);
    assert!(
        offered(&mut engine, card),
        "CR 118.3 requires more life than the payment, not at least as much"
    );

    // Decline after proving the offer: paying here would put the player at
    // zero and end this duel before the payment assertion could run.
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(engine.state().players[0].life, 8);
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Graveyard);
}

#[test]
fn madness_pays_its_alternative_cost_with_mana_made_after_accepting() {
    let mut engine = game(PAID, 20);
    let card = discard(&mut engine, PAID);
    assert!(offered(&mut engine, card));
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    let before = engine.snapshot_hash();
    assert_eq!(
        before,
        engine.snapshot_hash(),
        "unchanged answers are stable"
    );
    {
        let Some(PaymentWindow {
            suspended: PaymentContinuation::Miracle { wizard, .. },
            ..
        }) = &mut engine.mana_window
        else {
            panic!("madness uses the ordinary effect-cast payment window")
        };
        assert_eq!(
            wizard.alternative_cost,
            Some(crate::choice::CastModeKind::Madness)
        );
        wizard.chosen_player = Some(P1);
    }
    let changed = engine.snapshot_hash();
    assert_ne!(before, changed, "held cast answers are replay state");
    assert_eq!(
        changed,
        engine.snapshot_hash(),
        "the same new answer is stable"
    );
    let Some(PaymentWindow {
        suspended: PaymentContinuation::Miracle { wizard, .. },
        ..
    }) = &mut engine.mana_window
    else {
        panic!("madness payment window remains open")
    };
    wizard.chosen_player = None;
    assert_eq!(
        before,
        engine.snapshot_hash(),
        "restoring the answer restores the hash"
    );
    pay_and_resolve(&mut engine, "{1}{G}", 2);
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Battlefield);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

#[test]
fn madness_x_is_announced_against_the_madness_cost() {
    let mut engine = game(X_SPELL, 20);
    let card = discard(&mut engine, X_SPELL);
    assert!(offered(&mut engine, card));
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("madness {{X}} must announce X, got {:?}", engine.pending());
    };
    assert_eq!(min, 0);
    assert!(max >= 2);
    let context = engine.decision_context();
    assert_eq!(context.cast_mode, Some(crate::choice::CastModeKind::Normal));
    assert_eq!(
        context.alternative_cost,
        Some(crate::choice::CastModeKind::Madness)
    );
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    pay_and_resolve(&mut engine, "{2}{G}", 3);
    assert_eq!(
        engine.state().players[0].life,
        22,
        "the spell retained X = 2"
    );
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Graveyard);
}

#[test]
fn madness_pays_nonmana_parts_and_an_impossible_life_payment_is_not_offered() {
    let mut engine = game(LIFE, 20);
    let card = discard(&mut engine, LIFE);
    assert!(offered(&mut engine, card));
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pay_and_resolve(&mut engine, "{1}", 1);
    assert_eq!(engine.state().players[0].life, 12);
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Battlefield);

    let mut engine = game(LIFE, 7);
    let card = discard(&mut engine, LIFE);
    assert!(
        !offered(&mut engine, card),
        "a player cannot pay more life than they have"
    );
    assert_eq!(engine.state().players[0].life, 7);
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Graveyard);
}

#[test]
fn no_required_target_falls_back_without_a_cast() {
    let mut engine = game(TARGETED, 20);
    let card = discard(&mut engine, TARGETED);
    assert!(!offered(&mut engine, card));
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Graveyard);
}

#[test]
fn an_ordinary_mode_keeps_its_separate_madness_cost() {
    let mut engine = game(MODAL, 20);
    let card = discard(&mut engine, MODAL);
    assert!(offered(&mut engine, card));
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    let Some(PaymentWindow {
        suspended: PaymentContinuation::Miracle { wizard, .. },
        ..
    }) = &engine.mana_window
    else {
        panic!("the modal madness cast is at its payment window")
    };
    assert_eq!(wizard.option, Some(crate::choice::CastModeKind::Mode(0)));
    assert_eq!(
        wizard.alternative_cost,
        Some(crate::choice::CastModeKind::Madness)
    );
    pay_and_resolve(&mut engine, "{G}", 1);
    assert_eq!(
        engine.state().players[0].life,
        23,
        "only the ordinary mode was available"
    );
}

#[test]
fn modes_with_only_their_own_alternative_costs_do_not_offer_madness() {
    let mut engine = game(ALL_ALTERNATIVES, 20);
    let card = discard(&mut engine, ALL_ALTERNATIVES);
    assert!(
        !offered(&mut engine, card),
        "CR 118.9a permits only one alternative cost; no mode combines with madness"
    );
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Graveyard);
    assert!(!matches!(engine.pending(), Pending::YesNo { .. }));
}

/// Casting the counter spell through the normal wizard exercises targeting,
/// payment and the ordinary stack rules instead of removing an object directly.
fn counter_stack_object<L: CardLookup>(engine: &mut Engine<L>, spell: CardIndex, target: ObjectId) {
    for _ in 0..20 {
        if matches!(engine.pending(), Pending::Priority { player: P1, .. }) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(
            synthetic::walk_past(engine, &pending),
            "unexpected {pending:?}"
        );
    }
    let counter = hand_card(engine, P1, spell);
    engine
        .apply(P1, PlayerAction::CastSpell { card: counter })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("counter spell needs its target, got {:?}", engine.pending());
    };
    assert!(options.contains(&target));
    engine
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    tap_lands(engine, P1, if spell == STIFLE { 1 } else { 2 });
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    resolve_stack(engine);
}

#[test]
fn countering_the_trigger_leaves_exile_but_countering_the_spell_uses_the_graveyard() {
    let mut engine = game(basking_rootwalla(), 20);
    let card = discard(&mut engine, basking_rootwalla());
    let trigger = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.kind == ObjectKind::AbilityOnStack)
        })
        .expect("madness is a triggered ability on the stack");
    counter_stack_object(&mut engine, STIFLE, trigger);
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Exile);

    let mut engine = game(basking_rootwalla(), 20);
    let card = discard(&mut engine, basking_rootwalla());
    assert!(offered(&mut engine, card));
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Stack);
    counter_stack_object(&mut engine, counterspell(), card);
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Graveyard);
}

#[test]
fn a_moved_incarnation_is_not_moved_again_or_recorded_as_a_second_discard() {
    let mut engine = game(basking_rootwalla(), 20);
    let card = discard(&mut engine, basking_rootwalla());
    let before = engine.state().object(card).unwrap().version;
    engine
        .dev_state_mut(P0)
        .unwrap()
        .move_object(
            card,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::DevCommand,
        )
        .unwrap();
    assert_ne!(engine.state().object(card).unwrap().version, before);
    resolve_stack(&mut engine);
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Graveyard);
    let discards = engine
        .state()
        .journal
        .entries()
        .iter()
        .filter(
            |entry| matches!(entry.event, GameEvent::Discarded { object, .. } if object == card),
        )
        .count();
    assert_eq!(discards, 1);
    assert!(
        !engine
            .state()
            .per_turn
            .discarded_to_graveyard
            .iter()
            .any(|(reference, _)| reference.object == card),
        "the normal discard of the other card is unrelated"
    );
}

#[test]
fn a_copied_trigger_cannot_cast_the_same_card_twice() {
    static COPY: &[Effect] = &[Effect::CopyTargetAbility];
    let mut engine = game(basking_rootwalla(), 20);
    let from = engine.state().journal.last_seq();
    let card = discard(&mut engine, basking_rootwalla());
    let mut copying = crate::trigger::collect(&engine.state, &engine.lookup, from)
        .into_iter()
        .find(|trigger| matches!(trigger.synthetic_effects, Some([Effect::Madness])))
        .expect("the discard produced madness's trigger");
    let original = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .copied()
        .find(|id| {
            engine.state().object(*id).is_some_and(|object| {
                object.kind == ObjectKind::AbilityOnStack
                    && object.ability.is_some_and(|ability| ability.source == card)
            })
        })
        .expect("the original madness trigger is on the stack");
    copying.synthetic_effects = Some(COPY);
    engine.push_synthetic_trigger_with_targets(&copying, smallvec::smallvec![original]);

    // CR 707.10: the copy makes its own cast decision. Once that decision
    // moves the card to the stack, the original can no longer find its exile
    // incarnation and must neither cast nor move it to the graveyard.
    assert!(offered(&mut engine, card));
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Stack);
    resolve_stack(&mut engine);

    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        event_count(&engine, |event| {
            matches!(event, GameEvent::SpellCast { object, .. } if *object == card)
        }),
        1
    );
    assert_eq!(
        event_count(&engine, |event| {
            matches!(event, GameEvent::CastCancelled { card: cancelled, .. } if *cancelled == card)
        }),
        0
    );
}

#[test]
fn a_bystander_leaving_preserves_the_madness_question() {
    let bystander = PlayerId::new(2);
    let mut engine = game_with_seats(PAID, 20, 3);
    let card = discard(&mut engine, PAID);
    assert!(offered(&mut engine, card));
    let question = format!("{:?}", engine.pending());

    engine.apply(bystander, PlayerAction::Concede).unwrap();

    assert_eq!(format!("{:?}", engine.pending()), question);
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Exile);
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Graveyard);
}

#[test]
fn a_player_leaving_the_madness_question_takes_the_card_with_them() {
    let mut engine = game_with_seats(PAID, 20, 3);
    let card = discard(&mut engine, PAID);
    assert!(offered(&mut engine, card));

    engine.apply(P0, PlayerAction::Concede).unwrap();

    assert!(engine.state().has_left(P0));
    assert!(engine.state().object(card).is_none());
    assert!(engine.mana_window.is_none());
    assert!(!matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::Madness { .. },
            ..
        }
    ));
    assert_eq!(
        event_count(&engine, |event| {
            matches!(event, GameEvent::SpellCast { object, .. } if *object == card)
        }),
        0
    );
}

#[test]
fn leaving_a_madness_payment_preserves_others_but_pays_nothing() {
    let bystander = PlayerId::new(2);
    let mut engine = game_with_seats(PAID, 20, 3);
    let card = discard(&mut engine, PAID);
    assert!(offered(&mut engine, card));
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    let question = format!("{:?}", engine.pending());

    engine.apply(bystander, PlayerAction::Concede).unwrap();
    assert_eq!(format!("{:?}", engine.pending()), question);
    assert!(engine.mana_window.is_some());
    pay_and_resolve(&mut engine, "{1}{G}", 2);
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Battlefield);

    let mut engine = game_with_seats(PAID, 20, 3);
    let card = discard(&mut engine, PAID);
    assert!(offered(&mut engine, card));
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_lands(&mut engine, P0, 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    engine.apply(P0, PlayerAction::Concede).unwrap();

    assert!(engine.state().has_left(P0));
    assert!(engine.state().object(card).is_none());
    assert!(engine.mana_window.is_none());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the mana already produced was not spent on the abandoned cast"
    );
    assert_eq!(
        event_count(&engine, |event| {
            matches!(event, GameEvent::SpellCast { object, .. } if *object == card)
        }),
        0,
        "CR 800.4f: the departed player pays no cost and casts nothing"
    );
}

#[test]
fn madness_replaces_discarding_the_card_to_pay_its_own_hand_ability() {
    let mut engine = game(SELF_DISCARD, 20);
    let card = hand_card(&engine, P0, SELF_DISCARD);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(legal.abilities.contains(&(card, 0)));

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: card,
                ability_index: 0,
            },
        )
        .unwrap();

    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Exile);
    assert!(offered(&mut engine, card));
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    resolve_stack(&mut engine);
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "the original hand ability still resolves beneath the madness cast"
    );
    assert_eq!(
        event_count(&engine, |event| {
            matches!(event, GameEvent::Discarded { object, .. } if *object == card)
        }),
        1
    );
}

#[test]
fn madness_also_replaces_a_discard_paid_as_an_activation_cost() {
    let putrid_imp = real("7192ff07-9a84-49ad-a40c-ce255ce21544");
    let mut engine = super::testkit::Duel::new(732, CardIndex::new(synthetic::forest()))
        .hand(0, &[basking_rootwalla()])
        .battlefield(0, &[putrid_imp])
        .start();
    super::testkit::keep_mulligans(&mut engine);
    super::testkit::reach_main_phase(&mut engine, P0);
    let imp = super::testkit::on_battlefield(&engine, P0, putrid_imp).expect("Putrid Imp");
    let card = hand_card(&engine, P0, basking_rootwalla());

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: imp,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseCards {
        player: P0,
        min: 1,
        max: 1,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the discard cost asks for one card, got {:?}",
            engine.pending()
        );
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![card],
            },
        )
        .unwrap();

    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Exile);
    assert!(offered(&mut engine, card));
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        event_count(&engine, |event| {
            matches!(event, GameEvent::Discarded { object, .. } if *object == card)
        }),
        1
    );
}

#[test]
fn madness_also_replaces_the_cleanup_discard_and_uses_that_cleanups_priority() {
    for cast in [false, true] {
        let mut engine = super::testkit::Duel::new(733, CardIndex::new(synthetic::forest()))
            .hand(0, &[basking_rootwalla()])
            .start();
        super::testkit::keep_mulligans(&mut engine);
        super::testkit::reach_main_phase(&mut engine, P0);
        let card = hand_card(&engine, P0, basking_rootwalla());
        engine.dev_state_mut(P0).unwrap().draw_cards(P0, 8);
        let ordinary = engine
            .state()
            .zones
            .list(ZoneLocation::Hand(P0))
            .iter()
            .copied()
            .find(|other| *other != card)
            .expect("another card for the cleanup discard");
        for _ in 0..100 {
            if matches!(
                engine.pending(),
                Pending::DiscardChoice {
                    player: P0,
                    count: 2,
                    ..
                }
            ) {
                break;
            }
            let pending = engine.pending().clone();
            assert!(
                synthetic::walk_past(&mut engine, &pending),
                "unexpected {pending:?}"
            );
        }
        assert!(
            matches!(
                engine.pending(),
                Pending::DiscardChoice {
                    player: P0,
                    count: 2,
                    ..
                }
            ),
            "nine cards require two cleanup discards, got {:?}",
            engine.pending()
        );

        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![card, ordinary],
                },
            )
            .unwrap();

        assert_eq!(engine.state().object(card).unwrap().zone, Zone::Exile);
        assert_eq!(
            engine.state().object(ordinary).unwrap().zone,
            Zone::Graveyard
        );
        assert_eq!(
            engine
                .state()
                .per_turn
                .discarded_to_graveyard
                .iter()
                .filter(|(reference, player)| reference.object == ordinary && *player == P0)
                .count(),
            1,
            "the ordinary cleanup discard is recorded once while madness holds the turn open"
        );
        assert!(offered(&mut engine, card));
        engine.apply(P0, PlayerAction::YesNo(cast)).unwrap();
        if cast {
            assert_eq!(engine.state().object(card).unwrap().zone, Zone::Stack);
            resolve_stack(&mut engine);
            assert_eq!(engine.state().object(card).unwrap().zone, Zone::Battlefield);
        } else {
            assert_eq!(engine.state().object(card).unwrap().zone, Zone::Graveyard);
        }
        assert_eq!(
            event_count(&engine, |event| {
                matches!(event, GameEvent::Discarded { object, .. } if *object == card)
            }),
            1
        );
    }
}

/// Split second (CR 702.61a) says players can't cast other spells, and a
/// madness cast is one: the trigger still goes on the stack above the spell
/// (CR 702.61b), and it can only send the card on to the graveyard (CR
/// 702.35a). A discard paid as the spell's own cost is the way a madness
/// trigger comes to stand above a split second spell at all. Asked of the
/// door every cast shares, so the gate is read here for both players, on
/// the stack and after it.
#[test]
fn a_madness_trigger_above_a_split_second_spell_cannot_cast_the_card() {
    let island = real("b2c6aa39-2d2a-459c-a555-fb48ba993373");
    let mut engine = game_holding(&[basking_rootwalla(), SPLIT_SECOND_DISCARD], 20, 2);
    let card = hand_card(&engine, P0, basking_rootwalla());
    let spell = hand_card(&engine, P0, SPLIT_SECOND_DISCARD);
    assert!(crate::casting::may_begin_casting(engine.state(), P0));
    tap_sources(&mut engine, P0, 1, Some(island));
    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    for _ in 0..100 {
        if matches!(
            engine.pending(),
            Pending::ChooseCards {
                player: P0,
                min: 1,
                max: 1,
                ..
            }
        ) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(
            synthetic::walk_past(&mut engine, &pending),
            "unexpected {pending:?}"
        );
    }
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![card],
            },
        )
        .unwrap();

    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(spell).unwrap().zone, Zone::Stack);
    assert!(
        !offered(&mut engine, card),
        "the cast could not be begun, so nobody is asked"
    );
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state().object(spell).unwrap().zone,
        Zone::Stack,
        "the trigger stood above the spell"
    );
    for player in [P0, P1] {
        assert!(
            !crate::casting::may_begin_casting(engine.state(), player),
            "nobody begins a cast over split second"
        );
    }
    assert_eq!(
        event_count(&engine, |event| {
            matches!(event, GameEvent::Discarded { object, .. } if *object == card)
        }),
        1
    );
    resolve_stack(&mut engine);
    assert!(crate::casting::may_begin_casting(engine.state(), P0));
}
