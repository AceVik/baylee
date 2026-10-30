use super::*;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController, SeatSpec,
};
use baylee_engine::choice::PlayerAction;
use baylee_protocol::v1::envelope::Msg;
use baylee_view::Phase;

const OWNER: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);

fn entry(name: &str) -> DeckEntry {
    let card: CardIndex = baylee_cards::all()
        .find(|card| card.name() == name)
        .unwrap()
        .index;
    DeckEntry {
        card,
        print: PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

fn session() -> Session {
    session_with(&["Shock"], &["Mountain"])
}

fn session_with(hand: &[&str], board: &[&str]) -> Session {
    let seats = (0..2)
        .map(|index| SeatSpec {
            controller: SeatController::Open,
            capabilities: baylee_core::preset::SeatCapabilities::default(),
            deck: vec![entry("Forest"); 32],
            sideboard: vec![],
            commanders: vec![],
            starting_life: Some(20),
            starting_hand: Some(if index == 0 {
                hand.iter().map(|name| entry(name)).collect()
            } else {
                vec![entry("Counterspell")]
            }),
            starting_battlefield: if index == 0 {
                board.iter().map(|name| entry(name)).collect()
            } else {
                vec![]
            },
            emblems: vec![],
            team: None,
        })
        .collect();
    Session::new(&GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format: FormatId::Freeform,
        seed: 12,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        seats,
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "en".into(),
            finish: Finish::Normal,
        }],
    })
    .unwrap()
}

fn act(session: &mut Session, action: PlayerAction) {
    let seat = session.pending().asked().unwrap();
    session.act(seat, action).unwrap();
}

fn socket_view(session: &Session, seat: PlayerId) -> PlayerView {
    session
        .snapshot(seat)
        .iter()
        .find_map(|envelope| {
            if let Some(Msg::StateDelta(delta)) = &envelope.msg {
                Some(serde_json::from_slice(&delta.view_json).unwrap())
            } else {
                None
            }
        })
        .expect("socket snapshot contains the authorized view")
}

fn assert_context_parity(session: &Session) {
    let (_, enriched) = session.view_for_with_context(OWNER).unwrap();
    let legacy = session.view_for(OWNER).unwrap().1;
    let wire = socket_view(session, OWNER);
    assert_eq!(enriched.casting, wire.casting);
    assert_eq!(enriched.targeting, wire.targeting);
    assert_eq!(enriched.owed, wire.owed);
    let passive = session.observe_view_with_context(OWNER);
    assert_eq!(passive.casting, enriched.casting);
    assert_eq!(passive.targeting, enriched.targeting);
    let mut stripped = enriched;
    stripped.casting = None;
    stripped.targeting = None;
    assert_eq!(
        stripped, legacy,
        "all archived deciding-view fields stay identical"
    );
    let mut stripped = passive;
    stripped.casting = None;
    stripped.targeting = None;
    assert_eq!(stripped, session.observe_view(OWNER));
    for other in [
        session.observe_view_with_context(OTHER),
        socket_view(session, OTHER),
    ] {
        assert!(other.casting.is_none());
        assert!(other.targeting.is_none());
        assert!(other.hand.iter().all(|card| card.name != "Shock"));
    }
    assert!(session.view_for_with_context(OTHER).is_none());
}

#[test]
fn offline_choice_context_matches_socket_without_leaking_to_opponent() {
    let mut session = session();
    for _ in 0..64 {
        if session.observe_view(OWNER).phase == Phase::FirstMain {
            break;
        }
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected opening {other:?}"),
        };
        act(&mut session, action);
    }
    let shock = session
        .observe_view(OWNER)
        .hand
        .iter()
        .find(|card| card.name == "Shock")
        .unwrap()
        .id;
    act(&mut session, PlayerAction::CastSpell { card: shock });
    while matches!(session.pending(), Pending::ChooseCastMode { .. }) {
        act(&mut session, PlayerAction::ChooseMode(0));
    }
    assert!(matches!(session.pending(), Pending::ChooseTargets { .. }));
    assert_context_parity(&session);
    assert_eq!(
        session.observe_view_with_context(OWNER).casting,
        Some(shock)
    );
    assert!(session.observe_view_with_context(OWNER).targeting.is_some());
    act(
        &mut session,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![OTHER],
        },
    );
    assert_context_parity(&session);
    assert!(session.observe_view_with_context(OWNER).owed.is_some());
    act(&mut session, PlayerAction::CancelCast);
    assert!(session.observe_view_with_context(OWNER).casting.is_none());
    assert!(session.observe_view_with_context(OWNER).owed.is_none());
    assert!(
        session
            .observe_view(OWNER)
            .hand
            .iter()
            .any(|card| card.id == shock)
    );
}

fn assert_payment_view_compatibility(session: &Session, card: baylee_core::ids::ObjectId) {
    let legacy = session.observe_view_with_context(OWNER);
    let mut payment = session.observe_view_with_payment_context(OWNER);
    assert_eq!(payment.announced_cast, Some(card));
    assert!(payment.hand.iter().any(|hand| hand.id == card));
    assert!(legacy.announced_cast.is_none());
    assert!(
        !serde_json::to_value(&legacy)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("announced_cast")
    );
    payment.announced_cast = None;
    assert_eq!(
        serde_json::to_vec(&payment).unwrap(),
        serde_json::to_vec(&legacy).unwrap()
    );
    let (old_pending, old_deciding) = session.view_for_with_context(OWNER).unwrap();
    let (new_pending, mut new_deciding) = session.view_for_with_payment_context(OWNER).unwrap();
    assert_eq!(
        serde_json::to_vec(&old_pending).unwrap(),
        serde_json::to_vec(&new_pending).unwrap()
    );
    assert_eq!(new_deciding.announced_cast, Some(card));
    new_deciding.announced_cast = None;
    assert_eq!(
        serde_json::to_vec(&new_deciding).unwrap(),
        serde_json::to_vec(&old_deciding).unwrap()
    );
    assert!(session.observe_view(OWNER).announced_cast.is_none());
    assert!(socket_view(session, OWNER).announced_cast.is_none());
    assert!(
        session
            .observe_view_with_payment_context(OTHER)
            .announced_cast
            .is_none()
    );
    assert!(session.view_for_with_payment_context(OTHER).is_none());
}

#[test]
fn announced_spell_survives_nested_dual_land_color_choice_without_cancellation_right() {
    use baylee_core::mana::ManaColor;
    let mut session = session_with(
        &["Vitalizing Cascade"],
        &["Blossoming Sands", "Blossoming Sands"],
    );
    for _ in 0..64 {
        if session.observe_view(OWNER).phase == Phase::FirstMain {
            break;
        }
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected opening {other:?}"),
        };
        act(&mut session, action);
    }
    let card = session
        .observe_view(OWNER)
        .hand
        .iter()
        .find(|card| card.name == "Vitalizing Cascade")
        .unwrap()
        .id;
    act(&mut session, PlayerAction::CastSpell { card });
    while matches!(session.pending(), Pending::ChooseCastMode { .. }) {
        act(&mut session, PlayerAction::ChooseMode(0));
    }
    assert!(matches!(
        session.pending(),
        Pending::ChooseNumber {
            reason: baylee_engine::choice::NumberPrompt::X,
            ..
        }
    ));
    assert_payment_view_compatibility(&session, card);
    assert_eq!(session.observe_view_with_context(OWNER).casting, Some(card));
    act(&mut session, PlayerAction::ChooseNumber(0));
    assert_payment_view_compatibility(&session, card);
    for color in [ManaColor::White, ManaColor::Green] {
        let Pending::Priority { legal, .. } = session.pending() else {
            panic!("expected payment priority");
        };
        let (source, ability_index) = legal
            .abilities
            .iter()
            .copied()
            .find(|(id, _)| {
                session
                    .observe_view(OWNER)
                    .battlefield
                    .iter()
                    .any(|row| row.id == *id && row.name == "Blossoming Sands")
            })
            .expect("untapped dual-land mana ability");
        act(
            &mut session,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        );
        assert!(matches!(session.pending(), Pending::ChooseColor { .. }));
        assert_eq!(
            session
                .observe_view_with_announcement_context(OWNER)
                .announced_x,
            Some(0)
        );
        assert_eq!(
            session
                .observe_view_with_announcement_context(OTHER)
                .announced_x,
            None
        );
        assert_payment_view_compatibility(&session, card);
        assert!(session.observe_view_with_context(OWNER).casting.is_none());
        assert!(session.act(OWNER, PlayerAction::CancelCast).is_err());
        assert_payment_view_compatibility(&session, card);
        act(&mut session, PlayerAction::ChooseColor(color));
    }
    for _ in 0..4 {
        if session
            .observe_view_with_payment_context(OWNER)
            .announced_cast
            .is_none()
        {
            break;
        }
        act(&mut session, PlayerAction::PassPriority);
    }
    let view = session.observe_view_with_payment_context(OWNER);
    assert!(view.announced_cast.is_none());
    assert!(view.casting.is_none());
    assert!(view.stack.iter().any(|row| row.id == card));
}

mod announcement;
