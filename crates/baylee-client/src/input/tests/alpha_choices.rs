//! Real input dispatch for the new resolving choices.
use super::*;
use crate::{Duel, input::pick_choice};
use baylee_client_core::Lang;
use baylee_core::ids::DamageSourceRef;
use baylee_engine::choice::{ManaAbilityChoice, ManaChoiceId, NumberPrompt};

#[test]
fn word_choice_requires_two_distinct_words_and_explicit_confirmation() {
    let mut duel = Duel::default();
    let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    let seat = view.seat;
    duel.view = Some(view);
    duel.receive_choice(Pending::ChooseNumber {
        player: seat,
        min: 0,
        max: 19,
        reason: NumberPrompt::TextReplacement {
            kind: baylee_cards_dsl::TextWordKind::Color,
            target: DamageSourceRef {
                object: ObjectId::new(9, 0),
                version: 2,
            },
        },
    });
    pick_choice(&mut duel, 3);
    assert!(!duel.interaction.as_ref().unwrap().can_confirm());
    pick_choice(&mut duel, 6);
    assert!(
        duel.outbox().is_empty(),
        "choosing the second word must not submit"
    );
    assert_eq!(
        duel.interaction.as_ref().unwrap().confirm(),
        Some(PlayerAction::ChooseNumber(13))
    );
    let prompt = duel.interaction.as_ref().unwrap().prompt();
    let rows = crate::choices::options(
        &prompt,
        Lang::De,
        None,
        "",
        crate::choices::FaceNames::default(),
    )
    .unwrap();
    assert_eq!(rows.iter().filter(|r| r.label.starts_with('✓')).count(), 2);
    let keys = press(bevy::prelude::KeyCode::KeyS);
    let keymap = baylee_client_core::prefs::Keymap::standard();
    assert!(crate::input::damage_keys(
        crate::keys::Fired::of(&keys, &keymap),
        &mut duel
    ));
    assert_eq!(duel.interaction.as_ref().unwrap().chosen_index(), Some(7));
    assert!(duel.outbox().is_empty());
}

#[test]
fn required_mana_selection_uses_nonce_and_explicit_confirm() {
    let mut duel = Duel::default();
    let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    let seat = view.seat;
    duel.view = Some(view);
    let source = DamageSourceRef {
        object: ObjectId::new(9, 0),
        version: 2,
    };
    let choice = ManaChoiceId { source, step: 3 };
    duel.receive_choice(Pending::ChooseManaAbility {
        player: seat,
        choice,
        options: vec![ManaAbilityChoice {
            source,
            ability_index: Some(1),
        }],
    });
    pick_choice(&mut duel, 0);
    assert!(duel.outbox().is_empty());
    assert_eq!(
        duel.interaction.as_ref().unwrap().confirm(),
        Some(PlayerAction::ChooseManaAbility {
            choice,
            source,
            ability_index: Some(1)
        })
    );
    duel.receive_choice(Pending::ChooseManaAbility {
        player: seat,
        choice: ManaChoiceId { step: 4, ..choice },
        options: vec![ManaAbilityChoice {
            source,
            ability_index: Some(1),
        }],
    });
    assert!(!duel.interaction.as_ref().unwrap().can_confirm());
}

#[test]
fn controlled_segment_uses_subject_hand_and_drops_it_when_control_ends() {
    use baylee_client_core::test_support::ViewBuilder;
    let mut view = ViewBuilder::new(2)
        .with_hand(vec![("Own card", 1, 10)])
        .build();
    let controlled = ViewBuilder::new(2)
        .with_hand(vec![("Controlled card", 2, 20)])
        .build()
        .hand;
    view.controlled_hands.push(baylee_view::SharedHand {
        player: PlayerId::new(1),
        cards: controlled,
    });
    view.decision_player = Some(PlayerId::new(1));
    let mut duel = Duel {
        view: Some(view),
        ..Duel::default()
    };
    duel.receive_choice(Pending::ChooseCards {
        player: PlayerId::new(1),
        options: vec![obj(20)],
        min: 1,
        max: 1,
        prompt: baylee_engine::choice::ChoicePrompt::CommandCard,
        total: None,
    });
    assert!(duel.interaction.as_ref().unwrap().is_mine());
    assert_eq!(
        baylee_client_core::decision::hand(duel.view.as_ref().unwrap())[0].id,
        obj(20)
    );
    assert!(!duel.interaction.as_ref().unwrap().is_selectable(obj(10)));
    let view = duel.view.as_mut().unwrap();
    view.decision_player = None;
    view.controlled_hands.clear();
    assert_eq!(baylee_client_core::decision::hand(view)[0].id, obj(10));
    assert!(!baylee_client_core::decision::known_cards(view).any(|card| card.id == obj(20)));
}

#[test]
fn mask_presenter_keeps_receipt_separate_and_allows_explicit_decline() {
    let mut duel = Duel::default();
    let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    let seat = view.seat;
    duel.view = Some(view);
    duel.receive_choice(Pending::ChooseCards {
        player: seat,
        options: vec![obj(20)],
        min: 0,
        max: 1,
        prompt: baylee_engine::choice::ChoicePrompt::CastFaceDown {
            x: 3,
            paid: [1, 0, 0, 2, 0, 0],
            fixed_cost: baylee_core::mana::ManaCost::default(),
        },
        total: None,
    });
    let interaction = duel.interaction.as_ref().unwrap();
    let lines = crate::choices::target_question(
        interaction,
        duel.view.as_ref().unwrap(),
        Lang::De,
        &crate::cardtext::CardTexts::default(),
        None,
    );
    assert!(lines[0].contains("Tatsächliche Manazahlung: 1 × {W}, 2 × {R}"));
    assert!(lines[0].contains("feste Aktivierungskosten:"));
    assert_eq!(
        interaction.confirm(),
        Some(PlayerAction::ChooseObjects { objects: vec![] })
    );
    assert!(duel.outbox().is_empty());
}
