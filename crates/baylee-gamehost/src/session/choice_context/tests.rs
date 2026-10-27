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
    let seats = (0..2)
        .map(|index| SeatSpec {
            controller: SeatController::Open,
            capabilities: baylee_core::preset::SeatCapabilities::default(),
            deck: vec![entry("Forest"); 32],
            sideboard: vec![],
            commanders: vec![],
            starting_life: Some(20),
            starting_hand: Some(vec![entry(if index == 0 {
                "Shock"
            } else {
                "Counterspell"
            })]),
            starting_battlefield: if index == 0 {
                vec![entry("Mountain")]
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
