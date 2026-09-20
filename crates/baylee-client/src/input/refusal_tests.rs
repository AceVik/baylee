use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{LegalActions, Pending, PlayerAction};

fn priority() -> Pending {
    Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(LegalActions {
            can_pass: true,
            ..LegalActions::default()
        }),
    }
}

/// A refusal stands until this seat tries something else.
///
/// It cannot be cleared when the next question arrives, which is the
/// obvious place: the acting seat is re-sent its own question every time
/// anybody at the table says anything, so the line would have been wiped
/// a frame or two after it appeared — and the click that earned it is the
/// click the player is waiting to understand.
#[test]
fn a_refusal_outlives_the_question_it_answered() {
    let mut duel = crate::Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            priority(),
            PlayerId::new(0),
        )),
        // The engine's own sentence, which arrives as prose and stays as
        // prose — the arm that is not going away.
        last_error: Some(baylee_client_core::i18n::Refusal::Verbatim(
            "illegal action for your seat".to_string(),
        )),
        ..crate::Duel::default()
    };

    // The same question again — an opponent said something. This goes
    // through the arm that installs it, because assigning the field would
    // only be testing that writing one field leaves another alone.
    duel.receive_choice(priority());
    assert_eq!(
        duel.last_error
            .as_ref()
            .map(|r| r.text(baylee_client_core::Lang::En)),
        Some("illegal action for your seat".to_string()),
        "a re-sent question is not this player doing anything"
    );

    // This player tries again: the old refusal is about the old attempt.
    duel.submit(PlayerAction::PassPriority);
    assert!(duel.last_error.is_none());
    assert_eq!(duel.outbox(), &[PlayerAction::PassPriority]);
}

/// An armed deed the engine has withdrawn says so, and says it without
/// sending anything.
#[test]
fn a_stale_deed_reports_instead_of_firing() {
    let mut duel = crate::Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            priority(),
            PlayerId::new(0),
        )),
        armed: Some(crate::Armed {
            object: ObjectId::new(3, 0),
            deed: crate::Deed::Play,
        }),
        ..crate::Duel::default()
    };
    super::fire_armed(&mut duel);
    assert!(duel.outbox().is_empty(), "nothing goes on the wire");
    assert_eq!(
        duel.last_error,
        Some(baylee_client_core::i18n::Refusal::Said(
            baylee_client_core::i18n::Phrase::DeedWithdrawn
        )),
        "the stale-deed line is this client's own sentence, so it is a phrase"
    );
}
