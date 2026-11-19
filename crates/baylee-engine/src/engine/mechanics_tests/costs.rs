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
use baylee_cards_dsl::{AltCondition, AlternativeCost, Condition, CostPart, CounterKind, Filter};

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
const RETURN: u32 = 7192;
const DOUBLE_RETURN: u32 = 7193;
const CONDITIONAL_BARGAIN: u32 = 7194;
const MIRE: u32 = 7195;
const DISCARD_ALT: u32 = 7196;
const REVERSE_DISCARD_ALT: u32 = 7197;
const PITCH_AND_DISCARD_ALT: u32 = 7198;

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
static RETURN_A_CREATURE: &[AlternativeCost] = &[instead(&[CostPart::ReturnToHand(&A_CREATURE)])];
static RETURN_TWO_CREATURES: &[AlternativeCost] = &[instead(&[
    CostPart::ReturnToHand(&A_CREATURE),
    CostPart::ReturnToHand(&A_CREATURE),
])];
static A_SWAMP: Filter = Filter::HasSubtype(baylee_core::generated::subtypes::land::SWAMP);
static CONTROL_A_SWAMP: Condition = Condition::ControlCount(&A_SWAMP, 1);
static PAY_FOUR_IF_SWAMP: &[AlternativeCost] = &[AlternativeCost {
    cost: Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::PayLife(4)],
    },
    condition: AltCondition::If(&CONTROL_A_SWAMP),
}];
static DISCARD_SWAMP_AND_OTHER: &[AlternativeCost] = &[instead(&[
    CostPart::Discard(&A_SWAMP),
    CostPart::Discard(&Filter::Any),
])];
static DISCARD_OTHER_AND_SWAMP: &[AlternativeCost] = &[instead(&[
    CostPart::Discard(&Filter::Any),
    CostPart::Discard(&A_SWAMP),
])];
static PITCH_INSTANT_AND_DISCARD_OTHER: &[AlternativeCost] = &[instead(&[
    CostPart::ExileFromHand(&AN_INSTANT),
    CostPart::Discard(&Filter::Any),
])];

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
        out_of_reach(RETURN, "Return", RETURN_A_CREATURE),
        out_of_reach(DOUBLE_RETURN, "Double Return", RETURN_TWO_CREATURES),
        out_of_reach(
            CONDITIONAL_BARGAIN,
            "Conditional Bargain",
            PAY_FOUR_IF_SWAMP,
        ),
        out_of_reach(DISCARD_ALT, "Discard Alt", DISCARD_SWAMP_AND_OTHER),
        out_of_reach(
            REVERSE_DISCARD_ALT,
            "Reverse Discard Alt",
            DISCARD_OTHER_AND_SWAMP,
        ),
        out_of_reach(
            PITCH_AND_DISCARD_ALT,
            "Pitch And Discard Alt",
            PITCH_INSTANT_AND_DISCARD_OTHER,
        ),
        card(
            MIRE,
            FaceDef {
                subtypes: &[baylee_core::generated::subtypes::land::SWAMP],
                ..land_face("Mire")
            },
            KeywordSet::EMPTY,
            &[],
        ),
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

/// An alternative cost gated by `AltCondition::If` (CR 118.9, 601.2b) is
/// offered and payable when its caster satisfies the condition, and is neither
/// offered nor selectable when the caster controls no matching permanent —
/// including when only an opponent controls one.
#[test]
fn a_conditional_alternative_cost_requires_the_caster_to_satisfy_the_condition() {
    // (a) Caster controls a Swamp-subtype land and has no mana: offered and
    // castable through the 4-life alternative cost.
    let mut with_swamp = bench(
        7198,
        cards(),
        [
            Seat::with(&[MIRE]).holding(&[CONDITIONAL_BARGAIN]).at(20),
            Seat::default(),
        ],
    );
    to_main(&mut with_swamp, me());
    assert!(
        offered(&with_swamp, CONDITIONAL_BARGAIN),
        "controlling a Swamp opens the alternative cost"
    );
    let spell = cast(&mut with_swamp, CONDITIONAL_BARGAIN);
    assert!(on_the_stack(&with_swamp, spell).alt_cast);
    assert_eq!(life(&with_swamp, me()), 16, "four life paid");
    settle(&mut with_swamp, me());
    assert_eq!(
        objects(
            &with_swamp,
            ZoneLocation::Graveyard(me()),
            CONDITIONAL_BARGAIN
        ),
        vec![spell],
    );

    // (b) Caster controls no Swamp (empty board, a tapped Forest, and when
    // only the opponent controls a Swamp): not offered and `CastSpell` is
    // refused.
    for (seed, mine, theirs, label) in [
        (7199, &[][..], &[][..], "no lands at all"),
        (7200, &[forest()][..], &[][..], "only a Forest"),
        (
            7201,
            &[forest()][..],
            &[MIRE][..],
            "only the opponent controls a Swamp",
        ),
    ] {
        let mut without = bench(
            seed,
            cards(),
            [
                Seat::with(mine).holding(&[CONDITIONAL_BARGAIN]).at(20),
                Seat::with(theirs),
            ],
        );
        to_main(&mut without, me());
        assert!(
            !offered(&without, CONDITIONAL_BARGAIN),
            "alternative cost must not be offered before tapping ({label})"
        );
        float_all(&mut without, me());
        assert!(
            !offered(&without, CONDITIONAL_BARGAIN),
            "alternative cost must not be offered ({label})"
        );
        let card = the(&without, ZoneLocation::Hand(me()), CONDITIONAL_BARGAIN);
        assert!(
            without
                .apply(me(), PlayerAction::CastSpell { card })
                .is_err(),
            "and `CastSpell` must refuse it ({label})"
        );
    }

    // Even when `{5}{G}` is floating so the spell is castable normally, the
    // wizard must not list the alternative mode unless the caster controls a
    // Swamp.
    let mut paid_without_swamp = bench(
        7202,
        cards(),
        [
            Seat::with(&[forest(), forest(), forest(), forest(), forest(), forest()])
                .holding(&[CONDITIONAL_BARGAIN])
                .at(20),
            Seat::with(&[MIRE]),
        ],
    );
    to_main(&mut paid_without_swamp, me());
    float_all(&mut paid_without_swamp, me());
    let spell = cast(&mut paid_without_swamp, CONDITIONAL_BARGAIN);
    assert!(
        !on_the_stack(&paid_without_swamp, spell).alt_cast,
        "only the printed cost is legal, so the wizard commits it directly"
    );
    assert_eq!(life(&paid_without_swamp, me()), 20);
    assert_eq!(paid_without_swamp.state().players[0].mana_pool.total(), 0);
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

/// A permanent returned as an alternative cost goes to its owner's hand
/// through the same cost question and payment path as an activated ability.
#[test]
fn a_return_alternative_is_chosen_and_paid_before_the_spell_is_on_the_stack() {
    let mut engine = bench(
        7194,
        cards(),
        [Seat::with(&[OX, OX]).holding(&[RETURN]), Seat::default()],
    );
    to_main(&mut engine, me());
    let oxen = objects(&engine, ZoneLocation::Battlefield, OX);
    assert!(offered(&engine, RETURN));
    let spell = cast(&mut engine, RETURN);
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("which permanent to return: {:?}", engine.pending())
    };
    assert_eq!(
        (min, max, prompt),
        (1, 1, crate::choice::ChoicePrompt::CostReturn)
    );
    assert_eq!(options, oxen);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![oxen[0]],
            },
        )
        .unwrap();
    assert!(on_the_stack(&engine, spell).alt_cast);
    assert_eq!(
        objects(&engine, ZoneLocation::Hand(me()), OX),
        vec![oxen[0]]
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, OX),
        vec![oxen[1]]
    );
}

/// "Its owner's hand" is not the caster's hand when the caster controls a
/// permanent owned by another player. The payment reads the owner directly.
#[test]
fn a_return_alternative_goes_to_the_permanents_owners_hand() {
    let owner = PlayerId::new(1);
    let mut engine = bench(
        7197,
        cards(),
        [Seat::default().holding(&[RETURN]), Seat::with(&[OX])],
    );
    to_main(&mut engine, me());
    let ox = the(&engine, ZoneLocation::Battlefield, OX);
    engine
        .dev_state_mut(me())
        .unwrap()
        .object_mut(ox)
        .unwrap()
        .set_controller(me());
    engine.refresh_offer();
    assert!(offered(&engine, RETURN));
    cast(&mut engine, RETURN);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "which controlled permanent to return: {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![ox]);
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![ox] })
        .unwrap();
    assert_eq!(objects(&engine, ZoneLocation::Hand(owner), OX), vec![ox]);
    assert!(objects(&engine, ZoneLocation::Hand(me()), OX).is_empty());
}

/// Two return costs need distinct permanents, both when the alternative is
/// offered and when it is paid. One creature cannot be returned twice.
#[test]
fn a_double_return_alternative_requires_and_returns_two_distinct_creatures() {
    let mut one = bench(
        7195,
        cards(),
        [Seat::with(&[OX]).holding(&[DOUBLE_RETURN]), Seat::default()],
    );
    to_main(&mut one, me());
    assert!(!offered(&one, DOUBLE_RETURN));
    let card = the(&one, ZoneLocation::Hand(me()), DOUBLE_RETURN);
    assert!(one.apply(me(), PlayerAction::CastSpell { card }).is_err());

    let mut engine = bench(
        7196,
        cards(),
        [
            Seat::with(&[OX, OX]).holding(&[DOUBLE_RETURN]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let oxen = objects(&engine, ZoneLocation::Battlefield, OX);
    assert!(offered(&engine, DOUBLE_RETURN));
    let spell = cast(&mut engine, DOUBLE_RETURN);
    for (index, ox) in oxen.iter().enumerate() {
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            panic!("one return question per creature: {:?}", engine.pending())
        };
        assert_eq!(options, oxen[index..]);
        if index == 1 {
            assert!(
                engine
                    .apply(
                        me(),
                        PlayerAction::ChooseObjects {
                            objects: vec![oxen[0]]
                        }
                    )
                    .is_err(),
                "the first creature cannot pay the second return"
            );
        }
        engine
            .apply(me(), PlayerAction::ChooseObjects { objects: vec![*ox] })
            .unwrap();
    }
    assert!(on_the_stack(&engine, spell).alt_cast);
    assert_eq!(objects(&engine, ZoneLocation::Hand(me()), OX), oxen);
    assert!(objects(&engine, ZoneLocation::Battlefield, OX).is_empty());
}

/// Discarding cards as an alternative cost (CR 118.9, 601.2b, 601.2h, 701.9a):
/// - requires distinct cards in hand excluding the spell itself (CR 601.2a);
/// - asks the caster in printed order (`ChoicePrompt::CostDiscard`) before the
///   spell becomes cast;
/// - discards both chosen cards into their owner's graveyard under `Cause::Cost`.
#[test]
fn a_discard_alternative_requires_and_discards_distinct_matching_hand_cards() {
    // Missing cards or missing the filtered card: neither offered nor castable.
    for (seed, hand, label) in [
        (7210, &[DISCARD_ALT][..], "the spell alone"),
        (
            7211,
            &[DISCARD_ALT, MIRE][..],
            "one Swamp card short of a second card",
        ),
        (
            7212,
            &[DISCARD_ALT, FODDER, HYMN][..],
            "two other cards, neither a Swamp card",
        ),
    ] {
        let mut short = holding(seed, hand, 20);
        assert!(
            !offered(&short, DISCARD_ALT),
            "alternative must not be offered ({label})"
        );
        let card = the(&short, ZoneLocation::Hand(me()), DISCARD_ALT);
        assert!(
            short.apply(me(), PlayerAction::CastSpell { card }).is_err(),
            "`CastSpell` must refuse ({label})"
        );
    }

    // Swamp card + another card: offered, asked in printed order, and discarded.
    let mut engine = holding(7213, &[DISCARD_ALT, MIRE, FODDER], 20);
    let mire = the(&engine, ZoneLocation::Hand(me()), MIRE);
    let fodder = the(&engine, ZoneLocation::Hand(me()), FODDER);
    assert!(offered(&engine, DISCARD_ALT));
    let spell = cast(&mut engine, DISCARD_ALT);

    // First question: the Swamp-subtype discard.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected first discard choice, got {:?}", engine.pending());
    };
    assert_eq!(
        (player, min, max, prompt),
        (me(), 1, 1, crate::choice::ChoicePrompt::CostDiscard)
    );
    assert_eq!(
        options,
        vec![mire],
        "only the Swamp card satisfies the first discard"
    );
    assert!(
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![fodder],
                },
            )
            .is_err(),
        "a non-Swamp card is refused for the first discard"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![mire],
            },
        )
        .unwrap();

    // Second question: "another card" — neither the spell nor the already-chosen Swamp.
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected second discard choice, got {:?}", engine.pending());
    };
    assert_eq!(
        (min, max, prompt),
        (1, 1, crate::choice::ChoicePrompt::CostDiscard)
    );
    assert_eq!(options, vec![fodder]);
    for illegal in [mire, spell] {
        assert!(
            engine
                .apply(
                    me(),
                    PlayerAction::ChooseObjects {
                        objects: vec![illegal],
                    },
                )
                .is_err(),
            "neither the already-chosen card nor the spell itself can pay the second discard"
        );
    }
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();

    assert!(on_the_stack(&engine, spell).alt_cast);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(me()))
            .is_empty(),
        "both discarded cards and the cast spell have left the hand"
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), MIRE),
        vec![mire]
    );
    assert_eq!(
        objects(&engine, ZoneLocation::Graveyard(me()), FODDER),
        vec![fodder]
    );
    for discarded in [mire, fodder] {
        assert!(
            engine
                .state()
                .journal
                .entries()
                .iter()
                .any(|entry| matches!(
                    entry.event,
                    crate::event::GameEvent::Discarded { object, player, .. }
                        if object == discarded && player == me()
                )),
            "payment records a real discard event (CR 701.9a)"
        );
    }
}

/// Augmenting-path lookahead across hand-card alternative costs:
/// - when a broad `Discard(Any)` precedes a narrow `Discard(Swamp)` and only one
///   Swamp card is in hand beside a non-Swamp card, the first prompt withholds the
///   sole Swamp card so the second discard remains payable;
/// - an `ExileFromHand` part and a `Discard` part share the same hand-card pool
///   and cannot both be paid with one card.
#[test]
fn hand_card_alternative_costs_match_distinct_cards_without_greedy_dead_ends() {
    let mut engine = holding(7214, &[REVERSE_DISCARD_ALT, MIRE, FODDER], 20);
    let mire = the(&engine, ZoneLocation::Hand(me()), MIRE);
    let fodder = the(&engine, ZoneLocation::Hand(me()), FODDER);
    assert!(offered(&engine, REVERSE_DISCARD_ALT));
    let spell = cast(&mut engine, REVERSE_DISCARD_ALT);

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected first discard choice, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![fodder],
        "spending the sole Swamp on `Any` would leave nothing for the second discard"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected second discard choice, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![mire]);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![mire],
            },
        )
        .unwrap();
    assert!(on_the_stack(&engine, spell).alt_cast);

    // One instant card cannot pay both `ExileFromHand(Instant)` and `Discard(Any)`.
    let mut one_card = holding(7215, &[PITCH_AND_DISCARD_ALT, FODDER], 20);
    assert!(!offered(&one_card, PITCH_AND_DISCARD_ALT));
    let card = the(&one_card, ZoneLocation::Hand(me()), PITCH_AND_DISCARD_ALT);
    assert!(
        one_card
            .apply(me(), PlayerAction::CastSpell { card })
            .is_err()
    );

    // Adding a second card makes both parts payable on distinct cards.
    let mut two_cards = holding(7216, &[PITCH_AND_DISCARD_ALT, FODDER, HYMN], 20);
    let fodder = the(&two_cards, ZoneLocation::Hand(me()), FODDER);
    let hymn = the(&two_cards, ZoneLocation::Hand(me()), HYMN);
    assert!(offered(&two_cards, PITCH_AND_DISCARD_ALT));
    let spell = cast(&mut two_cards, PITCH_AND_DISCARD_ALT);
    two_cards
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = two_cards.pending().clone() else {
        panic!(
            "expected discard choice after pitch, got {:?}",
            two_cards.pending()
        );
    };
    assert_eq!(
        options,
        vec![hymn],
        "the card chosen to exile is not offered again for the discard"
    );
    two_cards
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![hymn],
            },
        )
        .unwrap();
    assert!(on_the_stack(&two_cards, spell).alt_cast);
    assert_eq!(
        objects(&two_cards, ZoneLocation::Exile(me()), FODDER),
        vec![fodder]
    );
    assert_eq!(
        objects(&two_cards, ZoneLocation::Graveyard(me()), HYMN),
        vec![hymn]
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
/// reversed at payment (CR 601.2h, 734.1), the card back in hand. The reversal
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
