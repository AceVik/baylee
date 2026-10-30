//! New opt-in facts must not rewrite any archived observation projection.
use super::*;
use baylee_core::ids::ObjectId;

fn before_x(extra_mana: bool) -> (Session, ObjectId) {
    let board = if extra_mana {
        vec!["Mountain", "Wastes", "Wastes"]
    } else {
        vec!["Mountain"]
    };
    let mut session = session_with(&["Fireball"], &board);
    for _ in 0..64 {
        if session.observe_view(OWNER).phase == Phase::FirstMain
            && matches!(session.pending(), Pending::Priority { player, .. } if *player==OWNER)
        {
            break;
        }
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected opening {other:?}"),
        };
        act(&mut session, action);
    }
    let sources = session
        .observe_view(OWNER)
        .battlefield
        .iter()
        .filter(|object| object.controller == OWNER)
        .map(|object| object.id)
        .collect::<Vec<_>>();
    for source in sources {
        let action = if matches!(session.pending(),Pending::Priority{legal,..} if legal.mana_abilities.contains(&source))
        {
            PlayerAction::ActivateManaAbility { source }
        } else {
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            }
        };
        act(&mut session, action);
    }
    let source = session
        .observe_view(OWNER)
        .hand
        .iter()
        .find(|card| card.name == "Fireball")
        .unwrap()
        .id;
    act(&mut session, PlayerAction::CastSpell { card: source });
    while matches!(session.pending(), Pending::ChooseCastMode { .. }) {
        act(&mut session, PlayerAction::ChooseMode(0));
    }
    assert!(matches!(session.pending(), Pending::ChooseNumber { .. }));
    (session, source)
}

fn assert_legacy_unchanged(session: &Session) {
    for seat in [OWNER, OTHER] {
        for view in [
            session.observe_view(seat),
            session.observe_view_with_context(seat),
            session.observe_view_with_payment_context(seat),
            socket_view(session, seat),
        ] {
            assert!(view.announced_x.is_none());
            assert!(
                !serde_json::to_value(view)
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .contains_key("announced_x")
            );
        }
    }
    let mut new = session.observe_view_with_announcement_context(OWNER);
    let old = session.observe_view_with_payment_context(OWNER);
    new.announced_x = None;
    assert_eq!(
        serde_json::to_vec(&new).unwrap(),
        serde_json::to_vec(&old).unwrap()
    );
}

#[test]
fn settled_x_breaks_target_view_alias_without_changing_payment_views() {
    let (mut zero, source) = before_x(true);
    assert_eq!(
        zero.observe_view_with_announcement_context(OWNER)
            .announced_cast,
        Some(source)
    );
    assert_eq!(
        zero.observe_view_with_announcement_context(OWNER)
            .announced_x,
        None
    );
    assert_legacy_unchanged(&zero);
    let (mut one, _) = before_x(true);
    assert_eq!(zero.snapshot_hash(), one.snapshot_hash());
    act(&mut zero, PlayerAction::ChooseNumber(0));
    act(&mut one, PlayerAction::ChooseNumber(1));
    assert!(matches!(zero.pending(), Pending::ChooseTargets { .. }));
    assert_eq!(
        serde_json::to_vec(&zero.observe_view_with_payment_context(OWNER)).unwrap(),
        serde_json::to_vec(&one.observe_view_with_payment_context(OWNER)).unwrap()
    );
    for (session, x) in [(&zero, 0), (&one, 1)] {
        let before = session.snapshot_hash();
        let new = session.observe_view_with_announcement_context(OWNER);
        assert_eq!(new.announced_x, Some(x));
        assert_eq!(
            session.view_for_with_announcement_context(OWNER).unwrap().1,
            new
        );
        assert_eq!(session.snapshot_hash(), before, "observation is read-only");
        assert!(
            session
                .observe_view_with_announcement_context(OTHER)
                .announced_x
                .is_none()
        );
        assert!(
            session
                .observe_view_with_announcement_context(OTHER)
                .announced_cast
                .is_none()
        );
        assert_legacy_unchanged(session);
    }
    act(&mut one, PlayerAction::CancelCast);
    assert_eq!(
        one.observe_view_with_announcement_context(OWNER)
            .announced_x,
        None
    );
    assert_eq!(
        one.observe_view_with_announcement_context(OWNER)
            .announced_cast,
        None
    );
}

#[test]
fn commit_and_automatic_rollback_clear_settled_x() {
    for targets in [vec![OTHER], vec![OWNER, OTHER]] {
        let (mut session, source) = before_x(false);
        act(&mut session, PlayerAction::ChooseNumber(0));
        assert_eq!(
            session
                .observe_view_with_announcement_context(OWNER)
                .announced_x,
            Some(0)
        );
        let commits = targets.len() == 1;
        act(
            &mut session,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: targets,
            },
        );
        let view = session.observe_view_with_announcement_context(OWNER);
        assert_eq!(view.announced_x, None);
        assert_eq!(view.announced_cast, None);
        assert_eq!(view.stack.iter().any(|object| object.id == source), commits);
        assert_eq!(view.hand.iter().any(|object| object.id == source), !commits);
        assert_legacy_unchanged(&session);
    }
}
