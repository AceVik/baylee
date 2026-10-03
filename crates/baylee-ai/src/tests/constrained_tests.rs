use super::*;
use baylee_cards_dsl::TextWordKind;
use baylee_core::ids::DamageSourceRef;
use baylee_core::mana::{ManaCost, ManaPayment, ManaSymbol};
use baylee_engine::choice::{LegalActions, ManaAbilityChoice, ManaChoiceId, NumberPrompt};

#[test]
fn mandatory_mana_answers_keep_the_exact_step_and_incarnation() {
    let view = view(0, &[20, 20], vec![]);
    let source = DamageSourceRef {
        object: obj(3),
        version: 8,
    };
    let choice = ManaChoiceId { source, step: 11 };
    let pending = Pending::ChooseManaAbility {
        player: view.seat,
        choice,
        options: vec![ManaAbilityChoice {
            source,
            ability_index: Some(4),
        }],
    };
    let agent = agent();
    let answer = agent.act(&view, &pending);
    assert_eq!(
        answer,
        PlayerAction::ChooseManaAbility {
            choice,
            source,
            ability_index: Some(4)
        }
    );
    assert_eq!(pending.answer_fault(&answer), None);
    assert_eq!(agent.fallbacks(), 0);
}

#[test]
fn word_changes_choose_a_present_public_word_and_a_distinct_replacement() {
    let owner = PlayerId::new(0);
    let mut creature = permanent(obj(1), owner, 2);
    creature.keywords = baylee_cards_dsl::KeywordSet::PROTECTION_BLACK.bits();
    let mut enemy = permanent(obj(2), PlayerId::new(1), 4);
    enemy.colors = ColorSet::of(baylee_core::color::Color::Red);
    let view = view(0, &[20, 20], vec![creature, enemy]);
    let target = view.target_objects[0].source;
    let pending = Pending::ChooseNumber {
        player: owner,
        min: 0,
        max: 19,
        reason: NumberPrompt::TextReplacement {
            kind: TextWordKind::Color,
            target,
        },
    };
    let PlayerAction::ChooseNumber(answer) = agent().act(&view, &pending) else {
        panic!("word pair");
    };
    let pair =
        baylee_engine::text_changes::TextReplacement::from_choice(TextWordKind::Color, answer)
            .unwrap();
    assert_eq!((pair.from, pair.to), (2, 3));
}

#[test]
fn controlled_payment_uses_the_subjects_pool_and_offered_lands() {
    let subject = PlayerId::new(1);
    let mut plains = permanent(obj(3), subject, 0);
    plains.types = TypeSet::LAND;
    plains
        .subtypes
        .insert(baylee_core::generated::subtypes::land::PLAINS);
    let mut view = view(0, &[20, 20], vec![plains]);
    view.awaiting = Some(view.seat);
    view.decision_player = Some(subject);
    view.owed = Some(ManaPayment::Fixed(ManaCost::from_symbol(ManaSymbol::White)));
    view.seats[0].mana_pool.white = 90_000;
    let pending = Pending::Priority {
        player: subject,
        legal: Box::new(LegalActions {
            can_pass: true,
            mana_abilities: vec![obj(3)],
            ..Default::default()
        }),
    };
    assert_eq!(
        agent().act(&view, &pending),
        PlayerAction::ActivateManaAbility { source: obj(3) }
    );
    view.seats[1].mana_pool.white = 1;
    assert_eq!(agent().act(&view, &pending), PlayerAction::PassPriority);
}

#[test]
fn masked_card_selection_uses_the_entitled_controlled_hand() {
    let subject = PlayerId::new(1);
    let mut view = view(0, &[20, 20], vec![]);
    view.awaiting = Some(view.seat);
    view.decision_player = Some(subject);
    view.controlled_hands.push(baylee_view::SharedHand {
        player: subject,
        cards: vec![hand_card(1, "Shivan Dragon")],
    });
    let pending = Pending::ChooseCards {
        player: subject,
        options: vec![obj(1)],
        min: 0,
        max: 1,
        prompt: ChoicePrompt::CastFaceDown {
            x: 6,
            paid: [0, 0, 0, 2, 0, 4],
            fixed_cost: ManaCost::ZERO,
        },
        total: None,
    };
    assert_eq!(
        agent().act(&view, &pending),
        PlayerAction::ChooseObjects {
            objects: vec![obj(1)]
        }
    );
}
