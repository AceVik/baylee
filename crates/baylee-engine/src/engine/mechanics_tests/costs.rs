//! The non-mana parts of a spell's cost: whether an alternative cost can be
//! paid at all (the offer's `alternative_parts_payable`), and what paying
//! one takes (the cost-part arms of `cast_wizard::finish_cast`).
//!
//! Two halves of one promise. A card offered on the strength of an
//! alternative cost has to be paid for by that cost when it is cast, and a
//! card whose alternative cost cannot be paid must not be offered: a button
//! the wizard then refuses is the same defect as a cost it walks past.
//!
//! The spells here print a mana cost nobody at the table can pay, so the
//! alternative is the only way to cast them and every question below is
//! about it (CR 118.9, 601.2b).

use super::*;
use baylee_cards_dsl::{AltCondition, AlternativeCost, CostPart, CounterKind, Filter};

const BARGAIN: u32 = 7180;
const PITCH: u32 = 7181;
const FODDER: u32 = 7182;
const HYMN: u32 = 7183;
const CHARGE: u32 = 7184;
const TALLY: u32 = 7185;
const DELUGE: u32 = 7186;
const TITHE: u32 = 7187;
const OFFERING: u32 = 7188;
const OX: u32 = 7189;
const RAY: u32 = 7190;
const DOUBLE_PITCH: u32 = 7191;

/// A spell ability that does nothing: what is under test is how the spell
/// was paid for, and an effect would be one more thing to read around.
static NOTHING: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[],
    targets: None,
    second_targets: None,
    condition: None,
}];

static AN_INSTANT: Filter = Filter::HasType(TypeSet::INSTANT);
static A_CREATURE: Filter = Filter::CREATURE;

/// "You may pay `parts` rather than pay this spell's mana cost."
const fn instead(parts: &'static [CostPart]) -> AlternativeCost {
    AlternativeCost {
        cost: Cost {
            mana: ManaCost::ZERO,
            parts,
        },
        condition: AltCondition::Always,
    }
}

static PITCH_TWO: &[AlternativeCost] = &[instead(&[
    CostPart::ExileFromHand(&AN_INSTANT),
    CostPart::ExileFromHand(&AN_INSTANT),
])];
static PAY_THREE: &[AlternativeCost] = &[instead(&[CostPart::PayLife(3)])];
/// Force of Will's shape: one life and an instant card from your hand.
static PAY_ONE_AND_PITCH: &[AlternativeCost] = &[instead(&[
    CostPart::PayLife(1),
    CostPart::ExileFromHand(&AN_INSTANT),
])];
static REMOVE_A_COUNTER: &[AlternativeCost] = &[instead(&[CostPart::RemoveCounterSelf {
    kind: CounterKind::P1P1,
    n: 1,
}])];
static REMOVE_X_COUNTERS: &[AlternativeCost] = &[instead(&[CostPart::RemoveCounterSelfX {
    kind: CounterKind::P1P1,
}])];

static SACRIFICE_A_CREATURE: &[CostPart] = &[CostPart::Sacrifice(&A_CREATURE)];

/// An instant priced out of reach, castable only through `alts`.
fn out_of_reach(
    index: u32,
    name: &'static str,
    alts: &'static [AlternativeCost],
) -> &'static CardDef {
    card(
        index,
        FaceDef {
            alternative_costs: alts,
            ..face(name, "{5}{G}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        NOTHING,
    )
}

/// A `{0}` sorcery with `parts` as its mandatory additional cost (CR 118.8).
fn with_additional(index: u32, name: &'static str, parts: &'static [CostPart]) -> &'static CardDef {
    card(
        index,
        FaceDef {
            mandatory_additional_costs: parts,
            ..face(name, "{0}", TypeSet::SORCERY)
        },
        KeywordSet::EMPTY,
        NOTHING,
    )
}

fn cards() -> Vec<&'static CardDef> {
    vec![
        out_of_reach(BARGAIN, "Bargain", PAY_THREE),
        out_of_reach(PITCH, "Pitch", PAY_ONE_AND_PITCH),
        out_of_reach(DOUBLE_PITCH, "Double Pitch", PITCH_TWO),
        card(
            FODDER,
            face("Fodder", "{5}{U}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            NOTHING,
        ),
        card(
            HYMN,
            face("Hymn", "{5}{U}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            NOTHING,
        ),
        out_of_reach(CHARGE, "Charge", REMOVE_A_COUNTER),
        out_of_reach(TALLY, "Tally", REMOVE_X_COUNTERS),
        with_additional(DELUGE, "Deluge", &[CostPart::PayLifeX]),
        with_additional(TITHE, "Tithe", &[CostPart::PayLife(2)]),
        with_additional(OFFERING, "Offering", SACRIFICE_A_CREATURE),
        card(
            OX,
            creature_face("Ox", "{2}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            RAY,
            face("Ray", "{X}{G}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            NOTHING,
        ),
    ]
}

/// Seat 0 at its main phase with `hand`, at `life`, and no mana anywhere.
fn holding(seed: u64, hand: &[u32], life: i32) -> Bench {
    let mut engine = bench(
        seed,
        cards(),
        [Seat::default().holding(hand).at(life), Seat::default()],
    );
    to_main(&mut engine, me());
    engine
}

/// Whether the one `index` card in seat 0's hand is offered.
fn offered(engine: &Bench, index: u32) -> bool {
    let card = the(engine, ZoneLocation::Hand(me()), index);
    offer(engine).1.castable.contains(&card)
}

// ------------------------------------------- can the alternative be paid?

/// Life is paid only out of a total at least that large (CR 119.4): three
/// life at three is payable and at two is not.
#[test]
fn a_life_alternative_is_offered_only_while_the_life_is_there() {
    assert!(offered(&holding(7180, &[BARGAIN], 3), BARGAIN));
    let engine = holding(7181, &[BARGAIN], 2);
    assert!(!offered(&engine, BARGAIN), "two life does not pay three");
    let card = the(&engine, ZoneLocation::Hand(me()), BARGAIN);
    let mut engine = engine;
    assert!(
        engine
            .apply(me(), PlayerAction::CastSpell { card })
            .is_err(),
        "and the cast behind the offer refuses it too"
    );
}

/// "Exile an instant card from your hand": another instant in hand pays it;
/// the spell itself does not (it is on its way to the stack, not in the
/// hand to be exiled), and neither does a card the filter does not name.
#[test]
fn a_pitch_alternative_is_offered_only_over_another_card_it_names() {
    assert!(
        offered(&holding(7182, &[PITCH, FODDER], 20), PITCH),
        "another instant to exile"
    );
    assert!(!offered(&holding(7183, &[PITCH], 20), PITCH), "only itself");
    assert!(
        !offered(&holding(7184, &[PITCH, HYMN], 20), PITCH),
        "a sorcery is not an instant card"
    );
}

/// A counter removed from the card being cast: a card in a hand carries no
/// counters, so it is never payable from there.
#[test]
fn a_counter_alternative_is_not_payable_from_a_hand() {
    assert!(!offered(&holding(7186, &[CHARGE], 20), CHARGE));
}

/// "Remove X counters": X may be announced as zero, so there is nothing an
/// alternative cost like that could fail to pay, and the card is offered.
#[test]
fn a_part_that_can_always_be_paid_leaves_the_alternative_open() {
    assert!(offered(&holding(7187, &[TALLY], 20), TALLY));
}

// ----------------------------------------------- paying the alternative

/// Casts the one `index` card in seat 0's hand.
fn cast(engine: &mut Bench, index: u32) -> ObjectId {
    let card = the(engine, ZoneLocation::Hand(me()), index);
    engine
        .apply(me(), PlayerAction::CastSpell { card })
        .expect("the offered card is cast");
    card
}

/// The spell on the stack, and what it was cast with.
#[track_caller]
fn on_the_stack(engine: &Bench, card: ObjectId) -> &crate::object::GameObject {
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).last(),
        Some(&card),
        "the spell is on the stack: {:?}",
        engine.pending()
    );
    engine.state().object(card).expect("the spell")
}

/// The alternative's life is paid as the spell is cast (CR 601.2h), and the
/// spell is marked as cast for its alternative.
#[test]
fn an_alternative_life_payment_is_paid_as_the_spell_is_cast() {
    let mut engine = holding(7188, &[BARGAIN], 20);
    let bargain = cast(&mut engine, BARGAIN);
    assert!(on_the_stack(&engine, bargain).alt_cast);
    assert_eq!(life(&engine, me()), 17, "three life paid");
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), BARGAIN),
        vec![bargain],
        "and the spell resolved"
    );
}

/// The pitched card is chosen and exiled from the hand, and the life beside
/// it is paid — the whole alternative, not the half that asked a question.
#[test]
fn an_alternative_pitch_exiles_the_card_chosen_and_pays_the_life() {
    let mut engine = holding(7189, &[PITCH, FODDER], 20);
    let fodder = the(&engine, ZoneLocation::Hand(me()), FODDER);
    let pitch = cast(&mut engine, PITCH);
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("which card to exile: {:?}", engine.pending())
    };
    assert_eq!((player, options), (me(), vec![fodder]), "the Fodder, alone");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
    assert!(on_the_stack(&engine, pitch).alt_cast);
    assert_eq!(
        objects(&engine, ZoneLocation::Exile(me()), FODDER),
        vec![fodder],
        "the pitched card is exiled"
    );
    assert_eq!(life(&engine, me()), 19, "and one life paid");
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(me()))
            .is_empty()
    );
}

/// Two exile-from-hand parts consume two distinct cards, in two questions.
/// One matching card neither offers the alternative nor pays both parts.
#[test]
fn a_double_pitch_alternative_requires_and_exiles_two_distinct_cards() {
    assert!(!offered(
        &holding(7192, &[DOUBLE_PITCH, FODDER], 20),
        DOUBLE_PITCH
    ));
    let mut engine = holding(7193, &[DOUBLE_PITCH, FODDER, FODDER], 20);
    let fodder = objects(&engine, ZoneLocation::Hand(me()), FODDER);
    assert_eq!(fodder.len(), 2);
    let pitch = cast(&mut engine, DOUBLE_PITCH);
    for card in &fodder {
        let Pending::ChooseCards {
            options, min, max, ..
        } = engine.pending().clone()
        else {
            panic!("one question per pitched card: {:?}", engine.pending())
        };
        assert_eq!((min, max), (1, 1));
        assert!(options.contains(card));
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![*card],
                },
            )
            .unwrap();
    }
    assert!(on_the_stack(&engine, pitch).alt_cast);
    let exiled = objects(&engine, ZoneLocation::Exile(me()), FODDER);
    assert_eq!(exiled.len(), 2);
    assert!(fodder.iter().all(|card| exiled.contains(card)));
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(me()))
            .is_empty()
    );
}

// --------------------------------------------- paying the additional cost

/// "As an additional cost, pay X life": X is announced (capped at the life
/// there is to pay, CR 119.4), and exactly that much is paid.
#[test]
fn an_additional_x_life_payment_pays_the_x_announced() {
    let mut engine = holding(7190, &[DELUGE], 20);
    let deluge = cast(&mut engine, DELUGE);
    let Pending::ChooseNumber { player, max, .. } = engine.pending().clone() else {
        panic!("X is announced: {:?}", engine.pending())
    };
    assert_eq!((player, max), (me(), 20), "up to the life there is");
    engine.apply(me(), PlayerAction::ChooseNumber(4)).unwrap();
    assert_eq!(on_the_stack(&engine, deluge).x_value, 4);
    assert_eq!(life(&engine, me()), 16, "four life paid");
}

/// "As an additional cost, pay 2 life": paid, with no question asked.
#[test]
fn an_additional_fixed_life_payment_is_paid() {
    let mut engine = holding(7191, &[TITHE], 20);
    let tithe = cast(&mut engine, TITHE);
    on_the_stack(&engine, tithe);
    assert_eq!(life(&engine, me()), 18, "two life paid");
}

/// "As an additional cost, sacrifice a creature": the creature chosen goes
/// to the graveyard as the cost is paid, and the spell remembers its mana
/// value as it last existed (CR 608.2h) for the sentences that ask.
#[test]
fn an_additional_sacrifice_is_paid_and_its_mana_value_remembered() {
    let mut engine = bench(
        7192,
        cards(),
        [Seat::with(&[OX]).holding(&[OFFERING]), Seat::default()],
    );
    to_main(&mut engine, me());
    let ox = the(&engine, ZoneLocation::Battlefield, OX);
    let offering = cast(&mut engine, OFFERING);
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("which creature to sacrifice: {:?}", engine.pending())
    };
    assert_eq!((player, options), (me(), vec![ox]));
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![ox] })
        .unwrap();
    let spell = on_the_stack(&engine, offering);
    assert_eq!(
        spell
            .paid
            .as_ref()
            .and_then(|p| p.sacrificed_lki)
            .map(|s| s.mana_value),
        Some(3),
        "the Ox's {{2}}{{G}}"
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), OX),
        vec![ox],
        "sacrificed"
    );
}

// ------------------------------------------------- a total nobody can pay

/// A total the pool cannot pay is not offered once nothing is left to tap.
/// The X question of a cast paid from the floating pool by a seat with no
/// source left names only the X that mana pays for the whole cost, so with
/// the one Forest tapped and one {G} floating `{X}{G}` is asked at X = 0 and
/// no more (a seat with a source left gets an open X and the CR 601.2g
/// payment window, `cast_wizard::pool_x_tests`). An
/// X above it is refused, the cast stays in its X question and nothing is
/// spent; X = 0 casts the spell.
///
/// The version of this test before the fix pinned the defect itself: the
/// question offered the pool's size, the test picked that X, and the cast was
/// reversed at payment (CR 601.2h, 732.1), the card back in hand. The reversal
/// is still covered where it can still happen, in a payment window that
/// closes short:
/// `cancel_cast_tests::a_window_passed_short_is_reversed_like_a_cancel` and
/// `pay_in_cast_tests::a_cast_its_window_cannot_pay_goes_back_to_the_hand`.
#[test]
fn an_x_the_pool_cannot_pay_is_not_offered() {
    let mut engine = bench(
        7193,
        cards(),
        [Seat::with(&[forest()]).holding(&[RAY]), Seat::default()],
    );
    to_main(&mut engine, me());
    float_all(&mut engine, me());
    let ray = cast(&mut engine, RAY);
    let Pending::ChooseNumber { min, max, .. } = engine.pending() else {
        panic!("X must be announced: {:?}", engine.pending());
    };
    assert_eq!(
        (*min, *max),
        (0, 0),
        "one {{G}} floating pays the fixed {{G}} and no X"
    );
    assert!(
        engine.apply(me(), PlayerAction::ChooseNumber(1)).is_err(),
        "X = 1 would cost two"
    );
    assert!(
        matches!(
            engine.pending(),
            Pending::ChooseNumber { min: 0, max: 0, .. }
        ),
        "the refusal leaves the X question where it was: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and nothing of the mana was spent"
    );

    engine.apply(me(), PlayerAction::ChooseNumber(0)).unwrap();
    assert_eq!(on_the_stack(&engine, ray).x_value, 0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "{{G}} paid");
}
