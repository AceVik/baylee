//! The choice contract's own tests: timeout answers, reserved indices, holds, standing answers and legal actions.

use super::*;
use baylee_core::ids::{AbilityRef, CardIndex, ObjectId};

fn ability(n: u32) -> AbilityRef {
    AbilityRef::new(CardIndex::new(n), n)
}

fn object() -> ObjectId {
    ObjectId::new(1, 0)
}

// ---- what the clock answers (#258) -----------------------------------

/// One question of every kind, each with the answer a seat that ran out
/// of time gets. `timeout_answer` has no wildcard arm, so a new kind of
/// question stops the build there; this table is the other half, and
/// says what each kind was decided to be.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one row per kind of question, and the table is the test"
)]
fn a_timed_out_seat_does_nothing_wherever_the_question_allows_it() {
    let p = PlayerId::new(0);
    let pass = Some(PlayerAction::PassPriority);
    let no = Some(PlayerAction::YesNo(false));
    let yes_no = |prompt| Pending::YesNo {
        player: p,
        prompt,
        source: None,
    };
    let cases: Vec<(Pending, Option<PlayerAction>)> = vec![
        (
            Pending::Priority {
                player: p,
                legal: Box::default(),
            },
            pass,
        ),
        (
            Pending::Mulligan {
                player: p,
                taken: 1,
                next_is_free: false,
                can_take: true,
            },
            Some(PlayerAction::MulliganKeep),
        ),
        (
            Pending::ChooseAttackers {
                player: p,
                attackers: vec![object()],
                defenders: vec![baylee_core::ids::Defender::Player(PlayerId::new(1))],
                required: Vec::new(),
                alone: Vec::new(),
                minimum_required: None,
                limits: Vec::new(),
            },
            Some(PlayerAction::DeclareAttackers {
                attackers: Vec::new(),
            }),
        ),
        (
            Pending::ChooseBlockers {
                demands: Vec::new(),
                player: p,
                attacker: PlayerId::new(1),
                blockers: vec![BlockOption {
                    blocker: object(),
                    attackers: vec![ObjectId::new(2, 0)],
                }],
                capacity: Vec::new(),
                obeying: Vec::new(),
                bounds: Vec::new(),
            },
            Some(PlayerAction::DeclareBlockers {
                blockers: Vec::new(),
            }),
        ),
        (
            yes_no(YesNoPrompt::PayLifeOrEnterTapped { amount: 2 }),
            no.clone(),
        ),
        (yes_no(YesNoPrompt::Kicker), no.clone()),
        (yes_no(YesNoPrompt::PayTax { mana: 1 }), no.clone()),
        (
            yes_no(YesNoPrompt::PayMana {
                cost: baylee_core::mana!("{U}"),
            }),
            no.clone(),
        ),
        (
            yes_no(YesNoPrompt::PayManaAndLife {
                cost: baylee_core::mana!("{B}"),
                life: 1,
            }),
            no.clone(),
        ),
        (
            yes_no(YesNoPrompt::RemoveCountersInsteadOfUntapping {
                permanent: object(),
                kind: baylee_cards_dsl::counters::WIND,
                count: 3,
            }),
            None,
        ),
        (
            yes_no(YesNoPrompt::UseShieldCounter {
                permanent: object(),
            }),
            None,
        ),
        (
            yes_no(YesNoPrompt::UseDamageClearingReplacement {
                permanent: object(),
            }),
            None,
        ),
        (
            yes_no(YesNoPrompt::ApplyAdditionalCountersNext {
                permanent: object(),
                additional: 1,
            }),
            None,
        ),
        (yes_no(YesNoPrompt::PayLife { amount: 7 }), no.clone()),
        (yes_no(YesNoPrompt::Miracle { card: object() }), no.clone()),
        (
            yes_no(YesNoPrompt::CastWithoutPaying { card: object() }),
            no.clone(),
        ),
        (
            yes_no(YesNoPrompt::CastPaying { card: object() }),
            no.clone(),
        ),
        (yes_no(YesNoPrompt::DrawOffer { proposer: p }), no.clone()),
        (yes_no(YesNoPrompt::MayDo), no.clone()),
        (
            yes_no(YesNoPrompt::SacrificeToEnter { count: 2 }),
            no.clone(),
        ),
        // No takes the turn as it would have been taken.
        (
            yes_no(YesNoPrompt::SkipTurn { source: object() }),
            no.clone(),
        ),
        (
            yes_no(YesNoPrompt::EnterTappedForCounters {
                kind: baylee_cards_dsl::CounterKind::Rad,
                amount: 2,
            }),
            no.clone(),
        ),
        (yes_no(YesNoPrompt::PayEnergy { amount: 2 }), no.clone()),
        (
            yes_no(YesNoPrompt::ProliferatePlayer { player: p }),
            no.clone(),
        ),
        (yes_no(YesNoPrompt::Generic), no.clone()),
        (yes_no(YesNoPrompt::ManaAnyType), no.clone()),
        (
            yes_no(YesNoPrompt::UseRegenerationShield {
                permanent: object(),
            }),
            no,
        ),
        (
            Pending::ChooseManaAbility {
                player: p,
                choice: ManaChoiceId {
                    source: baylee_core::ids::DamageSourceRef {
                        object: object(),
                        version: 1,
                    },
                    step: 1,
                },
                options: vec![ManaAbilityChoice {
                    source: baylee_core::ids::DamageSourceRef {
                        object: object(),
                        version: 1,
                    },
                    ability_index: Some(0),
                }],
            },
            None,
        ),
        (yes_no(YesNoPrompt::CommanderZone { card: object() }), None),
        // Declining a pact's payment loses the game, so nobody's clock
        // declines it for them.
        (
            yes_no(YesNoPrompt::PayPact {
                cost: baylee_core::mana::ManaCost::from_symbol_generic(2),
            }),
            None,
        ),
        (
            yes_no(YesNoPrompt::CommanderReplace {
                card: object(),
                to_library: false,
            }),
            None,
        ),
        (
            yes_no(YesNoPrompt::PayPact {
                cost: baylee_core::mana::ManaCost::ZERO,
            }),
            None,
        ),
        // Both answers move the card; the house picks the end.
        (yes_no(YesNoPrompt::TopOfLibrary { card: object() }), None),
        // No still moves the card, into the hand; the house casts it.
        (yes_no(YesNoPrompt::Discover { card: object() }), None),
        (
            Pending::MulliganBottom {
                player: p,
                count: 1,
            },
            None,
        ),
        (
            Pending::DiscardChoice {
                player: p,
                count: 1,
            },
            None,
        ),
        (
            Pending::LegendChoice {
                player: p,
                options: vec![object()],
            },
            None,
        ),
        (
            Pending::ChooseCards {
                player: p,
                options: vec![object()],
                min: 0,
                max: 1,
                prompt: ChoicePrompt::SearchLibrary,
                total: None,
            },
            None,
        ),
        (
            Pending::ChooseTargets {
                player: p,
                options: vec![object()],
                player_options: vec![],
                min: 0,
                max: 1,
                reason: TargetPrompt::Targets,
            },
            None,
        ),
        (
            Pending::ChooseSubtype {
                player: p,
                options: vec![],
            },
            None,
        ),
        (
            Pending::ChooseColor {
                player: p,
                options: vec![],
            },
            None,
        ),
        (
            Pending::ChooseCardName {
                player: p,
                types: baylee_core::types::TypeSet::EMPTY,
            },
            None,
        ),
        (
            Pending::ChooseCastMode {
                player: p,
                object: object(),
                options: vec![],
            },
            None,
        ),
        (
            Pending::ChooseNumber {
                player: p,
                min: 0,
                max: 3,
                reason: NumberPrompt::X,
            },
            None,
        ),
        (
            Pending::ChoosePlayer {
                player: p,
                options: vec![p],
            },
            None,
        ),
        (
            Pending::Arrange {
                player: p,
                cards: vec![object()],
                piles: vec![],
                prompt: ArrangePrompt::Order,
            },
            None,
        ),
        (
            Pending::ChoosePile {
                player: p,
                piles: vec![vec![object()], vec![]],
                label: None,
            },
            None,
        ),
        (
            Pending::ChooseDamageSource {
                player: p,
                choice: baylee_core::ids::SourceChoiceId::new(0),
                options: Vec::new(),
            },
            None,
        ),
        (
            Pending::ChooseDamageEffect {
                player: p,
                choice: DamageChoiceId { batch: 1, step: 1 },
                damage: vec![],
                options: vec![],
            },
            None,
        ),
        (
            Pending::AllocatePrevention {
                player: p,
                choice: DamageChoiceId { batch: 1, step: 2 },
                effect: DamageEffectOption {
                    id: 0,
                    source: None,
                    ability: None,
                    controller: p,
                    kind: DamageEffectKind::PreventNext { remaining: 1 },
                    parts: vec![],
                },
                damage: vec![],
                total: 0,
            },
            None,
        ),
        (
            Pending::GameOver(GameResult {
                winner: None,
                reason: crate::win::EndReason::Draw,
            }),
            None,
        ),
    ];
    for (pending, expected) in &cases {
        assert_eq!(
            timeout_answer(pending).as_ref(),
            expected.as_ref(),
            "{pending:?}"
        );
    }
    // Every kind is in the table. The yes/no is one kind per prompt,
    // because its prompts are answered differently.
    let covered: std::collections::BTreeSet<usize> =
        cases.iter().map(|(pending, _)| kind_of(pending)).collect();
    assert_eq!(
        covered,
        (0..KINDS).collect(),
        "a kind of question has no row in this table"
    );
}

/// How many kinds [`kind_of`] tells apart.
const KINDS: usize = 50;

/// Which kind of question this is, numbered without gaps. No wildcard
/// arm: a new `Pending` variant or yes/no prompt does not compile here
/// until it has a number, and then the table above is missing it.
fn kind_of(pending: &Pending) -> usize {
    match pending {
        Pending::ChooseDamageSource { .. } => 37,
        Pending::ChooseManaAbility { .. } => 38,
        Pending::ChooseDamageEffect { .. } => 35,
        Pending::AllocatePrevention { .. } => 36,
        Pending::Mulligan { .. } => 0,
        Pending::MulliganBottom { .. } => 1,
        Pending::Priority { .. } => 2,
        Pending::ChooseAttackers { .. } => 3,
        Pending::ChooseBlockers { .. } => 4,
        Pending::DiscardChoice { .. } => 5,
        Pending::LegendChoice { .. } => 6,
        Pending::ChooseCards { .. } => 7,
        Pending::ChooseTargets { .. } => 8,
        Pending::ChooseSubtype { .. } => 9,
        Pending::ChooseColor { .. } => 10,
        Pending::ChooseCastMode { .. } => 11,
        Pending::ChooseNumber { .. } => 12,
        Pending::ChoosePlayer { .. } => 13,
        Pending::Arrange { .. } => 14,
        Pending::GameOver(_) => 15,
        Pending::ChooseCardName { .. } => 16,
        Pending::YesNo { prompt, .. } => {
            17 + match prompt {
                YesNoPrompt::SacrificeToEnter { .. } => 25,
                YesNoPrompt::EnterTappedForCounters { .. } => 24,
                YesNoPrompt::PayEnergy { .. } => 22,
                YesNoPrompt::ProliferatePlayer { .. } => 23,
                YesNoPrompt::PayLifeOrEnterTapped { .. } => 0,
                YesNoPrompt::Kicker => 1,
                YesNoPrompt::ManaAnyType => 26,
                YesNoPrompt::UseRegenerationShield { .. } => 27,
                YesNoPrompt::UseShieldCounter { .. } => 28,
                YesNoPrompt::UseDamageClearingReplacement { .. } => 30,
                YesNoPrompt::ApplyAdditionalCountersNext { .. } => 29,
                YesNoPrompt::PayTax { .. } => 2,
                YesNoPrompt::PayLife { .. } => 9,
                YesNoPrompt::PayPact { .. } => 10,
                YesNoPrompt::Miracle { .. } => 3,
                YesNoPrompt::DrawOffer { .. } => 4,
                YesNoPrompt::CommanderZone { .. } => 5,
                YesNoPrompt::CommanderReplace { .. } => 6,
                YesNoPrompt::MayDo => 7,
                YesNoPrompt::Generic => 8,
                YesNoPrompt::TopOfLibrary { .. } => 11,
                YesNoPrompt::Discover { .. } => 12,
                YesNoPrompt::CastWithoutPaying { .. } => 13,
                YesNoPrompt::CastPaying { .. } => 14,
                YesNoPrompt::RemoveCountersInsteadOfUntapping { .. } => 32,
                YesNoPrompt::PayMana { .. } => 15,
                YesNoPrompt::PayManaAndLife { .. } => 31,
                YesNoPrompt::SkipTurn { .. } => 16,
            }
        }
        Pending::ChoosePile { .. } => 17 + 17,
    }
}

// ---- the reserved index block -------------------------------------

#[test]
fn every_granted_slot_decodes_to_the_slot_it_was_encoded_from() {
    // The one property that matters about this partition: it is written
    // in two places, an encoder and a decoder, and an engine that
    // decoded a slot differently from how the offer encoded it would
    // run a different ability than the player pressed.
    for n in [0, 1, 7, 8, 255, 65_535, MAX_GRANTED_IDS - 1] {
        assert_eq!(
            granted_slot(granted_ability(n)),
            Some(n),
            "slot {n} does not survive the round trip"
        );
    }
}

#[test]
fn slot_zero_is_still_the_constant_that_was_there_before_the_block() {
    // Urza's Saga grants itself two abilities; the block grew from one
    // index to eight and slot 0 had to keep its value, or every stored
    // offer of the first granted ability would have moved.
    assert_eq!(GRANTED_ABILITY, granted_ability(0));
    assert_eq!(granted_slot(GRANTED_ABILITY), Some(0));
}

#[test]
fn an_ordinary_ability_index_names_no_granted_slot() {
    // A printed ability is numbered by its position in a list, which is
    // why the reserved block counts down from the other end.
    for index in [0, 1, 2, 7, 100, 65_535] {
        assert_eq!(granted_slot(index), None, "index {index} decoded as a slot");
    }
}

#[test]
fn the_block_holds_exactly_its_own_slots_and_stops() {
    // The first index below the block must not decode, or the bound is
    // decoration: `MAX_GRANTED_IDS` would name a size nothing enforces.
    assert_eq!(
        granted_slot(granted_ability(MAX_GRANTED_IDS - 1)),
        Some(MAX_GRANTED_IDS - 1)
    );
    assert_eq!(granted_slot(0xffff_ff00), None);
}

#[test]
fn the_prepared_cast_sits_below_the_granted_block_and_not_inside_it() {
    // Emeritus of Woe's linked cast is not an ability the card prints
    // either, so it is carved out of the same end — and it moved when
    // the block grew. Two reserved indices that collided would offer
    // one thing and activate the other.
    assert_eq!(granted_slot(PREPARED_CAST), None);
    assert!(PREPARED_CAST > granted_ability(MAX_GRANTED_IDS - 1));
    for n in [0, 1, 7, 8, 255, 65_535, MAX_GRANTED_IDS - 1] {
        assert_ne!(PREPARED_CAST, granted_ability(n));
    }
}

#[test]
fn the_unlock_slots_sit_below_turning_face_up_and_decode_to_their_halves() {
    // A Room's two doors take the next two indices down. Each decodes
    // to its own half, none of them is another reserved index, and no
    // other reserved index or printed position decodes as a door: one
    // that did would unlock a door when a player pressed something else.
    assert_eq!(door_to_unlock(unlock_door(0)), Some(0));
    assert_eq!(door_to_unlock(unlock_door(1)), Some(1));
    assert_eq!(unlock_door(0), TURN_FACE_UP - 1);
    assert_eq!(unlock_door(1), TURN_FACE_UP - UNLOCK_SLOTS);
    let others = [0, 1, 7, 8, 255, 65_535, MAX_GRANTED_IDS - 1]
        .into_iter()
        .map(granted_ability)
        .chain([PREPARED_CAST, TURN_FACE_UP, TURN_FACE_UP - UNLOCK_SLOTS - 1])
        .chain([0, 1, 2, 7, 100, 65_535]);
    for index in others {
        assert_eq!(
            door_to_unlock(index),
            None,
            "index {index} decoded as a door"
        );
    }
    assert!(is_special_action(TURN_FACE_UP));
    assert!(is_special_action(unlock_door(1)));
    assert!(!is_special_action(PREPARED_CAST));
    assert!(!is_special_action(granted_ability(0)));
}

// ---- priority holds ------------------------------------------------

#[test]
fn a_hold_withholds_exactly_when_it_has_something_to_expire_from() {
    // Written as an exhaustive `match` rather than a list, so a new
    // variant is a compile error here instead of silently inheriting
    // one side of the answer. `PassWhenNothingToDo` is the interesting
    // one: it answers only where passing was the sole legal action, so
    // it withholds nothing and an indicator must not call it a hold.
    fn expected(hold: PriorityHold) -> bool {
        match hold {
            PriorityHold::Always | PriorityHold::PassWhenNothingToDo => false,
            PriorityHold::UntilStackEmpty { .. }
            | PriorityHold::UntilTopOfStack { .. }
            | PriorityHold::UntilEndOfTurn { .. } => true,
        }
    }
    for hold in [
        PriorityHold::Always,
        PriorityHold::PassWhenNothingToDo,
        PriorityHold::UntilStackEmpty { depth: 0 },
        PriorityHold::UntilTopOfStack { object: object() },
        PriorityHold::UntilEndOfTurn { turn: 3 },
    ] {
        assert_eq!(hold.suppresses(), expected(hold), "{hold:?}");
    }
}

#[test]
fn the_default_hold_is_the_one_that_asks_every_time() {
    assert_eq!(PriorityHold::default(), PriorityHold::Always);
    assert!(!PriorityHold::default().suppresses());
}

// ---- standing answers ----------------------------------------------

#[test]
fn a_standing_answer_is_the_boolean_it_stands_for() {
    assert!(StandingAnswer::Yes.as_bool());
    assert!(!StandingAnswer::No.as_bool());
}

#[test]
fn an_answer_is_found_under_its_own_ability_and_no_other() {
    let mut seat = SeatAutomation::default();
    seat.set_standing_answer(ability(2), Some(StandingAnswer::Yes));
    assert_eq!(seat.standing_answer(ability(2)), Some(StandingAnswer::Yes));
    assert_eq!(seat.standing_answer(ability(3)), None);
}

#[test]
fn answering_the_same_ability_twice_replaces_rather_than_duplicates() {
    let mut seat = SeatAutomation::default();
    seat.set_standing_answer(ability(1), Some(StandingAnswer::Yes));
    seat.set_standing_answer(ability(1), Some(StandingAnswer::No));
    assert_eq!(seat.standing_answer(ability(1)), Some(StandingAnswer::No));
    assert_eq!(seat.standing_answers().count(), 1);
}

#[test]
fn forgetting_removes_the_answer_and_forgetting_nothing_changes_nothing() {
    let mut seat = SeatAutomation::default();
    seat.set_standing_answer(ability(1), Some(StandingAnswer::Yes));
    seat.set_standing_answer(ability(1), None);
    assert_eq!(seat.standing_answer(ability(1)), None);
    assert_eq!(seat.standing_answers().count(), 0);

    // Forgetting one that was never remembered must not insert a hole.
    seat.set_standing_answer(ability(9), None);
    assert_eq!(seat.standing_answers().count(), 0);
    assert!(seat.is_default());
}

#[test]
fn remembered_answers_come_back_in_ability_order() {
    // Iteration order is part of the determinism contract — this list
    // is cloned with the engine on every AI lookahead ply — so an
    // insertion out of order must still read back sorted.
    let mut seat = SeatAutomation::default();
    for n in [5, 1, 4, 2] {
        seat.set_standing_answer(ability(n), Some(StandingAnswer::Yes));
    }
    let order: Vec<AbilityRef> = seat.standing_answers().map(|(a, _)| a).collect();
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(order, sorted);
    assert_eq!(order.len(), 4);
}

#[test]
fn a_seat_is_default_until_it_holds_or_remembers_something() {
    assert!(SeatAutomation::default().is_default());

    let holds = SeatAutomation {
        hold: PriorityHold::UntilEndOfTurn { turn: 1 },
        ..Default::default()
    };
    assert!(!holds.is_default());

    let mut remembers = SeatAutomation::default();
    remembers.set_standing_answer(ability(1), Some(StandingAnswer::No));
    assert!(!remembers.is_default());
}

// ---- what a standing answer may be kept for -------------------------

#[test]
fn only_the_questions_whose_answer_cannot_go_stale_are_automatable() {
    // Exhaustive for the same reason as the hold test. The pair worth
    // reading is `CommanderZone` (automatable — "always put Katara
    // back", and the `{2}` is a tax on a later cast) against
    // `CommanderReplace` (not — the right answer depends on where the
    // commander was going, which a stored `Yes` cannot see).
    fn expected(prompt: YesNoPrompt) -> bool {
        match prompt {
            YesNoPrompt::MayDo | YesNoPrompt::CommanderZone { .. } => true,
            YesNoPrompt::UseRegenerationShield { .. }
            | YesNoPrompt::UseDamageClearingReplacement { .. }
            | YesNoPrompt::UseShieldCounter { .. }
            | YesNoPrompt::ApplyAdditionalCountersNext { .. }
            | YesNoPrompt::SacrificeToEnter { .. }
            | YesNoPrompt::EnterTappedForCounters { .. }
            | YesNoPrompt::PayEnergy { .. }
            | YesNoPrompt::ProliferatePlayer { .. }
            | YesNoPrompt::PayLifeOrEnterTapped { .. }
            | YesNoPrompt::Kicker
            | YesNoPrompt::ManaAnyType
            | YesNoPrompt::PayTax { .. }
            | YesNoPrompt::RemoveCountersInsteadOfUntapping { .. }
            | YesNoPrompt::PayMana { .. }
            | YesNoPrompt::PayManaAndLife { .. }
            | YesNoPrompt::PayLife { .. }
            | YesNoPrompt::PayPact { .. }
            | YesNoPrompt::Miracle { .. }
            | YesNoPrompt::CastWithoutPaying { .. }
            | YesNoPrompt::CastPaying { .. }
            | YesNoPrompt::DrawOffer { .. }
            | YesNoPrompt::CommanderReplace { .. }
            | YesNoPrompt::TopOfLibrary { .. }
            | YesNoPrompt::Discover { .. }
            | YesNoPrompt::SkipTurn { .. }
            | YesNoPrompt::Generic => false,
        }
    }
    for prompt in [
        YesNoPrompt::UseRegenerationShield {
            permanent: object(),
        },
        YesNoPrompt::UseShieldCounter {
            permanent: object(),
        },
        YesNoPrompt::UseDamageClearingReplacement {
            permanent: object(),
        },
        YesNoPrompt::ApplyAdditionalCountersNext {
            permanent: object(),
            additional: 1,
        },
        YesNoPrompt::SacrificeToEnter { count: 2 },
        YesNoPrompt::EnterTappedForCounters {
            kind: baylee_cards_dsl::CounterKind::Rad,
            amount: 2,
        },
        YesNoPrompt::MayDo,
        YesNoPrompt::Generic,
        YesNoPrompt::SkipTurn { source: object() },
        YesNoPrompt::Kicker,
        YesNoPrompt::PayLifeOrEnterTapped { amount: 1 },
        YesNoPrompt::PayTax { mana: 2 },
        YesNoPrompt::PayMana {
            cost: baylee_core::mana!("{G}{G}"),
        },
        YesNoPrompt::PayLife { amount: 7 },
        YesNoPrompt::Miracle { card: object() },
        YesNoPrompt::CastWithoutPaying { card: object() },
        YesNoPrompt::CastPaying { card: object() },
        YesNoPrompt::CommanderZone { card: object() },
        YesNoPrompt::TopOfLibrary { card: object() },
        YesNoPrompt::Discover { card: object() },
        YesNoPrompt::CommanderReplace {
            card: object(),
            to_library: true,
        },
        YesNoPrompt::CommanderReplace {
            card: object(),
            to_library: false,
        },
    ] {
        assert_eq!(prompt.automatable(), expected(prompt), "{prompt:?}");
    }
}

// ---- what a seat may do --------------------------------------------

#[test]
fn a_seat_offered_nothing_can_only_pass_and_has_no_mana_to_make() {
    let nothing = LegalActions::default();
    assert!(nothing.nothing_but_passing());
    assert!(!nothing.has_mana_source());
}
