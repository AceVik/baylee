use super::*;
use crate::cardtext::CardTexts;
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_core::ids::Defender;
use baylee_view::{AttackerView, BlockerView};

fn attacked_by(power: i16, blocked: bool) -> PlayerView {
    let blockers = if blocked {
        vec![BlockerView {
            blocker: ObjectId::new(10, 0),
            attacker: ObjectId::new(1, 0),
        }]
    } else {
        Vec::new()
    };
    ViewBuilder::new(2)
        .with_battlefield(1, vec![token(1, 1, "Ogre", power, 3)])
        .with_battlefield(0, vec![token(10, 0, "Wall", 0, 4)])
        .with_combat(
            vec![AttackerView {
                creature: ObjectId::new(1, 0),
                defending: Defender::Player(PlayerId::new(0)),
                blocked,
            }],
            blockers,
        )
        .build()
}

#[test]
fn an_attack_aimed_at_this_seat_is_read_out_and_marked() {
    let view = attacked_by(3, false);
    let (line, threatened) =
        rail::incoming_line(&view, None, None, &CardTexts::default(), Lang::En)
            .expect("combat is declared");
    assert!(
        line.contains('3'),
        "the number that gets through is in the line: {line}"
    );
    assert!(
        threatened,
        "three unblocked power at this seat is worth a colour"
    );
}

#[test]
fn a_blocked_attack_is_still_read_out_but_no_longer_marked() {
    let view = attacked_by(3, true);
    let (line, threatened) =
        rail::incoming_line(&view, None, None, &CardTexts::default(), Lang::En)
            .expect("combat is declared");
    assert!(
        !threatened,
        "nothing reaches this seat once the attacker is blocked: {line}"
    );
}

#[test]
fn there_is_no_line_when_nobody_is_attacking() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![token(1, 0, "Bear", 2, 2)])
        .build();
    assert!(rail::incoming_line(&view, None, None, &CardTexts::default(), Lang::En).is_none());
}

#[test]
fn the_aim_line_says_nothing_about_a_card_choice() {
    // A target prompt has an aim too, and it is aimed at the candidate a
    // click would pick rather than at a defender — so `combat_focus` has
    // nothing to name and this line, drawn anyway, would say "aiming at
    // nothing (1 of 3)" every time a spell asked which card to discard.
    let view = ViewBuilder::new(2).build();
    let choice = baylee_engine::choice::Pending::ChooseCards {
        player: PlayerId::new(0),
        options: vec![
            ObjectId::new(1, 0),
            ObjectId::new(2, 0),
            ObjectId::new(3, 0),
        ],
        min: 1,
        max: 1,
        prompt: baylee_engine::choice::ChoicePrompt::SearchLibrary,
    };
    let interaction = baylee_client_core::Interaction::new(choice, PlayerId::new(0));
    assert!(interaction.focus_position().is_some(), "there is an aim");
    assert!(
        rail::combat_line(&interaction, &view, None, &CardTexts::default(), Lang::En).is_none(),
        "but it is not a combat aim, and this line only speaks for combat"
    );
}
